use crate::rtp::error::RtpError;
use bytes::Bytes;

const FIXED_HEADER_LEN: usize = 12;
const CSRC_LEN: usize = 4;

/// RTP header fields extracted from the raw packet.
#[derive(Debug, Clone)]
pub struct RtpHeader {
    /// RTP protocol version (should be 2)
    pub version: u8,
    /// Whether padding bytes are present at the end
    pub padding: bool,
    /// Whether a header extension follows the CSRC list
    pub extension: bool,
    /// Marker bit (application-specific)
    pub marker: bool,
    /// Payload type (7 bits, 0-127)
    pub payload_type: u8,
    /// Sequence number (incremented by sender per packet)
    pub sequence_number: u16,
    /// Timestamp (sampling instant of the first octet)
    pub timestamp: u32,
    /// Synchronization source identifier
    pub ssrc: u32,
    /// Contributing source identifiers
    pub csrcs: Vec<u32>,
}

/// A parsed RTP packet.
#[derive(Debug, Clone)]
pub struct RtpPacket {
    /// Decoded RTP header
    pub header: RtpHeader,
    /// The payload bytes (after header, before any padding)
    pub payload: Bytes,
}

/// Well-known RTP payload types.
pub mod payload_type {
    pub const PCMU: u8 = 0;
    pub const PCMA: u8 = 8;
    pub const G722: u8 = 9;
    pub const L16_STEREO: u8 = 10;
    pub const L16_MONO: u8 = 11;
    pub const CN: u8 = 13;
    pub const MPA: u8 = 14;
    pub const H263: u8 = 34;
    pub const H264: u8 = 96;
    pub const H265: u8 = 97;
    pub const VP8: u8 = 98;
    pub const VP9: u8 = 99;
    pub const OPUS: u8 = 100;
    pub const DYNAMIC_MIN: u8 = 96;
    pub const DYNAMIC_MAX: u8 = 127;
}

/// Returns a human-readable name for a given RTP payload type.
pub fn payload_type_name(pt: u8) -> &'static str {
    match pt {
        0 => "PCMU",
        1 => "1016",
        2 => "G721",
        3 => "GSM",
        4 => "G723",
        5 => "DVI4_8k",
        6 => "DVI4_16k",
        7 => "LPC",
        8 => "PCMA",
        9 => "G722",
        10 => "L16_STEREO",
        11 => "L16_MONO",
        12 => "QCELP",
        13 => "CN",
        14 => "MPA",
        15 => "G728",
        16 => "DVI4_11k",
        17 => "DVI4_22k",
        18 => "G729",
        25 => "CelB",
        26 => "JPEG",
        28 => "nv",
        31 => "H261",
        32 => "MPV",
        33 => "MP2T",
        34 => "H263",
        96..=127 => "Dynamic",
        _ => "Unknown",
    }
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

impl RtpPacket {
    /// Parse an RTP packet from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, RtpError> {
        if data.len() < FIXED_HEADER_LEN {
            return Err(RtpError::BufferTooShort {
                need: FIXED_HEADER_LEN,
                got: data.len(),
            });
        }

        let b0 = data[0];
        let version = (b0 >> 6) & 0x03;
        let padding = ((b0 >> 5) & 0x01) != 0;
        let extension = ((b0 >> 4) & 0x01) != 0;
        let csrc_count = (b0 & 0x0f) as usize;

        let b1 = data[1];
        let marker = ((b1 >> 7) & 0x01) != 0;
        let payload_type = b1 & 0x7f;

        let sequence_number = read_u16(data, 2);
        let timestamp = read_u32(data, 4);
        let ssrc = read_u32(data, 8);

        let mut offset = FIXED_HEADER_LEN;
        let header_end = offset + csrc_count * CSRC_LEN;
        if data.len() < header_end {
            return Err(RtpError::BufferTooShort {
                need: header_end,
                got: data.len(),
            });
        }

        let mut csrcs = Vec::with_capacity(csrc_count);
        for _ in 0..csrc_count {
            csrcs.push(read_u32(data, offset));
            offset += CSRC_LEN;
        }

