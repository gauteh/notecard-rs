//! COBS encoding with EOP=`'\n'` (0x0A) as used by the Blues Notecard binary protocol.
//!
//! This is standard COBS encoding where all output bytes (code bytes and data bytes) are XOR'd
//! with the EOP byte (0x0A). This eliminates all 0x0A bytes from the encoded output so that
//! `'\n'` can unambiguously mark the end of a binary packet.
//!
//! See: <https://github.com/blues/note-c/blob/master/n_cobs.c>

/// End-of-packet byte used by the Blues Notecard binary protocol.
pub const EOP: u8 = b'\n';

/// Minimum buffer size needed to COBS-encode `input_len` bytes, **including** the EOP byte.
///
/// Pass the result of this function as the minimum `enc_buf` length when calling
/// [`crate::note::Note::add_binary`].
pub fn max_encoded_len(input_len: usize) -> usize {
    let code_bytes = input_len / 254 + 1;
    input_len + code_bytes + 1 // +1 for EOP
}

/// COBS-encode `src` into `dst` using `EOP = '\n'`.
///
/// Returns the encoded length, **not** including the EOP byte. The caller must write `EOP`
/// (`b'\n'`) at `dst[enc_len]` to terminate the packet.
///
/// `dst` must be at least `max_encoded_len(src.len()) - 1` bytes long.
pub fn encode(src: &[u8], dst: &mut [u8]) -> usize {
    let mut code: u8 = 1;
    let mut code_pos = 0usize;
    let mut dst_idx = 1usize;

    for &byte in src {
        if code == 0xFF {
            dst[code_pos] = code ^ EOP;
            code_pos = dst_idx;
            dst_idx += 1;
            code = 1;
        }
        if byte == 0x00 {
            dst[code_pos] = code ^ EOP;
            code_pos = dst_idx;
            dst_idx += 1;
            code = 1;
        } else {
            dst[dst_idx] = byte ^ EOP;
            dst_idx += 1;
            code += 1;
        }
    }

    dst[code_pos] = code ^ EOP;
    dst_idx
}

/// COBS-decode `src` (encoded with `EOP = '\n'`) into `dst`.
///
/// `src` must **not** include the trailing EOP byte.
/// Returns the decoded length.
pub fn decode(src: &[u8], dst: &mut [u8]) -> usize {
    let mut dst_idx = 0usize;
    let mut src_idx = 0usize;
    let mut code: u8 = 0xFF; // sentinel: skip zero insertion on first iteration

    while src_idx < src.len() {
        if code != 0xFF {
            dst[dst_idx] = 0;
            dst_idx += 1;
        }

        code = src[src_idx] ^ EOP;
        src_idx += 1;

        if code == 0 {
            break;
        }

        let n = (code - 1) as usize;
        let available = src.len() - src_idx;
        let n = n.min(available);

        for i in 0..n {
            dst[dst_idx + i] = src[src_idx + i] ^ EOP;
        }
        dst_idx += n;
        src_idx += n;
    }

    dst_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) {
        let enc_max = max_encoded_len(data.len());
        let mut enc_buf = vec![0u8; enc_max];
        let enc_len = encode(data, &mut enc_buf);
        enc_buf[enc_len] = EOP;
        for &b in &enc_buf[..enc_len] {
            assert_ne!(b, EOP, "encoded output contains EOP byte");
        }
        let mut dec_buf = vec![0u8; data.len() + 1];
        let dec_len = decode(&enc_buf[..enc_len], &mut dec_buf);
        assert_eq!(dec_len, data.len());
        assert_eq!(&dec_buf[..dec_len], data);
    }

    #[test]
    fn test_empty() {
        roundtrip(&[]);
    }

    #[test]
    fn test_single_zero() {
        roundtrip(&[0x00]);
    }

    #[test]
    fn test_single_eop() {
        roundtrip(&[EOP]);
    }

    #[test]
    fn test_zeros() {
        roundtrip(&[0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_mixed() {
        roundtrip(&[0x01, 0x00, 0x02, EOP, 0x03]);
    }

    #[test]
    fn test_all_eop() {
        roundtrip(&[EOP; 10]);
    }

    #[test]
    fn test_long() {
        let data: Vec<u8> = (0..512).map(|i| i as u8).collect();
        roundtrip(&data);
    }

    #[test]
    fn test_no_eop_in_encoded() {
        // Any byte sequence should produce encoded output with no EOP bytes
        for len in 0..=300usize {
            let data: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let mut enc_buf = vec![0u8; max_encoded_len(len)];
            let enc_len = encode(&data, &mut enc_buf);
            for (pos, &b) in enc_buf[..enc_len].iter().enumerate() {
                assert_ne!(b, EOP, "EOP found at position {pos} for len {len}");
            }
        }
    }
}
