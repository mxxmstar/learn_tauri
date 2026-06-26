//! AAC 音频解码器
//!
//! 通过 FFI 对接 C++/FFmpeg 实现 AAC 解码
//!
//! 支持 AAC-LC、AAC-HE、AAC-HEv2 等Profile

use bytes::Bytes;
use crate::rtp::decoder::trait_::{Decoder, DecodeResult, DecodeError};
use crate::rtp::decoder::types::{CodecType, MediaType, SampleFormat};
use crate::rtp::decoder::frame::{MediaFrame, MediaPacket};
use crate::rtp::decoder::ffi::{FfiDecoder, DecodedFrame};

/// AAC 音频解码器
pub struct AacDecoder {
    /// FFI 解码器
    ffi_decoder: FfiDecoder,
    /// 采样率
    sample_rate: i32,
    /// 声道数
    channels: i32,
    /// 输出采样格式
    sample_format: SampleFormat,
}

impl AacDecoder {
    /// 创建新的 AAC 解码器
    pub fn new() -> Result<Self, String> {
        // 创建 FFI 解码器
        // AAC codec_type = 7, 输出格式 S16 (packed)
        let ffi_decoder = FfiDecoder::new_audio(7, SampleFormat::S16.to_u32() as i32)?;

        Ok(Self {
            ffi_decoder,
            sample_rate: 48000, // 默认采样率
            channels: 2,        // 默认立体声
            sample_format: SampleFormat::S16,
        })
    }

    /// 设置采样率
    pub fn with_sample_rate(mut self, sample_rate: i32) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    /// 设置声道数
    pub fn with_channels(mut self, channels: i32) -> Self {
        self.channels = channels;
        self
    }

    /// 设置输出采样格式
    pub fn with_sample_format(mut self, sample_format: SampleFormat) -> Self {
        self.sample_format = sample_format;
        self
    }
}

impl Decoder for AacDecoder {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
        if packet.data.is_empty() {
            return Ok(None);
        }

        // 通过 FFI 解码
        self.ffi_decoder.decode(
            packet.as_slice(),
            packet.pts,
            packet.dts,
            packet.keyframe,
        ).map_err(DecodeError::DecodeFailed)?;

        // 获取解码后的帧
        match self.ffi_decoder.get_frame().map_err(DecodeError::DecodeFailed)? {
            Some(decoded_frame) => {
                let data = Bytes::copy_from_slice(unsafe {
                    std::slice::from_raw_parts(decoded_frame.data, decoded_frame.size)
                });

                let frame = decoded_frame.to_media_frame(data)
                    .with_timestamps(decoded_frame.pts, decoded_frame.dts);

                Ok(Some(frame))
            }
            None => Ok(None),
        }
    }

    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>> {
        self.ffi_decoder.flush().map_err(DecodeError::DecodeFailed)?;

        // 获取缓冲的帧
        match self.ffi_decoder.get_frame().map_err(DecodeError::DecodeFailed)? {
            Some(decoded_frame) => {
                let data = Bytes::copy_from_slice(unsafe {
                    std::slice::from_raw_parts(decoded_frame.data, decoded_frame.size)
                });

                let frame = decoded_frame.to_media_frame(data);
                Ok(vec![frame])
            }
            None => Ok(Vec::new()),
        }
    }

    fn reset(&mut self) {
        let _ = self.ffi_decoder.reset();
    }

    fn codec_type(&self) -> CodecType {
        CodecType::AAC
    }

    fn name(&self) -> &str {
        "AacDecoder"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_aac_decoder_create() {
        // 注意：这个测试需要 FFI 实现
        // 当前应该返回错误
        let result = AacDecoder::new();
        assert!(result.is_err()); // FFI 尚未实现，应该返回错误
    }
}
