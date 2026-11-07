//! Cartridges: Native Interactive Desktop Applications for Kinetix OS.
//!
//! Encompasses Scribe (Notes Chamber), Ouroboros (Snake Arena),
//! and Telemetry (System Diagnostics).

pub mod ouroboros;
pub mod scribe;

#[allow(unused_imports)]
pub use ouroboros::{OuroborosArena, OUROBOROS};
#[allow(unused_imports)]
pub use scribe::{ScribeChamber, SCRIBE};
