//! AVI 录像器实现
//!
//! 使用 FFI 对接 C++/FFmpeg 实现 AVI 封装（支持 MJPEG）

use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use crate::rtp::decoder::{CodecType, frame::MediaFrame};

use super::error::{RecordError, RecordResult};
use super::config::RecorderConfig;
use super::trait_::{Recorder, RecordStats};
use super::ffi::{FfiRecorder, RecorderHandle, RecorderStatsFFI};

/// AVI 录像器
///
/// 封装 FFI 接口，提供 AVI 封装功能（支持 MJPEG）
///
/// # 示例
///
/// ```no_run
/// use learn_tauri_lib::recorder::{AviRecorder, Recorder, RecorderConfig};
/// use learn_tauri_lib::rtp::decoder::CodecType;
/// use std::path::PathBuf;
///
/// let mut recorder = AviRecorder::new(
///     CodecType::MJPEG,
///     PathBuf::from("output.avi"),
/// )?;
///
/// let config = RecorderConfig::new(CodecType::MJPEG, PathBuf::from("output.avi"))
///     .with_dimensions(1920, 1080)
///     .with_framerate(30.0);
/// recorder.start(config)?;
///
/// // 写入视频帧
/// recorder.write_frame(&[0u8; 1024], Some(100))?;
///
/// recorder.finish()?;
/// # Ok::<(), learn_tauri_lib::recorder::RecordError>(())
/// ```
pub struct AviRecorder {
    /// FFI 录像器实例
    ffi_recorder: FfiRecorder,
    /// 录像配置
    config: RecorderConfig,
    /// 录像统计信息
    stats: RecordStats,
}

impl AviRecorder {
    /// 创建新的 AVI 录像器
    ///
    /// # 参数
    /// * `codec_type` - 编解码器类型（目前仅支持 MJPEG）
    /// * `output_path` - 输出文件路径
    ///
    /// # 返回
    /// * `Ok(AviRecorder)` - 成功创建
    /// * `Err(RecordError)` - 创建失败
    ///
    /// # 错误
    /// * `RecordError::InvalidArgument` - 编解码器不支持 AVI 封装
    /// * `RecordError::InitError` - 初始化 FFI 录像器失败
    pub fn new(codec_type: CodecType, output_path: PathBuf) -> RecordResult<Self> {
        // 检查编解码器是否支持 AVI 封装
        if !matches!(codec_type, CodecType::MJPEG) {
            return Err(RecordError::InvalidArgument(
                format!("AVI 封装不支持编解码器 {:?}", codec_type)
            ));
        }

        // 创建配置
        let config = RecorderConfig::new(codec_type, output_path)
            .with_container_format(super::config::ContainerFormat::AVI);

        // 验证配置
        config.validate()
            .map_err(|e| RecordError::InvalidArgument(e))?;

        // 创建 FFI 录像器
        let codec_type_i32 = codec_type as i32;
        let container_format_i32 = super::config::ContainerFormat::AVI as i32;
        let output_path_str = config.output_path.to_str()
            .ok_or_else(|| RecordError::InvalidArgument("无效的输出路径".to_string()))?;

        let ffi_recorder = FfiRecorder::new(
            codec_type_i32,
            container_format_i32,
            output_path_str,
        ).map_err(|e| RecordError::InitError(e))?;

        Ok(Self {
            ffi_recorder,
            config,
            stats: RecordStats::new(),
        })
    }

    /// 获取录像器信息
    pub fn get_info(&self) -> super::trait_::RecorderInfo {
        super::trait_::RecorderInfo {
            name: "AVI Recorder (FFI)".to_string(),
            supported_codecs: vec![CodecType::MJPEG],
            supported_containers: vec!["avi".to_string()],
            supports_timestamp: true,
        }
    }

    /// 获取 FFI 句柄（用于高级用法）
    pub fn get_handle(&self) -> RecorderHandle {
        self.ffi_recorder.handle()
    }
}

