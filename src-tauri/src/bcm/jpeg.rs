//! BCM JPEG 编码器
//!
//! 将 Broadcom 设备传输的原始帧数据组装为标准的 JPEG 图像。
//! 包含 JPEG 文件头的构建（量化表、霍夫曼表、SOF、SOS 等）
//! 以及帧数据的拼接。
//!
//! 从 streamplayer/decoder/bcm_jpeg.cpp 移植，完全使用 Rust 重写。

// ====== JPEG 标记常量 ======
const JPG_MARKER_SOI: u8 = 0xD8;  ///< Start of Image
const JPG_MARKER_APP0: u8 = 0xE0; ///< JFIF APP0
const JPG_MARKER_DQT: u8 = 0xDB;  ///< Define Quantization Table
const JPG_MARKER_SOF: u8 = 0xC0;  ///< Start of Frame (baseline DCT)
const JPG_MARKER_DHT: u8 = 0xC4;  ///< Define Huffman Table
const JPG_MARKER_SOS: u8 = 0xDA;  ///< Start of Scan
const JPG_MARKER_DRI: u8 = 0xDD;  ///< Define Restart Interval

/// 将值限制在 [min, max] 范围内
fn jpg_limit(val: i32, min: i32, max: i32) -> i32 {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

/// JFIF 文件头（SOI + APP0 段）
const JPG_HEADER_JFIF: [u8; 20] = [
    0xFF, JPG_MARKER_SOI,       // SOI 标记
    0xFF, JPG_MARKER_APP0,      // APP0 标记
    0, 16,                       // 段长度
    b'J', b'F', b'I', b'F', 0,  // JFIF 标识
    1, 1,                        // 版本 1.1
    0,                           // 无密度单位
    0, 1, 0, 1,                  // 像素宽高比
    0, 0,                        // 无缩略图
];

/// Zig-zag 反序查找表（用于将 default 量化表映射到 zig-zag 顺序）
const JPG_ZIGZAG_INV: [u8; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
];

/// 默认亮度量化表
const JPG_DEFAULT_QUANT_LUMA: [u8; 64] = [
    16, 11, 10, 16,  24, 40,   51,  61,
    12, 12, 14, 19,  26, 58,   60,  55,
    14, 13, 16, 24,  40, 57,   69,  56,
    14, 17, 22, 29,  51, 87,   80,  62,
    18, 22, 37, 56,  68, 109, 103,  77,
    24, 35, 55, 64,  81, 104, 113,  92,
    49, 64, 78, 87, 103, 121, 120, 101,
    72, 92, 95, 98, 112, 100, 103,  99,
];

/// 默认色度量化表
const JPG_DEFAULT_QUANT_CHROMA: [u8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99,
    18, 21, 26, 66, 99, 99, 99, 99,
    24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
];

// ====== 霍夫曼表数据 ======

/// 亮度 DC 系数霍夫曼表 —— 每个位长的编码数量
const JPG_DC_LUMA_BITS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
/// 亮度 DC 系数霍夫曼表 —— 值列表
const JPG_DC_LUMA_VALS: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
/// 亮度 AC 系数霍夫曼表 —— 每个位长的编码数量
const JPG_AC_LUMA_BITS: [u8; 16] = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125];
/// 亮度 AC 系数霍夫曼表 —— 值列表
const JPG_AC_LUMA_VALS: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
    0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xA1, 0x08, 0x23, 0x42, 0xB1, 0xC1, 0x15, 0x52, 0xD1, 0xF0,
    0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0A, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2A, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
    0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7,
    0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3, 0xC4, 0xC5,
    0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xE1, 0xE2,
    0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8,
    0xF9, 0xFA,
];

/// 色度 DC 系数霍夫曼表 —— 每个位长的编码数量
const JPG_DC_CHROMA_BITS: [u8; 16] = [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0];
/// 色度 DC 系数霍夫曼表 —— 值列表
const JPG_DC_CHROMA_VALS: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
/// 色度 AC 系数霍夫曼表 —— 每个位长的编码数量
const JPG_AC_CHROMA_BITS: [u8; 16] = [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 119];
/// 色度 AC 系数霍夫曼表 —— 值列表
const JPG_AC_CHROMA_VALS: [u8; 162] = [
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
    0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xA1, 0xB1, 0xC1, 0x09, 0x23, 0x33, 0x52, 0xF0,
    0x15, 0x62, 0x72, 0xD1, 0x0A, 0x16, 0x24, 0x34, 0xE1, 0x25, 0xF1, 0x17, 0x18, 0x19, 0x1A, 0x26,
    0x27, 0x28, 0x29, 0x2A, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
    0x49, 0x4A, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
    0x69, 0x6A, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
    0x88, 0x89, 0x8A, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0xA2, 0xA3, 0xA4, 0xA5,
    0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xB9, 0xBA, 0xC2, 0xC3,
    0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA,
    0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8,
    0xF9, 0xFA,
];

