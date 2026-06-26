//! G.711 音频解码器
//!
//! 支持 G.711 A-law 和 μ-law 解码
//!
//! G.711 是一种简单的对数压缩算法，常用于电话系统
//! - A-law: 欧洲标准
//! - μ-law: 北美和日本标准

use bytes::Bytes;
use crate::rtp::decoder::trait_::{Decoder, DecodeResult, DecodeError};
use crate::rtp::decoder::types::{CodecType, SampleFormat};
use crate::rtp::decoder::frame::{MediaFrame, MediaPacket};

/// G.711 A-law 解码器
pub struct G711ADecoder {
    /// 采样率（默认 8000 Hz）
    sample_rate: i32,
    /// 声道数（默认 1）
    channels: i32,
}

impl G711ADecoder {
    /// 创建新的 G.711 A-law 解码器
    pub fn new() -> DecodeResult<Self> {
        Ok(Self {
            sample_rate: 8000,
            channels: 1,
        })
    }

    /// 设置采样率
    pub fn with_sample_rate(mut self, sample_rate: i32) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    /// 设置声道数
    pub fn with_channels(mut self, channels: i32) -> Self {
        self.channels = channels;
        self
    }
}

impl Decoder for G711ADecoder {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
        if packet.data.is_empty() {
            return Ok(None);
        }

        // G.711 A-law 解码：8-bit -> 16-bit PCM
        let input_data = packet.as_slice();
        let mut output_data = Vec::with_capacity(input_data.len() * 2);

        for &byte in input_data {
            // A-law 解码到 16-bit PCM
            let pcm = alaw_to_pcm(byte);
            output_data.extend_from_slice(&pcm.to_le_bytes());
        }

        let nb_samples = input_data.len() as i32;
        let data = Bytes::from(output_data);

        let frame = MediaFrame::new_audio(
            SampleFormat::S16, // 输出 S16 PCM
            self.sample_rate,
            self.channels,
            nb_samples,
            data,
        )
        .with_timestamps(packet.pts, packet.dts);

        Ok(Some(frame))
    }

    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>> {
        Ok(Vec::new())
    }

    fn reset(&mut self) {
        // G.711 是无状态解码器，无需重置
    }

    fn codec_type(&self) -> CodecType {
        CodecType::G711A
    }

    fn name(&self) -> &str {
        "G711ADecoder"
    }
}

/// G.711 μ-law 解码器
pub struct G711UDecoder {
    /// 采样率（默认 8000 Hz）
    sample_rate: i32,
    /// 声道数（默认 1）
    channels: i32,
}

impl G711UDecoder {
    /// 创建新的 G.711 μ-law 解码器
    pub fn new() -> DecodeResult<Self> {
        Ok(Self {
            sample_rate: 8000,
            channels: 1,
        })
    }

    /// 设置采样率
    pub fn with_sample_rate(mut self, sample_rate: i32) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    /// 设置声道数
    pub fn with_channels(mut self, channels: i32) -> Self {
        self.channels = channels;
        self
    }
}

impl Decoder for G711UDecoder {
    fn decode(&mut self, packet: &MediaPacket) -> DecodeResult<Option<MediaFrame>> {
        if packet.data.is_empty() {
            return Ok(None);
        }

        // G.711 μ-law 解码：8-bit -> 16-bit PCM
        let input_data = packet.as_slice();
        let mut output_data = Vec::with_capacity(input_data.len() * 2);

        for &byte in input_data {
            // μ-law 解码到 16-bit PCM
            let pcm = ulaw_to_pcm(byte);
            output_data.extend_from_slice(&pcm.to_le_bytes());
        }

        let nb_samples = input_data.len() as i32;
        let data = Bytes::from(output_data);

        let frame = MediaFrame::new_audio(
            SampleFormat::S16, // 输出 S16 PCM
            self.sample_rate,
            self.channels,
            nb_samples,
            data,
        )
        .with_timestamps(packet.pts, packet.dts);

        Ok(Some(frame))
    }

    fn flush(&mut self) -> DecodeResult<Vec<MediaFrame>> {
        Ok(Vec::new())
    }

    fn reset(&mut self) {
        // G.711 是无状态解码器，无需重置
    }

    fn codec_type(&self) -> CodecType {
        CodecType::G711U
    }

    fn name(&self) -> &str {
        "G711UDecoder"
    }
}

/// A-law 解码表（预计算）
const ALAW_TO_PCM: [i16; 256] = {
    let mut table = [0i16; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = alaw_decode(i as u8);
        i += 1;
    }
    table
};

/// μ-law 解码表（预计算）
const ULAW_TO_PCM: [i16; 256] = {
    let mut table = [0i16; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = ulaw_decode(i as u8);
        i += 1;
    }
    table
};

