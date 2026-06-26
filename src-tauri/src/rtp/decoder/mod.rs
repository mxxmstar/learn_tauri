//! 解码器模块
//!
//! 提供统一的解码器接口，支持多种编解码器
//! 包括视频（H.264, H.265, MJPEG）和音频（AAC, MP3, Opus, G.711）

// 模块声明
pub mod types;
pub mod frame;
mod trait_;

// 音频解码器模块
pub mod audio;

// 条件编译模块
#[cfg(feature = "decoder-rust")]
mod mjpeg;

#[cfg(feature = "decoder-ffi")]
mod h264;
#[cfg(feature = "decoder-ffi")]
mod h265;
#[cfg(feature = "decoder-ffi")]
mod ffi;

// 公开导出
pub use types::{
    MediaType, CodecType, PixelFormat, SampleFormat, BackendHandle,
};
pub use frame::{MediaPacket, MediaFrame};
pub use trait_::{
    DecodeError, DecodeResult, Decoder, DecoderInfo, DecoderStats, StatsDecoder,
};

// 音频解码器导出
// G.711 解码器（纯 Rust，始终可用）
pub use audio::{G711ADecoder, G711UDecoder};
// AAC/MP3/Opus 解码器（需要 FFI）
#[cfg(feature = "decoder-ffi")]
pub use audio::AacDecoder;
#[cfg(feature = "decoder-ffi")]
pub use audio::Mp3Decoder;
#[cfg(feature = "decoder-ffi")]
pub use audio::OpusDecoder;

// 条件导出
#[cfg(feature = "decoder-rust")]
pub use mjpeg::{MjpegDecoder, get_jpeg_dimensions};

#[cfg(feature = "decoder-ffi")]
pub use h264::H264Decoder;

#[cfg(feature = "decoder-ffi")]
pub use h265::H265Decoder;

#[cfg(feature = "decoder-ffi")]
pub use ffi::{FfiDecoder, DecoderHandle, DecodedFrame, FfiErrorCode};

/// 创建指定编解码类型的解码器
///
/// # 参数
/// * `codec` - 编解码器类型
///
/// # 返回
/// * `Ok(decoder)` - 成功创建解码器
/// * `Err(e)` - 创建失败（不支持的编解码器或初始化失败）
///
/// # 示例
/// ```
/// use rtp::decoder::{create_decoder, CodecType};
///
/// let decoder = create_decoder(CodecType::MJPEG).unwrap();
/// ```
pub fn create_decoder(codec: CodecType) -> DecodeResult<Box<dyn Decoder + Send>> {
    match codec {
        // 视频编解码器
        CodecType::MJPEG => {
            #[cfg(feature = "decoder-rust")]
            {
                Ok(Box::new(MjpegDecoder::new()))
            }
            #[cfg(not(feature = "decoder-rust"))]
            {
                Err(DecodeError::UnsupportedCodec(codec))
            }
        }
        CodecType::H264 => {
            #[cfg(feature = "decoder-ffi")]
            {
                Ok(Box::new(H264Decoder::new()?))
            }
            #[cfg(not(feature = "decoder-ffi"))]
            {
                Err(DecodeError::UnsupportedCodec(codec))
            }
        }
        CodecType::H265 => {
            #[cfg(feature = "decoder-ffi")]
            {
                Ok(Box::new(H265Decoder::new()?))
            }
            #[cfg(not(feature = "decoder-ffi"))]
            {
                Err(DecodeError::UnsupportedCodec(codec))
            }
        }
        // 音频编解码器
        // G.711 解码器始终可用（纯 Rust）
        // AAC/MP3/Opus 解码器需要 decoder-ffi feature
        CodecType::AAC | CodecType::MP3 | CodecType::OPUS | CodecType::G711A | CodecType::G711U => {
            audio::create_audio_decoder(codec)
        }
        _ => Err(DecodeError::UnsupportedCodec(codec)),
    }
}

/// 创建指定编解码类型和输出格式的解码器
///
/// # 参数
/// * `codec` - 编解码器类型
/// * `output_format` - 输出像素格式
///
/// # 返回
/// * `Ok(decoder)` - 成功创建解码器
/// * `Err(e)` - 创建失败
pub fn create_decoder_with_format(
    codec: CodecType,
    output_format: PixelFormat,
) -> DecodeResult<Box<dyn Decoder + Send>> {
    match codec {
        CodecType::MJPEG => {
            #[cfg(feature = "decoder-rust")]
            {
                Ok(Box::new(MjpegDecoder::with_output_format(output_format)))
            }
            #[cfg(not(feature = "decoder-rust"))]
            {
                Err(DecodeError::UnsupportedCodec(codec))
            }
        }
        CodecType::H264 => {
            #[cfg(feature = "decoder-ffi")]
            {
                Ok(Box::new(H264Decoder::with_output_format(output_format)?))
            }
            #[cfg(not(feature = "decoder-ffi"))]
            {
                Err(DecodeError::UnsupportedCodec(codec))
            }
        }
        CodecType::H265 => {
            #[cfg(feature = "decoder-ffi")]
            {
                Ok(Box::new(H265Decoder::with_output_format(output_format)?))
            }
            #[cfg(not(feature = "decoder-ffi"))]
            {
                Err(DecodeError::UnsupportedCodec(codec))
            }
        }
        _ => Err(DecodeError::UnsupportedCodec(codec)),
    }
}

