//! Reading pcap and pcapng files into packets, UDP datagrams, and reassembled
//! TCP flows.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, bail};
use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use pcap_file::DataLink;
use pcap_file::pcap::PcapReader;
use pcap_file::pcapng::{Block, PcapNgReader};

/// One captured frame with its link-layer type.
#[derive(Debug, Clone)]
pub struct Frame {
    /// 1-based, as Wireshark numbers them.
    pub number: usize,
    pub ts: Option<Duration>,
    pub data: Vec<u8>,
    pub link: DataLink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TcpFlags {
    pub syn: bool,
    pub fin: bool,
    pub rst: bool,
}

#[derive(Debug, Clone)]
pub enum Transport {
    Tcp {
        seq: u32,
        flags: TcpFlags,
        payload: Vec<u8>,
    },
    Udp {
        payload: Vec<u8>,
    },
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub frame: usize,
    pub src: SocketAddr,
    pub dst: SocketAddr,
    pub transport: Transport,
}

/// Reads every frame of a pcap or pcapng file.
pub fn read_capture(path: &Path) -> anyhow::Result<Vec<Frame>> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if bytes.len() >= 4 && bytes[..4] == [0x0a, 0x0d, 0x0d, 0x0a] {
        read_pcapng(&bytes)
    } else {
        read_pcap(&bytes)
    }
}

fn read_pcap(bytes: &[u8]) -> anyhow::Result<Vec<Frame>> {
    let mut reader = PcapReader::new(bytes).context("pcap header")?;
    let link = reader.header().datalink;
    let mut frames = Vec::new();
    while let Some(pkt) = reader.next_packet() {
        let pkt = pkt.context("pcap packet")?;
        frames.push(Frame {
            number: frames.len() + 1,
            ts: Some(pkt.timestamp),
            data: pkt.data.into_owned(),
            link,
        });
    }
    Ok(frames)
}

fn read_pcapng(bytes: &[u8]) -> anyhow::Result<Vec<Frame>> {
    let mut reader = PcapNgReader::new(bytes).context("pcapng header")?;
    // (interface id, timestamp, data); link types are looked up once all
    // interface blocks have been read.
    let mut raw: Vec<(Option<usize>, Option<Duration>, Vec<u8>)> = Vec::new();
    while let Some(block) = reader.next_block() {
        let block = block.context("pcapng block")?;
        let item = match block {
            Block::EnhancedPacket(p) => (
                Some(p.interface_id as usize),
                Some(p.timestamp),
                p.data.into_owned(),
            ),
            Block::SimplePacket(p) => (None, None, p.data.into_owned()),
            // The obsolete Packet block: its timestamp units are per interface; left out.
            Block::Packet(p) => (Some(usize::from(p.interface_id)), None, p.data.into_owned()),
            _ => continue,
        };
        raw.push(item);
    }
    let interfaces = reader.interfaces();
    if interfaces.is_empty() {
        bail!("pcapng file has no interfaces");
    }
    Ok(raw
        .into_iter()
        .enumerate()
        .map(|(i, (iface, ts, data))| Frame {
            number: i + 1,
            ts,
            data,
            link: interfaces
                .get(iface.unwrap_or(0))
                .map(|i| i.linktype)
                .unwrap_or(DataLink::ETHERNET),
        })
        .collect())
}

/// Parses the TCP and UDP packets out of the frames; other frames are skipped.
pub fn parse_packets(frames: &[Frame]) -> Vec<Packet> {
    frames.iter().filter_map(parse_frame).collect()
}

fn parse_frame(frame: &Frame) -> Option<Packet> {
    let data = &frame.data;
    let sliced = match frame.link {
        DataLink::ETHERNET => SlicedPacket::from_ethernet(data).ok()?,
        DataLink::LINUX_SLL => SlicedPacket::from_linux_sll(data).ok()?,
        DataLink::LINUX_SLL2 => {
            // 20-byte header; the protocol type is the first u16.
            if data.len() < 20 {
                return None;
            }
            let ether_type = u16::from_be_bytes([data[0], data[1]]);
            SlicedPacket::from_ether_type(ether_type.into(), &data[20..]).ok()?
        }
        DataLink::NULL | DataLink::LOOP => {
            if data.len() < 4 {
                return None;
            }
            // BSD loopback: a 4-byte AF_* in host order; macOS writes it little-endian.
            SlicedPacket::from_ip(&data[4..]).ok()?
        }
        DataLink::RAW | DataLink::IPV4 | DataLink::IPV6 => SlicedPacket::from_ip(data).ok()?,
        _ => return None,
    };
    let (src_ip, dst_ip): (IpAddr, IpAddr) = match sliced.net.as_ref()? {
        NetSlice::Ipv4(v4) => (
            v4.header().source_addr().into(),
            v4.header().destination_addr().into(),
        ),
        NetSlice::Ipv6(v6) => (
            v6.header().source_addr().into(),
            v6.header().destination_addr().into(),
        ),
        _ => return None,
    };
    let (src_port, dst_port, transport) = match sliced.transport.as_ref()? {
        TransportSlice::Tcp(tcp) => (
            tcp.source_port(),
            tcp.destination_port(),
            Transport::Tcp {
                seq: tcp.sequence_number(),
                flags: TcpFlags {
                    syn: tcp.syn(),
                    fin: tcp.fin(),
                    rst: tcp.rst(),
                },
                payload: tcp.payload().to_vec(),
            },
        ),
        TransportSlice::Udp(udp) => (
            udp.source_port(),
            udp.destination_port(),
            Transport::Udp {
                payload: udp.payload().to_vec(),
            },
        ),
        _ => return None,
    };
    Some(Packet {
        frame: frame.number,
        src: SocketAddr::new(src_ip, src_port),
        dst: SocketAddr::new(dst_ip, dst_port),
        transport,
    })
}

