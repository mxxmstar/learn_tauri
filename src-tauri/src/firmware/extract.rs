/*!
 * 固件文件解压
 *
 * 该模块提供了从 ZIP 文件中提取固件文件的功能。
 *
 * # 功能说明
 *
 * 固件文件通常打包为 ZIP 格式，该模块可以：
 * - 从 ZIP 文件中提取指定的固件文件（如 `rev1.img`）
 * - 返回文件名和文件数据
 *
 * # C++ 原代码
 *
 * ```cpp
 * int32_t extractZip(QString& fileName, QByteArray& fileBytes)
 * {
 *     QBuffer buffer(&fileBytes);
 *     QuaZip zip(&buffer);
 *     if (!zip.open(QuaZip::mdUnzip)) {
 *         return -1;
 *     }
 *
 *     QuaZipFile file(&zip);
 *     for (bool more = zip.goToFirstFile(); more; more = zip.goToNextFile()) {
 *         // 获取当前文件名
 *         fileName = zip.getCurrentFileName();
 *         if (!fileName.contains("rev1.img")) {
 *             continue;
 *         }
 *
 *         if (!file.open(QIODevice::ReadOnly)) {
 *             return -1;
 *         }
 *
 *         fileBytes = file.readAll();
 *         zip.close();
 *         return 0;
 *     }
 *
 *     zip.close();
 *     return 0;
 * }
 * ```
 *
 * # 使用示例
 *
 * ```rust
 * use crate::firmware::extract::extract_firmware_from_zip;
 *
 * let zip_data = std::fs::read("firmware.zip")?;
 * let (file_name, file_data) = extract_firmware_from_zip(&zip_data, "rev1.img")?;
 * println!("Extracted: {} ({} bytes)", file_name, file_data.len());
 * ```
 *
 * # 依赖
 *
 * 该模块依赖 `zip` crate 进行 ZIP 文件解压。
 *
 * 在 `Cargo.toml` 中添加：
 * ```toml
 * [dependencies]
 * zip = "0.6"
 * ```
 */

use std::io::Cursor;

use crate::firmware::error::{Result, FirmwareError};

/// 从 ZIP 文件中提取固件文件
///
/// 该函数从 ZIP 文件中提取指定的固件文件（如 `rev1.img`）。
///
/// # 参数
/// - `zip_data`: ZIP 文件的二进制数据
/// - `target_file_name`: 要提取的文件名（如 `rev1.img`）
///
/// # 返回值
/// - `Ok((String, Vec<u8>))`: 文件名和文件数据
/// - `Err(FirmwareError)`: 解压失败
///
/// # 错误
/// - `ZipError`: ZIP 文件格式错误
/// - `FileNotFound`: 指定的文件在 ZIP 中不存在
///
/// # 示例
///
/// ```rust
/// use crate::firmware::extract::extract_firmware_from_zip;
///
/// let zip_data = std::fs::read("firmware.zip")?;
/// let (file_name, file_data) = extract_firmware_from_zip(&zip_data, "rev1.img")?;
/// ```
pub fn extract_firmware_from_zip(
    zip_data: &[u8],
    target_file_name: &str,
) -> Result<(String, Vec<u8>)> {
    // 1. 创建 ZIP 读取器
    let cursor = Cursor::new(zip_data);
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|e| FirmwareError::ZipError(e.to_string()))?;

    // 2. 遍历 ZIP 中的文件
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)
            .map_err(|e| FirmwareError::ZipError(e.to_string()))?;

        let file_name = file.name().to_string();

        // 3. 检查是否为目标文件
        if file_name.contains(target_file_name) {
            // 4. 读取文件数据
            let mut file_data = Vec::with_capacity(file.size() as usize);
            std::io::copy(&mut file, &mut file_data)
                .map_err(|e| FirmwareError::IoError(e.to_string()))?;

            return Ok((file_name, file_data));
        }
    }

    // 5. 未找到目标文件
    Err(FirmwareError::FileNotFound(target_file_name.to_string()))
}

