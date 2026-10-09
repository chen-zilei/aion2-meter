//! Live capture: one read-only pcap handle per network adapter, filtered to the game server port (both directions:
//! the client's side is only used to spot the game's ping for the ping readout).
//!
//! Every adapter is opened (including the Npcap loopback adapter, which is where the game traffic shows up when
//! a ping reducer or VPN relays it locally). Packets are only ever read; nothing is sent.

use crate::Shared;
use meter_core::net::LinkType;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CaptureStatus {
    /// Npcap is not installed (Windows).
    NpcapMissing,
    /// Built without live capture, or demo mode is on.
    Off,
    /// Adapters are open but no game traffic seen yet.
    Waiting { adapters: usize },
    Capturing { adapter: String },
    Error { message: String },
}

/// Bumped on every restart; capture threads exit when it no longer matches theirs.
static GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn stop() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
}

#[cfg(windows)]
pub fn npcap_available() -> bool {
    use std::os::windows::ffi::OsStrExt;
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let npcap = std::path::Path::new(&root).join("System32").join("Npcap");
    if npcap.join("wpcap.dll").exists() {
        // Npcap installs outside the normal DLL search path; point the delay-loaded import at it.
        let wide: Vec<u16> = npcap.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe { windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW(wide.as_ptr()) };
        return true;
    }
    std::path::Path::new(&root).join("System32").join("wpcap.dll").exists() // WinPcap-compatible install
}

#[cfg(not(windows))]
pub fn npcap_available() -> bool {
    true
}

#[cfg(feature = "live")]
pub fn start(shared: Arc<Shared>, ports: &[u16]) {
    stop();
    let generation = GENERATION.load(Ordering::SeqCst);

    if !npcap_available() {
        shared.set_status(CaptureStatus::NpcapMissing);
        return;
    }
    let devices = match pcap::Device::list() {
        Ok(d) if !d.is_empty() => d,
        Ok(_) => {
            shared.set_status(CaptureStatus::Error { message: "No network adapters found. Run as administrator?".into() });
            return;
        }
        Err(e) => {
            shared.set_status(CaptureStatus::Error { message: format!("Could not list adapters: {e}") });
            return;
        }
    };

    let filter = ports.iter().map(|p| format!("tcp port {p}")).collect::<Vec<_>>().join(" or ");
    let mut opened = 0;
    for dev in devices {
        let label = dev.desc.clone().unwrap_or_else(|| dev.name.clone());
        let cap = pcap::Capture::from_device(dev)
            .and_then(|c| c.promisc(false).snaplen(65_535).timeout(250).immediate_mode(true).open());
        let mut cap = match cap {
            Ok(c) => c,
            Err(_) => continue, // adapters that are down or unsupported
        };
        if cap.filter(&filter, true).is_err() {
            continue;
        }
        let Some(link) = LinkType::from_pcap(cap.get_datalink().0 as u32) else { continue };
        opened += 1;

        let shared = shared.clone();
        std::thread::spawn(move || {
            while GENERATION.load(Ordering::SeqCst) == generation {
                match cap.next_packet() {
                    Ok(pkt) => {
                        let ts = &pkt.header.ts;
                        let t_ms = ts.tv_sec as u64 * 1000 + ts.tv_usec as u64 / 1000;
                        let mut pipe = shared.pipeline.lock();
                        let had_flow = pipe.has_game_flow();
                        pipe.push(link, t_ms.max(1), pkt.data);
                        if !had_flow && pipe.has_game_flow() {
                            drop(pipe);
                            shared.set_status(CaptureStatus::Capturing { adapter: label.clone() });
                        }
                    }
                    Err(pcap::Error::TimeoutExpired) => {}
                    Err(_) => break,
                }
            }
        });
    }

    shared.set_status(if opened == 0 {
        CaptureStatus::Error { message: "Could not open any adapter. Try running as administrator.".into() }
    } else {
        CaptureStatus::Waiting { adapters: opened }
    });
}

#[cfg(not(feature = "live"))]
pub fn start(shared: Arc<Shared>, _ports: &[u16]) {
    stop();
    shared.set_status(CaptureStatus::Off);
}
