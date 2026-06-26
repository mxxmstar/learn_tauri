//! 录像模块错误类型

use std::fmt;
use std::io;

/// 录像错误类型
#[derive(Debug, Clone)]
pub enum RecordError {
    /// 不支持的编解码器
    UnsupportedCodec(String),
    /// 不支持的容器格式
    UnsupportedContainer(String),
    /// 初始化失败
    InitError(String),
    /// 写入失败
    WriteError(String),
    /// 文件操作错误
    FileError(String),
    /// 时间戳错误
    TimestampError(String),
    /// 参数错误
    InvalidArgument(String),
    /// 录像未开始
    NotStarted,
    /// 录像已结束
    AlreadyFinished,
    /// FFI 调用错误
    FfiError(String),
    /// IO 错误
    IoError(String),
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordError::UnsupportedCodec(codec) => write!(f, "不支持的编解码器: {}", codec),
            RecordError::UnsupportedContainer(container) => write!(f, "不支持的容器格式: {}", container),
            RecordError::InitError(msg) => write!(f, "初始化失败: {}", msg),
            RecordError::WriteError(msg) => write!(f, "写入失败: {}", msg),
            RecordError::FileError(msg) => write!(f, "文件操作错误: {}", msg),
            RecordError::TimestampError(msg) => write!(f, "时间戳错误: {}", msg),
            RecordError::InvalidArgument(msg) => write!(f, "参数错误: {}", msg),
            RecordError::NotStarted => write!(f, "录像未开始"),
            RecordError::AlreadyFinished => write!(f, "录像已结束"),
            RecordError::FfiError(msg) => write!(f, "FFI 调用错误: {}", msg),
            RecordError::IoError(msg) => write!(f, "IO 错误: {}", msg),
        }
    }
}

impl std::error::Error for RecordError {}

impl From<io::Error> for RecordError {
    fn from(err: io::Error) -> Self {
        RecordError::IoError(err.to_string())
    }
}

/// 录像结果类型
pub type RecordResult<T> = Result<T, RecordError>;