/// 编码 JPEG 文件头
///
/// 构建完整的 JPEG 文件头，包含：
/// - JFIF APP0 标记
/// - 量化表（DQT）
/// - 帧起始标记（SOF0）
/// - 霍夫曼表（DHT）
/// - 重启间隔（DRI，可选）
/// - 扫描起始标记（SOS）
///
/// # 参数
/// * `img_width` - 图像宽度（像素）
/// * `img_height` - 图像高度（像素）
/// * `quality` - 图像质量（0-100）
/// * `restart_int` - 重启间隔（0 = 不启用）
///
/// # 返回值
/// * `Vec<u8>` - 完整的 JPEG 文件头数据
pub fn encode_header(img_width: u16, img_height: u16, quality: u8, restart_int: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(1024);

    // 根据质量因子计算缩放后的量化表
    // 公式来自 JPEG 规范
    let qf = if quality < 50 {
        5000u16 / quality as u16
    } else {
        200u16 - quality as u16 * 2
    };

    let mut quant_luma = [0u8; 64];
    let mut quant_chroma = [0u8; 64];
    for i in 0..64usize {
        let luma = (JPG_DEFAULT_QUANT_LUMA[JPG_ZIGZAG_INV[i] as usize] as i32 * qf as i32 + 50) / 100;
        let chroma = (JPG_DEFAULT_QUANT_CHROMA[JPG_ZIGZAG_INV[i] as usize] as i32 * qf as i32 + 50) / 100;
        quant_luma[i] = jpg_limit(luma, 1, 255) as u8;
        quant_chroma[i] = jpg_limit(chroma, 1, 255) as u8;
    }

    // JFIF 文件头
    out.extend_from_slice(&JPG_HEADER_JFIF);

    // 量化表 DQT：亮度 + 色度
    write_marker(&mut out, JPG_MARKER_DQT, 2 + 2 * (1 + 64));
    out.push(0x00); // 表 0（亮度）
    out.extend_from_slice(&quant_luma);
    out.push(0x01); // 表 1（色度）
    out.extend_from_slice(&quant_chroma);

    // 帧起始 SOF0
    write_marker(&mut out, JPG_MARKER_SOF, 2 + 6 + 3 * 3);
    out.push(0x08); // 8 位精度
    out.push((img_height >> 8) as u8);
    out.push((img_height & 0xFF) as u8);
    out.push((img_width >> 8) as u8);
    out.push((img_width & 0xFF) as u8);
    // 三通道：Y(4:2:2), Cb(4:4:4), Cr(4:4:4)
    out.extend_from_slice(&[
        0x03,                // 3 个颜色分量
        0x01, 0x22, 0x00,   // Y: 采样 2x2, 量化表 0
        0x02, 0x11, 0x01,   // Cb: 采样 1x1, 量化表 1
        0x03, 0x11, 0x01,   // Cr: 采样 1x1, 量化表 1
    ]);

    // 霍夫曼表 DHT：DC/AC + Luma/Chroma
    write_marker(&mut out, JPG_MARKER_DHT, 2 + 208 + 208);

    // DC Luma
    out.push(0x00);
    out.extend_from_slice(&JPG_DC_LUMA_BITS);
    out.extend_from_slice(&JPG_DC_LUMA_VALS);
    // AC Luma
    out.push(0x10);
    out.extend_from_slice(&JPG_AC_LUMA_BITS);
    out.extend_from_slice(&JPG_AC_LUMA_VALS);
    // DC Chroma
    out.push(0x01);
    out.extend_from_slice(&JPG_DC_CHROMA_BITS);
    out.extend_from_slice(&JPG_DC_CHROMA_VALS);
    // AC Chroma
    out.push(0x11);
    out.extend_from_slice(&JPG_AC_CHROMA_BITS);
    out.extend_from_slice(&JPG_AC_CHROMA_VALS);

    // 可选的重启间隔
    if restart_int != 0 {
        write_marker(&mut out, JPG_MARKER_DRI, 4);
        out.push(((restart_int >> 8) & 0xFF) as u8);
        out.push((restart_int & 0xFF) as u8);
    }

    // 扫描起始 SOS
    write_marker(&mut out, JPG_MARKER_SOS, 2 + 1 + 2 * 3 + 3);
    out.extend_from_slice(&[
        0x03,            // 3 个颜色分量
        0x01, 0x00,     // Y: DC表0, AC表0
        0x02, 0x11,     // Cb: DC表1, AC表1
        0x03, 0x11,     // Cr: DC表1, AC表1
        0x00, 0x3F, 0x00, // SS=0, SE=63, AH=0, AL=0
    ]);

    out
}

/// 写入 JPEG 标记（0xFF + 标记ID + 大端长度）
fn write_marker(out: &mut Vec<u8>, id: u8, length: u16) {
    out.push(0xFF);
    out.push(id);
    out.push((length >> 8) as u8);
    out.push((length & 0xFF) as u8);
}

/// 将原始帧数据写入 JPEG 图像缓冲区
///
/// 设备传输的帧数据格式：
/// - bytes[0-4]   : 帧头部信息
/// - byte[5]      : 质量参数 (qp)
/// - byte[6]      : 宽度（块数 = 像素/8）
/// - byte[7]      : 高度（块数 = 像素/8）
/// - bytes[8-9]   : 重启间隔
/// - bytes[10-11] : 重启计数
/// - bytes[12+]   : 实际 JPEG 编码数据
///
/// # 参数
/// * `img` - JPEG 图像缓冲区（输入输出）
/// * `frame_start` - 是否为帧起始（非 0 时会先编码文件头）
/// * `_frame_end` - 是否为帧结束
/// * `data` - 原始帧数据
/// * `size` - 数据长度
pub fn write_frame(
    img: &mut Vec<u8>,
    frame_start: u8,
    _frame_end: u8,
    data: &[u8],
    size: u32,
) {
    // 如果是帧起始，先根据帧头信息编码 JPEG 文件头
    if frame_start != 0 && size >= 12 {
        let qp = data[5];
        let width = data[6] as u16 * 8;
        let height = data[7] as u16 * 8;
        let rst_int = ((data[8] as u16) << 8) | data[9] as u16;

        let header = encode_header(width, height, qp, rst_int);
        img.clear();
        img.extend_from_slice(&header);
    }

    // 跳过 12 字节帧头，将编码数据附加到图像缓冲区
    if size > 12 {
        let frame_data = &data[12..size as usize];
        img.extend_from_slice(frame_data);
    }
}
