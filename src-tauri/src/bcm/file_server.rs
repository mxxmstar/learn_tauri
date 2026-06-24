//! 固件传输 TCP 文件服务器
//!
//! 在固件升级过程中，本机作为 TCP 服务端监听一个随机端口，
//! 等待设备连接后将固件数据分块发送给设备。
//! 完全使用 tokio 异步实现，替代原 Qt 的 FileTransferServer。

use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

/// 每次发送的块大小（256 字节）
const CHUNK_SIZE: usize = 256;

/// 文件传输服务器
///
/// 启动后绑定一个随机端口，通过 `port` 字段可获取实际端口号，
/// 用于通知设备连接。
pub struct FileTransferServer {
    /// 服务端口号
    pub port: u16,
    /// 本地地址
    pub addr: SocketAddr,
    /// 后台任务句柄
    task: tokio::task::JoinHandle<()>,
}

/// 传输进度信息
#[derive(Debug, Clone)]
pub struct TransferProgress {
    /// 完成百分比（0-100）
    pub percent: u32,
    /// 已发送字节数
    pub done: usize,
    /// 总字节数
    pub total: usize,
}

impl FileTransferServer {
    /// 启动文件传输服务
    ///
    /// 绑定随机端口并开始监听，等待设备 TCP 连接。
    /// 设备连接后将固件数据以 256 字节为单位分块发送。
    ///
    /// # 参数
    /// * `file_bytes` - 固件文件数据
    ///
    /// # 返回值
    /// * `Ok((server, progress_rx))` - 服务器实例和进度接收通道
    pub async fn start(
        file_bytes: Vec<u8>,
    ) -> std::io::Result<(Self, mpsc::Receiver<TransferProgress>)> {
        let listener = TcpListener::bind("0.0.0.0:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();

        let (tx, rx) = mpsc::channel::<TransferProgress>(32);

        let task = tokio::spawn(async move {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let total = file_bytes.len();
                    let mut done = 0usize;

                    while done < total {
                        let end = (done + CHUNK_SIZE).min(total);
                        let chunk = &file_bytes[done..end];

                        // 分块发送固件数据
                        if stream.write_all(chunk).await.is_err() {
                            break;
                        }

                        done = end;
                        let percent = (done * 100 / total) as u32;

                        let _ = tx.send(TransferProgress { percent, done, total }).await;
                    }
                }
                Err(_) => {}
            }
        });

        Ok((
            FileTransferServer {
                port,
                addr,
                task,
            },
            rx,
        ))
    }

    /// 等待文件传输任务完成
    pub async fn wait_completion(self) {
        let _ = self.task.await;
    }
}
