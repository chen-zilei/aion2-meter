//! Saves captured game traffic as `.pcap` files, so a fight the meter got wrong can be decoded again after a fix.
//!
//! The files open in Wireshark and in the `replay` tool. Each file is capped in size, and the oldest files in the
//! folder are deleted once the folder holds more than [`MAX_TOTAL_BYTES`].

use pcap_file::pcap::{PcapHeader, PcapPacket, PcapWriter};
use pcap_file::DataLink;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A new file is started past this size, so one long session doesn't become one huge file.
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
/// Recordings past this total are deleted, oldest first.
pub const MAX_TOTAL_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Writes one adapter's packets. Files are named by start time (UTC) and `tag`, so they sort oldest first.
pub struct Recorder {
    dir: PathBuf,
    tag: String,
    link: u32,
    file: Option<(PcapWriter<File>, u64)>,
}

impl Recorder {
    /// `link` is the pcap link type code of the packets that will be written.
    pub fn new(dir: impl Into<PathBuf>, tag: &str, link: u32) -> Self {
        let tag = tag.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
        Self { dir: dir.into(), tag, link, file: None }
    }

    /// Appends one packet. `t_us` is the capture time in microseconds since the Unix epoch.
    pub fn write(&mut self, t_us: u64, orig_len: u32, data: &[u8]) -> io::Result<()> {
        if self.file.as_ref().is_some_and(|(_, bytes)| *bytes >= MAX_FILE_BYTES) {
            self.file = None;
        }
        if self.file.is_none() {
            std::fs::create_dir_all(&self.dir)?;
            prune(&self.dir, MAX_TOTAL_BYTES.saturating_sub(MAX_FILE_BYTES));
            let path = self.dir.join(format!("{}_{}.pcap", utc_stamp(t_us / 1_000_000), self.tag));
            let header = PcapHeader { datalink: DataLink::from(self.link), ..Default::default() };
            let writer = PcapWriter::with_header(File::create(path)?, header).map_err(io::Error::other)?;
            self.file = Some((writer, 24));
        }
        let (writer, bytes) = self.file.as_mut().expect("opened above");
        // Unbuffered, so a crash or a quit loses nothing already captured.
        *bytes += writer.write_packet(&PcapPacket::new(Duration::from_micros(t_us), orig_len, data)).map_err(io::Error::other)? as u64;
        Ok(())
    }
}

/// The recordings in `dir`, oldest first, with their sizes.
pub fn recordings(dir: &Path) -> Vec<(PathBuf, u64)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "pcap"))
        .filter_map(|e| Some((e.path(), e.metadata().ok()?.len())))
        .collect();
    files.sort();
    files
}

/// Deletes the oldest recordings until the rest fit in `max_bytes`.
pub fn prune(dir: &Path, max_bytes: u64) {
    let files = recordings(dir);
    let mut total: u64 = files.iter().map(|(_, n)| n).sum();
    for (path, n) in files {
        if total <= max_bytes {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= n;
        }
    }
}

/// `2026-10-15_06-13-11` for a Unix time in seconds.
fn utc_stamp(secs: u64) -> String {
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + (month <= 2) as i64;
    format!("{year:04}-{month:02}-{day:02}_{:02}-{:02}-{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::LinkType;
    use crate::replay::for_each_packet;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("meter-record-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn stamps_utc_dates() {
        assert_eq!(utc_stamp(0), "1970-01-01_00-00-00");
        assert_eq!(utc_stamp(1_791_958_391), "2026-10-14_06-13-11");
        assert_eq!(utc_stamp(951_782_400), "2000-02-29_00-00-00");
    }

    #[test]
    fn recordings_replay() {
        let dir = temp_dir("replay");
        let mut rec = Recorder::new(&dir, "Npcap Loopback", 0);
        rec.write(1_791_958_391_000_000, 4, &[2, 0, 0, 0]).unwrap();
        rec.write(1_791_958_391_250_000, 5, &[2, 0, 0, 0, 9]).unwrap();
        drop(rec);

        let files = recordings(&dir);
        assert_eq!(files.len(), 1);
        assert!(files[0].0.ends_with("2026-10-14_06-13-11_Npcap-Loopback.pcap"));
        let mut got = Vec::new();
        for_each_packet(&files[0].0, |p| got.push((p.link, p.t_ms, p.data))).unwrap();
        assert_eq!(
            got,
            vec![
                (LinkType::Null, 1_791_958_391_000, vec![2, 0, 0, 0]),
                (LinkType::Null, 1_791_958_391_250, vec![2, 0, 0, 0, 9])
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prunes_oldest_first() {
        let dir = temp_dir("prune");
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["2026-01-01_00-00-00_a.pcap", "2026-01-02_00-00-00_a.pcap", "2026-01-03_00-00-00_a.pcap"] {
            std::fs::write(dir.join(name), [0u8; 100]).unwrap();
        }
        std::fs::write(dir.join("notes.txt"), [0u8; 1000]).unwrap();
        prune(&dir, 250);
        let left: Vec<_> = recordings(&dir).iter().map(|(p, _)| p.file_name().unwrap().to_owned()).collect();
        assert_eq!(left, ["2026-01-02_00-00-00_a.pcap", "2026-01-03_00-00-00_a.pcap"]);
        assert!(dir.join("notes.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
