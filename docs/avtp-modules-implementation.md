# AVTP 解析模块实现计划

## 目标
实现两个 AVTP 解析模块：
1. `stonkam_avtp` - Stonkam 自定义协议（EtherType 0x0022）
2. `avtp` - 标准 AVTP 协议（IEEE 1722，EtherType 0x22F0）

## 模块结构

### stonkam_avtp 模块
```
src-tauri/src/stonkam_avtp/
├── mod.rs          # 模块声明 + 详细文档注释
├── error.rs        # StonkamAvtpError 错误类型
├── header.rs       # StonkamAvtpHeader 自定义协议头
└── parser.rs       # 解析逻辑 + 与 pcap 集成示例
```

### avtp 模块（标准 IEEE 1722）
```
src-tauri/src/avtp/
├── mod.rs          # 模块声明 + 详细文档注释
├── error.rs        # AvtpError 错误类型
├── header.rs       # AvtpHeader 标准 AVTP 头
├── packet.rs       # AvtpPacket 数据包
└── parser.rs       # 解析逻辑 + 与 pcap 集成示例
```

## 协议格式

### Stonkam 自定义协议（EtherType 0x0022）
- 以太网头：14 字节
- 自定义协议头：24 字节（偏移 14-37）
  - 偏移 15：帧起始标志（bit 0）
  - 偏移 34-35：负载长度（大端）
  - 偏移 36：帧结束标志（bit 4）
- JPEG 数据：从偏移 38 开始
  - 嵌入式头部：12 字节
    - 偏移 5：质量因子 QP
    - 偏移 6：图像宽度（×8）
    - 偏移 7：图像高度（×8）
    - 偏移 8-9：重启间隔
  - JPEG 熵编码数据：从嵌入式头部偏移 12 开始

### 标准 AVTP（IEEE 1722，EtherType 0x22F0）
- 以太网头：14 字节
- AVTP Common Stream Header：24 字节
  - 偏移 0：subtype（子类型）
  - 偏移 1：version + sequence_num
  - 偏移 2-9：stream_id（8 字节）
  - 偏移 10-13：timestamp（4 字节）
  - 偏移 14-15：gateway_info
  - 偏移 16-17：packet_count
- AVTP 视频流头部（MJPEG 格式，subtype = 0x07）

## 实现步骤

### 步骤 1：创建 stonkam_avtp 模块
- [ ] 创建 `error.rs` - 定义错误类型
- [ ] 创建 `header.rs` - 定义 StonkamAvtpHeader
- [ ] 创建 `parser.rs` - 解析逻辑
- [ ] 创建 `mod.rs` - 模块声明

### 步骤 2：创建 avtp 模块
- [ ] 创建 `error.rs` - 定义错误类型
- [ ] 创建 `header.rs` - 定义 AvtpHeader
- [ ] 创建 `packet.rs` - 定义 AvtpPacket
- [ ] 创建 `parser.rs` - 解析逻辑
- [ ] 创建 `mod.rs` - 模块声明

### 步骤 3：集成到 lib.rs
- [ ] 在 lib.rs 中注册模块
- [ ] 添加示例命令（可选）

### 步骤 4：创建文档
- [ ] 创建 `docs/stonkam-avtp-guide.md`
- [ ] 创建 `docs/avtp-guide.md`

### 步骤 5：Git 提交
- [ ] 提交代码
- [ ] 提交文档
