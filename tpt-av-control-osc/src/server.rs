//! OSC server: UDP reception with blocking and async loops.

use crate::bundle::OscPacket;
use crate::message::OscMessage;
use std::net::{SocketAddr, UdpSocket};
use tpt_av_control_utils::ControlError;

/// Maximum UDP payload we are willing to process (OSC packets are small;
/// 65507 is the practical IPv4 UDP limit).
pub const MAX_PACKET_SIZE: usize = 65_507;

type OscHandler = Box<dyn FnMut(OscMessage, SocketAddr) + Send>;

/// An OSC server bound to a UDP socket.
///
/// The handler runs on the receiving thread, which may allocate; hand
/// messages to the real-time thread through a lock-free queue (see
/// `tpt_av_control_utils::SpscRing`) if they reach an audio callback.
pub struct OscServer {
    socket: UdpSocket,
    handler: Option<OscHandler>,
}

impl OscServer {
    /// Creates a server listening on `0.0.0.0:port`.
    pub fn new(port: u16) -> Result<Self, ControlError> {
        Self::bind(SocketAddr::from(([0, 0, 0, 0], port)))
    }

    /// Creates a server bound to a specific address.
    pub fn bind(addr: SocketAddr) -> Result<Self, ControlError> {
        let socket = UdpSocket::bind(addr)?;
        Ok(Self { socket, handler: None })
    }

    /// The bound local address.
    pub fn local_addr(&self) -> Result<SocketAddr, ControlError> {
        Ok(self.socket.local_addr()?)
    }

    /// Sets the handler invoked for every received message. Passing a
    /// handler replaces any previous one.
    pub fn set_handler(&mut self, handler: impl FnMut(OscMessage, SocketAddr) + Send + 'static) {
        self.handler = Some(Box::new(handler));
    }

    /// Receives one UDP datagram and parses it. Blocking. Returns the
    /// parsed packet (message or bundle) and the sender's address. If a
    /// handler is set, it is invoked for every message contained in the
    /// packet.
    pub fn recv_packet(&mut self) -> Result<(OscPacket, SocketAddr), ControlError> {
        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        let (len, src) = self.socket.recv_from(&mut buf)?;
        let packet = crate::bundle::parse_packet(&buf[..len])
            .map_err(|e| ControlError::InvalidData(format!("invalid OSC packet: {e}")))?;
        if let Some(handler) = self.handler.as_mut() {
            deliver(handler, &packet, src);
        }
        Ok((packet, src))
    }

    /// Receives messages until the socket fails. Blocking.
    pub fn run(&mut self) -> Result<(), ControlError> {
        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let (len, src) = self.socket.recv_from(&mut buf)?;
            self.dispatch_bytes(&buf[..len], src)?;
        }
    }

    /// Receives messages until the socket fails, using tokio's async I/O.
    pub async fn run_async(&mut self) -> Result<(), ControlError> {
        let std_socket = self.socket.try_clone()?;
        std_socket.set_nonblocking(true)?;
        let socket = tokio::net::UdpSocket::from_std(std_socket)?;
        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let (len, src) = socket.recv_from(&mut buf).await?;
            self.dispatch_bytes(&buf[..len], src)?;
        }
    }

    /// Parses `data` and invokes the handler for every message contained
    /// (expanding bundles in order). Bundle time tags are informational:
    /// packets are delivered immediately.
    pub fn dispatch_bytes(&mut self, data: &[u8], src: SocketAddr) -> Result<(), ControlError> {
        let packet = crate::bundle::parse_packet(data)?;
        if let Some(handler) = self.handler.as_mut() {
            deliver(handler, &packet, src);
        }
        Ok(())
    }

    /// Like [`OscServer::dispatch_bytes`] but returns parsed messages
    /// instead of invoking a handler.
    pub fn parse_bytes(data: &[u8]) -> Result<Vec<OscMessage>, ControlError> {
        let packet = crate::bundle::parse_packet(data)?;
        let mut out = Vec::new();
        collect_messages(&packet, &mut out);
        Ok(out)
    }
}

