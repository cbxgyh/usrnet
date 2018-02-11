use std;

use {
    Error,
    Result,
};
use core::storage::Slice;

/// Dynamically sized slice.
pub struct Buffer<'a> {
    slice: Slice<'a, u8>,
    len: usize,
}

impl<'a> From<&'a mut [u8]> for Buffer<'a> {
    fn from(slice: &'a mut [u8]) -> Buffer<'a> {
        let len = slice.len();
        Buffer {
            slice: Slice::from(slice),
            len,
        }
    }
}

impl<'a> From<std::vec::Vec<u8>> for Buffer<'a> {
    fn from(vec: std::vec::Vec<u8>) -> Buffer<'a> {
        let len = vec.len();
        Buffer {
            slice: Slice::from(vec),
            len,
        }
    }
}

impl<'a> std::ops::Deref for Buffer<'a> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.slice[..self.len]
    }
}

impl<'a> std::ops::DerefMut for Buffer<'a> {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.slice[..self.len]
    }
}

impl<'a> Buffer<'a> {
    /// Attempts to resize the the buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying slice does not have sufficient
    /// capacity.
    pub fn try_resize(&mut self, buffer_len: usize) -> Result<()> {
        if buffer_len > self.slice.len() {
            return Err(Error::Exhausted);
        }

        self.len = buffer_len;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_too_big() {
        let mut buffer = Buffer::from(vec![0; 8]);
        assert_eq!(buffer.len(), 8);
        assert_matches!(buffer.try_resize(16), Err(Error::Exhausted));
        assert_eq!(buffer.len(), 8);
    }

    #[test]
    fn test_resize_with_capacity() {
        let mut buffer = Buffer::from(vec![0; 8]);
        assert_eq!(buffer.len(), 8);
        assert_matches!(buffer.try_resize(4), Ok(()));
        assert_eq!(buffer.len(), 4);
    }
}