/// One direction of a TCP connection, reassembled.
#[derive(Debug, Clone, Default)]
pub struct Stream {
    pub data: Vec<u8>,
    /// `(offset into data, frame number)` for each segment that added bytes.
    pub segments: Vec<(usize, usize)>,
    /// A FIN or RST was seen in this direction: the sender closed.
    pub closed: bool,
}

impl Stream {
    /// The frame that carried the byte at `offset`.
    pub fn frame_at(&self, offset: usize) -> usize {
        self.segments
            .iter()
            .take_while(|(o, _)| *o <= offset)
            .last()
            .or(self.segments.first())
            .map(|(_, f)| *f)
            .unwrap_or(0)
    }
}

/// A TCP connection: the side that connected to a server port is the client.
#[derive(Debug, Clone)]
pub struct Flow {
    pub client: SocketAddr,
    pub server: SocketAddr,
    pub c2s: Stream,
    pub s2c: Stream,
    pub first_frame: usize,
}

#[derive(Default)]
struct Reassembly {
    isn: Option<u32>,
    segments: BTreeMap<u32, (usize, Vec<u8>)>,
    closed: bool,
}

impl Reassembly {
    fn add(&mut self, seq: u32, flags: TcpFlags, payload: &[u8], frame: usize) {
        if flags.syn {
            self.isn = Some(seq.wrapping_add(1));
        } else if self.isn.is_none() {
            self.isn = Some(seq);
        }
        if flags.fin || flags.rst {
            self.closed = true;
        }
        if payload.is_empty() {
            return;
        }
        let rel = seq.wrapping_sub(self.isn.unwrap_or(seq));
        self.segments
            .entry(rel)
            .or_insert_with(|| (frame, payload.to_vec()));
    }

    fn finish(self) -> Stream {
        let mut stream = Stream {
            closed: self.closed,
            ..Stream::default()
        };
        for (rel, (frame, payload)) in self.segments {
            let rel = rel as usize;
            if rel < stream.data.len() {
                // retransmission or overlap: keep only the new tail
                let skip = stream.data.len() - rel;
                if skip >= payload.len() {
                    continue;
                }
                stream.segments.push((stream.data.len(), frame));
                stream.data.extend_from_slice(&payload[skip..]);
            } else {
                stream.segments.push((stream.data.len(), frame));
                stream.data.extend_from_slice(&payload);
            }
        }
        stream
    }
}

/// Groups TCP packets into flows. `is_server_port` tells which end is the server;
/// a flow whose ends are both or neither is skipped.
pub fn tcp_flows(packets: &[Packet], is_server_port: impl Fn(u16) -> bool) -> Vec<Flow> {
    struct Pending {
        client: SocketAddr,
        server: SocketAddr,
        c2s: Reassembly,
        s2c: Reassembly,
        first_frame: usize,
    }
    let mut flows: Vec<Pending> = Vec::new();
    for p in packets {
        let Transport::Tcp {
            seq,
            flags,
            payload,
        } = &p.transport
        else {
            continue;
        };
        let (client, server) = match (is_server_port(p.dst.port()), is_server_port(p.src.port())) {
            (true, false) => (p.src, p.dst),
            (false, true) => (p.dst, p.src),
            _ => continue,
        };
        let idx = match flows
            .iter()
            .position(|f| f.client == client && f.server == server)
        {
            Some(i) => i,
            None => {
                flows.push(Pending {
                    client,
                    server,
                    c2s: Reassembly::default(),
                    s2c: Reassembly::default(),
                    first_frame: p.frame,
                });
                flows.len() - 1
            }
        };
        let f = &mut flows[idx];
        if p.src == client {
            f.c2s.add(*seq, *flags, payload, p.frame);
        } else {
            f.s2c.add(*seq, *flags, payload, p.frame);
        }
    }
    flows
        .into_iter()
        .map(|f| Flow {
            client: f.client,
            server: f.server,
            c2s: f.c2s.finish(),
            s2c: f.s2c.finish(),
            first_frame: f.first_frame,
        })
        .collect()
}

/// A UDP datagram sent to `port`.
#[derive(Debug, Clone)]
pub struct Datagram {
    pub frame: usize,
    pub src: SocketAddr,
    pub dst: SocketAddr,
    pub payload: Vec<u8>,
}

pub fn udp_datagrams(packets: &[Packet], port: u16) -> Vec<Datagram> {
    packets
        .iter()
        .filter_map(|p| match &p.transport {
            Transport::Udp { payload } if p.dst.port() == port => Some(Datagram {
                frame: p.frame,
                src: p.src,
                dst: p.dst,
                payload: payload.clone(),
            }),
            _ => None,
        })
        .collect()
}