fn deliver(handler: &mut OscHandler, packet: &OscPacket, src: SocketAddr) {
    match packet {
        OscPacket::Message(m) => handler(m.clone(), src),
        OscPacket::Bundle(b) => {
            for element in &b.elements {
                deliver(handler, element, src);
            }
        }
    }
}

fn collect_messages(packet: &OscPacket, out: &mut Vec<OscMessage>) {
    match packet {
        OscPacket::Message(m) => out.push(m.clone()),
        OscPacket::Bundle(b) => {
            for element in &b.elements {
                collect_messages(element, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{OscBundle, OscPacket};
    use crate::message::OscArg;
    use std::sync::mpsc;

    #[test]
    fn loopback_blocking_roundtrip() {
        // Bind to loopback so clients can target the reported local address
        // (on Windows, sending to 0.0.0.0 fails with AddrNotAvailable).
        let mut server = OscServer::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        let addr = server.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        server.set_handler(move |msg, _src| {
            tx.send(msg).unwrap();
        });
        let t = std::thread::spawn(move || {
            // One receive is enough for the test.
            let _ = server.recv_packet();
            let _ = server.recv_packet();
        });

        let mut client = crate::client::OscClient::new(addr).unwrap();
        client
            .send(&OscMessage::new("/test/a", &[OscArg::Int(7)]).unwrap())
            .unwrap();
        client
            .send_bundle(&crate::bundle::OscBundle::new(
                None,
                vec![crate::bundle::OscPacket::Message(
                    OscMessage::new("/test/b", &[OscArg::Float(0.25)]).unwrap(),
                )],
            ))
            .unwrap();

        let a = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert_eq!(a.address, "/test/a");
        assert_eq!(a.arguments, vec![OscArg::Int(7)]);
        let b = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert_eq!(b.address, "/test/b");
        t.join().unwrap();
    }

    #[tokio::test]
    async fn loopback_async_roundtrip() {
        let mut server =
            OscServer::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        let addr = server.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        server.set_handler(move |msg, _src| {
            tx.send(msg).unwrap();
        });

        let handle = tokio::spawn(async move {
            // Run the server until the test finishes (or 10 s pass).
            if let Ok(Err(e)) = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                server.run_async(),
            )
            .await
            {
                eprintln!("run_async failed: {e}");
            }
        });

        // Give the async task a moment to start.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let mut client = crate::client::OscClient::new(addr).unwrap();
        client
            .send(&OscMessage::new("/async", &[OscArg::String("hi".into())]).unwrap())
            .unwrap();

        // Receive on a blocking thread: blocking the runtime thread would
        // stall the I/O driver on a current_thread runtime.
        let got = tokio::task::spawn_blocking(move || {
            rx.recv_timeout(std::time::Duration::from_secs(5))
                .expect("message should arrive via async server")
        })
        .await
        .expect("recv thread");
        assert_eq!(got.address, "/async");
        assert_eq!(got.arguments, vec![OscArg::String("hi".into())]);
        handle.abort();
    }

    #[test]
    fn deliver_bundles_recursively() {
        let mut server = OscServer::new(0).unwrap();
        let (tx, rx) = mpsc::channel();
        server.set_handler(move |msg, _src| {
            tx.send(msg.address).unwrap();
        });
        let nested = OscPacket::Bundle(OscBundle::new(
            None,
            vec![OscPacket::Message(OscMessage::new("/deep", &[]).unwrap())],
        ));
        let bundle = OscBundle::new(
            None,
            vec![
                OscPacket::Message(OscMessage::new("/one", &[]).unwrap()),
                nested,
            ],
        );
        let addr: SocketAddr = "127.0.0.1:1".parse().unwrap();
        server
            .dispatch_bytes(&bundle.encode(), addr)
            .unwrap();
        assert_eq!(rx.recv().unwrap(), "/one");
        assert_eq!(rx.recv().unwrap(), "/deep");
    }
}
