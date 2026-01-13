//! Swank protocol framing codec.
//!
//! Swank uses a simple framing protocol:
//! - 6 hex ASCII digits indicating payload length
//! - Payload as S-expression string

use bytes::{Buf, BufMut, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder};

const HEADER_LEN: usize = 6;

pub struct SwankCodec;

impl Decoder for SwankCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < HEADER_LEN {
            return Ok(None);
        }

        // Parse 6-digit hex header
        let header = std::str::from_utf8(&src[..HEADER_LEN])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let payload_len = usize::from_str_radix(header, 16)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if src.len() < HEADER_LEN + payload_len {
            return Ok(None);
        }

        // Extract payload
        src.advance(HEADER_LEN);
        let payload = src.split_to(payload_len);

        let message = String::from_utf8(payload.to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(Some(message))
    }
}

impl Encoder<String> for SwankCodec {
    type Error = io::Error;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload = item.as_bytes();
        let header = format!("{:06x}", payload.len());

        dst.reserve(HEADER_LEN + payload.len());
        dst.put_slice(header.as_bytes());
        dst.put_slice(payload);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_complete_message() {
        let mut codec = SwankCodec;
        let mut buf = BytesMut::from("00000d(:return nil)");

        let result = codec.decode(&mut buf).unwrap();
        assert_eq!(result, Some("(:return nil)".to_string()));
    }

    #[test]
    fn test_decode_partial_header() {
        let mut codec = SwankCodec;
        let mut buf = BytesMut::from("0000");

        let result = codec.decode(&mut buf).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_decode_partial_payload() {
        let mut codec = SwankCodec;
        // Header says 13 bytes, but only 5 bytes of payload present
        let mut buf = BytesMut::from("00000d(:ret");

        let result = codec.decode(&mut buf).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_encode_message() {
        let mut codec = SwankCodec;
        let mut buf = BytesMut::new();

        codec
            .encode("(:return nil)".to_string(), &mut buf)
            .unwrap();
        assert_eq!(&buf[..], b"00000d(:return nil)");
    }

    #[test]
    fn test_encode_empty_message() {
        let mut codec = SwankCodec;
        let mut buf = BytesMut::new();

        codec.encode("".to_string(), &mut buf).unwrap();
        assert_eq!(&buf[..], b"000000");
    }

    #[test]
    fn test_decode_multiple_messages() {
        let mut codec = SwankCodec;
        let mut buf = BytesMut::from("00000d(:return nil)000005hello");

        let result1 = codec.decode(&mut buf).unwrap();
        assert_eq!(result1, Some("(:return nil)".to_string()));

        let result2 = codec.decode(&mut buf).unwrap();
        assert_eq!(result2, Some("hello".to_string()));
    }
}