/// 解码器配置
#[derive(Debug, Clone)]
pub struct DecoderConfig {
    /// 编解码器类型
    pub codec_type: CodecType,
    /// 输出像素格式
    pub output_format: PixelFormat,
    /// 是否启用统计信息
    pub enable_stats: bool,
}

impl Default for DecoderConfig {
    fn default() -> Self {
        Self {
            codec_type: CodecType::MJPEG,
            output_format: PixelFormat::RGBA,
            enable_stats: false,
        }
    }
}

impl DecoderConfig {
    /// 创建新的解码器配置
    pub fn new(codec_type: CodecType) -> Self {
        Self {
            codec_type,
            ..Default::default()
        }
    }

    /// 设置输出像素格式
    pub fn with_output_format(mut self, format: PixelFormat) -> Self {
        self.output_format = format;
        self
    }

    /// 启用统计信息
    pub fn with_stats(mut self, enable: bool) -> Self {
        self.enable_stats = enable;
        self
    }

    /// 根据配置创建解码器
    pub fn create_decoder(&self) -> DecodeResult<Box<dyn Decoder + Send>> {
        let decoder = create_decoder_with_format(self.codec_type, self.output_format)?;

        if self.enable_stats {
            Ok(Box::new(StatsDecoder::new(decoder)))
        } else {
            Ok(decoder)
        }
    }
}

/// 检查指定编解码器是否支持
pub fn is_codec_supported(codec: CodecType) -> bool {
    match codec {
        // 视频编解码器
        CodecType::MJPEG => {
            #[cfg(feature = "decoder-rust")]
            {
                true
            }
            #[cfg(not(feature = "decoder-rust"))]
            {
                false
            }
        }
        CodecType::H264 | CodecType::H265 => {
            #[cfg(feature = "decoder-ffi")]
            {
                true
            }
            #[cfg(not(feature = "decoder-ffi"))]
            {
                false
            }
        }
        // 音频编解码器
        // G.711 解码器始终支持（纯 Rust）
        CodecType::G711A | CodecType::G711U => true,
        // AAC/MP3/Opus 解码器需要 decoder-ffi feature
        CodecType::AAC | CodecType::MP3 | CodecType::OPUS => {
            #[cfg(feature = "decoder-ffi")]
            {
                true
            }
            #[cfg(not(feature = "decoder-ffi"))]
            {
                false
            }
        }
        _ => false,
    }
}

/// 获取支持的解码器列表
pub fn supported_codecs() -> Vec<CodecType> {
    let mut codecs = Vec::new();

    // 视频编解码器
    #[cfg(feature = "decoder-rust")]
    {
        codecs.push(CodecType::MJPEG);
    }

    #[cfg(feature = "decoder-ffi")]
    {
        codecs.push(CodecType::H264);
        codecs.push(CodecType::H265);
    }

    // 音频编解码器
    // G.711 解码器始终可用（纯 Rust）
    codecs.push(CodecType::G711A);
    codecs.push(CodecType::G711U);
    
    // AAC/MP3/Opus 解码器需要 decoder-ffi feature
    #[cfg(feature = "decoder-ffi")]
    {
        codecs.push(CodecType::AAC);
        codecs.push(CodecType::MP3);
        codecs.push(CodecType::OPUS);
    }

    codecs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mjpeg_decoder() {
        #[cfg(feature = "decoder-rust")]
        {
            let result = create_decoder(CodecType::MJPEG);
            assert!(result.is_ok());
            let decoder = result.unwrap();
            assert_eq!(decoder.codec_type(), CodecType::MJPEG);
        }
    }

    #[test]
    fn test_create_unsupported_codec() {
        let result = create_decoder(CodecType::H265);
        // 如果 decoder-ffi feature 未启用，应该返回错误
        if !cfg!(feature = "decoder-ffi") {
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_decoder_config() {
        let config = DecoderConfig::new(CodecType::MJPEG)
            .with_output_format(PixelFormat::RGB)
            .with_stats(true);

        assert_eq!(config.codec_type, CodecType::MJPEG);
        assert_eq!(config.output_format, PixelFormat::RGB);
        assert!(config.enable_stats);
    }

    #[test]
    fn test_is_codec_supported() {
        #[cfg(feature = "decoder-rust")]
        {
            assert!(is_codec_supported(CodecType::MJPEG));
        }

        #[cfg(feature = "decoder-ffi")]
        {
            assert!(is_codec_supported(CodecType::H264));
            assert!(is_codec_supported(CodecType::H265));
        }
    }

    #[test]
    fn test_supported_codecs() {
        let codecs = supported_codecs();
        assert!(!codecs.is_empty());
    }
}
