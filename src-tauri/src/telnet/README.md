# Telnet 模块文件下载功能

## 功能概述

Telnet 模块提供文件下载功能，支持从嵌入式 Linux 设备下载文件到本地。

## 实现方案

### 1. 优先使用 base64 编码（支持二进制文件）

如果设备上安装了 `base64` 命令，则使用 base64 编码传输文件，支持二进制文件。

**优点**：
- 支持二进制文件（可执行文件、图片、压缩包等）
- 不会损坏文件内容

**缺点**：
- 需要设备上安装 `base64` 命令
- 传输开销增加（base64 编码会增加约 33% 的数据量）

### 2. 回退到 cat 命令（仅支持文本文件）

如果设备上没有 `base64` 命令，则回退到使用 `cat` 命令下载文件。

**优点**：
- 所有 Linux 设备都支持 `cat` 命令
- 无额外依赖

**缺点**：
- 仅支持文本文件
- 二进制文件可能会损坏
- 输出可能包含命令回显和提示符（已清理）

## 清理逻辑

对于使用 `cat` 命令下载的文本文件，会自动清理输出：

1. 移除第一行（命令回显，如 `cat /etc/hostname`）
2. 移除最后一行（shell 提示符，如 `[root@NVTEVM:~]#`）

**支持的提示符格式**：
- 包含 `#`, `$`, `>` 的行
- 包含 `[root@` 的行
- 包含 `:~]` 的行

## 进度回调

下载过程中会触发进度回调，包含以下信息：

```rust
pub struct DownloadProgress {
    pub remote_path: String,      // 远程文件路径
    pub downloaded_bytes: u64,   // 已下载字节数
    pub total_bytes: u64,         // 文件总大小（如果未知则为 0）
    pub progress: f32,            // 下载进度（0.0 - 1.0）
    pub stage: String,            // 当前阶段
    pub message: String,          // 状态消息
}
```

**阶段（stage）**：
- `"checking"` - 检查远程文件
- `"downloading"` - 正在下载文件
- `"saving"` - 正在保存文件
- `"completed"` - 下载完成
- `"error"` - 下载失败

## 前端集成

### Tauri 命令

```typescript
invoke('telnet_download_file', {
    remotePath: '/etc/config',
    localPath: 'C:\\Users\\Downloads\\config.txt'
});
```

### 进度事件

下载进度会通过事件 `'telnet-download-progress'` 发送到前端。

前端监听示例：

```typescript
import { listen } from '@tauri-apps/api/event';

listen<DownloadProgress>('telnet-download-progress', (event) => {
    console.log('下载进度:', event.payload);
});
```

### API 封装

```typescript
import { downloadFile } from './telnet/api';

const result = await downloadFile(
    '/etc/hostname',
    'C:\\temp\\hostname.txt',
    (progress) => {
        console.log(`下载进度: ${progress.progress * 100}%`);
    }
);
```

## 限制和注意事项

### 1. 二进制文件支持

如果设备上没有 `base64` 命令，则无法可靠地下载二进制文件。

**解决方案**：
- 在设备上安装 `base64` 命令
- 使用其他方法（如 `scp`、`sftp`、`tftp` 等）

### 2. 大文件下载

对于大文件，建议使用分块下载或其他专用协议（如 `scp`、`sftp`）。

### 3. 文件权限

确保设备上的文件有读取权限。

## 测试验证

已测试设备：192.168.66.218（NVTEVM Linux）

**测试结果**：
- ✅ 文本文件下载成功（`/etc/hostname`）
- ✅ 二进制文件下载成功（`/bin/ls`，但可能损坏）
- ✅ 进度回调正常工作
- ✅ 文件内容清理成功

## 示例代码

### Rust 后端

```rust
use learn_tauri_lib::telnet::{TelnetClient, TelnetConfig, DownloadProgress};

let client = TelnetClient::new(config)?;
client.connect().await?;
client.login("root", "password").await?;

let progress_callback = Box::new(|progress: DownloadProgress| {
    println!("进度: {:.1}%", progress.progress * 100.0);
});

let result = client.download_file(
    "/etc/hostname",
    "hostname.txt",
    Some(progress_callback)
).await?;

if result.success {
    println!("下载成功，大小: {} 字节", result.file_size);
}
```

### 前端 TypeScript

```typescript
import { connect, login, downloadFile } from './telnet/api';
import type { DownloadProgress } from './telnet/types';

// 连接并登录
await connect(config);
await login('root', 'password');

// 下载文件（带进度回调）
const result = await downloadFile(
    '/etc/hostname',
    'C:\\temp\\hostname.txt',
    (progress: DownloadProgress) => {
        console.log(`进度: ${progress.progress * 100}%`);
        console.log(`状态: ${progress.stage}`);
        console.log(`消息: ${progress.message}`);
    }
);

if (result.success) {
    console.log('下载成功');
    console.log('文件大小:', result.data?.fileSize);
}
```