        // Skip header extension if present
        if extension {
            if data.len() < offset + 4 {
                return Err(RtpError::BufferTooShort {
                    need: offset + 4,
                    got: data.len(),
                });
            }
            let ext_len = read_u16(data, offset + 2) as usize * 4;
            offset += 4 + ext_len;
            if data.len() < offset {
                return Err(RtpError::BufferTooShort {
                    need: offset,
                    got: data.len(),
                });
            }
        }

        // Determine payload end (accounting for padding)
        let payload_end = if padding && !data.is_empty() {
            let pad_size = data[data.len() - 1] as usize;
            if pad_size > data.len() - offset {
                return Err(RtpError::ParseError("invalid padding size".into()));
            }
            data.len() - pad_size
        } else {
            data.len()
        };

        let payload = Bytes::copy_from_slice(&data[offset..payload_end]);

        Ok(Self {
            header: RtpHeader {
                version,
                padding,
                extension,
                marker,
                payload_type,
                sequence_number,
                timestamp,
                ssrc,
                csrcs,
            },
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a valid RTP packet bytes.
    fn make_rtp_bytes(
        version: u8,
        padding: bool,
        extension: bool,
        csrc_count: u8,
        marker: bool,
        pt: u8,
        seq: u16,
        ts: u32,
        ssrc: u32,
        csrcs: &[u32],
        payload: &[u8],
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        let v_p_x_cc = (version << 6)
            | ((padding as u8) << 5)
            | ((extension as u8) << 4)
            | (csrc_count & 0x0f);
        let m_pt = ((marker as u8) << 7) | (pt & 0x7f);
        buf.push(v_p_x_cc);
        buf.push(m_pt);
        buf.extend_from_slice(&seq.to_be_bytes());
        buf.extend_from_slice(&ts.to_be_bytes());
        buf.extend_from_slice(&ssrc.to_be_bytes());
        for csrc in csrcs {
            buf.extend_from_slice(&csrc.to_be_bytes());
        }
        buf.extend_from_slice(payload);
        buf
    }

    #[test]
    fn test_parse_minimal_header() {
        let data = make_rtp_bytes(
            2, false, false, 0, false, 0, 1, 12345, 0xdeadbeef, &[], &[0x01, 0x02, 0x03],
        );
        let pkt = RtpPacket::from_bytes(&data).unwrap();

        assert_eq!(pkt.header.version, 2);
        assert!(!pkt.header.padding);
        assert!(!pkt.header.extension);
        assert_eq!(pkt.header.payload_type, 0);
        assert_eq!(pkt.header.sequence_number, 1);
        assert_eq!(pkt.header.timestamp, 12345);
        assert_eq!(pkt.header.ssrc, 0xdeadbeef);
        assert_eq!(&pkt.payload[..], &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_parse_with_csrc() {
        let data = make_rtp_bytes(
            2, false, false, 2, true, 8, 42, 999, 0x11111111,
            &[0x22222222, 0x33333333], &[0xff],
        );
        let pkt = RtpPacket::from_bytes(&data).unwrap();

        assert_eq!(pkt.header.csrcs.len(), 2);
        assert_eq!(pkt.header.csrcs[0], 0x22222222);
        assert_eq!(pkt.header.csrcs[1], 0x33333333);
        assert!(pkt.header.marker);
        assert_eq!(pkt.header.payload_type, 8);
    }

    #[test]
    fn test_buffer_too_short() {
        let err = RtpPacket::from_bytes(&[0x80]).unwrap_err();
        assert!(matches!(err, RtpError::BufferTooShort { .. }));
    }

    #[test]
    fn test_empty_payload() {
        let data = make_rtp_bytes(2, false, false, 0, false, 0, 1, 0, 0, &[], &[]);
        let pkt = RtpPacket::from_bytes(&data).unwrap();
        assert!(pkt.payload.is_empty());
    }

    #[test]
    fn test_payload_type_name() {
        assert_eq!(payload_type_name(0), "PCMU");
        assert_eq!(payload_type_name(8), "PCMA");
        assert_eq!(payload_type_name(96), "Dynamic");
        assert_eq!(payload_type_name(127), "Dynamic");
        assert_eq!(payload_type_name(200), "Unknown");
    }
}
