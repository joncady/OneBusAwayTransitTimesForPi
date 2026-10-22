//! The documented cVEND IPP transport. This module sends no fare/card writes.
//! https://wiki.pm3.dev/cvend

pub fn crc8(bytes: &[u8]) -> u8 {
    let mut crc = 0;
    for byte in bytes {
        crc ^= byte;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0x8c } else { 0 };
        }
    }
    crc
}

pub fn jamcrc(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    crc
}

pub fn frame(sequence: u8, kind: u8, payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() <= u16::MAX as usize);
    let len = (payload.len() as u16).to_be_bytes();
    let mut result = vec![0xbc, sequence, 0, kind, len[0], len[1]];
    result.push(crc8(&result));
    result.extend_from_slice(payload);
    if kind >= 0x80 {
        result.extend_from_slice(&jamcrc(payload).to_le_bytes());
    }
    result
}

#[derive(Debug, PartialEq)]
pub struct Message {
    pub kind: u8,
    pub payload: Vec<u8>,
}

#[derive(Default)]
pub struct Decoder {
    buffer: Vec<u8>,
    pub rejected: u64,
}

impl Decoder {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Message> {
        self.buffer.extend_from_slice(bytes);
        let mut messages = vec![];
        loop {
            let Some(start) = self.buffer.iter().position(|byte| *byte == 0xbc) else {
                self.buffer.clear();
                break;
            };
            self.buffer.drain(..start);
            if self.buffer.len() < 7 {
                break;
            }
            if crc8(&self.buffer[..6]) != self.buffer[6] {
                self.rejected += 1;
                self.buffer.remove(0);
                continue;
            }
            let kind = self.buffer[3];
            let len = u16::from_be_bytes([self.buffer[4], self.buffer[5]]) as usize;
            let total = 7 + len + if kind >= 0x80 { 4 } else { 0 };
            if self.buffer.len() < total {
                break;
            }
            if kind >= 0x80 {
                let actual = u32::from_le_bytes(self.buffer[7 + len..total].try_into().unwrap());
                if jamcrc(&self.buffer[7..7 + len]) != actual {
                    self.rejected += 1;
                    self.buffer.drain(..total);
                    continue;
                }
            }
            messages.push(Message {
                kind,
                payload: self.buffer[7..7 + len].to_vec(),
            });
            self.buffer.drain(..total);
        }
        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_checksum_vectors() {
        assert_eq!(crc8(b"123456789"), 0xa1);
        assert_eq!(jamcrc(b"123456789"), 0x340bc6d9);
    }

    #[test]
    fn fragmented_frames_and_noise() {
        let bytes = frame(1, 0xe4, &[0, 7, 1, 1]);
        let mut decoder = Decoder::default();
        assert!(decoder.feed(&[0, 1, 2]).is_empty());
        assert!(decoder.feed(&bytes[..5]).is_empty());
        assert_eq!(
            decoder.feed(&bytes[5..]),
            vec![Message {
                kind: 0xe4,
                payload: vec![0, 7, 1, 1]
            }]
        );
    }

    #[test]
    fn rejects_corrupt_payload_and_resynchronizes() {
        let mut corrupt = frame(1, 0xe4, &[0, 7, 1, 1]);
        corrupt[8] ^= 1;
        corrupt.extend(frame(2, 4, &[]));
        let mut decoder = Decoder::default();
        assert_eq!(
            decoder.feed(&corrupt),
            vec![Message {
                kind: 4,
                payload: vec![]
            }]
        );
        assert_eq!(decoder.rejected, 1);
    }
}
