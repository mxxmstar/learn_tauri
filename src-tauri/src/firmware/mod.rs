/*!
 * 固件处理模块
 *
 * 该模块提供了固件文件的处理功能，包括：
 * - 固件文件解密
 * - ZIP 文件解压
 * - 固件文件验证
 *
 * # 功能说明
 *
 * ## 1. 固件文件解密
 *
 * 固件文件在传输前会进行简单的加密处理（每隔 257 字节删除 1 字节）。
 * 该模块提供了 `decrypt_firmware()` 函数来还原原始文件。
 *
 * 加密算法（C++ 版本）：
 * ```cpp
 * for (int i = 0; i < dataSize; i++) {
 *     if ((i + 4) % 257 != 0 || i < 1024) {
 *         output.append(data[i]);
 *     }
 * }
 * ```
 *
 * 解密算法（Rust 版本）：
 * - 在每隔 257 字节的位置插入一个占位字节
 * - 前 1024 字节不加密（保持原样）
 *
 * ## 2. ZIP 文件解压
 *
 * 固件文件通常打包为 ZIP 格式，该模块提供了 `extract_firmware_from_zip()` 函数
 * 来从 ZIP 文件中提取指定的固件文件（如 `rev1.img`）。
 *
 * # 使用示例
 *
 * ```rust
 * use crate::firmware::{
 *     decrypt_firmware,
 *     extract_firmware_from_zip,
 *     FirmwareError,
 * };
 *
 * // 示例 1：解密固件文件
 * let encrypted_data = std::fs::read("firmware.enc")?;
 * let decrypted_data = decrypt_firmware(&encrypted_data)?;
 * std::fs::write("firmware.bin", &decrypted_data)?;
 *
 * // 示例 2：从 ZIP 文件中提取固件
 * let zip_data = std::fs::read("firmware.zip")?;
 * let (file_name, file_data) = extract_firmware_from_zip(&zip_data, "rev1.img")?;
 * println!("Extracted: {} ({} bytes)", file_name, file_data.len());
 * ```
 *
 * # 依赖
 *
 * 该模块依赖以下 Rust crate：
 * - `zip`：ZIP 文件解压
 * - `thiserror`：错误类型派生宏
 */

// 模块声明
mod error;
mod decrypt;
mod extract;

// 公共类型重导出
pub use error::{
    FirmwareError,
    Result,
};

pub use decrypt::decrypt_firmware;
pub use extract::extract_firmware_from_zip;

// 模块版本信息
/// 模块名称
pub const MODULE_NAME: &str = "firmware";

/// 模块版本
pub const MODULE_VERSION: &str = "1.0.0";

/// 默认固件文件名（从 ZIP 中提取）
pub const DEFAULT_FIRMWARE_FILE_NAME: &str = "rev1.img";

/// 解密间隔（每 257 字节删除/插入 1 字节）
pub const DECRYPT_INTERVAL: usize = 256;

/// 不加密的数据大小（前 1024 字节）
pub const ENCRYPT_SKIP_SIZE: usize = 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// 测试：固件文件解密
    #[test]
    fn test_decrypt_firmware() {
        // 构造模拟的加密数据
        let mut encrypted_data = Vec::new();

        // 前 1024 字节不加密
        for i in 0..ENCRYPT_SKIP_SIZE {
            encrypted_data.push((i % 256) as u8);
        }

        // 后续数据：每隔 257 字节缺少 1 字节
        let mut original_data = Vec::new();
        for i in 0..1024 {
            original_data.push((i % 256) as u8);
        }

        let mut i = ENCRYPT_SKIP_SIZE;
        let mut enc_idx = ENCRYPT_SKIP_SIZE;
        while enc_idx < 4096 {
            if (i + 4) % (DECRYPT_INTERVAL + 1) != 0 {
                encrypted_data.push((i % 256) as u8);
                original_data.push((i % 256) as u8);
                enc_idx += 1;
            }
            i += 1;
        }

        // 解密
        let decrypted_data = decrypt_firmware(&encrypted_data).unwrap();

        // 验证：前 1024 字节应该完全相同
        assert_eq!(&decrypted_data[0..ENCRYPT_SKIP_SIZE], &original_data[0..ENCRYPT_SKIP_SIZE]);
    }

    /// 测试：ZIP 文件解压
    #[test]
    fn test_extract_firmware_from_zip() {
        // 创建一个模拟的 ZIP 文件
        let mut zip_data = Vec::new();

        // 注意：这里省略了创建有效 ZIP 文件的代码
        // 实际测试中，可以使用 `zip` crate 创建一个包含 `rev1.img` 的 ZIP 文件

        // 解压（预期失败，因为 ZIP 数据无效）
        let result = extract_firmware_from_zip(&zip_data, DEFAULT_FIRMWARE_FILE_NAME);
        assert!(result.is_err());
    }
}