/// 从 ZIP 文件中提取所有固件文件
///
/// 该函数从 ZIP 文件中提取所有匹配指定模式的文件。
///
/// # 参数
/// - `zip_data`: ZIP 文件的二进制数据
/// - `pattern`: 文件名模式（如 `*.img`）
///
/// # 返回值
/// - `Ok(Vec<(String, Vec<u8>)>)`: 文件名和文件数据列表
/// - `Err(FirmwareError)`: 解压失败
///
/// # 注意
///
/// 该函数使用简单的字符串匹配，不支持真正的通配符。
/// 如果需要更复杂的匹配，可以使用 `glob` crate。
pub fn extract_all_firmware(
    zip_data: &[u8],
    pattern: &str,
) -> Result<Vec<(String, Vec<u8>)>> {
    let cursor = Cursor::new(zip_data);
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|e| FirmwareError::ZipError(e.to_string()))?;

    let mut result = Vec::new();

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)
            .map_err(|e| FirmwareError::ZipError(e.to_string()))?;

        let file_name = file.name().to_string();

        // 检查是否匹配模式
        if file_name.contains(pattern) {
            let mut file_data = Vec::with_capacity(file.size() as usize);
            std::io::copy(&mut file, &mut file_data)
                .map_err(|e| FirmwareError::IoError(e.to_string()))?;

            result.push((file_name, file_data));
        }
    }

    Ok(result)
}

/// 列出 ZIP 文件中的所有文件名
///
/// 该函数返回 ZIP 文件中所有文件的名称。
///
/// # 参数
/// - `zip_data`: ZIP 文件的二进制数据
///
/// # 返回值
/// - `Ok(Vec<String>)`: 文件名列表
/// - `Err(FirmwareError)`: 读取失败
pub fn list_zip_files(zip_data: &[u8]) -> Result<Vec<String>> {
    let cursor = Cursor::new(zip_data);
    let mut zip = zip::ZipArchive::new(cursor)
        .map_err(|e| FirmwareError::ZipError(e.to_string()))?;

    let mut file_names = Vec::with_capacity(zip.len());

    for i in 0..zip.len() {
        let file = zip.by_index(i)
            .map_err(|e| FirmwareError::ZipError(e.to_string()))?;
        file_names.push(file.name().to_string());
    }

    Ok(file_names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// 测试：创建 ZIP 文件并提取
    #[test]
    fn test_create_and_extract_zip() {
        // 1. 创建一个测试 ZIP 文件
        let mut zip_data = Vec::new();
        {
            let cursor = Cursor::new(&mut zip_data);
            let mut zip_writer = zip::ZipWriter::new(cursor);

            let options = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            // 添加文件 rev1.img
            zip_writer.start_file("rev1.img", options).unwrap();
            let test_data = b"Hello, Firmware!";
            zip_writer.write_all(test_data).unwrap();

            // 添加文件 readme.txt
            zip_writer.start_file("readme.txt", options).unwrap();
            zip_writer.write_all(b"Firmware v1.0").unwrap();

            zip_writer.finish().unwrap();
        }

        // 2. 提取 rev1.img
        let (file_name, file_data) = extract_firmware_from_zip(&zip_data, "rev1.img").unwrap();
        assert_eq!(file_name, "rev1.img");
        assert_eq!(file_data, b"Hello, Firmware!");

        // 3. 列出所有文件
        let file_names = list_zip_files(&zip_data).unwrap();
        assert_eq!(file_names.len(), 2);
        assert!(file_names.contains(&"rev1.img".to_string()));
        assert!(file_names.contains(&"readme.txt".to_string()));
    }

    /// 测试：提取不存在的文件
    #[test]
    fn test_extract_nonexistent_file() {
        // 创建一个测试 ZIP 文件
        let mut zip_data = Vec::new();
        {
            let cursor = Cursor::new(&mut zip_data);
            let mut zip_writer = zip::ZipWriter::new(cursor);

            let options = zip::write::FileOptions::default();
            zip_writer.start_file("test.txt", options).unwrap();
            zip_writer.write_all(b"Test").unwrap();
            zip_writer.finish().unwrap();
        }

        // 提取不存在的文件
        let result = extract_firmware_from_zip(&zip_data, "nonexistent.img");
        assert!(result.is_err());
    }

    /// 测试：提取所有匹配的文件
    #[test]
    fn test_extract_all_firmware() {
        // 创建一个测试 ZIP 文件
        let mut zip_data = Vec::new();
        {
            let cursor = Cursor::new(&mut zip_data);
            let mut zip_writer = zip::ZipWriter::new(cursor);

            let options = zip::write::FileOptions::default();

            // 添加多个 .img 文件
            zip_writer.start_file("rev1.img", options).unwrap();
            zip_writer.write_all(b"Firmware 1").unwrap();

            zip_writer.start_file("rev2.img", options).unwrap();
            zip_writer.write_all(b"Firmware 2").unwrap();

            zip_writer.start_file("readme.txt", options).unwrap();
            zip_writer.write_all(b"Readme").unwrap();

            zip_writer.finish().unwrap();
        }

        // 提取所有 .img 文件
        let files = extract_all_firmware(&zip_data, ".img").unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "rev1.img");
        assert_eq!(files[1].0, "rev2.img");
    }
}
