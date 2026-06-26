//! 录像模块
//!
//! 提供视频录像功能，支持将 RTP/AVTP 流保存为 MP4 (H.264/H.265) 或 AVI (MJPEG) 文件
//! 包含开始/结束时间戳记录

// 模块声明
mod error;
mod config;
mod trait_;

// 条件编译模块
#[cfg(feature = "recorder-ffi")]
mod ffi;

#[cfg(feature = "recorder-mp4")]
mod mp4;

#[cfg(feature = "recorder-avi")]
mod avi;

// 公开导出
pub use error::{RecordError, RecordResult};
pub use config::{ContainerFormat, RecorderConfig};
pub use trait_::{Recorder, RecorderInfo, RecordStats};

// 条件导出
#[cfg(feature = "recorder-ffi")]
pub use ffi::{FfiRecorder, RecorderHandle, RecorderStatsFFI};

#[cfg(feature = "recorder-mp4")]
pub use mp4::Mp4Recorder;

#[cfg(feature = "recorder-avi")]
pub use avi::AviRecorder;

use crate::rtp::decoder::CodecType;

/// 创建录像器
///
/// # 参数
/// * `codec` - 编解码器类型
///
/// # 返回
/// * `Ok(recorder)` - 成功创建录像器
/// * `Err(e)` - 创建失败（不支持的编解码器或初始化失败）
///
/// # 示例
/// ```no_run
/// use crate::recorder::{create_recorder, RecorderConfig};
/// use crate::rtp::decoder::CodecType;
/// use std::path::PathBuf;
///
/// let config = RecorderConfig::new(CodecType::H264, PathBuf::from("output.mp4"));
/// let mut recorder = create_recorder(&config).unwrap();
/// ```
pub fn create_recorder(config: &RecorderConfig) -> RecordResult<Box<dyn Recorder + Send>> {
    // 验证配置
    config.validate()
        .map_err(|e| RecordError::InvalidArgument(e))?;

    // 根据 feature 标志选择实现
    #[cfg(feature = "recorder-ffi")]
    {
        // 使用 FFI 实现
        let codec_type = config.codec_type as i32;
        let container_format = config.get_container_format()
            .ok_or_else(|| RecordError::InvalidArgument("无法确定容器格式".to_string()))? as i32;
        
        let output_path = config.output_path.to_str()
            .ok_or_else(|| RecordError::InvalidArgument("无效的输出路径".to_string()))?;
        
        // 创建 FFI 录像器（不自动 start，由调用者通过 Recorder trait 的 start 方法启动）
        let recorder = FfiRecorder::new(codec_type, container_format, output_path)
            .map_err(|e| RecordError::InitError(e))?;
        
        Ok(Box::new(recorder))
    }

    #[cfg(not(feature = "recorder-ffi"))]
    {
        // 未启用 FFI 时返回错误
        Err(RecordError::InitError(
            "录像器尚未实现，请启用 'recorder-ffi' feature".to_string()
        ))
    }
}

/// 检查指定编解码器是否支持录像
pub fn is_codec_supported(codec: CodecType) -> bool {
    matches!(codec, CodecType::H264 | CodecType::H265 | CodecType::MJPEG)
}

/// 获取支持的编解码器列表
pub fn supported_codecs() -> Vec<CodecType> {
    vec![CodecType::H264, CodecType::H265, CodecType::MJPEG]
}

/// 获取支持的容器格式列表
pub fn supported_containers() -> Vec<ContainerFormat> {
    vec![ContainerFormat::MP4, ContainerFormat::AVI]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_container_format_from_codec() {
        assert_eq!(
            ContainerFormat::from_codec(CodecType::H264),
            Some(ContainerFormat::MP4)
        );
        assert_eq!(
            ContainerFormat::from_codec(CodecType::H265),
            Some(ContainerFormat::MP4)
        );
        assert_eq!(
            ContainerFormat::from_codec(CodecType::MJPEG),
            Some(ContainerFormat::AVI)
        );
    }

    #[test]
    fn test_container_format_is_codec_supported() {
        assert!(ContainerFormat::MP4.is_codec_supported(CodecType::H264));
        assert!(ContainerFormat::MP4.is_codec_supported(CodecType::H265));
        assert!(!ContainerFormat::MP4.is_codec_supported(CodecType::MJPEG));

        assert!(ContainerFormat::AVI.is_codec_supported(CodecType::MJPEG));
        assert!(!ContainerFormat::AVI.is_codec_supported(CodecType::H264));
    }

    #[test]
    fn test_recorder_config_new() {
        let config = RecorderConfig::new(
            CodecType::H264,
            PathBuf::from("test.mp4")
        );
        assert_eq!(config.codec_type, CodecType::H264);
        assert_eq!(config.get_container_format(), Some(ContainerFormat::MP4));
    }

    #[test]
    fn test_recorder_config_with_container_format() {
        let config = RecorderConfig::new(
            CodecType::MJPEG,
            PathBuf::from("test.avi")
        )
        .with_container_format(ContainerFormat::AVI);
        assert_eq!(config.get_container_format(), Some(ContainerFormat::AVI));
    }

    #[test]
    fn test_recorder_config_validate() {
        // 有效的配置
        let config = RecorderConfig::new(
            CodecType::H264,
            PathBuf::from("test.mp4")
        );
        assert!(config.validate().is_ok());

        // 无效的配置：容器格式不支持该编解码器
        let config = RecorderConfig::new(
            CodecType::MJPEG,
            PathBuf::from("test.mp4")
        )
        .with_container_format(ContainerFormat::MP4);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_record_stats() {
        let mut stats = RecordStats::new();
        assert!(stats.start_time.is_none());
        assert!(stats.end_time.is_none());

        stats.set_start_time();
        assert!(stats.start_time.is_some());
        assert!(stats.start_timestamp_ms.is_some());

        stats.set_end_time();
        assert!(stats.end_time.is_some());
        assert!(stats.end_timestamp_ms.is_some());
        assert!(stats.duration_ms.is_some());
    }

    #[test]
    fn test_is_codec_supported() {
        assert!(is_codec_supported(CodecType::H264));
        assert!(is_codec_supported(CodecType::H265));
        assert!(is_codec_supported(CodecType::MJPEG));
        assert!(!is_codec_supported(CodecType::AAC));
    }

    #[test]
    fn test_supported_codecs() {
        let codecs = supported_codecs();
        assert!(codecs.contains(&CodecType::H264));
        assert!(codecs.contains(&CodecType::H265));
        assert!(codecs.contains(&CodecType::MJPEG));
    }
}
