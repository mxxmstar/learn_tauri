# StreamPlayer 网卡数据抓取与视频播放技术分析文档

## 目录
1. [概述](#1-概述)
2. [整体架构](#2-整体架构)
3. [网卡数据抓取流程](#3-网卡数据抓取流程)
4. [自定义传输协议格式](#4-自定义传输协议格式)
5. [JPEG 帧解析与拼接](#5-jpeg-帧解析与拼接)
6. [视频播放流程](#6-视频播放流程)
7. [线程模型与同步机制](#7-线程模型与同步机制)
8. [关键数据结构](#8-关键数据结构)
9. [与 Rust pcap 模块的对应关系](#9-与-rust-pcap-模块的对应关系)

---

## 1. 概述

StreamPlayer 是一个基于 Qt 框架开发的网络视频流播放器，其核心功能是通过 **pcap 库（WinPcap/Npcap）** 直接从网卡捕获原始网络数据包，解析自定义的以太网协议（EtherType = 0x0022），拼接 JPEG 图像帧，并最终渲染显示成视频。

**技术栈**：
- **C++ / Qt 5** — 主框架与 UI
- **libpcap / Npcap** — 网卡数据包捕获
- **自定义以太网协议（EtherType 0x0022）** — 视频流传输协议
- **JPEG 基线编码** — 图像压缩格式

**核心流程**：
```
网卡 → pcap抓包 → 协议解析 → JPEG拼接 → QImage解码 → UI渲染
```

---

## 2. 整体架构

### 2.1 模块划分

```
┌─────────────────────────────────────────────────────────────┐
│                      MainWindow (UI层)                      │
│  - 网卡选择、播放控制、图像显示、配置管理                   │
└────────────────┬───────────────────────────────┬───────────┘
                 │                               │
                 ▼                               ▼
┌─────────────────────────┐       ┌─────────────────────────┐
│      Capture 模块        │       │      Decoder 模块        │
│  (capture/capture.cpp)  │       │  (decoder/decoder.cpp)   │
│  - 网卡枚举             │       │  - 协议解析              │
│  - 实时抓包             │       │  - JPEG帧拼接            │
│  - 数据包过滤           │       │  - QImage解码            │
└────────┬────────────────┘       └────────┬────────────────┘
             │                                   │
             │ 静态队列 + 条件变量               │
             ▼                                   ▼
┌─────────────────────────────────────────────────────────────┐
│          静态全局队列 (Capture类静态成员)                    │
│  s_packetArray[100][2000] + s_packetQueue + s_packetQueueCV │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 类关系图

```
MainWindow
    ├── Capture*           (m_pCaptureModule)
    ├── Decoder*           (m_pDecoderModule)
    └── QThread*           (m_pPlayThread)

Capture  [捕获线程]
    ├── pcap_t*            (captureHandler_)
    └── std::thread        (captureThread_)

Decoder  [解码线程]
    └── 引用 Capture 的静态队列
```

---

## 3. 网卡数据抓取流程

### 3.1 网卡枚举

**源码位置**：`capture/capture.cpp` → `Capture::getNetworkInterface()`

**调用 pcap API**：
```cpp
pcap_findalldevs(&alldevs, errbuf)  // 枚举所有网卡
pcap_freealldevs(alldevs)           // 释放网卡列表
```

**流程说明**：
1. 调用 `pcap_findalldevs()` 获取系统中所有网络接口
2. 遍历 `pcap_if_t` 链表，提取网卡名称（`device->name`）和描述（`device->description`）
3. 保存网卡名称到 `networkInterfaceList_`（用于后续打开）
4. 返回描述列表给 UI 显示

**关键数据结构**：
```cpp
struct pcap_if_t {
    char *name;                 // 网卡设备名称，如 "\Device\NPF_{GUID}"
    char *description;          // 网卡描述，如 "Intel(R) Ethernet Connection"
    struct pcap_addr *addresses;
    bpf_u_int32 flags;         // PCAP_IF_LOOPBACK 等标志
    struct pcap_if *next;      // 下一个网卡
};
```

### 3.2 打开网卡

**源码位置**：`capture/capture.cpp` → `Capture::setCaptureHandler(index)`

**调用 pcap API**：
```cpp
pcap_open_live(
    networkInterfaceList_[index].c_str(),  // 网卡设备名
    65536,   // snaplen: 捕获的最大字节数（足够大以捕获完整包）
    1,       // promisc: 混杂模式（1=开启）
    1000,    // timeout: 超时时间（毫秒）
    errbuf   // 错误信息缓冲区
)
```

**参数说明**：
| 参数 | 值 | 说明 |
|---|---|---|
| `snaplen` | 65536 | 足够捕获任意以太网帧（最大 1518 字节） |
| `promisc` | 1 | 混杂模式，捕获网络上所有流量（不仅限于本机） |
| `timeout` | 1000 ms | 读超时，避免频繁系统调用 |

**返回值**：`pcap_t*` 句柄，后续所有操作基于此句柄。

### 3.3 启动捕获线程

**源码位置**：`capture/capture.cpp` → `Capture::start()` / `Capture::threadLoop()`

**调用 pcap API**：
```cpp
pcap_loop(captureHandler_, 0, packetHandle, nullptr)
```

**参数说明**：
- `captureHandler_` — pcap 句柄
- `0` — 捕获包数量限制（0 = 无限）
- `packetHandle` — 回调函数指针
- `nullptr` — 传递给回调的用户数据

**线程模型**：
- `pcap_loop()` 是**阻塞式**调用，直到：
  1. 捕获到指定数量的包（此处为无限）
  2. 发生错误
  3. 调用 `pcap_breakloop()` 打破循环

### 3.4 数据包过滤回调

**源码位置**：`capture/capture.cpp` → `Capture::packetHandle()`

**过滤条件**：
```cpp
if ((packet[12] == 0x22)) {  // EtherType = 0x0022
    // 接受此包
}
```

**以太网帧结构**：
```
偏移量    长度    字段
0        6       目标 MAC 地址
6        6       源 MAC 地址
12       2       EtherType（协议类型）
14       N       协议数据（Payload）
```

- **EtherType = 0x0022**：这是自定义的协议类型（IEEE 未分配），用于标识视频流数据包。
- 标准 EtherType 参考：0x0800 = IPv4，0x0806 = ARP，0x86DD = IPv6

**数据包存储**：
```cpp
// 使用环形缓冲区避免动态内存分配
int index = s_packetAarrayIndex % PACKET_ARRAY_SIZE;  // PACKET_ARRAY_SIZE = 100
memcpy(s_packetArray[index], packet, header->caplen); // 拷贝整个以太网帧
s_packetQueue.push(s_packetArray[index]);             // 推入队列
s_packetAarrayIndex++;
s_packetCount++;
s_packetQueueCV.notify_one();  // 通知解码线程
```

**环形缓冲区设计**：
- `s_packetArray[100][2000]` — 预分配的 100 个包缓冲区，每个最大 2000 字节
- `s_packetAarrayIndex` — 原子递增索引，取模得到实际数组下标
- 当队列满时（超过 100 个包），旧包会被覆盖（环形行为）

### 3.5 停止捕获

**源码位置**：`capture/capture.cpp` → `Capture::end()`

**调用 pcap API**：
```cpp
pcap_breakloop(captureHandler_);  // 打破 pcap_loop 循环
captureThread_.join();             // 等待捕获线程退出
```

**注意**：`pcap_breakloop()` 是线程安全的，可以在任意线程调用。

---

## 4. 自定义传输协议格式

### 4.1 协议层次结构

```
┌──────────────────────────────────────────────────────────┐
│                   以太网帧 (Ethernet II)                 │
│  MAC目标(6) | MAC源(6) | EtherType(2) | 数据(N)       │
│                                                        │
│  EtherType = 0x0022 ──────────────────────────────────┐ │
│                                                        │ │
│  ┌──────────────────────────────────────────────────┐  │ │
│  │           自定义协议头部 (24 字节)               │  │ │
│  │  保留(1) | 帧标志(1) | ... | 负载长度(2) | ...  │  │ │
│  │  ... | 帧标志(1) | ... | JPEG数据(N)           │  │ │
│  │                                                  │  │ │
│  │  ┌────────────────────────────────────────────┐  │  │ │
│  │  │         JPEG 数据（原始 JPEG 字节流）      │  │  │ │
│  │  │  FF D8 ... FF D9 (完整或部分 JPEG 帧)      │  │  │ │
│  │  └────────────────────────────────────────────┘  │  │ │
│  └──────────────────────────────────────────────────┘  │ │
└──────────────────────────────────────────────────────────┘
```

### 4.2 以太网帧格式

**总长度**：至少 14 字节 + 协议数据

| 偏移量（相对于帧起始） | 长度（字节） | 字段名 | 说明 |
|---|---|---|---|
| 0 | 6 | 目标 MAC | 目的主机 MAC 地址 |
| 6 | 6 | 源 MAC | 发送方 MAC 地址 |
| 12 | 2 | **EtherType** | **协议类型标识** |
| 14 | N | 协议数据 | 自定义协议数据 |

**EtherType 值**：
- `0x0022` — 自定义视频流协议（本应用使用）
- 注意：代码中只检查 `packet[12] == 0x22`，即 EtherType 的低字节。正确的检查应该是 `packet[12] == 0x00 && packet[13] == 0x22` 或 `(packet[12] << 8 ｜ packet[13]) == 0x0022`。当前代码可能存在字节序问题。

### 4.3 自定义协议格式（EtherType 0x0022）

**重要说明**：以下偏移量均**相对于以太网帧起始（packet[0]）**，而非协议数据起始。

#### 4.3.1 协议头部字段详解

通过逆向分析 `decoder.cpp` 和 `bcm_jpeg.cpp`，协议格式如下：

| 以太网帧偏移量 | 协议内偏移量 | 长度（字节） | 字段名 | 说明 | 访问代码 |
|---|---|---|---|---|---|
| 14 | 0 | 1 | **保留/版本** | 未知用途，可能为协议版本或标志 | — |
| 15 | 1 | 1 | **帧起始标志** | bit 0 = 1 表示 JPEG 帧的起始包 | `raw[15] & 0x1` |
| 16-19 | 2-5 | 4 | **保留字段** | 未知用途 | — |
| 20-33 | 6-19 | 14 | **保留字段** | 未知用途，可能包含时间戳、序列号等 | — |
| 34 | 20 | 2 | **负载长度** | JPEG 数据的长度（大端字节序） | `(raw[34] << 8) ｜ raw[35]` |
| 36 | 22 | 1 | **帧结束标志** | bit 4 = 1 表示 JPEG 帧的结束包 | `raw[36] & 0x10` |
| 37 | 23 | 1 | **保留字段** | 未知用途 | — |
| 38+ | 24+ | N | **JPEG 数据** | 原始的 JPEG 字节流（部分或完整） | `raw + 38` |

**注意**：协议头部总计 **24 字节**（偏移量 14-37）。

#### 4.3.2 帧标志位详解

**帧起始标志**（以太网帧偏移量 15，协议内偏移量 1）：
```
Bit 7 6 5 4 3 2 1 0
    - - - - - - - - -
    ^^^^^^^^^^^^^^^^ 保留（未知）
                    ^  帧起始标志（1 = 起始包，0 = 中间包）
```

**帧结束标志**（以太网帧偏移量 36，协议内偏移量 22）：
```
Bit 7 6 5 4 3 2 1 0
    - - - - - - - - -
    ^^^^^^^^^^^^^^^^ 保留（未知）
          ^          帧结束标志（1 = 结束包，0 = 中间包）
```

**包类型判断逻辑**：
| 帧起始标志 | 帧结束标志 | 包类型 |
|---|---|---|
| 1 | 0 | 起始包（JPEG 帧的第一部分） |
| 0 | 0 | 中间包（JPEG 帧的中间部分） |
| 0 | 1 | 结束包（JPEG 帧的最后一部分） |
| 1 | 1 | 完整包（JPEG 帧在一个包内完整传输） |

#### 4.3.3 JPEG 图像参数（仅起始包包含）

**重要发现**：通过进一步分析 `bcm_jpeg.cpp` 中的 `BCM_JPG_WriteToImg()` 函数，发现**图像参数实际上嵌入在 JPEG 数据的前 12 字节中**，而非协议头部。

**数据布局**（相对于 `raw + 38`，即 JPEG 数据起始位置）：

| 偏移量（相对于 JPEG 数据起始） | 长度（字节） | 字段名 | 说明 | 访问代码 |
|---|---|---|---|---|
| 0 | 12 | **嵌入式头部** | 包含图像参数的 12 字节头部 | — |
| 5 | 1 | **质量因子 (QP)** | JPEG 质量参数（1-100） | `data[5]` |
| 6 | 1 | **图像宽度** | 实际宽度 = 值 × 8 像素 | `data[6] * 8U` |
| 7 | 1 | **图像高度** | 实际高度 = 值 × 8 像素 | `data[7] * 8U` |
| 8-9 | 2 | **重启间隔 (RST)** | JPEG 重启标记间隔 | `(data[8] << 8) ｜ data[9]` |
| 10-11 | 2 | **帧计数** | 某种计数器（低 10 位有效） | `(data[10] << 8 ｜ data[11]) & 0x3FF` |
| 12+ | N | **JPEG 数据** | 实际的 JPEG 字节流（跳过 12 字节头部后） | `data + 12` |

**参数提取代码**（`bcm_jpeg.cpp`）：
```cpp
if (frameStart) {
    uint8_t qp = data[5];                          // 质量因子
    uint16_t width = data[6] * 8U;                 // 图像宽度（像素）
    uint16_t height = data[7] * 8U;                // 图像高度（像素）
    uint16_t rstInt = ((uint32_t)data[8]<<8)|data[9];  // 重启间隔
    uint16_t rstCount = (((uint32_t)data[10]<<8)|data[11])&0x3FFU;  // 帧计数（未使用）
    
    // 根据参数生成 JPEG 文件头
    BCM_JPG_EncodeHeader(width, height, qp, rstInt);
}
```

#### 4.3.4 完整的协议数据包格式

```
以太网帧（14 字节）
├── 目标 MAC (6 字节)
├── 源 MAC (6 字节)
└── EtherType = 0x0022 (2 字节)

自定义协议数据（24+ 字节）
├── 保留/版本 (1 字节)                   [以太网帧偏移量 14]
├── 帧起始标志 (1 字节)                  [以太网帧偏移量 15]
├── 保留字段 (4 字节)                    [以太网帧偏移量 16-19]
├── 保留字段 (14 字节)                   [以太网帧偏移量 20-33]
├── 负载长度 (2 字节，大端)             [以太网帧偏移量 34-35]
├── 帧结束标志 (1 字节)                  [以太网帧偏移量 36]
├── 保留字段 (1 字节)                    [以太网帧偏移量 37]
│
└── JPEG 数据 (N 字节)                   [以太网帧偏移量 38+]
    ├── 嵌入式头部 (12 字节)             [JPEG 数据偏移量 0-11]
    │   ├── 未知 (5 字节)
    │   ├── 质量因子 QP (1 字节)
    │   ├── 图像宽度 (1 字节，×8)
    │   ├── 图像高度 (1 字节，×8)
    │   ├── 重启间隔 (2 字节)
    │   └── 帧计数 (2 字节)
    │
    └── JPEG 字节流 (N-12 字节)         [JPEG 数据偏移量 12+]
        └── FF D8 ... FF D9 (标准 JPEG 格式)
```

### 4.4 协议设计分析

#### 4.4.1 设计特点

1. **基于以太网层**：不经过 IP/TCP/UDP，直接通过自定义 EtherType 传输，延迟低。
2. **无连接**：每个包独立传输，无需建立连接。
3. **分片传输**：大 JPEG 帧被分割成多个包传输，通过帧起始/结束标志重组。
4. **嵌入式参数**：图像参数（宽、高、质量）嵌入在 JPEG 数据流的前 12 字节中，而非协议头部。

#### 4.4.2 潜在问题

1. **EtherType 检查不完整**：代码只检查 `packet[12] == 0x22`，未检查 `packet[13] == 0x00`。如果网络中有其他使用 0x22xx EtherType 的协议，可能误收。
2. **无错误校验**：协议没有 CRC 或校验和字段，无法检测数据传输错误。
3. **无序列号**：无法检测丢包或乱序包。
4. **协议头部冗余**：以太网帧偏移量 16-33（14 字节）的用途未知，可能是预留字段或未使用的历史字段。

---

## 5. JPEG 帧解析与拼接

### 5.1 解码线程主循环

**源码位置**：`decoder/decoder.cpp` → `Decoder::start()`

**流程**：
```cpp
while (running.load()) {
    // 1. 等待队列中有数据
    dataQueueCV_.wait(lock, [this]() { return !dataQueue_.empty(); });
    
    // 2. 取出数据包
    raw = dataQueue_.front();
    dataQueue_.pop();
    
    // 3. 解析协议头部
    frameStart = raw[15] & 0x1;         // 帧起始标志
    frameEnd = raw[36] & 0x10;         // 帧结束标志
    payloadLen = (raw[34] << 8) | raw[35];  // 负载长度
    
    // 4. 拼接 JPEG 数据
    BCM_JPG_WriteToImg(imgArr, frameStart, frameEnd, raw + 38, payloadLen);
    
    // 5. 如果是结束包，解码并显示
    if (frameEnd) {
        img.loadFromData(imgArr, "JPEG");
        emit sendImg(img);
        imgArr.resize(0);  // 清空缓冲区
    }
}
```

### 5.2 JPEG 帧拼接逻辑

**源码位置**：`decoder/bcm_jpeg.cpp` → `BCM_JPG_WriteToImg()`

#### 5.2.1 帧起始处理

当 `frameStart == 1` 时：

```cpp
if (frameStart) {
    // 从 JPEG 数据的前 12 字节提取图像参数
    uint8_t qp = data[5];                          // 质量因子 (1-100)
    uint16_t width = data[6] * 8U;                 // 图像宽度（像素）
    uint16_t height = data[7] * 8U;                // 图像高度（像素）
    uint16_t rstInt = ((uint32_t)data[8]<<8)|data[9];  // 重启间隔
    
    // 生成 JPEG 文件头（JFIF 格式）
    JPG_HeaderSize = 0;
    BCM_JPG_EncodeHeader(width, height, qp, rstInt);
    
    // 将 JPEG 文件头拷贝到输出缓冲区
    img.resize(JPG_HeaderSize);
    memcpy(img.data(), JPG_Header, JPG_HeaderSize);
    
    JPG_FrameSize = 0;  // 重置帧大小计数器
}
```

**JPEG 文件头结构**（由 `BCM_JPG_EncodeHeader()` 生成）：

```
生成的 JPEG 文件头（约 420 字节）
├── JFIF APP0 标记 (16 字节)
│   ├── 0xFF 0xE0 (APP0 标记)
│   ├── 长度 (2 字节)
│   ├── "JFIF\0" (5 字节)
│   ├── 版本 (2 字节)
│   ├── 密度单位 (1 字节)
│   ├── 密度 (4 字节)
│   └── 缩略图尺寸 (2 字节)
│
├── 量化表 DQT (130 字节)
│   ├── 0xFF 0xDB (DQT 标记)
│   ├── 长度 (2 字节)
│   ├── 亮度量化表 (64 字节)
│   └── 色度量化表 (64 字节)
│
├── 帧开始 SOF (15 字节)
│   ├── 0xFF 0xC0 (SOF0 标记)
│   ├── 长度 (2 字节)
│   ├── 精度 (1 字节)
│   ├── 高度 (2 字节)
│   ├── 宽度 (2 字节)
│   └── 分量信息 (3 × 3 = 9 字节)
│
├── 霍夫曼表 DHT (208 字节)
│   ├── 0xFF 0xC4 (DHT 标记)
│   ├── 长度 (2 字节)
│   ├── DC 亮度表
│   ├── AC 亮度表
│   ├── DC 色度表
│   └── AC 色度表
│
├── 重启间隔 DRI (4 字节，可选)
│   ├── 0xFF 0xDD (DRI 标记)
│   ├── 长度 (2 字节)
│   └── 重启间隔值 (2 字节)
│
└── 扫描开始 SOS (10 字节)
    ├── 0xFF 0xDA (SOS 标记)
    ├── 长度 (2 字节)
    ├── 分量数 (1 字节)
    ├── 分量信息 (2 × 3 = 6 字节)
    └── 光谱选择 (3 字节)
```

**关键点**：
- 发送端只传输 JPEG 的**熵编码数据**（没有文件头）
- 接收端根据嵌入在数据流中的参数（宽、高、QP）**重新生成** JPEG 文件头
- 这种设计的优点是减少带宽占用，缺点是接收端需要预先知道量化表和霍夫曼表

#### 5.2.2 JPEG 数据拼接

```cpp
if (size > 12) {
    // 跳过前 12 字节（嵌入式头部），拷贝 JPEG 数据
    originSize = img.size();
    img.resize(originSize + size - 12);
    memcpy(img.data() + originSize, data + 12, size - 12);
    JPG_FrameSize += size - 12;
}
```

**数据处理流程**：
1. `data` 指向 `raw + 38`（JPEG 数据起始位置）
2. 跳过前 12 字节（嵌入式头部，包含图像参数）
3. 将剩余的 `size - 12` 字节拷贝到输出缓冲区
4. 累加 `JPG_FrameSize`（用于调试或统计）

#### 5.2.3 帧结束处理

在 `decoder.cpp` 中处理：

```cpp
if (frameEnd) {
    if (!img.loadFromData(imgArr, "JPEG")) {
        // JPEG 解码失败
        emit consolePrint("Parsing image error!", PrintLevel::Warn);
    } else {
        // JPEG 解码成功，发送图像到 UI
        emit sendImg(img);
    }
    imgArr.resize(0);  // 清空缓冲区，准备下一帧
}
```

**JPEG 解码**：
- 使用 Qt 的 `QImage::loadFromData()` 解码 JPEG 数据
- 输入：完整的 JPEG 字节流（文件头 + 熵编码数据）
- 输出：`QImage` 对象（RGB 格式）

### 5.3 JPEG 质量因子与量化表

**源码位置**：`decoder/bcm_jpeg.cpp` → `BCM_JPG_EncodeHeader()`

**质量因子转换公式**：
```cpp
qf = (quality < 50U) ? (5000U / quality) : (200U - quality * 2U);
```

**量化表生成**：
```cpp
for (i = 0; i < 8*8; i++) {
    int32_t luma   = (JPG_DefaultQuantLuma  [JPG_ZigZagInv[i]] * qf + 50) / 100;
    int32_t chroma = (JPG_DefaultQuantChroma[JPG_ZigZagInv[i]] * qf + 50) / 100;
    
    quantLuma[i]   = JPG_LIMIT(luma,   1, 255);
    quantChroma[i] = JPG_LIMIT(chroma, 1, 255);
}
```

**默认量化表**（JPEG 标准 Annex K）：
- `JPG_DefaultQuantLuma` — 亮度量化表（8×8）
- `JPG_DefaultQuantChroma` — 色度量化表（8×8）

**ZigZag 扫描顺序**：
- `JPG_ZigZagInv` — 逆 ZigZag 扫描表，用于将量化表从之字形顺序还原为自然顺序

---

## 6. 视频播放流程

### 6.1 播放控制

**源码位置**：`mainwindow.cpp` → `MainWindow::openDev()` / `playVideo()` / `stopVideo()`

#### 6.1.1 开始播放

```cpp
void MainWindow::openDev() {
    if (!m_isPlaying) {
        playVideo();
        m_isPlaying = true;
        // 更新 UI：播放按钮 → 停止按钮
    } else {
        stopVideo();
        m_isPlaying = false;
        // 更新 UI：停止按钮 → 播放按钮
    }
}

int MainWindow::playVideo() {
    // 1. 启动捕获线程
    m_pCaptureModule->start();
    
    // 2. 启动定时器（用于统计 FPS）
    m_pTimer->start();
    
    // 3. 创建解码器（移动到独立线程）
    m_pDecoderModule = new Decoder(Capture::s_packetQueue, 
                                   Capture::s_packetQueueMutex, 
                                   Capture::s_packetQueueCV);
    m_pPlayThread = new QThread();
    m_pDecoderModule->moveToThread(m_pPlayThread);
    
    // 4. 连接信号槽
    connect(m_pDecoderModule, &Decoder::sendImg, this, &MainWindow::displayFrame);
    connect(m_pPlayThread, &QThread::started, m_pDecoderModule, &Decoder::start);
    
    // 5. 启动解码线程
    m_pPlayThread->start();
}
```

#### 6.1.2 停止播放

```cpp
void MainWindow::stopVideo() {
    // 1. 停止解码线程
    m_pDecoderModule->end();       // 设置 running = false
    m_pPlayThread->quit();         // 退出事件循环
    m_pPlayThread->wait();         // 等待线程结束
    
    // 2. 停止捕获线程
    m_pCaptureModule->end();       // 调用 pcap_breakloop()
    
    // 3. 停止定时器
    m_pTimer->stop();
}
```

### 6.2 图像显示

**源码位置**：`mainwindow.cpp` → `MainWindow::displayFrame()`

```cpp
void MainWindow::displayFrame(const QImage& img) {
    // 1. 将 QImage 转换为 QPixmap
    QPixmap pixmap = QPixmap::fromImage(img);
    
    // 2. 获取显示区域尺寸
    QSize labelSize = ui->canvas->size();
    
    // 3. 缩放图像（保持宽高比，平滑缩放）
    QPixmap scaleImg = pixmap.scaled(labelSize, 
                                     Qt::KeepAspectRatio, 
                                     Qt::SmoothTransformation);
    
    // 4. 显示在 QLabel 上
    ui->canvas->setPixmap(scaleImg);
    ui->canvas->show();
    
    // 5. 更新 FPS 计数
    m_fps++;
}
```

**显示优化**：
- `Qt::KeepAspectRatio` — 保持图像宽高比，避免拉伸变形
- `Qt::SmoothTransformation` — 使用双线性滤波缩放，图像更平滑

### 6.3 性能统计

**源码位置**：`mainwindow.cpp` → `MainWindow::onTimeout()`

```cpp
void MainWindow::onTimeout() {
    // 每秒更新一次统计信息
    consolePrint(QString("packet count: %1, fps: %2")
                .arg(Capture::s_packetCount.load())
                .arg(m_fps.load()));
    
    // 重置计数器
    Capture::s_packetCount.store(0);
    m_fps.store(0);
}
```

**统计指标**：
- `packet count` — 每秒捕获的数据包数量
- `fps` — 每秒解码并显示的图像帧数

---

## 7. 线程模型与同步机制

### 7.1 线程划分

```
┌─────────────────────────────────────────────────────────────┐
│                     主线程 (UI 线程)                        │
│  - Qt 事件循环                                              │
│  - 用户交互（按钮点击、下拉框选择）                          │
│  - 图像显示（displayFrame）                                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ 信号槽通信
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  捕获线程 (Capture 线程)                    │
│  - pcap_loop() 阻塞抓包                                     │
│  - packetHandle() 回调函数                                  │
│  - 将数据包推入队列                                         │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ 静态队列 + 互斥锁 + 条件变量
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                  解码线程 (Decoder 线程)                     │
│  - 从队列取出数据包                                         │
│  - 解析协议头部                                             │
│  - 拼接 JPEG 帧                                             │
│  - 解码 JPEG 图像                                            │
│  - 发送 sendImg 信号                                        │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 线程同步机制

#### 7.2.1 队列与条件变量

**生产者-消费者模型**：
- **生产者**：捕获线程（`packetHandle()` 回调）
- **消费者**：解码线程（`Decoder::start()` 循环）

**同步原语**（Capture 类静态成员）：
```cpp
static const int PACKET_ARRAY_SIZE = 100;
static const int PACKET_LENGTH = 2000;
static unsigned char s_packetArray[PACKET_ARRAY_SIZE][PACKET_LENGTH];  // 环形缓冲区
static std::queue<const unsigned char*> s_packetQueue;                // 数据包队列
static std::mutex s_packetQueueMutex;                                 // 互斥锁
static std::condition_variable s_packetQueueCV;                       // 条件变量
static std::atomic<int> s_packetCount;                                // 包计数器
```

**生产流程**（捕获线程）：
```cpp
void Capture::packetHandle(...) {
    int index = s_packetAarrayIndex % PACKET_ARRAY_SIZE;
    {
        std::lock_guard<std::mutex> lock(s_packetQueueMutex);
        memcpy(s_packetArray[index], packet, header->caplen);
        s_packetQueue.push(s_packetArray[index]);
    }
    s_packetAarrayIndex++;
    s_packetCount++;
    s_packetQueueCV.notify_one();  // 通知消费者
}
```

**消费流程**（解码线程）：
```cpp
void Decoder::start() {
    while (running.load()) {
        std::unique_lock<std::mutex> lock(dataQueueMutex_);
        dataQueueCV_.wait(lock, [this]() { return !dataQueue_.empty(); });
        
        raw = dataQueue_.front();
        dataQueue_.pop();
        lock.unlock();
        
        // 处理数据包...
    }
}
```

#### 7.2.2 线程停止机制

**捕获线程停止**：
```cpp
void Capture::end() {
    pcap_breakloop(captureHandler_);  // 打破 pcap_loop 循环
    if (captureThread_.joinable()) {
        captureThread_.join();        // 等待线程退出
    }
}
```

**解码线程停止**：
```cpp
void Decoder::end() {
    running.store(false);            // 设置停止标志
    dataQueueCV_.notify_one();        // 唤醒等待的线程（使其检查 running 标志）
}

// 在 Decoder::start() 中
while (running.load()) {
    // 处理数据包...
}
```

### 7.3 线程安全分析

**潜在问题**：
1. **静态成员共享**：`s_packetArray`、`s_packetQueue` 等是 Capture 类的静态成员，被多个线程共享。虽然使用了互斥锁保护队列操作，但 `s_packetAarrayIndex` 的递增操作**不是原子操作**（尽管 `s_packetCount` 是原子的）。
2. **环形缓冲区覆盖**：当 `s_packetAarrayIndex` 超过 `PACKET_ARRAY_SIZE` 时，旧包会被覆盖。如果解码线程处理速度慢于捕获线程，会导致丢包。
3. **条件变量通知**：`notify_one()` 只唤醒一个等待线程，此处只有一个解码线程，所以没有问题。

---

## 8. 关键数据结构

### 8.1 Capture 类

**头文件**：`capture/include/capture.h`

```cpp
class Capture : public QObject {
    Q_OBJECT
    
signals:
    void consolePrint(const QString& str, PrintLevel level);
    
public:
    static const int PACKET_ARRAY_SIZE = 100;   // 环形缓冲区大小
    static const int PACKET_LENGTH = 2000;      // 每个包的最大长度
    
    static int s_packetAarrayIndex;             // 环形缓冲区索引
    static unsigned char s_packetArray[PACKET_ARRAY_SIZE][PACKET_LENGTH];
    static std::queue<const unsigned char*> s_packetQueue;
    static std::mutex s_packetQueueMutex;
    static std::condition_variable s_packetQueueCV;
    static std::atomic<int> s_packetCount;      // 包计数器（原子操作）
    
    Capture(QObject *parent = nullptr);
    ~Capture();
    
    bool isReady();                             // 检查捕获句柄是否有效
    void start();                               // 启动捕获线程
    void end();                                 // 停止捕获线程
    std::vector<std::string> getNetworkInterface();  // 枚举网卡
    void setCaptureHandler(const int index);     // 打开指定网卡
    
private:
    std::vector<std::string> networkInterfaceList_;  // 网卡名称列表
    pcap_t* captureHandler_;                   // pcap 句柄
    std::thread captureThread_;                 // 捕获线程
    
    static void packetHandle(unsigned char* userData, 
                           const struct pcap_pkthdr* header, 
                           const unsigned char* packet);  // 静态回调函数
};
```

### 8.2 Decoder 类

**头文件**：`decoder/include/decoder.h`

```cpp
class Decoder : public QObject {
    Q_OBJECT
    
signals:
    void consolePrint(const QString& str, PrintLevel level);
    void sendImg(const QImage& img);            // 发送解码后的图像
    void finished();                             // 线程结束信号
    
public slots:
    void start();                               // 启动解码循环
    void end();                                 // 停止解码循环
    
private:
    std::queue<const unsigned char*>& dataQueue_;    // 数据包队列（引用）
    std::mutex& dataQueueMutex_;                // 队列互斥锁（引用）
    std::condition_variable& dataQueueCV_;      // 条件变量（引用）
    std::atomic<bool> running;                  // 运行标志
};
```

### 8.3 以太网帧头部结构（pcap 提供）

```cpp
struct pcap_pkthdr {
    struct timeval ts;      // 时间戳（秒 + 微秒）
    bpf_u_int32 caplen;     // 捕获的数据包长度
    bpf_u_int32 len;        // 实际的数据包长度（可能大于 caplen）
};
```

**注意**：
- `caplen` — pcap 实际捕获的字节数（受 snaplen 限制）
- `len` — 数据包的实际长度（如果数据包被截断，`len > caplen`）

---

## 9. 与 Rust pcap 模块的对应关系

### 9.1 功能映射

| C++ StreamPlayer | Rust pcap 模块 | 说明 |
|---|---|---|
| `pcap_findalldevs()` | `pcap::Device::list()` | 枚举网卡 |
| `pcap_open_live()` | `pcap::Capture::from_device().open()` | 打开网卡 |
| `pcap_loop()` | `capture.for_each()` 或 `capture.next_packet()` | 循环抓包 |
| `pcap_breakloop()` | `capture.break_loop()` | 打破循环 |
| `pcap_close()` | `Drop` trait 自动释放 | 关闭句柄 |
| `packetHandle()` 回调 | `mpsc::Receiver<Packet>` 或闭包回调 | 数据包处理 |

### 9.2 Rust 模块优势

1. **内存安全**：Rust 的所有权系统避免悬垂指针和缓冲区溢出
2. **类型安全**：`Device::list()` 返回类型化的 `Device` 结构体，而非原始字符串
3. **现代化 API**：使用 `mpsc` 通道传递数据包，而非原始的队列+互斥锁
4. **跨平台**：`pcap` crate 支持 Windows、Linux、macOS，无需修改代码

### 9.3 迁移建议

如果需要将 StreamPlayer 迁移到 Rust（如 Tauri 应用），建议：

1. **保留协议解析逻辑**：自定义协议格式（EtherType 0x0022）是应用层协议，与语言无关
2. **使用 Rust pcap 模块**：替换 C++ 的 pcap API 调用
3. **使用通道替代队列**：Rust 的 `mpsc` 通道更安全、更易用
4. **JPEG 解码**：使用 `image` crate 替代 Qt 的 `QImage::loadFromData()`

---

## 10. 总结

### 10.1 技术要点

1. **网卡数据抓取**：使用 pcap 库直接从网卡捕获原始数据包，绕过操作系统网络栈
2. **自定义协议**：基于以太网层（EtherType 0x0022），无 IP/TCP/UDP 开销
3. **JPEG 分片传输**：大 JPEG 帧被分割成多个包，通过帧起始/结束标志重组
4. **嵌入式参数**：图像参数（宽、高、质量）嵌入在 JPEG 数据流的前 12 字节
5. **动态 JPEG 文件头生成**：接收端根据嵌入参数重新生成 JPEG 文件头

### 10.2 改进建议

1. **协议规范化**：定义完整的协议规范文档，明确每个字段的含义
2. **错误检测**：添加 CRC 或校验和字段，检测数据传输错误
3. **序列号**：添加序列号字段，检测丢包和乱序包
4. **流量控制**：当解码线程处理速度慢于捕获线程时，需要丢包策略
5. **安全性**：当前协议无加密和认证，建议添加 TLS/DTLS 或 MAC 地址过滤

### 10.3 参考资料

- **pcap 官方文档**：https://www.tcpdump.org/manpages/pcap.3pcap.html
- **EtherType 列表**：https://www.iana.org/assignments/ieee-802-numbers/ieee-802-numbers.xhtml
- **JPEG 标准**：ITU-T T.81 | ISO/IEC 10918-1
- **Qt 信号槽机制**：https://doc.qt.io/qt-5/signalslots.html

---

**文档版本**：v1.0  
**编写时间**：2026-06-24  
**作者**：AI Assistant（基于 StreamPlayer 源代码分析）
