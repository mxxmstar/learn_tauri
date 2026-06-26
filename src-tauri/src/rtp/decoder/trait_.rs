//! 解码器 Trait 定义
//!
//! 定义统一的解码器接口，支持多种编解码器

use crate::rtp::decoder::frame::{MediaFrame, MediaPacket};
use crate::rtp::decoder::types::CodecType;
use crate::rtp::error::RtpError;

/// 解码器结果类型
pub type DecodeResult<T> = std::result::Result<T, DecodeError>;

/// 解码器错误类型
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// 不支持的编解码器
    #[error("unsupported codec: {0:?}")]
    UnsupportedCodec(CodecType),

    /// 解码失败
    #[error("decode failed: {0}")]
    DecodeFailed(String),

    /// 无效的数据
    #[error("invalid data: {0}")]
    InvalidData(String),

    /// 内部错误
    #[error("internal error: {0}")]
    InternalError(String),

    /// 参数错误
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// 缓冲区溢出
    #[error("buffer overflow")]
    BufferOverflow,

    /// 超时
    #[error("timeout")]
    Timeout,

    /// 底层错误
    #[error("underlying error: {0}")]
    Underlying(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<RtpError> for DecodeError {
    fn from(err: RtpError) -> Self {
        DecodeError::InternalError(err.to_string())
    }
}

impl From<String> for DecodeError {
    fn from(err: String) -> Self {
        DecodeError::DecodeFailed(err)
    }
}

/// 统一的解码器 Trait
///
/// 所有解码器实现此 trait，提供统一的解码接口
pub trait Decoder: Send {
    /// 解码一个编码包
    ///
    /// 对于需要多个包才能解码出一帧的编解码器（如 H264/H265），
    /// 此函数可能返回 None，直到有足够的数据
    ///
    /// # 参数
    /// * `packet` - 编码数据包
    ///
    /// # 返回
    /// * `Ok(Some(frame))` - 成功解码出一帧
    /// * `Ok(None)` - 数据已缓存，尚未有足够数据解码出完整帧
    /// * `Err(e)` - 解码失败
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>>;

    /// 刷新解码器，返回所有缓存的帧
    ///
    /// 在流结束时调用，以获取所有已解码但未输出的帧
    ///
    /// # 返回
    /// * `Ok(frames)` - 所有缓存的帧
    /// * `Err(e)` - 刷新失败
    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>>;

    /// 重置解码器状态
    ///
    /// 清除缓存，重置内部状态，准备解码新的流
    fn reset(&mut self);

    /// 获取此解码器支持的编解码器类型
    fn codec_type(&self) -> CodecType;

    /// 获取解码器名称（用于日志和调试）
    fn name(&self) -> &str;

    /// 获取解码器信息
    fn info(&self) -> DecoderInfo {
        DecoderInfo {
            name: self.name().to_string(),
            codec_type: self.codec_type(),
        }
    }
}

/// 解码器信息
#[derive(Debug, Clone)]
pub struct DecoderInfo {
    /// 解码器名称
    pub name: String,
    /// 支持的编解码器类型
    pub codec_type: CodecType,
}

/// 解码器统计信息
#[derive(Debug, Clone, Default)]
pub struct DecoderStats {
    /// 输入包数量
    pub packets_in: u64,
    /// 输出帧数量
    pub frames_out: u64,
    /// 解码错误数量
    pub decode_errors: u64,
    /// 丢帧数量
    pub frames_dropped: u64,
}

/// 带统计信息的解码器包装器
///
/// 包装一个 Decoder，自动统计解码性能
pub struct StatsDecoder<D: Decoder> {
    inner: D,
    stats: DecoderStats,
}

impl<D: Decoder> StatsDecoder<D> {
    /// 创建新的带统计信息的解码器
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            stats: DecoderStats::default(),
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> &DecoderStats {
        &self.stats
    }

    /// 获取可变统计信息
    pub fn stats_mut(&mut self) -> &mut DecoderStats {
        &mut self.stats
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats = DecoderStats::default();
    }

    /// 获取内部解码器
    pub fn inner(&self) -> &D {
        &self.inner
    }

    /// 获取可变内部解码器
    pub fn inner_mut(&mut self) -> &mut D {
        &mut self.inner
    }
}

impl<D: Decoder> Decoder for StatsDecoder<D> {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
        self.stats.packets_in += 1;

        match self.inner.decode(packet) {
            Ok(Some(frame)) => {
                self.stats.frames_out += 1;
                Ok(Some(frame))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                self.stats.decode_errors += 1;
                Err(e)
            }
        }
    }

    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>> {
        match self.inner.flush() {
            Ok(frames) => {
                self.stats.frames_out += frames.len() as u64;
                Ok(frames)
            }
            Err(e) => {
                self.stats.decode_errors += 1;
                Err(e)
            }
        }
    }

    fn reset(&mut self) {
        self.inner.reset();
        self.reset_stats();
    }

    fn codec_type(&self) -> CodecType {
        self.inner.codec_type()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn info(&self) -> DecoderInfo {
        self.inner.info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtp::decoder::frame::MediaPacket;
    use crate::rtp::decoder::types::{CodecType, PixelFormat};
    use bytes::Bytes;

    /// 测试用的空解码器
    struct DummyDecoder {
        codec_type: CodecType,
    }

    impl Decoder for DummyDecoder {
        fn decode(&mut self, _packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
            Ok(None)
        }

        fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>> {
            Ok(Vec::new())
        }

        fn reset(&mut self) {
            // do nothing
        }

        fn codec_type(&self) -> CodecType {
            self.codec_type
        }

        fn name(&self) -> &str {
            "DummyDecoder"
        }
    }

    #[test]
    fn test_decoder_info() {
        let decoder = DummyDecoder {
            codec_type: CodecType::H264,
        };
        let info = decoder.info();
        assert_eq!(info.name, "DummyDecoder");
        assert_eq!(info.codec_type, CodecType::H264);
    }

    #[test]
    fn test_stats_decoder() {
        let dummy = DummyDecoder {
            codec_type: CodecType::H264,
        };
        let mut decoder = StatsDecoder::new(dummy);

        // 检查初始统计
        assert_eq!(decoder.stats().packets_in, 0);
        assert_eq!(decoder.stats().frames_out, 0);

        // 检查信息
        assert_eq!(decoder.name(), "DummyDecoder");
        assert_eq!(decoder.codec_type(), CodecType::H264);

        // 重置统计
        decoder.reset_stats();
        assert_eq!(decoder.stats().packets_in, 0);
    }
}

/// 为 Box<dyn Decoder + Send> 实现 Decoder trait
/// 这样 Box<dyn Decoder + Send> 可以用作 Decoder
impl Decoder for Box<dyn Decoder + Send> {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
        (**self).decode(packet)
    }
    
    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>> {
        (**self).flush()
    }
    
    fn reset(&mut self) {
        (**self).reset()
    }
    
    fn codec_type(&self) -> CodecType {
        (**self).codec_type()
    }
    
    fn name(&self) -> &str {
        (**self).name()
    }
    
    fn info(&self) -> DecoderInfo {
        (**self).info()
    }
}
