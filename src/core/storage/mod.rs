//! Storage/buffers for packets, frames, etc.

pub mod ring;
pub mod slice;

pub use self::ring::Ring;
pub use self::slice::Slice;
