use std;

use {
    Error,
    Result,
};

/// Represents ownership of a T's buffer. Based on ideas from
/// [https://github.com/m-labs/rust-managed](https://github.com/m-labs/rust-managed).
pub enum Slice<'a, T: 'a> {
    Borrow(&'a mut [T], usize),
    Owned(std::vec::Vec<T>),
}

impl<'a, T> From<&'a mut [T]> for Slice<'a, T> {
    fn from(slice: &'a mut [T]) -> Self {
        let len = slice.len();
        Slice::Borrow(slice, len)
    }
}

impl<'a, T> From<std::vec::Vec<T>> for Slice<'a, T> {
    fn from(vec: std::vec::Vec<T>) -> Self {
        Slice::Owned(vec)
    }
}

impl<'a, T> AsRef<[T]> for Slice<'a, T> {
    fn as_ref(&self) -> &[T] {
        match *self {
            Slice::Borrow(ref slice, len) => &slice[..len],
            Slice::Owned(ref vec) => vec.as_slice(),
        }
    }
}

impl<'a, T> AsMut<[T]> for Slice<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        match *self {
            Slice::Borrow(ref mut slice, len) => &mut slice[..len],
            Slice::Owned(ref mut vec) => vec.as_mut_slice(),
        }
    }
}

impl<'a, T> std::ops::Deref for Slice<'a, T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_ref()
    }
}

impl<'a, T> std::ops::DerefMut for Slice<'a, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut()
    }
}

impl<'a, T> Slice<'a, T>
where
    T: Clone,
{
    /// Attempts to resize the the slice, filling additional slots with value.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying slice does not have sufficient
    /// capacity.
    pub fn try_resize(&mut self, buffer_len: usize, value: T) -> Result<()> {
        match *self {
            Slice::Borrow(ref mut slice, ref mut len) => {
                if slice.len() < buffer_len {
                    Err(Error::Exhausted)
                } else {
                    let mut i = *len;
                    while i < buffer_len {
                        slice[i] = value.clone();
                        i += 1;
                    }
                    *len = buffer_len;
                    Ok(())
                }
            }
            Slice::Owned(ref mut vec) => {
                vec.resize(buffer_len, value);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_owned() {
        let mut slice = Slice::from(vec![0, 1, 2, 3]);
        assert_eq!(&slice[..], &[0, 1, 2, 3]);
        assert_matches!(slice.try_resize(8, 0), Ok(()));
        assert_eq!(&slice[..], &[0, 1, 2, 3, 0, 0, 0, 0]);
    }

    #[test]
    fn test_resize_borrowed_too_big() {
        let mut buffer = [0, 1, 2, 3];
        let mut slice = Slice::from(&mut buffer[..]);
        assert_eq!(&slice[..], &[0, 1, 2, 3]);
        assert_matches!(slice.try_resize(8, 0), Err(Error::Exhausted));
        assert_eq!(&slice[..], &[0, 1, 2, 3]);
    }

    #[test]
    fn test_resize_borrowed_with_capacity() {
        let mut buffer = [0, 1, 2, 3, 4, 5, 6, 7];
        let mut slice = Slice::from(&mut buffer[..]);
        assert_eq!(&slice[..], &[0, 1, 2, 3, 4, 5, 6, 7]);
        assert_matches!(slice.try_resize(4, 0), Ok(()));
        assert_eq!(&slice[..], &[0, 1, 2, 3]);
        assert_matches!(slice.try_resize(8, 0), Ok(()));
        assert_eq!(&slice[..], &[0, 1, 2, 3, 0, 0, 0, 0]);
    }
}
