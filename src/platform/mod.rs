//! Platform boundary. On Windows this is the Win32 layer; the module is the only place
//! `unsafe` and Win32 type names are allowed.

#[cfg(windows)]
pub mod win32;

#[cfg(windows)]
pub use win32::*;
