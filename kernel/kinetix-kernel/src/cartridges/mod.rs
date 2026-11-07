//! Cartridges: Native Interactive Desktop Applications for Kinetix OS.
//!
//! Encompasses Scribe (Notes Chamber), Ouroboros (Snake Arena),
//! and Telemetry (System Diagnostics).

pub mod scribe;

#[allow(unused_imports)]
pub use scribe::{SCRIBE, ScribeChamber};
