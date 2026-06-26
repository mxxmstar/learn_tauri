//! 录像模块配置结构

use crate::rtp::decoder::CodecType;
use std::fmt;
use std::path::PathBuf;

/// 容器格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerFormat {
    /// MP4 容器（用于 H.264/H.265）
    MP4,
    /// AVI 容器（用于 MJPEG）
    AVI,
}

impl ContainerFormat {
    /// 根据编解码器自动选择合适的容器格式
    pub fn from_codec(codec: CodecType) -> Option<Self> {
        match codec {
            CodecType::H264 | CodecType::H265 => Some(ContainerFormat::MP4),
            CodecType::MJPEG => Some(ContainerFormat::AVI),
            _ => None,
        }
    }

    /// 检查编解码器是否支持该容器格式
    pub fn is_codec_supported(&self, codec: CodecType) -> bool {
        match self {
            ContainerFormat::MP4 => matches!(codec, CodecType::H264 | CodecType::H265),
            ContainerFormat::AVI => matches!(codec, CodecType::MJPEG),
        }
    }
}

impl fmt::Display for ContainerFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerFormat::MP4 => write!(f, "MP4"),
            ContainerFormat::AVI => write!(f, "AVI"),
        }
    }
}

/// 录像配置
#[derive(Debug, Clone)]
pub struct RecorderConfig {
    /// 编解码器类型
    pub codec_type: CodecType,
    /// 容器格式（如果为 None，则根据编解码器自动选择）
    pub container_format: Option<ContainerFormat>,
    /// 输出文件路径
    pub output_path: PathBuf,
    /// 视频宽度（可选，用于某些编码器）
    pub width: Option<u32>,
    /// 视频高度（可选，用于某些编码器）
    pub height: Option<u32>,
    /// 帧率（可选，用于某些编码器）
    pub framerate: Option<f64>,
    /// 是否启用时间戳记录
    pub enable_timestamp: bool,
}

impl RecorderConfig {
    /// 创建新的录像配置
    pub fn new(codec_type: CodecType, output_path: PathBuf) -> Self {
        Self {
            codec_type,
            container_format: ContainerFormat::from_codec(codec_type),
            output_path,
            width: None,
            height: None,
            framerate: None,
            enable_timestamp: true,
        }
    }

    /// 设置容器格式
    pub fn with_container_format(mut self, format: ContainerFormat) -> Self {
        self.container_format = Some(format);
        self
    }

    /// 设置视频尺寸
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// 设置帧率
    pub fn with_framerate(mut self, framerate: f64) -> Self {
        self.framerate = Some(framerate);
        self
    }

    /// 启用/禁用时间戳记录
    pub fn with_timestamp(mut self, enable: bool) -> Self {
        self.enable_timestamp = enable;
        self
    }

    /// 获取有效的容器格式（如果未设置则自动选择）
    pub fn get_container_format(&self) -> Option<ContainerFormat> {
        self.container_format
            .or_else(|| ContainerFormat::from_codec(self.codec_type))
    }

    /// 验证配置是否有效
    pub fn validate(&self) -> Result<(), String> {
        // 检查容器格式是否支持该编解码器
        if let Some(container) = self.get_container_format() {
            if !container.is_codec_supported(self.codec_type) {
                return Err(format!(
                    "容器格式 {:?} 不支持编解码器 {:?}",
                    container, self.codec_type
                ));
            }
        } else {
            return Err(format!(
                "无法为编解码器 {:?} 确定容器格式",
                self.codec_type
            ));
        }

        // 检查输出路径的父目录是否存在（如果有父目录的话）
        if let Some(parent) = self.output_path.parent() {
            // 只有当父目录不是空路径或当前目录时，才检查是否存在
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(format!(
                    "输出目录不存在: {}",
                    parent.display()
                ));
            }
        }

        Ok(())
    }
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            codec_type: CodecType::H264,
            container_format: Some(ContainerFormat::MP4),
            output_path: PathBuf::from("output.mp4"),
            width: None,
            height: None,
            framerate: Some(30.0),
            enable_timestamp: true,
        }
    }
}

