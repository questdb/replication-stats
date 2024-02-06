mod literal_bytes;
mod writer;

use crate::literal_bytes::LiteralBytes;
use crate::writer::Record;
use etherparse::{InternetSlice, SlicedPacket, TcpHeaderSlice, TransportSlice};
use pcap::{Capture, Device, Packet};
use std::collections::HashSet;
use std::fmt::Debug;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;

/// Capture time series for TCP IPv4 traffic to specific ports
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CommandArgs {
    /// Destination directory for output files
    #[clap(short, long, default_value = "data")]
    dir: PathBuf,

    /// List of destination ports to monitor
    /// At least one port must be specified
    #[clap(required = true)]
    ports: Vec<u16>,

    /// Verbosity level
    /// 0: silent
    /// 1: print a dot for each packet received
    /// 2: print packet details
    /// 3: print packet flags and zero-data packets
    /// 4: also print sent data
    #[clap(short, long, default_value_t = 0)]
    verbosity: u8,
}

fn get_loopback_device() -> anyhow::Result<Device> {
    for device in Device::list()? {
        let if_flags = &device.flags.if_flags;
        let is_loopback = if_flags.contains(pcap::IfFlags::UP)
            && if_flags.contains(pcap::IfFlags::RUNNING)
            && if_flags.contains(pcap::IfFlags::LOOPBACK);
        if is_loopback {
            return Ok(device);
        }
    }
    Err(anyhow::anyhow!("No loopback device found"))
}

fn ignore_timeouts(result: Result<Packet, pcap::Error>) -> Result<Option<Packet>, pcap::Error> {
    match result {
        Ok(packet) => Ok(Some(packet)),
        Err(pcap::Error::TimeoutExpired) => Ok(None),
        Err(error) => Err(error),
    }
}

#[derive(Debug)]
struct Addr {
    ip: Ipv4Addr,
    port: u16,
}

#[derive(Debug)]
struct TcpData {
    src: Addr,
    dest: Addr,
    data_offset: usize,
    flags: TcpMeta,
    ts: SystemTime,
}

struct TcpMeta {
    ns: bool,
    cwr: bool,
    ece: bool,
    urg: bool,
    ack: bool,
    psh: bool,
    rst: bool,
    syn: bool,
    fin: bool,
    seq: u32,
}

impl TcpMeta {
    fn from_tcp_header(tcp: &TcpHeaderSlice) -> Self {
        Self {
            ns: tcp.ns(),
            cwr: tcp.cwr(),
            ece: tcp.ece(),
            urg: tcp.urg(),
            ack: tcp.ack(),
            psh: tcp.psh(),
            rst: tcp.rst(),
            syn: tcp.syn(),
            fin: tcp.fin(),
            seq: tcp.sequence_number(),
        }
    }
}

impl Debug for TcpMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let flags = [
            (self.ns, "NS"),
            (self.cwr, "CWR"),
            (self.ece, "ECE"),
            (self.urg, "URG"),
            (self.ack, "ACK"),
            (self.psh, "PSH"),
            (self.rst, "RST"),
            (self.syn, "SYN"),
            (self.fin, "FIN"),
        ];
        for (_, label) in flags.iter().filter(|(is_set, _)| *is_set) {
            write!(f, "{} ", label)?;
        }
        write!(f, "[{}]", self.seq)
    }
}

fn to_system_time(ts: libc::timeval) -> SystemTime {
    UNIX_EPOCH + std::time::Duration::new(ts.tv_sec as u64, ts.tv_usec as u32 * 1000)
}

fn parse_tcp(packet: &Packet) -> anyhow::Result<Option<TcpData>> {
    // Note that we're parsing on LINKTYPE_NULL, i.e. loopback, not ethernet.
    if packet.header.caplen < 32 {
        return Ok(None);
    }
    let ipv4data = &packet.data[4..]; // skip the loopback header
    let sliced = SlicedPacket::from_ip(ipv4data)?;
    let Some(InternetSlice::Ipv4(ipv4slice, _)) = sliced.ip else {
        return Ok(None);
    };
    let src_addr = ipv4slice.source_addr();
    let dest_addr = ipv4slice.destination_addr();
    let TransportSlice::Tcp(tcp) = sliced.transport.unwrap() else {
        return Ok(None);
    };
    let src_port = tcp.source_port();
    let dest_port = tcp.destination_port();

    let link_bytes_len = 4;
    let data_offset = link_bytes_len + ((ipv4slice.ihl() * 4) + (tcp.data_offset() * 4)) as usize;
    let tcp_data = TcpData {
        ts: to_system_time(packet.header.ts),
        src: Addr {
            ip: src_addr,
            port: src_port,
        },
        dest: Addr {
            ip: dest_addr,
            port: dest_port,
        },
        data_offset,
        flags: TcpMeta::from_tcp_header(&tcp),
    };
    Ok(Some(tcp_data))
}

fn print_tcp(tcp_data: TcpData, data_len: u64, data_part: &[u8], is_recv: bool, verbosity: u8) {
    if !is_recv && verbosity < 4 {
        return;
    }
    if data_len > 0 {
        if verbosity == 1 {
            print!(".");
        }
        if verbosity >= 2 {
            println!(
                "{} [{}:{} -> {}:{}] {:?}: {}",
                if is_recv { "recv" } else { "sent" },
                tcp_data.src.ip,
                tcp_data.src.port,
                tcp_data.dest.ip,
                tcp_data.dest.port,
                data_len,
                LiteralBytes(data_part)
            );
        }
        if verbosity >= 3 {
            println!("     flags: {:?}", tcp_data.flags);
        }
    } else if verbosity >= 3 {
        println!(
            "{} [{}:{} -> {}:{}] flags: {:?}",
            if is_recv { "recv" } else { "sent" },
            tcp_data.src.ip,
            tcp_data.src.port,
            tcp_data.dest.ip,
            tcp_data.dest.port,
            tcp_data.flags
        );
    }
}

fn main() -> anyhow::Result<()> {
    let CommandArgs {
        dir,
        ports,
        verbosity,
    } = CommandArgs::parse();
    let ports: HashSet<_> = ports.into_iter().collect();
    let writer_queue = writer::Writer::run(dir);
    let device = get_loopback_device()?;
    let mut cap = Capture::from_device(device)?
        .promisc(false)
        .snaplen(128)
        .timeout(1)
        .buffer_size(4 * 1024 * 1024)
        .open()?;
    cap.filter("tcp", true)?;
    loop {
        let Some(packet) = ignore_timeouts(cap.next_packet())? else {
            continue;
        };
        let Some(tcp_data) = parse_tcp(&packet)? else {
            continue;
        };
        let data_len = packet.header.len as u64 - tcp_data.data_offset as u64;
        let data_part = &packet.data[tcp_data.data_offset..];
        if ports.contains(&tcp_data.dest.port) {
            if data_len > 0 {
                writer_queue.send(Record {
                    port: tcp_data.dest.port,
                    ts: tcp_data.ts,
                    val: data_len,
                })?;
            }
            print_tcp(tcp_data, data_len, data_part, true, verbosity);
        } else if ports.contains(&tcp_data.src.port) && verbosity >= 4 {
            print_tcp(tcp_data, data_len, data_part, false, verbosity);
        }
    }
}
