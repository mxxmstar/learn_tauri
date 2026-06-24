//! bcm 模块核心类型定义
//!
//! 包含与 Broadcom 设备通信所需的所有 C 兼容类型、协议常量和消息 ID。
//! 这些类型与 C 头文件（bcm_common.h, bcm_dmon.h, bcm_config.h 等）严格对应。

/// RPC 连接句柄（对应 C 的 BCM_HandleType = uint64_t）
#[allow(non_camel_case_types)]
pub type BCM_HandleType = u64;
/// RPC 消息命令 ID（对应 C 的 BCM_MsgType = uint32_t）
#[allow(non_camel_case_types)]
pub type BCM_MsgType = u32;

// ====== RPC 错误码（对应 bcm_common.h 中的 BCM_ErrorType 枚举） ======
/// 成功
pub const BCM_ERR_OK: i32 = 0x0;
/// 找不到设备
pub const BCM_ERR_NODEV: i32 = 0x2;
/// 内存不足
pub const BCM_ERR_NOMEM: i32 = 0x4;
/// 不支持的操作
pub const BCM_ERR_NOSUPPORT: i32 = 0x5;
/// 无效参数
pub const BCM_ERR_INVAL_PARAMS: i32 = 0x6;
/// 无效的 magic 数值
pub const BCM_ERR_INVAL_MAGIC: i32 = 0x7;
/// 请重试（数据未收完）
pub const BCM_ERR_EAGAIN: i32 = 0xA;
/// 超时
pub const BCM_ERR_TIME_OUT: i32 = 0xB;
/// 权限不足
pub const BCM_ERR_NOPERM: i32 = 0x16;

// ====== DMON 消息结构体（对应 bcm_dmon.h） ======

/// 软件版本信息（从设备读取，100 字节字符串）
#[repr(C)]
pub struct DMON_SwVersionMsgType {
    pub str: [i8; 100],
}

/// 硬件版本信息
#[repr(C)]
pub struct DMON_HwVersionMsgType {
    /// 厂商 ID
    pub manuf: u32,
    /// 型号
    pub model: u32,
    /// 修订号
    pub rev: u32,
    /// 安全模式
    pub sec_mode: u32,
}

/// Ping 响应消息（返回启动模式和硬件版本）
#[repr(C)]
pub struct DMON_PingMsgType {
    /// 启动模式（ROM/BL/FW）
    pub mode: u32,
    /// 硬件版本
    pub version: DMON_HwVersionMsgType,
}

/// 同步状态消息
#[repr(C)]
pub struct DMON_SyncMsgType {
    /// 启动模式
    pub mode: u32,
    /// 设备状态（UNINIT/INIT/READY/RUN）
    pub state: u32,
    /// 硬件版本
    pub version: DMON_HwVersionMsgType,
    /// 初始化时间
    pub init_time: u64,
    /// 就绪时间
    pub ready_time: u64,
    /// 运行时间
    pub run_time: u64,
    /// 保留字段
    pub rsvd: [u8; 208],
}

/// 内存访问消息（用于读写设备内存）
#[repr(C)]
pub struct DMON_MemAccessMsgType {
    /// 地址
    pub addr: u32,
    /// 数据宽度（8/16/32 位）
    pub width: u32,
    /// 数据长度
    pub len: u32,
    /// 设备 ID
    pub device_id: u32,
    /// 数据缓冲区
    pub data: [u8; 128],
}

// ====== CONFIG 消息结构体（对应 bcm_config.h） ======

/// 配置读写消息
#[repr(C)]
pub struct CONFIG_RpcMsg {
    /// 配置数据缓冲区
    pub ctx: [u8; 256],
    /// 数据长度
    pub len: u32,
}

// ====== 消息 ID 构造常量 ======

/// 系统分组 ID
pub const BCM_GROUPID_SYS: u32 = 0x01;
/// NVM（非易失性存储）分组 ID
pub const BCM_GROUPID_NVM: u32 = 0x03;

/// DMON（设备监控）组件 ID
pub const BCM_DMN_ID: u32 = 0x0123;
/// CONFIG（配置）组件 ID
pub const BCM_CFG_ID: u32 = 0x0324;
/// UPDATE（固件更新）组件 ID
pub const BCM_UPD_ID: u32 = 0x0323;

/// 构造 RPC 消息 ID
///
/// 按 Group(6bit) | Component(16bit) | MsgID(8bit) 的格式组合成 32 位命令字
pub const fn bcm_msg(grp: u32, comp: u32, id: u32) -> u32 {
    ((grp & 0x3F) << 24) | ((comp & 0xFFFF) << 8) | (id & 0xFF)
}

