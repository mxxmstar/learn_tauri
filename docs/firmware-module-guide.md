# 固件处理模块使用指南

## 模块概述

`firmware` 模块提供了固件文件的完整处理功能，包括：
- 固件文件解密
- ZIP 文件解压
- 固件文件验证

该模块是将 `QT_StreamPlayer` 中的 `util.cpp` 功能移植到 Rust 的实现。

## 模块结构

```
src-tauri/src/firmware/
├── mod.rs          # 模块入口（本文件）
├── error.rs        # 错误类型定义
├── decrypt.rs      # 固件文件解密
└── extract.rs      # ZIP 文件解压
```

## 快速开始

### 1. 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
zip = "0.6"
```

### 2. 基本使用

```rust
use crate::firmware::{
    decrypt_firmware,
    extract_firmware_from_zip,
    FirmwareError,
    Result,
};

// 示例 1：解密固件文件
let encrypted_data = std::fs::read("firmware.enc")?;
let decrypted_data = decrypt_firmware(&encrypted_data)?;
std::fs::write("firmware.bin", &decrypted_data)?;

// 示例 2：从 ZIP 文件中提取固件
let zip_data = std::fs::read("firmware.zip")?;
let (file_name, file_data) = extract_firmware_from_zip(&zip_data, "rev1.img")?;
println!("Extracted: {} ({} bytes)", file_name, file_data.len());
```

## API 参考

### 1. 固件文件解密 (`decrypt.rs`)

#### `decrypt_firmware(encrypted_data: &[u8]) -> Result<Vec<u8>>`

解密固件文件。

**参数**：
- `encrypted_data`: 加密的固件数据

**返回值**：
- `Ok(Vec<u8>)`: 解密后的数据
- `Err(FirmwareError)`: 解密失败

**算法说明**：
1. 前 1024 字节直接复制（不加密）
2. 后续数据：在每隔 257 字节的位置插入一个占位字节（0x00）

**示例**：

```rust
use crate::firmware::decrypt::decrypt_firmware;

let encrypted = std::fs::read("firmware.enc")?;
let decrypted = decrypt_firmware(&encrypted)?;
std::fs::write("firmware.bin", &decrypted)?;
```

#### `encrypt_firmware(original_data: &[u8]) -> Vec<u8>`

加密固件文件（用于测试）。

**参数**：
- `original_data`: 原始的固件数据

**返回值**：
- `Ok(Vec<u8>)`: 加密后的数据

**注意**：该函数主要用于测试，验证解密算法的正确性。

### 2. ZIP 文件解压 (`extract.rs`)

#### `extract_firmware_from_zip(zip_data: &[u8], target_file_name: &str) -> Result<(String, Vec<u8>)>`

从 ZIP 文件中提取固件文件。

**参数**：
- `zip_data`: ZIP 文件的二进制数据
- `target_file_name`: 要提取的文件名（如 `rev1.img`）

**返回值**：
- `Ok((String, Vec<u8>))`: 文件名和文件数据
- `Err(FirmwareError)`: 解压失败

**错误**：
- `ZipError`: ZIP 文件格式错误
- `FileNotFound`: 指定的文件在 ZIP 中不存在

**示例**：

```rust
use crate::firmware::extract::extract_firmware_from_zip;

let zip_data = std::fs::read("firmware.zip")?;
let (file_name, file_data) = extract_firmware_from_zip(&zip_data, "rev1.img")?;
println!("Extracted: {} ({} bytes)", file_name, file_data.len());
```

#### `extract_all_firmware(zip_data: &[u8], pattern: &str) -> Result<Vec<(String, Vec<u8>)>>`

从 ZIP 文件中提取所有匹配指定模式的文件。

**参数**：
- `zip_data`: ZIP 文件的二进制数据
- `pattern`: 文件名模式（如 `.img`）

**返回值**：
- `Ok(Vec<(String, Vec<u8>)>)`: 文件名和文件数据列表
- `Err(FirmwareError)`: 解压失败

**注意**：该函数使用简单的字符串匹配，不支持真正的通配符。

#### `list_zip_files(zip_data: &[u8]) -> Result<Vec<String>>`

列出 ZIP 文件中的所有文件名。

**参数**：
- `zip_data`: ZIP 文件的二进制数据

**返回值**：
- `Ok(Vec<String>)`: 文件名列表
- `Err(FirmwareError)`: 读取失败

### 3. 错误类型 (`error.rs`)

#### `FirmwareError`

固件处理模块错误类型。

**变体**：
- `ZipError(String)`: ZIP 文件解压错误
- `IoError(String)`: 文件 I/O 错误
- `FileNotFound(String)`: 指定的文件在 ZIP 中不存在
- `InvalidData(String)`: 数据格式错误
- `DecryptionFailed(String)`: 解密失败

#### `Result<T>`

固件处理模块结果类型。

**定义**：
```rust
pub type Result<T> = std::result::Result<T, FirmwareError>;
```

## 与 QT_StreamPlayer 的对比

| 功能 | QT_StreamPlayer (C++) | Rust 模块 | 说明 |
|---|---|---|---|
| 固件解密 | `decryption()` (`util.cpp`) | `decrypt_firmware()` | 算法完全相同 |
| ZIP 解压 | `extractZip()` (`util.cpp`) | `extract_firmware_from_zip()` | 使用 `zip` crate |
| 错误处理 | 返回 `int32_t` 错误码 | `Result<T, FirmwareError>` | Rust 使用类型安全错误处理 |

## 完整示例

### 示例 1：解密并解压固件文件

```rust
use crate::firmware::{
    decrypt_firmware,
    extract_firmware_from_zip,
    FirmwareError,
};

