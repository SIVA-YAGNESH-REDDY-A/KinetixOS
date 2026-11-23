//! Cartridges: Native Interactive Desktop Applications for Kinetix OS.
//!
//! Encompasses Scribe (Notes Chamber), Ouroboros (Snake Arena),
//! and Telemetry (System Diagnostics).

pub mod kinetix_pc;
pub mod ouroboros;
pub mod scribe;
pub mod speed_test;
pub mod telemetry;
pub mod terminal;
pub mod vault;

#[allow(unused_imports)]
pub use kinetix_pc::{KinetixPcExplorer, KINETIX_PC};
#[allow(unused_imports)]
pub use ouroboros::{OuroborosArena, OUROBOROS};
#[allow(unused_imports)]
pub use scribe::{ScribeChamber, SCRIBE};
#[allow(unused_imports)]
pub use speed_test::{KinetixSpeedTest, SPEED_TEST};
#[allow(unused_imports)]
pub use telemetry::{TelemetryDeck, TELEMETRY};
#[allow(unused_imports)]
pub use terminal::{KinetixTerminal, TERMINAL};
#[allow(unused_imports)]
pub use vault::{KinetixVault, VAULT};
