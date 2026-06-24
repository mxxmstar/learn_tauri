//! C 语言 FFI 绑定
//!
//! 通过 `extern "C"` 声明 libbcmcore 静态库中导出的 C 函数，
//! 让 Rust 可以直接调用底层的 RPC 通信、DMON、CONFIG 等 C 实现。

use std::os::raw::c_char;

use super::types::*;

extern "C" {
    // ====== RPC 传输层（rpc_connect.c） ======

    // 打开 RPC 连接
    //
    // 通过 TCP 连接到设备的指定端口，返回连接句柄。
    pub fn RPC_Open(
        sock_name: *const c_char,
        port: u16,
        timeout_ms: u32,
        hdl: *mut BCM_HandleType,
    ) -> i32;

    // 发送并接收 RPC 消息（请求-响应模式）
    //
    // 发送命令和数据到设备，然后等待设备响应。
    pub fn RPC_SendRecv(
        hdl: BCM_HandleType,
        cmd: BCM_MsgType,
        in_msg: *const u8,
        in_len: u32,
        out_msg: *mut u8,
        out_len: *mut u32,
    ) -> i32;

    /// 关闭 RPC 连接，释放资源
    pub fn RPC_Close(hdl: BCM_HandleType) -> i32;

    // ====== DMON（设备监控，bcm_dmon.c） ======

    /// 重启设备（发送重启命令后设备延迟 10ms 重启）
    pub fn DMON_Reboot(hdl: BCM_HandleType) -> i32;

    /// 获取软件版本字符串
    pub fn DMON_GetSwVersion(
        hdl: BCM_HandleType,
        ver: *mut DMON_SwVersionMsgType,
    ) -> i32;

    /// 获取硬件版本信息（厂商、型号、修订号、安全模式）
    pub fn DMON_GetHwVersion(
        hdl: BCM_HandleType,
        ver: *mut DMON_HwVersionMsgType,
    ) -> i32;

    /// Ping 设备，获取启动模式和硬件版本
    pub fn DMON_Ping(
        hdl: BCM_HandleType,
        ping: *mut DMON_PingMsgType,
    ) -> i32;

    /// 获取设备同步状态（模式、状态、各阶段时间等）
    pub fn DMON_Sync(
        hdl: BCM_HandleType,
        sync: *mut DMON_SyncMsgType,
    ) -> i32;

    /// 等待设备达到指定状态后返回同步信息
    pub fn DMON_SyncWait(
        hdl: BCM_HandleType,
        state: u32,
        sync: *mut DMON_SyncMsgType,
    ) -> i32;

    /// 读取设备内存
    pub fn DMON_ReadMem(
        hdl: BCM_HandleType,
        addr: u32,
        width: u32,
        device_id: u32,
        data: *mut u32,
    ) -> i32;

    /// 写入设备内存
    pub fn DMON_WriteMem(
        hdl: BCM_HandleType,
        addr: u32,
        width: u32,
        device_id: u32,
        data: u32,
    ) -> i32;

    /// 让设备进入深度睡眠模式
    pub fn DMON_DeepSleep(hdl: BCM_HandleType) -> i32;

    // ====== CONFIG（配置读写，bcm_config.c） ======

    /// 读取设备全部配置
    pub fn CONFIG_RpcRead(
        hdl: BCM_HandleType,
        msg: *mut CONFIG_RpcMsg,
    ) -> i32;

    /// 写入配置到设备
    pub fn CONFIG_RpcWrite(
        hdl: BCM_HandleType,
        msg: *mut CONFIG_RpcMsg,
    ) -> i32;

    // 从配置数据缓冲区中解析一个配置项
    //
    // 返回配置项名称、值和占用长度。
    pub fn CONFIG_ExtractItem(
        ctx: *mut u8,
        name: *mut c_char,
        val: *mut c_char,
    ) -> u32;
}
