//! 录像器 trait 定义

use crate::rtp::decoder::frame::MediaFrame;
use crate::rtp::decoder::CodecType;
use std::time::{SystemTime, UNIX_EPOCH};

use super::error::RecordResult;
use super::config::RecorderConfig;

/// 录像统计信息
#[derive(Debug, Clone, Default)]
pub struct RecordStats {
    /// 开始时间戳（系统时间）
    pub start_time: Option<SystemTime>,
    /// 结束时间戳（系统时间）
    pub end_time: Option<SystemTime>,
    /// 开始时间戳（Unix 毫秒）
    pub start_timestamp_ms: Option<u64>,
    /// 结束时间戳（Unix 毫秒）
    pub end_timestamp_ms: Option<u64>,
    /// 写入的帧数
    pub frames_written: u64,
    /// 写入的字节数
    pub bytes_written: u64,
    /// 录像持续时间（毫秒）
    pub duration_ms: Option<u64>,
}

impl RecordStats {
    /// 创建新的统计信息
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新开始时间戳
    pub fn set_start_time(&mut self) {
        self.start_time = Some(SystemTime::now());
        self.start_timestamp_ms = self.start_time
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64);
    }

    /// 更新结束时间戳
    pub fn set_end_time(&mut self) {
        self.end_time = Some(SystemTime::now());
        self.end_timestamp_ms = self.end_time
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64);

        // 计算持续时间
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            if let Ok(duration) = end.duration_since(start) {
                self.duration_ms = Some(duration.as_millis() as u64);
            }
        }
    }

    /// 更新写入统计
    pub fn add_frame(&mut self, size: u64) {
        self.frames_written += 1;
        self.bytes_written += size;
    }
}

/// 录像器 trait
///
/// 定义录像器的基本接口，支持将视频帧写入文件
pub trait Recorder {
    /// 开始录像
    ///
    /// # 参数
    /// * `config` - 录像配置
    ///
    /// # 返回
    /// * `Ok(())` - 成功开始录像
    /// * `Err(e)` - 开始失败
    fn start(&mut self, config: RecorderConfig) -> RecordResult<()>;

    /// 写入视频帧
    ///
    /// # 参数
    /// * `frame` - 视频帧数据
    /// * `timestamp_ms` - 时间戳（毫秒，可选）
    ///
    /// # 返回
    /// * `Ok(())` - 成功写入
    /// * `Err(e)` - 写入失败
    fn write_frame(&mut self, frame: &[u8], timestamp_ms: Option<u64>) -> RecordResult<()>;

    /// 写入 MediaFrame（更高级的接口）
    ///
    /// # 参数
    /// * `frame` - MediaFrame 对象
    ///
    /// # 返回
    /// * `Ok(())` - 成功写入
    /// * `Err(e)` - 写入失败
    fn write_media_frame(&mut self, frame: &MediaFrame) -> RecordResult<()> {
        // pts 是微秒，转换为毫秒
        // pts >= 0 时使用时间戳（0 也是有效时间戳）
        let timestamp_ms = if frame.pts >= 0 {
            Some((frame.pts / 1000) as u64)
        } else {
            None
        };
        self.write_frame(&frame.data, timestamp_ms)
    }

    /// 结束录像
    ///
    /// # 返回
    /// * `Ok(())` - 成功结束
    /// * `Err(e)` - 结束失败
    fn finish(&mut self) -> RecordResult<()>;

    /// 获取录像统计信息
    fn get_stats(&self) -> RecordStats;

    /// 检查是否正在录像
    fn is_recording(&self) -> bool;

    /// 获取当前配置
    fn get_config(&self) -> Option<&RecorderConfig>;

    /// 取消录像（不保存文件）
    fn cancel(&mut self) -> RecordResult<()> {
        // 默认实现：直接结束
        self.finish()
    }
}

/// 录像器信息
#[derive(Debug, Clone)]
pub struct RecorderInfo {
    /// 录像器名称
    pub name: String,
    /// 支持的编解码器列表
    pub supported_codecs: Vec<CodecType>,
    /// 支持的容器格式列表
    pub supported_containers: Vec<String>,
    /// 是否支持时间戳
    pub supports_timestamp: bool,
}

impl RecorderInfo {
    /// 创建新的录像器信息
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            supported_codecs: Vec::new(),
            supported_containers: Vec::new(),
            supports_timestamp: true,
        }
    }

    /// 添加支持的编解码器
    pub fn with_codec(mut self, codec: CodecType) -> Self {
        self.supported_codecs.push(codec);
        self
    }

    /// 添加支持的容器格式
    pub fn with_container(mut self, container: &str) -> Self {
        self.supported_containers.push(container.to_string());
        self
    }

    /// 设置是否支持时间戳
    pub fn with_timestamp(mut self, supports: bool) -> Self {
        self.supports_timestamp = supports;
        self
    }
}
