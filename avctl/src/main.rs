//! `avctl` — a small command-line tool for sending and monitoring OSC,
//! MIDI, and DMX traffic on-site. This is a debugging aid, not a
//! production show-control tool: it has no persistence, scripting, or
//! scheduling. See `avctl --help` for the full command list.

use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::time::Duration;
use tpt_av_control_dmx::{ArtNet, DmxUniverse, Sacn};
use tpt_av_control_midi::{enumerate_devices, open_input, MidiPort, PortDirection, PortId};
use tpt_av_control_osc::{OscArg, OscClient, OscMessage, OscServer};

#[derive(Parser)]
#[command(name = "avctl", version, about = "Debug OSC, MIDI, and DMX traffic")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// OSC send/monitor.
    #[command(subcommand)]
    Osc(OscCommand),
    /// MIDI device listing/monitoring.
    #[command(subcommand)]
    Midi(MidiCommand),
    /// DMX/Art-Net/sACN universe sniffing.
    #[command(subcommand)]
    Dmx(DmxCommand),
}

#[derive(Subcommand)]
enum OscCommand {
    /// Sends one OSC message.
    Send {
        /// Target address, e.g. 127.0.0.1:9000.
        target: SocketAddr,
        /// OSC address, e.g. /track/1/volume.
        address: String,
        /// Arguments as type:value (i:int, f:float, s:string), e.g. f:0.75.
        args: Vec<String>,
    },
    /// Binds a UDP OSC server and prints every received message.
    Monitor {
        /// Address to bind, e.g. 0.0.0.0:9000.
        bind: SocketAddr,
    },
}

#[derive(Subcommand)]
enum MidiCommand {
    /// Lists MIDI devices and their input/output ports.
    List,
    /// Opens an input port by index (see `avctl midi list`) and prints
    /// every decoded message.
    Monitor {
        /// Input port index, as shown by `avctl midi list`.
        port_index: usize,
    },
}

#[derive(Subcommand)]
enum DmxCommand {
    /// Receives universes over Art-Net or sACN and prints changed channels.
    Sniff {
        /// Protocol to listen for.
        #[arg(long, value_enum)]
        protocol: DmxProtocolArg,
        /// UDP port to bind (use the protocol's standard port if unsure:
        /// 6454 for Art-Net, 5568 for sACN).
        port: u16,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum DmxProtocolArg {
    Artnet,
    Sacn,
}

fn parse_arg(spec: &str) -> Result<OscArg, String> {
    let (tag, value) = spec
        .split_once(':')
        .ok_or_else(|| format!("argument {spec:?} must be type:value (e.g. f:0.75)"))?;
    match tag {
        "i" => value
            .parse::<i32>()
            .map(OscArg::Int)
            .map_err(|e| e.to_string()),
        "f" => value
            .parse::<f32>()
            .map(OscArg::Float)
            .map_err(|e| e.to_string()),
        "s" => Ok(OscArg::String(value.to_string())),
        other => Err(format!("unknown argument type {other:?} (use i, f, or s)")),
    }
}

fn run_osc(cmd: OscCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        OscCommand::Send {
            target,
            address,
            args,
        } => {
            let parsed_args = args
                .iter()
                .map(|a| parse_arg(a))
                .collect::<Result<Vec<_>, _>>()?;
            let mut client = OscClient::new(target)?;
            let message = OscMessage::new(address, &parsed_args)?;
            let bytes = client.send(&message)?;
            println!("sent {bytes} bytes to {target}");
        }
        OscCommand::Monitor { bind } => {
            let mut server = OscServer::bind(bind)?;
            println!("listening for OSC on {}", server.local_addr()?);
            server.set_handler(|msg, src| {
                println!(
                    "{src}  {}  {}",
                    msg.address,
                    msg.arguments
                        .iter()
                        .map(|a| format!("{a:?}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            });
            server.run()?;
        }
    }
    Ok(())
}

fn run_midi(cmd: MidiCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        MidiCommand::List => {
            let devices = enumerate_devices()?;
            if devices.is_empty() {
                println!("no MIDI devices found");
            }
            for device in &devices {
                println!("{}", device.name);
                for port in &device.inputs {
                    println!("  in  [{}] {}", port.id.index, port.name);
                }
                for port in &device.outputs {
                    println!("  out [{}] {}", port.id.index, port.name);
                }
            }
        }
        MidiCommand::Monitor { port_index } => {
            let mut input = open_input(&MidiPort::new(
                PortId::new(port_index),
                format!("input {port_index}"),
                PortDirection::Input,
            ))?;
            println!("listening on input {port_index}; Ctrl-C to quit");
            loop {
                match input.recv() {
                    Ok(inbound) => match &inbound.message {
                        Some(message) => println!("{message:?}"),
                        None => println!("(unparsed) {:02x?}", inbound.raw),
                    },
                    Err(e) => {
                        eprintln!("input closed: {e}");
                        std::thread::sleep(Duration::from_secs(1));
                    }
                }
            }
        }
    }
    Ok(())
}

enum DmxNode {
    ArtNet(ArtNet),
    Sacn(Sacn),
}

impl DmxNode {
    fn recv_universe(&mut self) -> Result<DmxUniverse, tpt_av_control_dmx::ControlError> {
        match self {
            DmxNode::ArtNet(node) => node.recv_universe(),
            DmxNode::Sacn(node) => node.recv_universe(),
        }
    }
}

fn run_dmx(cmd: DmxCommand) -> Result<(), Box<dyn std::error::Error>> {
    let DmxCommand::Sniff { protocol, port } = cmd;
    let mut node = match protocol {
        DmxProtocolArg::Artnet => DmxNode::ArtNet(ArtNet::new(port)?),
        DmxProtocolArg::Sacn => DmxNode::Sacn(Sacn::new(port)?),
    };
    let mut last = DmxUniverse::new(0);
    let mut have_last = false;
    println!("listening for {} on port {port}", protocol_name(&protocol));
    loop {
        let universe = node.recv_universe()?;
        if have_last && last.universe == universe.universe {
            let changed: Vec<(u16, u8)> = (0..512u16)
                .filter(|&ch| last.get_channel(ch) != universe.get_channel(ch))
                .map(|ch| (ch, universe.get_channel(ch)))
                .collect();
            if !changed.is_empty() {
                println!("universe {}: {changed:?}", universe.universe);
            }
        } else {
            println!("universe {}: initial frame received", universe.universe);
        }
        last = universe;
        have_last = true;
    }
}

fn protocol_name(protocol: &DmxProtocolArg) -> &'static str {
    match protocol {
        DmxProtocolArg::Artnet => "Art-Net",
        DmxProtocolArg::Sacn => "sACN",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Osc(cmd) => run_osc(cmd),
        Command::Midi(cmd) => run_midi(cmd),
        Command::Dmx(cmd) => run_dmx(cmd),
    }
}