fn process_firmware(encrypted_file: &str, zip_file: &str) -> Result<(), FirmwareError> {
    // 1. 解密固件文件
    let encrypted_data = std::fs::read(encrypted_file)
        .map_err(|e| FirmwareError::IoError(e.to_string()))?;
    let decrypted_data = decrypt_firmware(&encrypted_data)?;

    // 2. 写入解密后的数据
    std::fs::write("firmware.bin", &decrypted_data)
        .map_err(|e| FirmwareError::IoError(e.to_string()))?;

    // 3. 从 ZIP 文件中提取固件
    let zip_data = std::fs::read(zip_file)
        .map_err(|e| FirmwareError::IoError(e.to_string()))?;
    let (file_name, file_data) = extract_firmware_from_zip(&zip_data, "rev1.img")?;

    println!("Extracted: {} ({} bytes)", file_name, file_data.len());

    // 4. 写入提取的固件
    std::fs::write(&file_name, &file_data)
        .map_err(|e| FirmwareError::IoError(e.to_string()))?;

    Ok(())
}
```

### 示例 2：与 BCM 模块集成（固件更新）

```rust
use crate::{
    firmware::{
        decrypt_firmware,
        extract_firmware_from_zip,
    },
    bcm::{
        tool::BcmTool,
        update::FirmwareInstaller,
    },
};

async fn update_firmware(device_ip: &str, firmware_zip: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 从 ZIP 中提取固件
    let (file_name, file_data) = extract_firmware_from_zip(firmware_zip, "rev1.img")?;

    // 2. 解密固件（如果需要）
    let decrypted_data = decrypt_firmware(&file_data)?;

    // 3. 启动文件服务器
    let file_server = FirmwareInstaller::start_file_server("0.0.0.0:8080").await?;

    // 4. 获取主机 IP
    let host_ip = "192.168.1.100";  // 实际应从配置中读取

    // 5. 执行固件安装
    BcmTool::full_install(
        file_name,
        decrypted_data,
        device_ip,
        host_ip,
    ).await?;

    Ok(())
}
```

## 测试

模块包含完整的单元测试，覆盖：

1. **解密算法测试** (`decrypt.rs`):
   - 加密和解密的正确性
   - 前 1024 字节不加密
   - 空数据处理
   - 小数据（< 1024 字节）处理

2. **ZIP 解压测试** (`extract.rs`):
   - 创建 ZIP 文件并提取
   - 提取不存在的文件
   - 提取所有匹配的文件

运行测试：

```bash
cd src-tauri
cargo test --package learn_tauri --lib firmware::
```

## 性能考虑

1. **内存使用**：解密和 ZIP 解压都需要将整个文件加载到内存中。对于大文件（> 100MB），可能需要流式处理。

2. **ZIP 解压**：`zip` crate 支持多种压缩算法。如果固件 ZIP 使用特殊压缩算法，可能需要额外的依赖。

3. **并发安全**：模块中的函数都是纯函数（不修改全局状态），可以安全地在多线程环境中使用。

## 常见问题

### 1. 解密后的文件仍然无法使用？

**可能原因**：
- 加密算法参数不正确（间隔、跳过大小）
- 数据损坏

**解决方法**：
- 检查加密算法的参数是否与设备端一致
- 使用 `encrypt_firmware()` 和 `decrypt_firmware()` 测试向量验证

### 2. ZIP 解压失败？

**可能原因**：
- ZIP 文件格式错误或损坏
- 不支持的压缩算法

**解决方法**：
- 使用 `list_zip_files()` 检查 ZIP 文件是否有效
- 检查 `zip` crate 的版本和特性

### 3. 如何支持密码保护的 ZIP 文件？

**当前状态**：模块不支持密码保护的 ZIP 文件。

**解决方法**：
- 使用 `zip` crate 的 `ZipArchive::new_with_password()` 方法
- 修改 `extract_firmware_from_zip()` 函数，添加密码参数

## 参考资料

- **ZIP 文件格式**：https://en.wikipedia.org/wiki/ZIP_(file_format)
- **`zip` crate 文档**：https://docs.rs/zip/latest/zip/
- **QT_StreamPlayer 源码**：`QT_StreamPlayer/src/tool/util.cpp`

## 许可证

该模块是 `learn_tauri` 项目的一部分，遵循相同的许可证。
