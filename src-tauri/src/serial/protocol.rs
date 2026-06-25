//! 串口协议解析模块
//!
//! 定义 `ProtocolParser` trait，用于支持自定义协议解析器。
//! 提供内置的常见协议解析器实现：
//! - `DelimiterParser`：基于分隔符的帧解析（如换行符、自定义字节序列）
//! - `LengthPrefixParser`：基于长度前缀的帧解析（如长度字段 + 数据）
//!
//! # 自定义协议扩展
//!
//! 用户可以实现 `ProtocolParser` trait 来支持自定义协议：
//!
//! ```rust
//! use crate::serial::protocol::{ProtocolParser, ParseResult};
//!
//! struct MyProtocolParser;
//!
//! impl ProtocolParser for MyProtocolParser {
//!     fn parse_frame(&self, buffer: &[u8]) -> ParseResult {
//!         // 自定义解析逻辑
//!         // ...
//!         ParseResult::Incomplete
//!     }
//!
//!     fn encode_frame(&self, data: &[u8]) -> Vec<u8> {
//!         // 自定义编码逻辑
//!         // ...
//!         data.to_vec()
//!     }
//! }
//! ```

// std::sync::Arc 在此模块中未使用，已移除导入

/// 解析结果
///
/// 表示协议解析的结果状态。
pub enum ParseResult {
    /// 解析成功，返回完整帧数据和使用的字节数
    ///
    /// 第一个元素是帧数据（不包含分隔符或包含，取决于具体实现）
    /// 第二个元素是消耗的缓冲区字节数
    Complete(Vec<u8>, usize),

    /// 数据不完整，需要更多数据
    ///
    /// 解析器无法从当前缓冲区中提取完整帧
    Incomplete,

    /// 解析错误
    ///
    /// 数据格式错误，无法解析
    Error(String),
}

/// 协议解析器 Trait
///
/// 定义协议解析器的接口，用户可实现此 trait 来支持自定义协议。
///
/// # 设计目标
///
/// 1. **灵活性**：支持任意协议格式
/// 2. **零拷贝**：尽可能减少数据拷贝
/// 3. **可组合**：解析器可以嵌套组合
///
/// # 示例
///
/// ```rust
/// use crate::serial::protocol::{ProtocolParser, ParseResult};
///
/// /// 自定义协议解析器
/// struct MyProtocol;
///
/// impl ProtocolParser for MyProtocol {
///     fn parse_frame(&self, buffer: &[u8]) -> ParseResult {
///         // 实现自定义解析逻辑
///         // ...
///         ParseResult::Incomplete
///     }
///
///     fn encode_frame(&self, data: &[u8]) -> Vec<u8> {
///         // 实现自定义编码逻辑
///         // ...
///         data.to_vec()
///     }
/// }
/// ```
pub trait ProtocolParser: Send + Sync {
    /// 解析数据帧
    ///
    /// 从缓冲区中提取完整的数据帧。
    ///
    /// # 参数
    /// * `buffer` - 接收到的数据缓冲区
    ///
    /// # 返回值
    /// * `ParseResult::Complete(data, consumed)` - 解析成功，返回帧数据和消耗的字节数
    /// * `ParseResult::Incomplete` - 数据不完整，需要更多数据
    /// * `ParseResult::Error(msg)` - 解析错误
    fn parse_frame(&self, buffer: &[u8]) -> ParseResult;

    /// 编码数据帧
    ///
    /// 将数据封装为协议帧格式（添加帧头、帧尾、校验等）。
    ///
    /// # 参数
    /// * `data` - 要发送的原始数据
    ///
    /// # 返回值
    /// 返回编码后的完整帧数据
    fn encode_frame(&self, data: &[u8]) -> Vec<u8>;
}

/// 基于分隔符的帧解析器
///
/// 使用指定的分隔符来识别数据帧边界。
/// 常见用法：
/// - 换行符分隔（如文本协议，类似 Telnet）
/// - 自定义字节序列分隔（如 0xAA, 0xBB）
///
/// # 示例
///
/// ```rust
/// use crate::serial::protocol::DelimiterParser;
///
/// // 使用换行符作为分隔符（类似 Telnet 文本协议）
/// let parser = DelimiterParser::new(b"\n");
///
/// // 使用自定义分隔符
/// let parser = DelimiterParser::new(&[0xAA, 0xBB]);
/// ```
#[derive(Debug, Clone)]
pub struct DelimiterParser {
    /// 分隔符字节序列
    delimiter: Vec<u8>,
    /// 是否包含分隔符在返回的数据中
    include_delimiter: bool,
    /// 最大帧长度（防止缓冲区无限增长）
    max_frame_length: usize,
}

