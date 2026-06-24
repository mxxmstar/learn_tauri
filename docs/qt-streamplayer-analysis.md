# QT_StreamPlayer 协议解析文档

## 目录
1. [概述](#1-概述)
2. [与 StreamPlayer 的关系](#2-与-streamplayer-的关系)
3. [协议格式详解](#3-协议格式详解)
4. [代码实现分析](#4-代码实现分析)
5. [与标准 AVTP 协议的对比](#5-与标准-avtp-协议的对比)
6. [总结](#6-总结)

---

## 1. 概述

### 1.1 项目背景

`QT_StreamPlayer` 是一个基于 Qt 框架开发的网络视频流播放器，与 `StreamPlayer` 项目高度相似。两者都通过 **pcap 库（WinPcap/Npcap）** 直接从网卡捕获原始网络数据包，解析自定义的以太网协议（EtherType = 0x0022），拼接 JPEG 图像帧，并最终渲染显示成视频。

**技术栈**：
- **C++ / Qt 5** — 主框架与 UI
- **libpcap / Npcap** — 网卡数据包捕获
- **自定义以太网协议（EtherType 0x0022）** — 视频流传输协议
- **JPEG 基线编码** — 图像压缩格式

### 1.2 项目结构

```
QT_StreamPlayer/
├── src/
│   ├── main.cpp              # 程序入口
│   ├── mainwindow/          # 主窗口模块
│   │   ├── mainwindow.cpp   # 主窗口逻辑
│   │   └── config_dialog.cpp # 配置对话框
│   ├── capture/             # 网卡捕获模块
│   │   ├── capture.cpp      # pcap 抓包实现
│   │   └── include/capture.h
│   ├── decoder/             # 解码模块
│   │   ├── decoder.cpp      # JPEG 帧解析与拼接
│   │   ├── bcm_jpeg.cpp    # JPEG 头部生成
│   │   └── include/
│   ├── bcmtool/            # Broadcom 设备工具
│   │   ├── bcmtool.cpp     # 设备配置工具
│   │   ├── bcm_common.cpp  # 通用功能
│   │   ├── bcm_dmon.cpp    # 设备监控
│   │   ├── bcm_flash.cpp   # Flash 操作
│   │   └── bcm_rpc.cpp    # RPC 通信
│   ├── tool/                # 工具模块
│   │   ├── dhcptool.cpp    # DHCP 工具
│   │   └── util.cpp        # 通用工具函数
│   └── include/
│       └── console.h        # 日志级别定义
├── third_party/
│   ├── pcap/               # pcap 库头文件
│   └── quazip/             # ZIP 压缩库
├── output/                  # 编译输出
├── CMakeLists.txt           # CMake 构建配置
├── config.ini              # 配置文件
└── README.md              # 项目说明
```

---

## 2. 与 StreamPlayer 的关系

### 2.1 代码对比

通过对比 `QT_StreamPlayer` 和 `StreamPlayer` 的源代码，发现两者**几乎完全相同**：

| 对比项 | StreamPlayer | QT_StreamPlayer | 是否相同 |
|---|---|---|---|
| **捕获模块** | `capture/capture.cpp` | `src/capture/capture.cpp` | ✅ 完全相同 |
| **解码模块** | `decoder/decoder.cpp` | `src/decoder/decoder.cpp` | ✅ 完全相同 |
| **JPEG 处理** | `decoder/bcm_jpeg.cpp` | `src/decoder/bcm_jpeg.cpp` | ✅ 完全相同 |
| **协议过滤** | `packet[12] == 0x22` | `packet[12] == 0x22` | ✅ 完全相同 |
| **协议头部解析** | `raw[15]`, `raw[36]`, `raw[34-35]` | `raw[15]`, `raw[36]`, `raw[34-35]` | ✅ 完全相同 |
| **嵌入式头部** | JPEG 数据前 12 字节 | JPEG 数据前 12 字节 | ✅ 完全相同 |
| **构建系统** | qmake (.pro) | CMake (CMakeLists.txt) | ❌ 不同 |
| **目录结构** | 扁平化 | 模块化（src/ 子目录） | ❌ 不同 |

### 2.2 细微差别

虽然核心逻辑完全相同，但存在一些细微差别：

#### 2.2.1 返回值差异

**StreamPlayer**（`capture/capture.cpp`）：
```cpp
std::vector<std::string> Capture::getNetworkInterface()
{
    // ...
    return vecDes;  // 返回描述列表
}
```

**QT_StreamPlayer**（`src/capture/capture.cpp`）：
```cpp
void Capture::getNetworkInterface()
{
    // ...
    // 不返回，直接存储在 networkInterfaceDesList_ 成员变量中
}
```

#### 2.2.2 C++ 标准差异

**StreamPlayer**（`capture/include/capture.h`）：
```cpp
static const int PACKET_ARRAY_SIZE = 100;   // C++11 之前的方式
static const int PACKET_LENGTH = 2000;
```

**QT_StreamPlayer**（`src/capture/include/capture.h`）：
```cpp
static constexpr int PACKET_ARRAY_SIZE = 100;  // C++11 引入的 constexpr
static constexpr int PACKET_LENGTH = 2000;
```

#### 2.2.3 未使用变量

**QT_StreamPlayer**（`src/decoder/decoder.cpp`）：
```cpp
void Decoder::start()
{
    // ...
    uint32_t offset = 0;  // 未使用
    int i = 0;             // 未使用
    // ...
}
```

**StreamPlayer**（`decoder/decoder.cpp`）：
```cpp
void Decoder::start()
{
    // ...
    // 没有未使用的变量
    // ...
}
```

### 2.3 结论

**`QT_StreamPlayer` 和 `StreamPlayer` 使用的是完全相同的自定义协议格式**，不是标准 AVTP 协议。

可能的解释：
1. **同一项目的不同版本**：`QT_StreamPlayer` 可能是 `StreamPlayer` 的重构版本（从 qmake 迁移到 CMake）
2. **分支项目**：从同一个代码库分支出来，核心逻辑保持不变
3. **学习项目**：基于 `StreamPlayer` 的学习或实验项目

---

## 3. 协议格式详解

### 3.1 协议层次结构

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
│  │  │  [嵌入式头部 12 字节] [JPEG 熵编码数据]  │  │  │ │
│  │  └────────────────────────────────────────────┘  │  │ │
│  └──────────────────────────────────────────────────┘  │ │
└──────────────────────────────────────────────────────────┘
```

### 3.2 以太网帧格式

**总长度**：至少 14 字节 + 协议数据

| 偏移量（相对于帧起始） | 长度（字节） | 字段名 | 说明 |
|---|---|---|---|
| 0 | 6 | 目标 MAC | 目的主机 MAC 地址 |
| 6 | 6 | 源 MAC | 发送方 MAC 地址 |
| 12 | 2 | **EtherType** | **协议类型标识** |
| 14 | N | 协议数据 | 自定义协议数据 |

**EtherType 值**：
- `0x0022` — 自定义视频流协议（本应用使用）
- **注意**：代码中只检查 `packet[12] == 0x22`（低字节），未检查 `packet[13] == 0x00`（高字节）。正确的检查应该是：
  ```cpp
  uint16_t etherType = (packet[12] << 8) | packet[13];
  if (etherType == 0x0022) {
      // ...
  }
  ```

### 3.3 自定义协议头部格式

**重要说明**：以下偏移量均**相对于以太网帧起始（packet[0]）**，而非协议数据起始。

#### 3.3.1 协议头部字段详解

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

#### 3.3.2 帧标志位详解

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

### 3.4 JPEG 嵌入式头部格式

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

### 3.5 完整的协议数据包格式

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

---

## 4. 代码实现分析

### 4.1 数据包捕获（capture.cpp）

**源码位置**：`src/capture/capture.cpp`

#### 4.1.1 网卡枚举

```cpp
void Capture::getNetworkInterface()
{
    pcap_if_t *alldevs;
    pcap_if_t *device;
    char errbuf[PCAP_ERRBUF_SIZE];
    
    if (pcap_findalldevs(&alldevs, errbuf) == -1) {
        emit consolePrint(QString("Error pcap find device: %1").arg(errbuf), PrintLevel::Error);
        return;
    }
    
    networkInterfaceList_.clear();
    networkInterfaceDesList_.clear();
    
    for (device = alldevs; device != NULL; device = device->next) {
        networkInterfaceList_.push_back(device->name);
        networkInterfaceDesList_.push_back(device->description);
    }
    
    pcap_freealldevs(alldevs);
}
```

**关键 pcap API**：
- `pcap_findalldevs()` — 枚举所有网卡
- `pcap_freealldevs()` — 释放网卡列表

#### 4.1.2 打开网卡

```cpp
void Capture::setCaptureHandler(const int index)
{
    char errbuf[PCAP_ERRBUF_SIZE];
    
    if (captureHandler_ != nullptr) {
        pcap_close(captureHandler_);
    }
    
    captureHandler_ = pcap_open_live(
        networkInterfaceList_[index].c_str(),  // 网卡设备名
        65536,   // snaplen: 捕获的最大字节数
        1,       // promisc: 混杂模式
        1000,    // timeout: 超时时间（毫秒）
        errbuf   // 错误信息缓冲区
    );
}
```

**参数说明**：
- `snaplen = 65536` — 足够大以捕获完整以太网帧（最大 1518 字节）
- `promisc = 1` — 混杂模式，捕获网络上所有流量
- `timeout = 1000` — 读超时，避免频繁系统调用

#### 4.1.3 数据包过滤回调

```cpp
void Capture::packetHandle(unsigned char* userData, const struct pcap_pkthdr* header, const unsigned char* packet)
{
    // 只接受 EtherType = 0x0022 的数据包
    if ((packet[12] == 0x22)) {
        {
            int index = s_packetAarrayIndex % PACKET_ARRAY_SIZE;
            
            std::lock_guard<std::mutex> lock(s_packetQueueMutex);
            memcpy(s_packetArray[index], packet, header->caplen);
            s_packetQueue.push(s_packetArray[index]);
        }
        s_packetAarrayIndex++;
        s_packetCount++;
        s_packetQueueCV.notify_one();  // 通知解码线程
    }
}
```

**过滤逻辑**：
- 检查以太网帧的 EtherType 字段（偏移量 12）
- 只接受 `packet[12] == 0x22` 的数据包（**注意**：正确的检查应该是 `(packet[12] == 0x00 && packet[13] == 0x22) || (packet[12] == 0x22 && packet[13] == 0x00)`？实际上 EtherType 是 2 字节，正确的检查应该是 `((packet[12] << 8) | packet[13]) == 0x0022`）

**数据存储**：
- 使用环形缓冲区 `s_packetArray[100][2000]`
- 推入队列 `s_packetQueue`
- 通过条件变量 `s_packetQueueCV` 通知解码线程

### 4.2 JPEG 帧解析与拼接（decoder.cpp）

**源码位置**：`src/decoder/decoder.cpp`

#### 4.2.1 解码线程主循环

```cpp
void Decoder::start()
{
    QByteArray imgArr;
    QImage img;
    const uint8_t* raw = nullptr;
    uint8_t frameStart = 0;
    uint8_t frameEnd = 0;
    uint16_t payloadLen = 0;
    
    running.store(true);
    
    while (running.load()) {
        // 1. 等待队列中有数据
        std::unique_lock<std::mutex> lock(dataQueueMutex_);
        dataQueueCV_.wait(lock, [this]() { return !dataQueue_.empty(); });
        
        // 2. 取出数据包
        raw = dataQueue_.front();
        dataQueue_.pop();
        lock.unlock();
        
        // 3. 解析协议头部
        frameStart = raw[15] & 0x1;         // 帧起始标志
        frameEnd = raw[36] & 0x10;         // 帧结束标志
        payloadLen = (raw[34] << 8) | raw[35];  // 负载长度
        
        // 4. 拼接 JPEG 数据
        BCM_JPG_WriteToImg(imgArr, frameStart, frameEnd, raw + 38, payloadLen);
        
        // 5. 如果是结束包，解码并显示
        if (frameEnd) {
            if (!img.loadFromData(imgArr, "JPEG")) {
                emit consolePrint(QString("Parsing image error!"), PrintLevel::Warn);
            } else {
                emit sendImg(img);
            }
            imgArr.resize(0);  // 清空缓冲区
        }
    }
}
```

#### 4.2.2 协议头部解析

**关键偏移量**（相对于以太网帧起始）：
```cpp
frameStart = raw[15] & 0x1;   // 以太网帧偏移量 15
frameEnd = raw[36] & 0x10;   // 以太网帧偏移量 36
payloadLen = (raw[34] << 8) | raw[35];  // 以太网帧偏移量 34-35
```

**数据处理**：
```cpp
BCM_JPG_WriteToImg(imgArr, frameStart, frameEnd, raw + 38, payloadLen);
//                                                    ^^^^^^^^
//                                                    以太网帧偏移量 38（JPEG 数据起始位置）
```

### 4.3 JPEG 头部生成（bcm_jpeg.cpp）

**源码位置**：`src/decoder/bcm_jpeg.cpp`

#### 4.3.1 嵌入式头部解析

```cpp
void BCM_JPG_WriteToImg(QByteArray& img, uint8_t frameStart, uint8_t frameEnd, uint8_t const *data, uint32_t size)
{
    int ret;
    int originSize;
    
    if (frameStart) {
        // 从 JPEG 数据的前 12 字节提取图像参数
        uint8_t qp = data[5];                          // 质量因子 (偏移量 5)
        uint16_t width = data[6] * 8U;                 // 图像宽度 (偏移量 6)
        uint16_t height = data[7] * 8U;                // 图像高度 (偏移量 7)
        uint16_t rstInt = ((uint32_t)data[8]<<8)|data[9];  // 重启间隔 (偏移量 8-9)
        uint16_t rstCount = (((uint32_t)data[10]<<8)|data[11])&0x3FFU;  // 帧计数 (偏移量 10-11)
        
        (void)rstCount;  // 未使用
        
        // 生成 JPEG 文件头
        JPG_HeaderSize = 0;
        BCM_JPG_EncodeHeader(width, height, qp, rstInt);
        
        // 将 JPEG 文件头拷贝到输出缓冲区
        img.resize(JPG_HeaderSize);
        memcpy(img.data(), JPG_Header, JPG_HeaderSize);
        
        JPG_FrameSize = 0;
    }
    
    // 跳过前 12 字节（嵌入式头部），拷贝 JPEG 数据
    if(size > 12) {
        originSize = img.size();
        img.resize(originSize + size - 12);
        memcpy(img.data() + originSize, data + 12, size - 12);
        JPG_FrameSize += size - 12;
    }
}
```

#### 4.3.2 JPEG 文件头生成

**函数**：`BCM_JPG_EncodeHeader()`

**生成的 JPEG 文件头结构**（约 420 字节）：
```
生成的 JPEG 文件头
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
├── 霍夫曼表 DHT (208+ 字节)
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

---

## 5. 与标准 AVTP 协议的对比

### 5.1 标准 AVTP 协议（IEEE 1722）

**EtherType**：`0x22F0`

**AVTP Common Stream Header**（24 字节）：
```
偏移量    长度    字段名
0         8       subtype (子类型，如 0x00 = AAF Audio, 0x07 = MJPEG)
8         8       version (3 bits) | sequence_num (4 bits) | ...
16        64      stream_id (流 ID，8 字节)
80        32      timestamp (时间戳，4 字节)
112       16      gateway_info
128       16      packet_count
144       80      ... (保留字段)
```

### 5.2 对比分析

| 对比项 | 标准 AVTP | QT_StreamPlayer |
|---|---|---|
| **EtherType** | `0x22F0` | `0x0022` |
| **协议头部长度** | 24 字节（Common Stream Header） | 24 字节（自定义格式） |
| **流 ID 字段** | 有（8 字节） | 未发现 |
| **时间戳字段** | 有（4 字节） | 未发现 |
| **序列号字段** | 有（4 bits） | 未发现 |
| **负载长度字段** | 无（从头部推导） | 有（偏移 34-35） |
| **帧标志位** | 无 | 有（起始/结束标志） |
| **嵌入式参数** | 无 | 有（JPEG 数据前 12 字节） |

### 5.3 结论

**QT_StreamPlayer 使用的不是标准 AVTP 协议，而是自定义的简化版本。**

#### 5.3.1 可能的原因

1. **厂商定制**：Broadcom（BCM 前缀）可能定义了私有的 AVTP 简化版本
2. **历史原因**：早期版本使用标准 AVTP，后来改为简化版本，但 UI 配置项保留
3. **混合设计**：控制平面使用 AVTP 概念（如 Stream ID），数据平面使用简化协议

#### 5.3.2 如何验证

如果需要确认，可以：

1. **抓包分析**：使用 Wireshark 捕获实际数据包，查看 EtherType 字段
2. **查看设备配置**：通过 `getDeviceConfig()` 获取设备的 AVTP 配置，看是否生效
3. **对比标准 AVTP**：在 Wireshark 中启用 AVTP 解析器，看是否能正确解析

---

## 6. 总结

### 6.1 技术要点

1. **网卡数据抓取**：使用 pcap 库直接从网卡捕获原始数据包，绕过操作系统网络栈
2. **自定义协议**：基于以太网层（EtherType 0x0022），无 IP/TCP/UDP 开销
3. **JPEG 分片传输**：大 JPEG 帧被分割成多个包，通过帧起始/结束标志重组
4. **嵌入式参数**：图像参数（宽、高、质量）嵌入在 JPEG 数据流的前 12 字节
5. **动态 JPEG 文件头生成**：接收端根据嵌入参数重新生成 JPEG 文件头

### 6.2 与 StreamPlayer 的关系

**`QT_StreamPlayer` 和 `StreamPlayer` 使用的是完全相同的自定义协议格式**。

可能的解释：
1. **同一项目的不同版本**：`QT_StreamPlayer` 可能是 `StreamPlayer` 的重构版本（从 qmake 迁移到 CMake）
2. **分支项目**：从同一个代码库分支出来，核心逻辑保持不变
3. **学习项目**：基于 `StreamPlayer` 的学习或实验项目

### 6.3 改进建议

1. **协议规范化**：定义完整的协议规范文档，明确每个字段的含义
2. **错误检测**：添加 CRC 或校验和字段，检测数据传输错误
3. **序列号**：添加序列号字段，检测丢包和乱序包
4. **流量控制**：当解码线程处理速度慢于捕获线程时，需要丢包策略
5. **安全性**：当前协议无加密和认证，建议添加 TLS/DTLS 或 MAC 地址过滤
6. **EtherType 检查修复**：修正 `packet[12] == 0x22` 为 `((packet[12] << 8) | packet[13]) == 0x0022`

### 6.4 参考资料

- **pcap 官方文档**：https://www.tcpdump.org/manpages/pcap.3pcap.html
- **EtherType 列表**：https://www.iana.org/assignments/ieee-802-numbers/ieee-802-numbers.xhtml
- **JPEG 标准**：ITU-T T.81 | ISO/IEC 10918-1
- **AVTP 标准**：IEEE 1722-2016
- **Qt 信号槽机制**：https://doc.qt.io/qt-5/signalslots.html

---

**文档版本**：v1.0  
**编写时间**：2026-06-24  
**作者**：AI Assistant（基于 QT_StreamPlayer 源代码分析）