impl Recorder for AviRecorder {
    fn start(&mut self, config: RecorderConfig) -> RecordResult<()> {
        // 更新配置
        self.config = config;

        // 开始录像
        self.ffi_recorder.start(
            self.config.width,
            self.config.height,
            self.config.framerate,
        ).map_err(RecordError::InitError)?;

        // 更新统计信息
        self.stats.set_start_time();

        Ok(())
    }

    fn write_frame(&mut self, frame: &[u8], timestamp_ms: Option<u64>) -> RecordResult<()> {
        // 写入帧
        self.ffi_recorder.write_frame(frame, timestamp_ms, false)
            .map_err(|e| RecordError::WriteError(e))?;

        // 更新统计信息
        self.stats.add_frame(frame.len() as u64);

        Ok(())
    }

    fn write_media_frame(&mut self, frame: &MediaFrame) -> RecordResult<()> {
        // 转换时间戳
        let timestamp_ms = if frame.pts > 0 {
            Some((frame.pts / 1000) as u64)
        } else {
            None
        };

        // 写入帧
        self.ffi_recorder.write_frame(&frame.data, timestamp_ms, frame.keyframe)
            .map_err(|e| RecordError::WriteError(e))?;

        // 更新统计信息
        self.stats.add_frame(frame.data.len() as u64);

        Ok(())
    }

    fn finish(&mut self) -> RecordResult<()> {
        // 结束录像
        self.ffi_recorder.finish()
            .map_err(|e| RecordError::WriteError(e))?;

        // 更新统计信息
        self.stats.set_end_time();

        Ok(())
    }

    fn get_stats(&self) -> RecordStats {
        // 获取 FFI 统计信息
        if let Ok(ffi_stats) = self.ffi_recorder.get_stats() {
            let mut stats = RecordStats {
                start_time: if ffi_stats.start_timestamp_ms > 0 {
                    Some(UNIX_EPOCH + std::time::Duration::from_millis(ffi_stats.start_timestamp_ms))
                } else {
                    self.stats.start_time
                },
                end_time: if ffi_stats.end_timestamp_ms > 0 {
                    Some(UNIX_EPOCH + std::time::Duration::from_millis(ffi_stats.end_timestamp_ms))
                } else {
                    self.stats.end_time
                },
                start_timestamp_ms: if ffi_stats.start_timestamp_ms > 0 {
                    Some(ffi_stats.start_timestamp_ms)
                } else {
                    self.stats.start_timestamp_ms
                },
                end_timestamp_ms: if ffi_stats.end_timestamp_ms > 0 {
                    Some(ffi_stats.end_timestamp_ms)
                } else {
                    self.stats.end_timestamp_ms
                },
                frames_written: ffi_stats.frames_written,
                bytes_written: ffi_stats.bytes_written,
                duration_ms: if ffi_stats.duration_ms > 0 {
                    Some(ffi_stats.duration_ms)
                } else {
                    self.stats.duration_ms
                },
            };

            // 如果 FFI 统计信息不完整，使用本地统计信息
            if stats.frames_written == 0 {
                stats.frames_written = self.stats.frames_written;
            }
            if stats.bytes_written == 0 {
                stats.bytes_written = self.stats.bytes_written;
            }

            stats
        } else {
            self.stats.clone()
        }
    }

    fn is_recording(&self) -> bool {
        self.ffi_recorder.is_recording()
    }

    fn get_config(&self) -> Option<&RecorderConfig> {
        Some(&self.config)
    }

    fn cancel(&mut self) -> RecordResult<()> {
        // 取消录像
        self.ffi_recorder.cancel()
            .map_err(|e| RecordError::WriteError(e))?;

        // 更新统计信息
        self.stats.set_end_time();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_avi_recorder_invalid_codec() {
        // 测试使用不支持的编解码器创建 AVI 录像器
        // 此测试不调用 FFI，仅验证编解码器支持检查
        let result = AviRecorder::new(
            CodecType::H264,  // H264 不支持 AVI 封装
            PathBuf::from("test.avi"),
        );
        assert!(result.is_err());
        if let Err(RecordError::InvalidArgument(msg)) = result {
            assert!(msg.contains("不支持"));
        } else {
            panic!("Expected InvalidArgument error");
        }
    }
}
