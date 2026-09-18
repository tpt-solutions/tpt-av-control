//! OSC server demo: receives OSC messages over UDP and prints them.
//!
//! Send test messages from any OSC app targeting `udp://127.0.0.1:8000`.

use std::net::SocketAddr;
use tpt_av_control_osc::{OscArg, OscDispatcher, OscMessage, OscServer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let port: u16 = std::env::args()
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);

    let mut server = OscServer::new(port)?;
    println!("OSC server listening on {}", server.local_addr()?);

    // Route volume controls by pattern, print everything else raw.
    let mut dispatcher = OscDispatcher::new();
    dispatcher.add_route("/track/*/volume", |msg: OscMessage| {
        let value = msg.arguments.first().and_then(OscArg::as_f32);
        println!("volume {}: {value:?}", msg.address);
    });

    server.set_handler(move |msg: OscMessage, src: SocketAddr| {
        let handled = dispatcher.dispatch(msg.clone());
        if handled == 0 {
            print_raw(msg, src);
        }
    });

    server.run()?;
    Ok(())
}

fn print_raw(msg: OscMessage, src: SocketAddr) {
    println!("{src} {} {}", msg.address, msg.type_tags());
    for (i, arg) in msg.arguments.iter().enumerate() {
        match arg {
            OscArg::Int(v) => println!("  [{i}] int {v}"),
            OscArg::Float(v) => println!("  [{i}] float {v}"),
            OscArg::String(v) => println!("  [{i}] string {v:?}"),
            OscArg::Blob(v) => println!("  [{i}] blob {} bytes", v.len()),
            other => println!("  [{i}] tag {}", other.type_tag()),
        }
    }
}
