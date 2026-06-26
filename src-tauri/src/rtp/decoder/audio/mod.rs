//! 音频解码器模块
//!
//! 支持 AAC、MP3、Opus、G.711 A-law、G.711 μ-law 音频解码
//!
//! G.711 解码器使用纯 Rust 实现
//! AAC、MP3、Opus 解码器通过 FFI 对接 C++/FFmpeg 实现

// 声明子模块
mod g711;

// 条件编译：需要 FFI 的音频解码器
#[cfg(feature = "decoder-ffi")]
mod aac;
#[cfg(feature = "decoder-ffi")]
mod mp3;
#[cfg(feature = "decoder-ffi")]
mod opus;

// 重新导出
pub use g711::{G711ADecoder, G711UDecoder};

#[cfg(feature = "decoder-ffi")]
pub use aac::AacDecoder;
#[cfg(feature = "decoder-ffi")]
pub use mp3::Mp3Decoder;
#[cfg(feature = "decoder-ffi")]
pub use opus::OpusDecoder;

use crate::rtp::decoder::trait_::{Decoder, DecodeResult, DecodeError};
use crate::rtp::decoder::types::CodecType;

/// 创建音频解码器
///
/// # Arguments
/// * `codec_type` - 音频编解码器类型
///
/// # Returns
/// 返回对应的音频解码器实例
pub fn create_audio_decoder(codec_type: CodecType) -> DecodeResult<Box<dyn Decoder + Send>> {
    match codec_type {
        #[cfg(feature = "decoder-ffi")]
        CodecType::AAC => Ok(Box::new(AacDecoder::new()?)),
        CodecType::G711A => Ok(Box::new(G711ADecoder::new()?)),
        CodecType::G711U => Ok(Box::new(G711UDecoder::new()?)),
        // TODO: 添加 MP3 和 Opus 支持（需要 decoder-ffi feature）
        #[cfg(feature = "decoder-ffi")]
        CodecType::MP3 => Ok(Box::new(Mp3Decoder::new()?)),
        #[cfg(feature = "decoder-ffi")]
        CodecType::OPUS => Ok(Box::new(OpusDecoder::new()?)),
        _ => Err(DecodeError::UnsupportedCodec(codec_type)),
    }
}

/// 检查是否为支持的音频编解码器
pub fn is_supported_audio_codec(codec_type: CodecType) -> bool {
    matches!(
        codec_type,
        CodecType::AAC | CodecType::MP3 | CodecType::OPUS | CodecType::G711A | CodecType::G711U
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtp::decoder::types::CodecType;

    #[test]
    fn test_create_audio_decoder() {
        // 测试创建 G.711 A-law 解码器
        let result = create_audio_decoder(CodecType::G711A);
        assert!(result.is_ok());

        // 测试创建 G.711 μ-law 解码器
        let result = create_audio_decoder(CodecType::G711U);
        assert!(result.is_ok());

        // 测试不支持的编解码器
        let result = create_audio_decoder(CodecType::H264);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_supported_audio_codec() {
        assert!(is_supported_audio_codec(CodecType::AAC));
        assert!(is_supported_audio_codec(CodecType::G711A));
        assert!(is_supported_audio_codec(CodecType::G711U));
        assert!(!is_supported_audio_codec(CodecType::H264));
        assert!(!is_supported_audio_codec(CodecType::Unknown));
    }
}
