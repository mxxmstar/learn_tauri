# bcmtool 模块分析文档

## 概述

bcmtool 是一个通过 **RPC（Remote Procedure Call）协议** 与 Broadcom 设备通信的工具模块，实现设备的重启、配置读写、固件升级、健康检查、版本查询等功能。

整个模块采用**混合架构**：
- 底层 C 语言实现：RPC 连接管理、DMON 设备监控、CONFIG 配置、UPDATE 固件更新协议
- 上层 C++（Qt）封装：面向对象的类封装、信号槽异步通信、线程管理

---

## 目录结构

```
src-cpp/bcmtool/
├── bcmtool.cpp / .h          # 顶层业务门面
├── bcm_rpc.cpp / .h          # Qt 桥接层
├── bcm_flash.cpp / .h        # 固件传输服务 + UPDATE 协议
├── bcm_dmon.cpp / .h         # DMON 设备监控协议（纯 C）
├── bcm_config.cpp / .h       # CONFIG 配置读写协议（纯 C）
├── bcm_common.cpp / .h       # 公共工具函数（纯 C）
└── rpc_connect.cpp / .h      # RPC 套接字通信层（纯 C）
```

---

## 各文件详细说明

---

### 1. `bcm_common` — 公共类型定义与工具函数

| 项目 | 说明 |
|------|------|
| 语言 | **纯 C** |
| 头文件 | `bcm_common.h` |
| 源文件 | `bcm_common.cpp` |

**定义的核心类型：**

| 类型 | 描述 |
|------|------|
| `BCM_HandleType` (uint64_t) | RPC 连接句柄 |
| `BCM_BootModeType` (uint32_t) | 启动模式（BROM/BL/FW/DEFAULT） |
| `BCM_StateType` (uint32_t) | 模块状态（UNINIT/INIT/READY/RUN/ERROR） |
| `BCM_CompIDType` (uint16_t) | 组件 ID |
| `BCM_GroupIDType` (uint8_t) | 分组 ID（RPC/SYS/NVM/IO/CRYPTO 等） |
| `BCM_MsgType` (uint32_t) | 命令 ID（包含 Group + Component + MsgID） |
| `BCM_ErrorType` (int32_t) | 错误码 |

**MACRO 构造命令 ID：**
```c
BCM_MSG(aGrp, aComp, aId)
// 将 GroupID(6bit) + CompID(16bit) + MsgID(8bit) 拼成 32bit 命令字
```

**定义的工具函数：**

| 函数 | 说明 |
|------|------|
| `CPU_LEToNative32/64` | 小端 → 本机字节序 |
| `CPU_NativeToLE32/16` | 本机字节序 → 小端 |
| `CPU_BEToNative32/16` | 大端 → 本机字节序 |
| `ByteToU32` | 4 字节数组 → uint32_t |
| `ByteToU16` | 2 字节数组 → uint16_t |
| `BCM_MemSet` | 封装 memset |
| `BCM_MemCpy` | 封装 memcpy |

**定义的分组 ID（节选）：**
- `BCM_GROUPID_RPC = 0x00` — RPC 组
- `BCM_GROUPID_SYS = 0x01` — 系统组（含 DMON）
- `BCM_GROUPID_NVM = 0x03` — NVM 组（含 CONFIG、UPDATE）

**定义的组件 ID（节选）：**
- `BCM_RPC_ID = 0x0022` — RPC 模块
- `BCM_DMN_ID = 0x0123` — Device Monitor
- `BCM_CFG_ID = 0x0324` — Config 模块
- `BCM_UPD_ID = 0x0323` — Update 模块

---

### 2. `rpc_connect` — RPC 套接字通信层

| 项目 | 说明 |
|------|------|
| 语言 | **纯 C** |
| 头文件 | `rpc_connect.h` |
| 源文件 | `rpc_connect.cpp` |

**RPC 消息结构 (`RPC_MsgType`)：**

```
┌──────────────┬──────────────┬──────────────┬──────────────┐
│ magic(4B)   │ version(4B)  │    cmd(4B)   │ timeoutMs(4B)│
├──────────────┼──────────────┼──────────────┼──────────────┤
│ response(4B) │    len(4B)   │ appInfoTop(4B)│ appInfo(16B) │
├──────────────┼──────────────┼──────────────┼──────────────┤
│   rsvd(4B)   │        payload(448B)          │
└──────────────┘──────────────────────────────────┘
```

- Magic: `0x5250434D` ("RPCM")
- Header Size: `sizeof(RPC_MsgType) - 448`
- Payload: 448 字节

**核心函数：**

