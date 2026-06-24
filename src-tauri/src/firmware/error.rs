/*!
 * 固件处理模块错误类型定义
 *
 * 该模块定义了固件处理过程中可能遇到的所有错误类型。
 *
 * # 错误类型说明
 *
 * ## 1. ZipError
 *
 * ZIP 文件解压错误，通常由 `zip` crate 返回。
 *
 * ## 2. IoError
 *
 * 文件 I/O 错误，如文件不存在、权限错误等。
 *
 * ## 3. FileNotFound
 *
 * 指定的文件在 ZIP 中不存在。
 *
 * ## 4. InvalidData
 *
 * 数据格式错误，如无效的固件文件、解密失败等。
 *
 * # 使用示例
 *
 * ```rust
 * use crate::firmware::error::FirmwareError;
 *
 * fn process_firmware() -> Result<(), FirmwareError> {
 *     let zip_data = std::fs::read("firmware.zip")?;
 *     let (file_name, file_data) = extract_firmware_from_zip(&zip_data, "rev1.img")?;
 *     Ok(())
 * }
 * ```
 */

use thiserror::Error;

/// 固件处理模块错误类型
///
/// 该枚举定义了固件处理过程中可能遇到的所有错误类型。
#[derive(Error, Debug)]
pub enum FirmwareError {
    /// ZIP 文件解压错误
    ///
    /// 该错误发生在 ZIP 文件格式错误、文件损坏等情况下。
    #[error("ZIP error: {0}")]
    ZipError(String),

    /// 文件 I/O 错误
    ///
    /// 该错误发生在文件读取、写入失败时。
    #[error("I/O error: {0}")]
    IoError(String),

    /// 文件不存在
    ///
    /// 该错误发生在指定的文件在 ZIP 中不存在时。
    #[error("File not found in ZIP: {0}")]
    FileNotFound(String),

    /// 数据格式错误
    ///
    /// 该错误发生在数据格式错误、解密失败等情况下。
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// 解密失败
    ///
    /// 该错误发生在固件文件解密失败时。
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
}

/// 固件处理模块结果类型
///
/// 该类型是一个方便的别名，用于返回 `Result<T, FirmwareError>`。
pub type Result<T> = std::result::Result<T, FirmwareError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：错误类型创建
    #[test]
    fn test_error_creation() {
        let err = FirmwareError::FileNotFound("test.img".to_string());
        assert_eq!(err.to_string(), "File not found in ZIP: test.img");
    }

    /// 测试：错误类型匹配
    #[test]
    fn test_error_matching() {
        let err = FirmwareError::InvalidData("test error".to_string());

        match err {
            FirmwareError::InvalidData(msg) => {
                assert_eq!(msg, "test error");
            }
            _ => panic!("Unexpected error type"),
        }
    }
}
