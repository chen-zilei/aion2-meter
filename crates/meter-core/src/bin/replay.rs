//! Offline tool: feed a saved capture through the decoder.
//!
//! ```text
//! replay <file.pcapng> [--port 13328] [--opcodes] [--dump <hex opcode>] [--events] [--ping]
//! ```
//! `--opcodes` prints packet counts per opcode (the starting point for reverse engineering),
//! `--dump 0438` prints every body of one opcode as hex, `--events` prints each decoded combat event,
//! `--ping` prints the ping readout once a second.

use anyhow::{bail, Result};
use meter_core::combat::TrackerOptions;
use meter_core::opcodes;
use meter_core::pipeline::{Pipeline, DEFAULT_GAME_PORT};
use meter_core::replay::for_each_packet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut file = None;
    let mut port = DEFAULT_GAME_PORT;
    let mut show_opcodes = false;
    let mut show_events = false;
    let mut show_ping = false;
    let mut dump: Option<u16> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" => port = args.next().unwrap_or_default().parse()?,
            "--opcodes" => show_opcodes = true,
            "--events" => show_events = true,
            "--ping" => show_ping = true,
            "--dump" => dump = Some(u16::from_str_radix(&args.next().unwrap_or_default().replace(' ', ""), 16)?),
            _ if file.is_none() => file = Some(PathBuf::from(a)),
            other => bail!("unexpected argument {other}"),
        }
    }
    let Some(file) = file else { bail!("usage: replay <capture.pcapng> [--port N] [--opcodes] [--dump 0438] [--events] [--ping]") };

    let mut pipe = Pipeline::new(vec![port], TrackerOptions::default());
    if let Some(want) = dump {
        pipe.on_packet = Some(Box::new(move |t, op, body| {
            if op == want {
                let hex: Vec<String> = body.iter().map(|b| format!("{b:02x}")).collect();
                println!("{t} {} [{}] {}", opcodes::name(op), body.len(), hex.join(" "));
            }
        }));
    }
    if show_events {
        pipe.on_event = Some(Box::new(|ev| println!("{ev:?}")));
    }

    let first_ms = Arc::new(Mutex::new(None::<u64>));
    let mut last_ping_s = 0;
    for_each_packet(&file, |p| {
        first_ms.lock().unwrap().get_or_insert(p.t_ms);
        pipe.push(p.link, p.t_ms, &p.data);
        if show_ping && p.t_ms / 1000 != last_ping_s {
            last_ping_s = p.t_ms / 1000;
            let first = first_ms.lock().unwrap().unwrap_or(p.t_ms);
            match pipe.ping_ms(p.t_ms) {
                Some(ms) => println!("{:>6.1}s  ping {ms} ms", (p.t_ms - first) as f64 / 1000.0),
                None => println!("{:>6.1}s  ping -", (p.t_ms - first) as f64 / 1000.0),
            }
        }
    })?;
    let end_ms = pipe.last_ms;
    pipe.tracker.finish();
    if show_ping {
        for (server, st, ping) in pipe.ping_stats(end_ms) {
            eprintln!(
                "ping {server}: pongs {} (matched {}), heartbeats {}, arrival - echo {:?}, last reading {ping:?}",
                st.pongs, st.pongs_matched, st.heartbeats, st.last_arrival_minus_echo
            );
        }
    }

    let fs = pipe.frame_stats();
    let ps = &pipe.parser.stats;
    eprintln!(
        "frames {} (bundles {}, bad {}), resyncs {}, dropped {} B, heartbeats {}, damage records {} (rejected {})",
        fs.frames, fs.bundles, fs.bad_bundles, fs.resyncs, fs.dropped_bytes, ps.heartbeats, ps.damage_records, ps.damage_rejected
    );
    if fs.frames == 0 {
        eprintln!("no game traffic found on port {port}; check the port with Wireshark (tcp.srcport == N)");
    }

    if show_opcodes {
        let mut counts: Vec<_> = ps.opcode_counts.iter().collect();
        counts.sort_by(|a, b| b.1.cmp(a.1));
        println!("{:>8}  opcode", "count");
        for (op, n) in counts {
            println!("{n:>8}  {}", opcodes::name(*op));
        }
    }

    for enc in pipe.tracker.history.iter().rev() {
        println!(
            "\nencounter {} vs {}: {:.1}s, total {}, party dps {:.0}",
            enc.id, enc.main_target, enc.duration_s, enc.total_damage, enc.party_dps
        );
        for a in enc.actors.iter().take(10) {
            println!(
                "  {:<20} {:>12} dmg {:>10.0} dps {:>5.1}%  crit {:>4.1}%",
                a.name,
                a.damage,
                a.dps,
                a.share * 100.0,
                if a.hits > 0 { a.crits as f64 * 100.0 / a.hits as f64 } else { 0.0 }
            );
        }
    }
    Ok(())
}