impl DelimiterParser {
    /// 创建新的分隔符解析器
    ///
    /// # 参数
    /// * `delimiter` - 分隔符字节序列
    pub fn new(delimiter: &[u8]) -> Self {
        Self {
            delimiter: delimiter.to_vec(),
            include_delimiter: false, // 默认不包含分隔符
            max_frame_length: 64 * 1024, // 默认最大 64KB
        }
    }

    /// 设置是否包含分隔符（链式调用）
    ///
    /// # 参数
    /// * `include` - 是否包含分隔符
    pub fn include_delimiter(mut self, include: bool) -> Self {
        self.include_delimiter = include;
        self
    }

    /// 设置最大帧长度（链式调用）
    ///
    /// # 参数
    /// * `max_length` - 最大帧长度（字节）
    pub fn max_frame_length(mut self, max_length: usize) -> Self {
        self.max_frame_length = max_length;
        self
    }
}

impl ProtocolParser for DelimiterParser {
    fn parse_frame(&self, buffer: &[u8]) -> ParseResult {
        // 检查缓冲区长度
        if buffer.len() > self.max_frame_length {
            return ParseResult::Error(format!(
                "帧长度超过最大限制: {} > {}",
                buffer.len(),
                self.max_frame_length
            ));
        }

        // 查找分隔符位置
        for i in 0..buffer.len() {
            // 检查从位置 i 开始是否匹配分隔符
            if buffer[i..].len() >= self.delimiter.len() &&
               buffer[i..i + self.delimiter.len()] == self.delimiter[..] {
                // 找到分隔符
                let frame_end = if self.include_delimiter {
                    i + self.delimiter.len()
                } else {
                    i
                };
                let frame_data = buffer[..frame_end].to_vec();
                return ParseResult::Complete(frame_data, frame_end);
            }
        }

        // 未找到分隔符，数据不完整
        ParseResult::Incomplete
    }

    fn encode_frame(&self, data: &[u8]) -> Vec<u8> {
        // 在数据末尾添加分隔符
        let mut frame = data.to_vec();
        frame.extend_from_slice(&self.delimiter);
        frame
    }
}

/// 基于长度前缀的帧解析器
///
/// 使用固定长度的前缀字段来指示后续数据的长度。
/// 常见用法：
/// - 二进制协议（如 Modbus TCP、自定义二进制协议）
/// - 网络协议（如 TCP 粘包处理）
///
/// # 数据格式
///
/// ```text
/// +----------------+-----------------------------------+
/// | 长度字段 (N字节) | 数据负载 (长度字段指定的字节数) |
/// +----------------+-----------------------------------+
/// ```
///
/// # 示例
///
/// ```rust
/// use crate::serial::protocol::LengthPrefixParser;
///
/// // 使用 2 字节大端序长度前缀
/// let parser = LengthPrefixParser::new(2, true);
///
/// // 使用 4 字节小端序长度前缀
/// let parser = LengthPrefixParser::new(4, false);
/// ```
#[derive(Debug, Clone)]
pub struct LengthPrefixParser {
    /// 长度字段的字节数（1, 2, 4）
    length_field_length: usize,
    /// 长度字段的字节序（true = 大端序，false = 小端序）
    is_big_endian: bool,
    /// 长度字段是否包含自身长度
    ///
    /// 如果为 true，长度字段的值包含长度字段自身的长度
    /// 如果为 false，长度字段的值仅包含数据负载的长度
    length_includes_self: bool,
    /// 最大帧长度（防止恶意数据导致内存耗尽）
    max_frame_length: usize,
}

impl LengthPrefixParser {
    /// 创建新的长度前缀解析器
    ///
    /// # 参数
    /// * `length_field_length` - 长度字段的字节数（支持 1, 2, 4）
    /// * `is_big_endian` - 长度字段的字节序（true = 大端序，false = 小端序）
    ///
    /// # 返回值
    /// 返回 Result，成功时包含 LengthPrefixParser 实例
    pub fn new(length_field_length: usize, is_big_endian: bool) -> Result<Self, String> {
        // 验证长度字段字节数
        if ![1, 2, 4].contains(&length_field_length) {
            return Err(format!(
                "长度字段字节数必须为 1, 2 或 4，当前值: {}",
                length_field_length
            ));
        }

        Ok(Self {
            length_field_length,
            is_big_endian,
            length_includes_self: false,
            max_frame_length: 64 * 1024 * 1024, // 默认最大 64MB
        })
    }

    /// 设置长度字段是否包含自身长度（链式调用）
    pub fn length_includes_self(mut self, includes: bool) -> Self {
        self.length_includes_self = includes;
        self
    }

    /// 设置最大帧长度（链式调用）
    pub fn max_frame_length(mut self, max_length: usize) -> Self {
        self.max_frame_length = max_length;
        self
    }

