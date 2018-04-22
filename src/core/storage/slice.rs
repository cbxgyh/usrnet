use std::ops::{
    Deref,
    DerefMut,
};

use {
    Error,
    Result,
};

/// Owned slice which acts a resizable view over a non-resizable buffer.
#[derive(Clone, Debug)]
pub struct Slice<T> {
    buffer: Vec<T>,
    len: usize,
}

impl<T> From<Vec<T>> for Slice<T> {
    fn from(buffer: Vec<T>) -> Self {
        let len = buffer.len();
        Slice { buffer, len }
    }
}

impl<T> Deref for Slice<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.buffer[0 .. self.len]
    }
}

impl<T> DerefMut for Slice<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.buffer[0 .. self.len]
    }
}

impl<T: Clone> Slice<T> {
    /// Attempts to resize the slice, assigning fresh values to the tail end
    /// of the buffer in an upsizing operation.
    pub fn try_resize(&mut self, buffer_len: usize, value: T) -> Result<()> {
        if buffer_len > self.buffer.len() {
            Err(Error::Exhausted)
        } else {
            for i in self.len .. buffer_len {
                self.buffer[i] = value.clone();
            }
            self.len = buffer_len;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_too_big() {
        let mut slice = Slice::from(vec![0, 1, 2, 3]);
        assert_eq!(&slice[..], &[0, 1, 2, 3]);
        assert_matches!(slice.try_resize(8, 0), Err(Error::Exhausted));
        assert_eq!(&slice[..], &[0, 1, 2, 3]);
    }

    #[test]
    fn test_resize_with_capacity() {
        let mut slice = Slice::from(vec![0, 1, 2, 3]);
        assert_eq!(&slice[..], &[0, 1, 2, 3]);
        assert_matches!(slice.try_resize(0, 0), Ok(_));
        assert_eq!(&slice[..], &[]);
        assert_matches!(slice.try_resize(1, 0), Ok(_));
        assert_eq!(&slice[..], &[0]);
        assert_matches!(slice.try_resize(2, 0), Ok(_));
        assert_eq!(&slice[..], &[0, 0]);
        assert_matches!(slice.try_resize(3, 0), Ok(_));
        assert_eq!(&slice[..], &[0, 0, 0]);
        assert_matches!(slice.try_resize(4, 0), Ok(_));
        assert_eq!(&slice[..], &[0, 0, 0, 0]);
    }
}