| 函数 | 说明 |
|------|------|
| `RPC_Open` | TCP 连接设备，返回句柄 |
| `RPC_Send` | 发送 RPC 消息（填充 header + payload） |
| `RPC_Recv` | 接收 RPC 消息（处理粘包，验证 magic） |
| `RPC_SendRecv` | Send + Recv 组合（请求-响应模式） |
| `RPC_Close` | 关闭连接并释放内存 |

**RPC 会话上下文 (`RPC_ContextType`)：**
```c
typedef struct {
    uint32_t        magic;       // 0xA55AA55A
    SOCKET          fd;          // 套接字
    RPC_MsgVerType  version;     // 协议版本
    uint32_t        timeout;     // 超时时间(ms)
    uint32_t        filledSize;  // 已接收数据长度(粘包处理)
} RPC_ContextType;
```

**工作流程：**
```
RPC_Open → 创建 socket → connect
   ↓
RPC_SendRecv(cmd, payload)   ← 上层模块调用
   ├── RPC_Send(msg)         ← 填充 header + payload，send()
   └── RPC_Recv(resp)         ← recv()，解析 header，验证 magic
   ↓
RPC_Close → closesocket → free
```

---

### 3. `bcm_dmon` — DMON 设备监控协议

| 项目 | 说明 |
|------|------|
| 语言 | **纯 C** |
| 头文件 | `bcm_dmon.h` |
| 源文件 | `bcm_dmon.cpp` |

**DMON 命令 ID：**

| 命令 | ID | 说明 |
|------|----|------|
| `DMON_ID_PING` | `DMON_ID(0x01)` | Ping 设备 |
| `DMON_ID_SYNC` | `DMON_ID(0x02)` | 获取同步状态 |
| `DMON_ID_SYNC_WAIT` | `DMON_ID(0x03)` | 等待设备达到指定状态 |
| `DMON_ID_MEM_READ` | `DMON_ID(0x10)` | 读内存 |
| `DMON_ID_MEM_WRITE` | `DMON_ID(0x11)` | 写内存 |
| `DMON_ID_SW_VERSION` | `DMON_ID(0x20)` | 获取软件版本 |
| `DMON_ID_HW_VERSION` | `DMON_ID(0x21)` | 获取硬件版本 |
| `DMON_ID_REBOOT` | `DMON_ID(0x22)` | 重启设备 |
| `DMON_ID_DEEPSLEEP` | `DMON_ID(0x23)` | 深度睡眠 |

**核心函数：**

| 函数 | 说明 |
|------|------|
| `DMON_ReadMem` | 读设备内存（地址 + 宽度 + 设备 ID） |
| `DMON_WriteMem` | 写设备内存 |
| `DMON_Ping` | Ping 设备，获取模式 + 硬件版本 |
| `DMON_Sync` | 获取设备同步状态（模式、版本、运行时间等） |
| `DMON_SyncWait` | 阻塞等待设备进入指定状态 |
| `DMON_GetSwVersion` | 获取软件版本字符串 |
| `DMON_GetHwVersion` | 获取硬件版本（厂商、型号、修订号、安全模式） |
| `DMON_Reboot` | 重启设备（10ms 延时） |
| `DMON_DeepSleep` | 让设备进入深度睡眠 |

**消息结构：**
```
DMON_MsgType
├── magic    (4B)
├── id       (4B)  ← DMON_ID_xxx
├── status   (4B)
├── len      (4B)
└── u        (union)
    ├── ping        → DMON_PingMsgType
    ├── sync        → DMON_SyncMsgType
    ├── memAccess   → DMON_MemAccessMsgType
    ├── swVersion   → DMON_SwVersionMsgType
    ├── hwVersion   → DMON_HwVersionMsgType
    ├── reboot      → DMON_RebootMsgType
    ├── deepSleep   → DMON_DeepSleepMsgType
    └── heartbeat   → DMON_HeartBeatMsgType
```

---

### 4. `bcm_config` — CONFIG 配置读写协议

| 项目 | 说明 |
|------|------|
| 语言 | **纯 C** |
| 头文件 | `bcm_config.h` |
| 源文件 | `bcm_config.cpp` |

**CONFIG 命令 ID：**

| 命令 | ID | 说明 |
|------|----|------|
| `CONFIG_CMD_RPC_READ` | `CONFIG_ID_OF(1)` | 读取全部配置 |
| `CONFIG_CMD_RPC_WRITE` | `CONFIG_ID_OF(2)` | 写入配置项 |

**支持的配置项 ID：**

