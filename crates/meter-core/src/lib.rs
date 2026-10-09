//! Everything that turns captured bytes into DPS numbers, with no capture driver or UI.
//!
//! Pipeline: link-layer packet → [`net`] (TCP segment) → [`tcp`] (ordered stream) → [`frame`] (game packets)
//! → [`parser`] (combat events) → [`combat`] (encounters and DPS).
//!
//! This crate only ever reads bytes. It never sends, modifies or replays traffic.

pub mod combat;
pub mod demo;
pub mod frame;
pub mod latency;
pub mod names;
pub mod net;
pub mod opcodes;
pub mod parser;
pub mod pipeline;
pub mod replay;
pub mod tcp;
pub mod wire;
