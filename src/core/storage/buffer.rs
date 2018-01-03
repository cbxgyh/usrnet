use std::borrow::{Borrow, BorrowMut};

/// Generic readable byte buffer.
pub type Buffer = Borrow<[u8]>;

/// Generic writable byte buffer.
pub type BufferMut = BorrowMut<[u8]>;
