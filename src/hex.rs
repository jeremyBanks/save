pub fn decode_hex_nibbles(s: impl AsRef<str>) -> MaskedBytes {
    let mut hex_bytes = s.as_ref().as_bytes();
    if hex_bytes.get(0) == Some(&b'0') && matches!(hex_bytes.get(1), Some(b'x' | b'X')) {
        hex_bytes = &hex_bytes[2..];
    }
    let capacity = (hex_bytes.len() + 1) / 2;
    let mut bytes = Vec::<u8>::with_capacity(capacity);
    let mut mask = Vec::<u8>::with_capacity(capacity);
    let mut buffer_byte: Option<u8> = None;
    let mut buffer_mask_byte: Option<u8> = None;

    for byte in hex_bytes {
        let mut nibble = 0x0;
        let mut nibble_mask = 0xF;

        match byte {
            b'0'..=b'9' => nibble = byte.wrapping_sub(b'0'),
            b'a'..=b'f' => nibble = byte.wrapping_sub(b'a' - 0xa),
            b'A'..=b'F' => nibble = byte.wrapping_sub(b'A' - 0xA),
            b'_' => nibble_mask = 0x0,
            b' ' | b'\n' | b'\t' | b',' | b';' | b'"' | b'\'' => continue,
            _ => panic!("Invalid byte {byte:?} ({:?}) in hex input.", *byte as char),
        };

        if let Some(byte) = buffer_byte.take() {
            bytes.push(byte | nibble);
            mask.push(buffer_mask_byte.take().unwrap() | nibble_mask);
        } else {
            buffer_byte = Some(nibble << 4);
            buffer_mask_byte = Some(nibble_mask << 4);
        }
    }

    if let Some(byte) = buffer_byte {
        bytes.push(byte);
        mask.push(buffer_mask_byte.take().unwrap());
    }

    assert_eq!(bytes.len(), mask.len());

    MaskedBytes { bytes, mask }
}

#[derive(Debug, Clone, Default)]
pub struct MaskedBytes {
    pub bytes: Vec<u8>,
    pub mask: Vec<u8>,
}

impl IntoIterator for MaskedBytes {
    type IntoIter = std::iter::Zip<std::vec::IntoIter<u8>, std::vec::IntoIter<u8>>;
    type Item = (u8, u8);

    fn into_iter(self) -> Self::IntoIter {
        self.bytes.into_iter().zip(self.mask.into_iter())
    }
}

pub use crate::hex;
#[macro_export]
macro_rules! hex {
    [$($hex:tt)*] => {
        $crate::hex::decode_hex_nibbles(stringify!($($hex)*))
    }
}