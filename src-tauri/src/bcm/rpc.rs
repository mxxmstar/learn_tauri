//! RPC 连接管理
//!
//! RpcConnection 封装了 RPC 连接的打开和关闭，
//! 提供安全的 `send_recv` 接口供 DMON、CONFIG、UPDATE 等模块使用。

use std::ffi::CString;
use std::os::raw::c_char;

use super::error::{from_bcm_err, BcmError};
use super::ffi;
use super::types::*;

/// RPC 默认端口
const RPC_PORT: u16 = 5555;
/// RPC 默认超时时间（60 秒）
const RPC_TIMEOUT_MS: u32 = 60000;

/// RPC 连接封装
///
/// 通过 `RpcConnection::open()` 建立连接，操作完成后调用 `close()` 释放。
pub struct RpcConnection {
    /// C 层的连接句柄
    hdl: BCM_HandleType,
}

impl RpcConnection {
    /// 打开与设备的 RPC 连接
    ///
    /// # 参数
    /// * `device_ip` - 设备的 IP 地址字符串
    ///
    /// # 返回值
    /// * `Ok(RpcConnection)` - 连接成功
    /// * `Err(BcmError)` - 连接失败
    pub fn open(device_ip: &str) -> Result<Self, BcmError> {
        let c_ip = CString::new(device_ip).map_err(|_| BcmError::InvalidParams)?;
        let mut hdl: BCM_HandleType = 0;

        let ret = unsafe {
            ffi::RPC_Open(
                c_ip.as_ptr() as *const c_char,
                RPC_PORT,
                RPC_TIMEOUT_MS,
                &mut hdl,
            )
        };

        if ret != BCM_ERR_OK {
            return Err(from_bcm_err(ret));
        }

        Ok(RpcConnection { hdl })
    }

    /// 获取原始 C 层句柄（供 FFI 调用使用）
    pub fn handle(&self) -> BCM_HandleType {
        self.hdl
    }

    /// 关闭 RPC 连接并释放 C 层资源
    pub fn close(self) {
        unsafe {
            ffi::RPC_Close(self.hdl);
        }
    }

    /// 发送 RPC 命令并接收响应
    ///
    /// # 参数
    /// * `cmd` - RPC 命令 ID
    /// * `in_data` - 发送的数据
    /// * `out_data` - 接收数据的缓冲区
    ///
    /// # 返回值
    /// * `Ok(u32)` - 实际接收的数据长度
    /// * `Err(BcmError)` - 调用失败
    pub fn send_recv(
        &self,
        cmd: BCM_MsgType,
        in_data: &[u8],
        out_data: &mut [u8],
    ) -> Result<u32, BcmError> {
        let mut out_len: u32 = out_data.len() as u32;

        let ret = unsafe {
            ffi::RPC_SendRecv(
                self.hdl,
                cmd,
                in_data.as_ptr(),
                in_data.len() as u32,
                out_data.as_mut_ptr(),
                &mut out_len,
            )
        };

        if ret != BCM_ERR_OK {
            return Err(from_bcm_err(ret));
        }

        Ok(out_len)
    }
}
