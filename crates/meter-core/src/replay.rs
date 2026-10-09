//! Reads saved `.pcap` / `.pcapng` captures so the parser can be developed offline.

use crate::net::LinkType;
use anyhow::{bail, Context, Result};
use pcap_file::pcap::PcapReader;
use pcap_file::pcapng::{Block, PcapNgReader};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

pub struct RawPacket {
    pub link: LinkType,
    pub t_ms: u64,
    pub data: Vec<u8>,
}

/// Calls `f` for every packet in the file, in file order.
pub fn for_each_packet(path: &Path, mut f: impl FnMut(RawPacket)) -> Result<()> {
    let mut file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    file.seek(SeekFrom::Start(0))?;
    let reader = BufReader::new(file);

    if magic == [0x0A, 0x0D, 0x0D, 0x0A] {
        let mut ng = PcapNgReader::new(reader)?;
        let mut links: Vec<Option<LinkType>> = Vec::new();
        while let Some(block) = ng.next_block() {
            match block? {
                Block::InterfaceDescription(idb) => links.push(LinkType::from_pcap(u32::from(idb.linktype))),
                Block::EnhancedPacket(epb) => {
                    if let Some(Some(link)) = links.get(epb.interface_id as usize) {
                        f(RawPacket { link: *link, t_ms: epb.timestamp.as_millis() as u64, data: epb.data.to_vec() });
                    }
                }
                _ => {}
            }
        }
        return Ok(());
    }

    let mut pcap = PcapReader::new(reader)?;
    let code = u32::from(pcap.header().datalink);
    let Some(link) = LinkType::from_pcap(code) else { bail!("unsupported pcap link type {code}") };
    while let Some(pkt) = pcap.next_packet() {
        let pkt = pkt?;
        f(RawPacket { link, t_ms: pkt.timestamp.as_millis() as u64, data: pkt.data.to_vec() });
    }
    Ok(())
}
