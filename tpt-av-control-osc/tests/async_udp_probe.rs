use std::net::{SocketAddr, UdpSocket};
#[tokio::test]
async fn clone_inside_spawned_task() {
    let srv = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
    let addr = srv.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let std_srv = srv.try_clone().unwrap();
        std_srv.set_nonblocking(true).unwrap();
        eprintln!("task: converting socket");
        let tokio_srv = tokio::net::UdpSocket::from_std(std_srv).unwrap();
        eprintln!("task: awaiting recv");
        let mut buf = [0u8; 64];
        let (len, _src) = tokio_srv.recv_from(&mut buf).await.unwrap();
        eprintln!("task: got {len} bytes");
        len
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let client = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
    client.send_to(b"hello", addr).unwrap();
    eprintln!("main: sent");
    let got = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
    eprintln!("main: result {got:?}");
    assert!(got.is_ok());
}
