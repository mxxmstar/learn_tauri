use thiserror::Error;

#[derive(Error, Debug)]
pub enum RtpError {
    #[error("RTP payload type {0} is not supported")]
    UnsupportedPayloadType(u8),

    #[error("RTP packet parse error: {0}")]
    ParseError(String),

    #[error("RTP buffer too short: need {need} bytes, got {got}")]
    BufferTooShort { need: usize, got: usize },
}
