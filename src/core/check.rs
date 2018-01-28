use byteorder::{
    NetworkEndian,
    ReadBytesExt,
};

/// Calculates the Internet Checksum from [RFC1071](https://tools.ietf.org/html/rfc107).
///
/// See [IPv4 header checksum](https://en.wikipedia.org/wiki/IPv4_header_checksum) for an example.
pub fn internet_checksum(buffer: &[u8]) -> u16 {
    let mut acc = 0 as u32;

    for i in 0..(buffer.len() / 2) {
        let x = (&buffer[i * 2..i * 2 + 2])
            .read_u16::<NetworkEndian>()
            .unwrap();
        acc += x as u32;
    }

    if buffer.len() % 2 == 1 {
        let x = buffer[buffer.len() - 1];
        acc += (x as u32) << 8;
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
    fn test_internet_checksum() {
        let buffer: [u8; 20] = [
            0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0x00, 0x00, 0xc0, 0xa8,
            0x00, 0x01, 0xc0, 0xa8, 0x00, 0xc7,
        ];
        assert_eq!(0xB861, internet_checksum(&buffer));
    }
}