| 配置项 | ID | 值类型 | 长度 |
|--------|----|--------|------|
| `CONFIG_MEDIA_MIRROR` | 0x0101 | uint8 | 1B |
| `CONFIG_MEDIA_FPS` | 0x0102 | uint8 | 1B |
| `CONFIG_MEDIA_SOMEIPUDPPORT` | 0x0103 | uint16 | 2B |
| `CONFIG_MEDIA_SOMEIPRTPPORT` | 0x0104 | uint16 | 2B |
| `CONFIG_NETWORK_DHCP` | 0x0201 | uint8 | 1B |
| `CONFIG_NETWORK_IP` | 0x0202 | 字符串 | 16B |
| `CONFIG_NETWORK_MAC` | 0x0203 | 字符串 | 20B |
| `CONFIG_AVTP_DSTMAC` | 0x0301 | 字符串 | 20B |
| `CONFIG_AVTP_STREAMID` | 0x0302 | uint64 | 8B |

**配置项编码格式：**
```
┌───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┐
│ mask(4B)                    │ item_data(nB)              │
│ bit[31:24] = length        │                            │
│ bit[23:16] = id[7:0]       │                            │
│ bit[15:8]  = id[15:8]      │                            │
│ bit[7:0]   = 0xAB          │                            │
└─────────────────────────────┴────────────────────────────┘
```

**核心函数：**

| 函数 | 说明 |
|------|------|
| `CONFIG_RpcRead` | 发送 RPC 读取全部配置，返回 `CONFIG_RpcMsg`（ctx + len） |
| `CONFIG_RpcWrite` | 发送 RPC 写入配置项 |
| `CONFIG_ExtractItem` | 从配置缓冲区中逐条解析配置项（name / value） |

**配置消息结构 (`CONFIG_RpcMsg`)：**
```c
typedef struct {
    uint8_t  ctx[256];   // 配置数据缓冲区
    uint32_t len;        // 数据长度
} CONFIG_RpcMsg;
```

---

### 5. `bcm_flash` — 固件传输服务 + UPDATE 协议

| 项目 | 说明 |
|------|------|
| 语言 | **混合（纯 C 函数 + C++ 类）** |
| 头文件 | `bcm_flash.h` |
| 源文件 | `bcm_flash.cpp` |

#### 5.1 UPDATE 纯 C 函数层

**UPDATE 命令 ID：**

| 命令 | ID | 说明 |
|------|----|------|
| `UPDATE_ID_HEALTH_CHECK` | `UPDATE_ID(0x00)` | 健康检查 |
| `UPDATE_ID_GET_BOOT_COPY_CFG` | `UPDATE_ID(0x10)` | 获取启动副本配置 |
| `UPDATE_ID_SET_BOOT_COPY_CFG` | `UPDATE_ID(0x11)` | 设置启动副本配置 |
| `UPDATE_ID_SAFE_INSTALL` | `UPDATE_ID(0x20)` | 安全安装 |
| `UPDATE_ID_FULL_INSTALL` | `UPDATE_ID(0x21)` | 完整安装 |
| `UPDATE_ID_RAW_INSTALL` | `UPDATE_ID(0x22)` | 原始安装 |
| `UPDATE_ID_SYNC` | `UPDATE_ID(0x30)` | 同步 |

**核心函数：**

| 函数 | 说明 |
|------|------|
| `UPDATE_HealthCheck` | 检查指定分区的镜像版本（PID + 版本信息） |
| `UPDATE_InstallHost` | 安装主机通用函数（填充 install 消息，调用 RPC_SendRecv） |
| `UPDATE_FullInstall` | 完整安装（调用 `UPDATE_InstallHost` + `UPDATE_ID_FULL_INSTALL`） |

**关键结构体：**
```c
// 安装配置
typedef struct {
    IMGL_ChannelType nvmChannel;    // NVM 通道
    IMGL_ChannelType fetchChannel;  // 获取通道
    uint32_t         nvmEraseSize;  // 擦除大小
    uint32_t         fileSize;      // 文件大小
    uint32_t         ipAddr;        // 文件服务器 IP
    uint32_t         portNum;       // 文件服务器端口
    uint8_t          name[256];     // 文件名
} UPDATE_InstallCfgMsgType;

// 安装消息
typedef struct {
    UPDATE_InstallCfgMsgType cfg;
    uint32_t                 recvFileSize;  // 输出：设备已接收大小
} UPDATE_InstallMsgType;
```

**通道类型（IMGL_ChannelType）：**
```
IMGL_CHANNEL_ID_NVM_0   = 0x4E564D30  ("NVM0")
IMGL_CHANNEL_ID_RPC_FTP = 0x52465450  ("RFTP")
IMGL_CHANNEL_ID_RPC_IPC = 0x52495043  ("RIPC")
```

