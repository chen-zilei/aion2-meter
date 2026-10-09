//! Link layer → TCP segment. Handles the link types Npcap and tcpdump produce.

use etherparse::{NetSlice, SlicedPacket, TransportSlice};
use std::net::{IpAddr, SocketAddr};

/// pcap link types we understand (see tcpdump.org/linktypes.html).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Ethernet,
    /// BSD loopback / Npcap loopback adapter: 4-byte address family header.
    Null,
    /// Raw IPv4/IPv6, no link header.
    Raw,
    /// Linux "cooked" capture (tcpdump -i any).
    LinuxSll,
}

impl LinkType {
    pub fn from_pcap(code: u32) -> Option<Self> {
        match code {
            1 => Some(Self::Ethernet),
            0 | 108 => Some(Self::Null),
            12 | 101 | 228 | 229 => Some(Self::Raw),
            113 => Some(Self::LinuxSll),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FlowKey {
    pub src: SocketAddr,
    pub dst: SocketAddr,
}

#[derive(Debug)]
pub struct Segment<'a> {
    pub flow: FlowKey,
    pub seq: u32,
    pub syn: bool,
    pub fin: bool,
    pub rst: bool,
    pub payload: &'a [u8],
}

pub fn parse(link: LinkType, data: &[u8]) -> Option<Segment<'_>> {
    let sliced = match link {
        LinkType::Ethernet => SlicedPacket::from_ethernet(data).ok()?,
        LinkType::Null => SlicedPacket::from_ip(data.get(4..)?).ok()?,
        LinkType::Raw => SlicedPacket::from_ip(data).ok()?,
        LinkType::LinuxSll => SlicedPacket::from_linux_sll(data).ok()?,
    };
    let (src_ip, dst_ip): (IpAddr, IpAddr) = match sliced.net? {
        NetSlice::Ipv4(v4) => (v4.header().source_addr().into(), v4.header().destination_addr().into()),
        NetSlice::Ipv6(v6) => (v6.header().source_addr().into(), v6.header().destination_addr().into()),
        _ => return None,
    };
    let TransportSlice::Tcp(tcp) = sliced.transport? else { return None };
    Some(Segment {
        flow: FlowKey {
            src: SocketAddr::new(src_ip, tcp.source_port()),
            dst: SocketAddr::new(dst_ip, tcp.destination_port()),
        },
        seq: tcp.sequence_number(),
        syn: tcp.syn(),
        fin: tcp.fin(),
        rst: tcp.rst(),
        payload: tcp.payload(),
    })
}