/// A-law 解码函数
///
/// 标准 G.711 A-law 解码：先 XOR 0x55 还原交替位翻转，再对数扩展
#[inline]
fn alaw_to_pcm(alaw: u8) -> i16 {
    // 使用查找表
    ALAW_TO_PCM[alaw as usize]
}

/// μ-law 解码函数
///
/// 标准 G.711 μ-law 解码：先按位取反，再对数扩展
#[inline]
fn ulaw_to_pcm(ulaw: u8) -> i16 {
    // 使用查找表
    ULAW_TO_PCM[ulaw as usize]
}

/// 编译时计算 A-law 解码值（标准 G.711 算法）
const fn alaw_decode(alaw: u8) -> i16 {
    // 步骤 1: XOR 0x55 还原编码时的交替位翻转
    let alaw = alaw ^ 0x55;

    // 步骤 2: 提取符号位、指数、尾数
    let sign = alaw & 0x80;
    let exponent = (alaw >> 4) & 0x07;
    let mantissa = alaw & 0x0F;

    // 步骤 3: 对数扩展重建 PCM 值
    let mut pcm: i16 = ((mantissa as i16) << 4) + 8;
    if exponent != 0 {
        pcm += 0x100; // 添加隐含的前导 1
        pcm <<= exponent - 1;
    }

    // 步骤 4: 应用符号位
    // sign != 0 表示正数，sign == 0 表示负数
    if sign != 0 { pcm } else { -pcm }
}

/// 编译时计算 μ-law 解码值（标准 G.711 算法）
const fn ulaw_decode(ulaw: u8) -> i16 {
    // 步骤 1: 按位取反还原编码时的取反
    let ulaw = !ulaw;

    // 步骤 2: 提取符号位、指数、尾数
    let sign = ulaw & 0x80;
    let exponent = (ulaw >> 4) & 0x07;
    let mantissa = ulaw & 0x0F;

    // 步骤 3: 对数扩展重建 PCM 值
    // 公式: ((mantissa << 3) + bias) << exponent - bias
    // 其中 bias = 0x84 = 132
    let mut pcm: i16 = ((mantissa as i16) << 3) + 0x84;
    pcm <<= exponent;
    pcm -= 0x84;

    // 步骤 4: 应用符号位
    // sign != 0 表示正数，sign == 0 表示负数
    if sign != 0 { pcm } else { -pcm }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_g711a_decoder() {
        let mut decoder = G711ADecoder::new().unwrap();

        // 创建一个简单的测试包（全零 A-law 数据）
        let test_data = Bytes::from(vec![0x00; 160]); // 160 字节 = 20ms @ 8kHz
        let packet = crate::rtp::decoder::frame::MediaPacket::new(CodecType::G711A, test_data)
            .with_timestamps(0, 0);

        let result = decoder.decode(&packet);
        assert!(result.is_ok());

        let frame_opt = result.unwrap();
        assert!(frame_opt.is_some());

        let frame = frame_opt.unwrap();
        assert!(frame.is_audio());
        assert_eq!(frame.sample_format, SampleFormat::S16);
        assert_eq!(frame.sample_rate, 8000);
        assert_eq!(frame.nb_samples, 160);
        assert_eq!(frame.data.len(), 320); // 160 * 2 bytes
    }

    #[test]
    fn test_g711u_decoder() {
        let mut decoder = G711UDecoder::new().unwrap();

        // 创建一个简单的测试包（全零 μ-law 数据）
        let test_data = Bytes::from(vec![0x00; 160]);
        let packet = crate::rtp::decoder::frame::MediaPacket::new(CodecType::G711U, test_data)
            .with_timestamps(0, 0);

        let result = decoder.decode(&packet);
        assert!(result.is_ok());

        let frame_opt = result.unwrap();
        assert!(frame_opt.is_some());

        let frame = frame_opt.unwrap();
        assert!(frame.is_audio());
        assert_eq!(frame.sample_format, SampleFormat::S16);
        assert_eq!(frame.nb_samples, 160);
    }

    #[test]
    fn test_alaw_decode() {
        // 测试 A-law 解码
        // 0xD5 是 A-law 编码的静音（正方向），解码为 8
        let pcm_pos = alaw_to_pcm(0xD5);
        // 0x55 是 A-law 编码的静音（负方向），解码为 -8
        let pcm_neg = alaw_to_pcm(0x55);
        assert_eq!(pcm_pos, 8);   // 正静音
        assert_eq!(pcm_neg, -8);  // 负静音
        assert_eq!(pcm_pos, -pcm_neg); // 互为相反数
    }

    #[test]
    fn test_ulaw_decode() {
        // 测试 μ-law 解码
        // 0xFF 和 0x7F 是 μ-law 编码的静音，都解码为 0
        let pcm1 = ulaw_to_pcm(0xFF);
        let pcm2 = ulaw_to_pcm(0x7F);
        assert_eq!(pcm1, pcm2); // 两者都解码为 0
        assert_eq!(pcm1, 0);    // 静音应该解码为 0
    }
}