#### 5.2 FileTransferServer — TCP 文件传输服务（C++ 类）

| 方法 | 说明 |
|------|------|
| `start()` | 启动 TCP server，监听随机端口，等待设备连接，分块发送固件数据 |

**工作流程：**
```
1. socket() → bind(随机端口) → listen(2)
2. getsockname() 获取实际端口号
3. emit ready(port, fileName, fileSize, hostIP) → 通知设备连接
4. accept() 等待设备 TCP 连接
5. 分块（256B/次）send() 固件数据
6. 完成后 closesocket()
```

**信号：**
| 信号 | 参数 | 说明 |
|------|------|------|
| `consolePrint` | (QString, PrintLevel) | 日志输出 |
| `signal_progress` | (action, progress, finish) | 升级进度：`0`执行中 / `1`完成 / `-1`失败 |
| `ready` | (port, fileName, fileSize, ip) | 通知设备连接 |
| `finished` | — | 传输完成 |

---

### 6. `bcm_rpc` — Qt 桥接层

| 项目 | 说明 |
|------|------|
| 语言 | **C++ (Qt)** |
| 头文件 | `bcm_rpc.h` |
| 源文件 | `bcm_rpc.cpp` |
| 父类 | `QObject`，含 `Q_OBJECT` |

**职责：** 将底层 C 函数封装为 Qt 可用的信号槽接口，处理 `QString` ↔ `char*` 转换，异步执行 RPC 操作。

**公开槽方法：**

| 方法 | 调用底层 | 说明 |
|------|----------|------|
| `rebootFinished()` | `RPC_Open` → `DMON_Reboot` → `RPC_Close` | 重启设备，等待 60s 连接 |
| `reboot()` | 同上（无 `emit finished()`） | 内部重启，不通知上层 |
| `readConfig()` | `RPC_Open` → `CONFIG_RpcRead` → `showConfig` → `RPC_Close` | 读取并解析全部配置 |
| `writeConfig(name, val)` | `writeConfigMsg` → `RPC_Open` → `CONFIG_RpcWrite` → `RPC_Close` → `reboot()` | 写入配置后重启 |
| `writeConfigMsg(name, val, msg)` | — | 将 `name:val` 按配置项格式编码到消息中 |
| `getVersion()` | `RPC_Open` → `DMON_GetSwVersion` → `RPC_Close` | 获取固件版本号 |
| `healthCheck(pid)` | `RPC_Open` → `UPDATE_HealthCheck` → `RPC_Close` | 检查分区运行状态 |
| `fullInstall(...)` | `RPC_Open` → `UPDATE_FullInstall` → `RPC_Close` → `reboot()` | 固件完整安装（文件已由 FileTransferServer 传输） |

**信号：**
| 信号 | 说明 |
|------|------|
| `consolePrint(QString, PrintLevel)` | 日志输出 |
| `finished()` | 操作完成 |
| `versionInfo(QString)` | 版本信息 |
| `configPair(QString)` | 配置键值对（`"key:val"`） |

---

### 7. `bcmtool` — 顶层业务门面

| 项目 | 说明 |
|------|------|
| 语言 | **C++ (Qt)** |
| 头文件 | `bcmtool.h` |
| 源文件 | `bcmtool.cpp` |
| 父类 | `QObject`，含 `Q_OBJECT` |

**职责：** 为 QML/UI 层提供简洁的业务接口，内部创建子线程 + `BcmRPC` 对象，通过信号槽串联异步流程。

**公开方法：**

| 方法 | 说明 |
|------|------|
| `reboot(deviceIP)` | 在新线程中执行设备重启 |
| `readConfig(deviceIP)` | 在新线程中读取设备配置 |
| `writeConfig(deviceIP, name, value)` | 在新线程中写入配置项（通过信号触发） |
| `healthCheck(deviceIP, pid)` | 在新线程中健康检查（通过信号触发） |
| `getVersion(deviceIP)` | 在新线程中获取版本号 |
| `fullInstall(fileName, fileBytes, deviceIP, hostIP)` | 创建两个线程：`FileTransferServer`（TCP 文件传输）+ `BcmRPC`（RPC 安装），串行完成升级 |

**内部实现模式：**

每个方法都遵循相同的异步模式：
```
1. new QThread()
2. new BcmRPC(deviceIP) 或 new FileTransferServer(...)
3. moveToThread(thread)
4. connect(signals/slots) → 串联信号链
5. thread->start()
```

