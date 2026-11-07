//! Pulses: Human Interaction & Motion Vector Subsystem for Kinetix OS.
//!
//! Encompasses the universal kinetic pointer (mouse/touchpad/wireless dongle)
//! and the reticle cursor engine.

pub mod pointer;
pub mod reticle;
pub mod router;

#[allow(unused_imports)]
pub use pointer::{POINTER, PointerSubsystem, MouseState};
#[allow(unused_imports)]
pub use reticle::{RETICLE, Reticle};
#[allow(unused_imports)]
pub use router::{ROUTER, EventRouter, WindowFocus};

pub fn init(width: usize, height: usize) {
    pointer::POINTER.lock().init(width, height);
}