// DMON 消息 ID
pub const DMON_ID_PING: u32 = bcm_msg(BCM_GROUPID_SYS, BCM_DMN_ID, 0x01);
pub const DMON_ID_SYNC: u32 = bcm_msg(BCM_GROUPID_SYS, BCM_DMN_ID, 0x02);
pub const DMON_ID_SYNC_WAIT: u32 = bcm_msg(BCM_GROUPID_SYS, BCM_DMN_ID, 0x03);
pub const DMON_ID_MEM_READ: u32 = bcm_msg(BCM_GROUPID_SYS, BCM_DMN_ID, 0x11);
pub const DMON_ID_MEM_WRITE: u32 = bcm_msg(BCM_GROUPID_SYS, BCM_DMN_ID, 0x10);
pub const DMON_ID_SW_VERSION: u32 = bcm_msg(BCM_GROUPID_SYS, BCM_DMN_ID, 0x20);
pub const DMON_ID_HW_VERSION: u32 = bcm_msg(BCM_GROUPID_SYS, BCM_DMN_ID, 0x21);
pub const DMON_ID_REBOOT: u32 = bcm_msg(BCM_GROUPID_SYS, BCM_DMN_ID, 0x22);
pub const DMON_ID_DEEPSLEEP: u32 = bcm_msg(BCM_GROUPID_SYS, BCM_DMN_ID, 0x23);

// CONFIG 消息 ID
pub const CONFIG_CMD_RPC_READ: u32 = bcm_msg(BCM_GROUPID_NVM, BCM_CFG_ID, 1);
pub const CONFIG_CMD_RPC_WRITE: u32 = bcm_msg(BCM_GROUPID_NVM, BCM_CFG_ID, 2);

// UPDATE 消息 ID
pub const UPDATE_ID_HEALTH_CHECK: u32 = bcm_msg(BCM_GROUPID_NVM, BCM_UPD_ID, 0x00);
pub const UPDATE_ID_FULL_INSTALL: u32 = bcm_msg(BCM_GROUPID_NVM, BCM_UPD_ID, 0x21);

// ====== 配置项 ID（对应 bcm_config.h） ======
/// 镜像模式
pub const CONFIG_MEDIA_MIRROR: u16 = 0x0101;
/// 帧率
pub const CONFIG_MEDIA_FPS: u16 = 0x0102;
/// SOME/IP UDP 端口
pub const CONFIG_MEDIA_SOMEIPUDPPORT: u16 = 0x0103;
/// SOME/IP RTP 端口
pub const CONFIG_MEDIA_SOMEIPRTPPORT: u16 = 0x0104;
/// DHCP 开关
pub const CONFIG_NETWORK_DHCP: u16 = 0x0201;
/// IP 地址
pub const CONFIG_NETWORK_IP: u16 = 0x0202;
/// MAC 地址
pub const CONFIG_NETWORK_MAC: u16 = 0x0203;
/// AVTP 目标 MAC
pub const CONFIG_AVTP_DSTMAC: u16 = 0x0301;
/// AVTP 流 ID
pub const CONFIG_AVTP_STREAMID: u16 = 0x0302;

// ====== UPDATE 消息结构体（对应 bcm_flash.h） ======

/// 镜像版本信息
#[repr(C)]
pub struct IMGL_VersionType {
    /// 幻数
    pub magic: u32,
    /// 主版本号
    pub major: u32,
    /// 次版本号
    pub minor: u32,
    /// 构建信息
    pub build_info: [u8; 116],
}

/// 健康检查消息
#[repr(C)]
pub struct UPDATE_HealthCheckMsgType {
    /// 分区 ID
    pub pid: u16,
    /// 对齐填充
    pub _pad1: [u8; 2],
    /// 版本信息
    pub version: IMGL_VersionType,
}

/// 安装配置
#[repr(C)]
pub struct UPDATE_InstallCfgMsgType {
    /// NVM 通道
    pub nvm_channel: u32,
    /// 获取通道
    pub fetch_channel: u32,
    /// 保留
    pub reserved: [u32; 2],
    /// 擦除大小
    pub nvm_erase_size: u32,
    /// 文件大小
    pub file_size: u32,
    /// 文件服务器 IP
    pub ip_addr: u32,
    /// 文件服务器端口
    pub port_num: u32,
    /// 附加信息
    pub info: [u32; 4],
    /// 文件名
    pub name: [u8; 256],
}

/// 安装消息（包含配置 + 设备收到的文件大小）
#[repr(C)]
pub struct UPDATE_InstallMsgType {
    pub cfg: UPDATE_InstallCfgMsgType,
    /// 输出：设备已接收的文件大小
    pub recv_file_size: u32,
}

// ====== 上层应用类型 ======

/// 配置键值对（用于前端展示）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfigPair {
    pub name: String,
    pub value: String,
}