以 `reboot` 为例的信号链：
```
thread::started → BcmRPC::rebootFinished → BcmRPC::consolePrint (输出日志)
                                          → BcmRPC::finished → thread::quit
                                          → thread::finished → deleteLater (清理对象)
```

`fullInstall` 的双线程信号链：
```
                     FileTransferServer::ready
                    ↓
serverThread ──→ BcmRPC::fullInstall ──→ BcmRPC::consolePrint
                                          BcmRPC::finished → flashThread::quit
```

---

## 模块依赖关系

```
[QML/UI]
    │
    ▼
┌───────────────────────────────────────────┐
│              BcmTool (bcmtool)            │  ← C++ Qt Facade
│  - 线程管理、异步调度、信号桥接            │
└──────────┬────────────────────────────────┘
           │
    ┌──────┴──────┐
    ▼             ▼
┌──────────┐ ┌────────────┐
│ BcmRPC   │ │ FileTrans- │  ← C++ Qt
│ (bcm_rpc)│ │ ferServer  │
└────┬─────┘ │ (bcm_flash)│
     │       └────────────┘
     ▼
┌─────────────────────────────────────┐
│     纯 C 函数调用层                   │
├────────────┬──────────┬─────────────┤
│ bcm_dmon   │bcm_config│ bcm_flash   │  ← 纯 C
│ (DMON)     │(CONFIG)  │ (UPDATE)    │
└─────┬──────┴────┬─────┴──────┬──────┘
      │           │            │
      └───────────┼────────────┘
                  ▼
      ┌─────────────────────┐
      │   rpc_connect        │  ← 纯 C
      │ (RPC socket 通信)    │
      └─────────┬───────────┘
                ▼
          [TCP Socket]
          [Broadcom Device]
```

## 数据流示例

### 读取配置（完整调用链）

```
BcmTool::readConfig("192.168.1.1")
  → new QThread() + new BcmRPC("192.168.1.1")
  → thread->start()
      → BcmRPC::readConfig()
          → RPC_Open("192.168.1.1", 5555, 60000, &hdl)
              → socket() → connect()
          → CONFIG_RpcRead(hdl, &readMsg)
              → RPC_SendRecv(hdl, CONFIG_CMD_RPC_READ, ...)
                  → RPC_Send() → send()
                  → RPC_Recv() → recv()
          → showConfig(&readMsg)
              → CONFIG_ExtractItem() 逐一解析配置项
              → emit configPair("mirror mode:0")
              → emit configPair("FPS:30")
              → ...
          → RPC_Close(hdl)
              → closesocket() → free()
          → emit finished()
      → thread::quit → deleteLater
```

### 固件升级（完整调用链）

```
BcmTool::fullInstall("firmware.bin", bytes, "192.168.1.1", "10.0.0.1")
  → new serverThread + FileTransferServer
  → new flashThread + BcmRPC
  → serverThread->start()
  → flashThread->start()

  第一阶段：TCP 文件传输
  FileTransferServer::start()
    → socket() → bind(0) → listen(2)
    → getsockname() → 获取实际端口
    → emit ready(port, "firmware.bin", size, "10.0.0.1")
    → accept() (阻塞等待设备连接)
    → 分块 send() 固件数据 (256B/次)
    → emit finished()

  第二阶段：RPC 安装（由 ready 信号触发）
  BcmRPC::fullInstall(port, "firmware.bin", size, "10.0.0.1")
    → RPC_Open("192.168.1.1", 5555, 60000, &hdl)
    → UPDATE_FullInstall(hdl, &install, &rcvdSz)
        → UPDATE_InstallHost(...)
            → RPC_SendRecv(hdl, UPDATE_ID_FULL_INSTALL, ...)
    → RPC_Close(hdl)
    → reboot() (安装成功后重启)
    → emit finished()
```

---

## 协议栈层次总结

| 层次 | 文件 | 语言 | 职责 |
|------|------|------|------|
| **应用层** | `bcmtool` | C++ (Qt) | UI 接口、线程调度 |
| **服务层** | `bcm_rpc` | C++ (Qt) | QObject 封装、类型转换、信号桥接 |
| **服务层** | `bcm_flash` (FileTransferServer) | C++ (Qt) | TCP 文件传输 |
| **协议层** | `bcm_dmon` | 纯 C | DMON 命令封装 |
| **协议层** | `bcm_config` | 纯 C | CONFIG 命令封装 |
| **协议层** | `bcm_flash` (UPDATE 函数) | 纯 C | UPDATE 命令封装 |
| **传输层** | `rpc_connect` | 纯 C | RPC 消息收发、TCP socket |
| **公共层** | `bcm_common` | 纯 C | 类型定义、字节序转换、工具函数 |
