//! End-to-end pipeline demo: network thread → lock-free SPSC ring →
//! real-time consumer thread, as described in DESIGN.md §5.3.
//!
//! An OSC server runs on its own thread (allocating, parsing OSC), hands
//! each message over a bounded `MessageQueue` (`SpscRing`), and a
//! stand-in for the audio/video thread pops messages without allocating
//! or blocking.
//!
//! ```sh
//! cargo run -p tpt-av-control-examples --bin spsc_pipeline
//! # then: any OSC app → udp://127.0.0.1:<printed port>
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};
use tpt_av_control_osc::OscServer;
use tpt_av_control_utils::{
    Message, MessageBody, MessageQueue, MessageSource, ParameterId, ParameterValue,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let run_for = Duration::from_secs(
        std::env::args()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(15),
    );

    // The only object shared between threads: a bounded lock-free queue.
    let queue: Arc<MessageQueue> = Arc::new(MessageQueue::new(1024));

    // Network thread: blocking OSC server, may allocate.
    let mut server = OscServer::new(0)?;
    println!("OSC input on {}", server.local_addr()?);
    let producer = Arc::clone(&queue);
    let network = std::thread::spawn(move || {
        let mut next_id = 0u64;
        loop {
            match server.recv_packet() {
                Ok((packet, src)) => {
                    let messages = OscServer::parse_bytes(&match &packet {
                        tpt_av_control_osc::OscPacket::Message(m) => m.encode(),
                        tpt_av_control_osc::OscPacket::Bundle(b) => b.encode(),
                    })
                    .unwrap_or_default();
                    for msg in messages {
                        // Flatten any first numeric argument into a
                        // parameter change keyed by the OSC address.
                        let body = msg
                            .arguments
                            .first()
                            .and_then(|a| a.as_f32())
                            .map(|v| MessageBody::Parameter {
                                id: ParameterId::new(msg.address.trim_start_matches('/')),
                                value: ParameterValue::Float(v),
                            })
                            .unwrap_or(MessageBody::Text(msg.address));
                        let message = Message::now(next_id, MessageSource::Network(src), body);
                        next_id += 1;
                        // Drop policy: on overflow we drop oldest-equivalent
                        // (here: just the new message) and count it.
                        if producer.push(message).is_err() {
                            eprintln!("queue full; dropping message");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("recv error: {e}");
                }
            }
        }
    });

    // Real-time thread stand-in: pop() never allocates, locks, or blocks.
    let consumer = Arc::clone(&queue);
    let rt = std::thread::spawn(move || {
        let deadline = Instant::now() + run_for;
        let mut applied = 0u64;
        while Instant::now() < deadline {
            match consumer.pop() {
                Some(Message { body, .. }) => {
                    applied += 1;
                    match body {
                        MessageBody::Parameter { id, value } => {
                            println!("[RT] {id} = {value:?}");
                        }
                        MessageBody::Text(text) => println!("[RT] text: {text}"),
                        other => println!("[RT] {other:?}"),
                    }
                }
                None => std::thread::sleep(Duration::from_millis(1)),
            }
        }
        applied
    });

    let applied = rt.join().expect("rt thread");
    println!("applied {applied} messages in {run_for:?}");
    // Detach the network thread; the process exits here.
    drop(network);
    Ok(())
}
