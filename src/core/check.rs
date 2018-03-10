//! Compute internet checksums.

use std::iter::Cloned;
use std::slice::Iter as SliceIter;

use byteorder::{
    NetworkEndian,
    ReadBytesExt,
};

/// An iterator that inteprets a sequence of bytes as a sequence of network
/// byte order u16's.
pub struct ByteOrderIter<I: Iterator<Item = u8>> {
    iter: I,
}

impl<I: Iterator<Item = u8>> From<I> for ByteOrderIter<I> {
    fn from(iter: I) -> Self {
        ByteOrderIter { iter }
    }
}

impl<'a> From<&'a [u8]> for ByteOrderIter<Cloned<SliceIter<'a, u8>>> {
    fn from(slice: &'a [u8]) -> Self {
        ByteOrderIter {
            iter: slice.iter().cloned(),
        }
    }
}

impl<I: Iterator<Item = u8>> Iterator for ByteOrderIter<I> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        match self.iter.next() {
            None => None,
            Some(x) => {
                if let Some(y) = self.iter.next() {
                    let buffer = [x, y];
                    Some((&buffer[..]).read_u16::<NetworkEndian>().unwrap())
                } else {
                    Some((x as u16) << 8)
                }
            }
        }
    }
}

/// Calculates the Internet Checksum from [RFC1071](https://tools.ietf.org/html/rfc107).
pub fn internet_checksum<T, I>(iterable: T) -> u16
where
    T: Into<ByteOrderIter<I>>,
    I: Iterator<Item = u8>,
{
    let mut iter = iterable.into();
    let mut acc: u32 = 0;

    while let Some(i) = iter.next() {
        acc += i as u32;
    }

    while acc > 0xFFFF {
        acc -= 0xFFFF;
    }

    !acc as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_order_iter_empty() {
        let buffer: [u8; 0] = [];
        let mut iter = ByteOrderIter::from(&buffer[..]);
        assert_matches!(iter.next(), None);
    }

    #[test]
    fn test_byte_order_iter_even() {
        let buffer: [u8; 6] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let mut iter = ByteOrderIter::from(&buffer[..]);
        assert_matches!(iter.next(), Some(258));
        assert_matches!(iter.next(), Some(772));
        assert_matches!(iter.next(), Some(1286));
        assert_matches!(iter.next(), None);
    }

    #[test]
    fn test_byte_order_iter_odd() {
        let buffer: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x05];
        let mut iter = ByteOrderIter::from(&buffer[..]);
        assert_matches!(iter.next(), Some(258));
        assert_matches!(iter.next(), Some(772));
        assert_matches!(iter.next(), Some(1280));
        assert_matches!(iter.next(), None);
    }

    #[test]
    fn test_internet_checksum() {
        let buffer: [u8; 20] = [
            0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00, 0xc0, 0xa8,
            0x00, 0x01, 0xc0, 0xa8, 0x00, 0xc7,
        ];
        let iter = ByteOrderIter::from(&buffer[..]);
        assert_eq!(0xB861, internet_checksum(iter));
    }
}
