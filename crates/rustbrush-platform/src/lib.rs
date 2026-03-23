//! RustBrush Platform - OS-specific input simulation, screen capture, and hotkeys.
//!
//! SAFETY: This crate uses ONLY OS-level APIs (SendInput, xdotool, screen capture).
//! It never reads or writes game process memory, injects DLLs, or hooks processes.

pub mod input;
pub mod capture;
pub mod hotkey;
pub mod executor;
