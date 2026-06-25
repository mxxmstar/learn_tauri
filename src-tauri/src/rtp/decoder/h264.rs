//! H.264 解码器
//!
//! 预留接口，后续通过 FFI 对接 C++/FFmpeg

use crate::rtp::decoder::frame::{MediaFrame, MediaPacket};
use crate::rtp::decoder::trait::{DecodeError, DecodeResult, Decoder};
use crate::rtp::decoder::types::{CodecType, PixelFormat};

/// H.264 解码器
///
/// 预留接口，后续通过 FFI 对接 C++/FFmpeg 实现硬解码
///
/// # Feature Flags
/// - `decoder-ffi`: 启用 FFI 实现（需要链接 C++/FFmpeg 库）
/// - 默认: 返回 UnsupportedCodec 错误
pub struct H264Decoder {
    /// 输出像素格式
    output_format: PixelFormat,
    /// 是否已初始化
    initialized: bool,
}

impl H264Decoder {
    /// 创建新的 H.264 解码器
    pub fn new() -> DecodeResult<Self> {
        #[cfg(feature = "decoder-ffi")]
        {
            // FFI 实现：调用 C++ 端初始化解码器
            // TODO: 实现 FFI 调用
            return Err(DecodeError::UnsupportedCodec(CodecType::H264));
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err(DecodeError::UnsupportedCodec(CodecType::H264))
        }
    }

    /// 创建指定输出格式的 H.264 解码器
    pub fn with_output_format(output_format: PixelFormat) -> DecodeResult<Self> {
        let mut decoder = Self::new()?;
        decoder.output_format = output_format;
        Ok(decoder)
    }
}

impl Default for H264Decoder {
    fn default() -> Self {
        Self {
            output_format: PixelFormat::NV12, // H264 常用 NV12 格式输出
            initialized: false,
        }
    }
}

impl Decoder for H264Decoder {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
        // 检查编解码器类型
        if packet.codec_type != CodecType::H264 {
            return Err(DecodeError::InvalidParameter(format!(
                "expected H264 codec, got {:?}",
                packet.codec_type
            )));
        }

        #[cfg(feature = "decoder-ffi")]
        {
            // FFI 实现：调用 C++ 端解码
            // TODO: 实现 FFI 调用
            return Err(DecodeError::UnsupportedCodec(CodecType::H264));
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Err(DecodeError::UnsupportedCodec(CodecType::H264))
        }
    }

    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>> {
        #[cfg(feature = "decoder-ffi")]
        {
            // FFI 实现：调用 C++ 端刷新解码器
            // TODO: 实现 FFI 调用
            return Ok(Vec::new());
        }

        #[cfg(not(feature = "decoder-ffi"))]
        {
            Ok(Vec::new())
        }
    }

    fn reset(&mut self) {
        #[cfg(feature = "decoder-ffi")]
        {
            // FFI 实现：调用 C++ 端重置解码器
            // TODO: 实现 FFI 调用
        }

        self.initialized = false;
    }

    fn codec_type(&self) -> CodecType {
        CodecType::H264
    }

    fn name(&self) -> &str {
        "H264Decoder"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h264_decoder_new() {
        let result = H264Decoder::new();
        // 默认应该返回错误（未实现）
        assert!(result.is_err());
    }

    #[test]
    fn test_h264_decoder_codec_type() {
        // 由于 new() 会失败，我们测试 default()
        let decoder = H264Decoder::default();
        assert_eq!(decoder.codec_type(), CodecType::H264);
        assert_eq!(decoder.name(), "H264Decoder");
    }
}