    /// 从缓冲区读取长度字段的值
    fn read_length_field(&self, buffer: &[u8]) -> u64 {
        match self.length_field_length {
            1 => buffer[0] as u64,
            2 => {
                let mut bytes = [0u8; 2];
                bytes.copy_from_slice(&buffer[..2]);
                if self.is_big_endian {
                    u16::from_be_bytes(bytes) as u64
                } else {
                    u16::from_le_bytes(bytes) as u64
                }
            }
            4 => {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&buffer[..4]);
                if self.is_big_endian {
                    u32::from_be_bytes(bytes) as u64
                } else {
                    u32::from_le_bytes(bytes) as u64
                }
            }
            _ => 0, // 不会执行到这里，因为构造函数已验证
        }
    }
}

impl ProtocolParser for LengthPrefixParser {
    fn parse_frame(&self, buffer: &[u8]) -> ParseResult {
        // 检查是否有足够的字节读取长度字段
        if buffer.len() < self.length_field_length {
            return ParseResult::Incomplete;
        }

        // 读取长度字段的值
        let length_field_value = self.read_length_field(buffer);

        // 计算数据负载的长度
        let payload_length = if self.length_includes_self {
            // 长度字段包含自身长度，需要减去
            if length_field_value < self.length_field_length as u64 {
                return ParseResult::Error("长度字段值异常（小于自身长度）".to_string());
            }
            length_field_value - self.length_field_length as u64
        } else {
            length_field_value
        };

        // 检查帧长度是否超过最大限制
        let total_frame_length = self.length_field_length as u64 + payload_length;
        if total_frame_length > self.max_frame_length as u64 {
            return ParseResult::Error(format!(
                "帧长度超过最大限制: {} > {}",
                total_frame_length, self.max_frame_length
            ));
        }

        // 检查是否有足够的数据
        if buffer.len() < total_frame_length as usize {
            return ParseResult::Incomplete;
        }

        // 提取完整帧
        let frame_data = buffer[..total_frame_length as usize].to_vec();
        ParseResult::Complete(frame_data, total_frame_length as usize)
    }

    fn encode_frame(&self, data: &[u8]) -> Vec<u8> {
        // 计算长度字段的值
        let payload_length = data.len();
        let length_field_value = if self.length_includes_self {
            payload_length + self.length_field_length
        } else {
            payload_length
        };

        // 构建帧
        let mut frame = Vec::with_capacity(self.length_field_length + payload_length);

        // 添加长度字段
        match self.length_field_length {
            1 => {
                frame.push(length_field_value as u8);
            }
            2 => {
                let bytes = if self.is_big_endian {
                    (length_field_value as u16).to_be_bytes()
                } else {
                    (length_field_value as u16).to_le_bytes()
                };
                frame.extend_from_slice(&bytes);
            }
            4 => {
                let bytes = if self.is_big_endian {
                    (length_field_value as u32).to_be_bytes()
                } else {
                    (length_field_value as u32).to_le_bytes()
                };
                frame.extend_from_slice(&bytes);
            }
            _ => {} // 不会执行到这里
        }

        // 添加数据负载
        frame.extend_from_slice(data);
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delimiter_parser() {
        let parser = DelimiterParser::new(b"\n");

        // 测试完整帧
        let buffer = b"hello\nworld\n";
        match parser.parse_frame(buffer) {
            ParseResult::Complete(data, consumed) => {
                assert_eq!(&data, b"hello");
                assert_eq!(consumed, 6); // "hello\n"
            }
            _ => panic!("应该解析成功"),
        }

        // 测试不完整帧
        let buffer = b"hello";
        match parser.parse_frame(buffer) {
            ParseResult::Incomplete => {}
            _ => panic!("应该返回 Incomplete"),
        }

        // 测试编码
        let encoded = parser.encode_frame(b"hello");
        assert_eq!(&encoded, b"hello\n");
    }

    #[test]
    fn test_length_prefix_parser() {
        // 使用 2 字节大端序长度前缀
        let parser = LengthPrefixParser::new(2, true).unwrap();

        // 构建测试数据：长度 = 5, 数据 = "hello"
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&5u16.to_be_bytes()); // 长度字段
        buffer.extend_from_slice(b"hello"); // 数据

        match parser.parse_frame(&buffer) {
            ParseResult::Complete(data, consumed) => {
                assert_eq!(&data[2..], b"hello");
                assert_eq!(consumed, 7);
            }
            _ => panic!("应该解析成功"),
        }

        // 测试不完整帧
        let buffer = vec![0x00, 0x05, 0x68]; // 只有部分数据
        match parser.parse_frame(&buffer) {
            ParseResult::Incomplete => {}
            _ => panic!("应该返回 Incomplete"),
        }

        // 测试编码
        let encoded = parser.encode_frame(b"hello");
        assert_eq!(&encoded[2..], b"hello");
    }
}
