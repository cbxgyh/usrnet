use std;

/// Represents ownership of a T's buffer. Based on ideas from
/// [https://github.com/m-labs/rust-managed](https://github.com/m-labs/rust-managed).
pub enum Slice<'a, T: 'a> {
    Borrow(&'a mut [T]),
    Owned(std::vec::Vec<T>),
}

impl<'a, T> From<&'a mut [T]> for Slice<'a, T> {
    fn from(slice: &'a mut [T]) -> Self {
        Slice::Borrow(slice)
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
            Slice::Borrow(ref slice) => slice,
            Slice::Owned(ref vec) => vec.as_slice(),
        }
    }
}

impl<'a, T> AsMut<[T]> for Slice<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        match *self {
            Slice::Borrow(ref mut slice) => slice,
            Slice::Owned(ref mut vec) => vec.as_mut_slice(),
        }
    }
}

impl<'a, T> std::ops::Deref for Slice<'a, T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        match *self {
            Slice::Borrow(ref slice) => slice,
            Slice::Owned(ref vec) => vec.as_slice(),
        }
    }
}

impl<'a, T> std::ops::DerefMut for Slice<'a, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        match *self {
            Slice::Borrow(ref mut slice) => slice,
            Slice::Owned(ref mut vec) => vec.as_mut_slice(),
        }
    }
}
