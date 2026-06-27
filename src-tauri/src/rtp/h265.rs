//! H.265 (HEVC) RTP 解包器 (RFC 7798)
//!
//! 支持以下 H.265 RTP 负载格式：
//! - 单个 NAL 单元模式
//! - 分片单元模式 (FUs)
//! - 聚合包模式 (AP)
//! - 前向纠错 (FEC) - 暂不实现

use crate::rtp::decoder::frame::MediaPacket;
use crate::rtp::decoder::types::{CodecType, MediaType};
use crate::rtp::packet::RtpPacket;
use bytes::{Bytes, BytesMut};
use std::collections::BTreeMap;

/// H.265 NAL 单元类型 (RFC 7798 Section 4.3.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum H265NalUnitType {
    /// 未指定 / 保留
    Unspecified(u8),
    /// 序列参数集 (SPS)
    Sps,
    /// 视频参数集 (VPS)
    Vps,
    /// 图像参数集 (PPS)
    Pps,
    /// 补充增强信息 (SEI)
    Sei,
    /// 补充增强信息 (SEI) 前缀
    SeiPrefix,
    /// 访问单元分隔符
    Aud,
    /// 序列结束
    EndOfSequence,
    /// 流结束
    EndOfStream,
    /// 填充数据
    FillerData,
    BlaWLp,
    BlaWRadl,
    BlaNLp,
    /// 视频切片 (IDR)
    IdrNlp,
    /// 视频切片 (IDR) 前缀
    IdrWRadl,
    /// 视频切片 (CRA)
    Cra,
    /// 分片单元 (FUs)
    Fu,
    /// 聚合包 (AP)
    Ap,
    /// 切片段 (Trail) - 非关键帧
    TrailN,
    TrailR,
}

impl H265NalUnitType {
    pub fn from_u8(t: u8) -> Self {
        match t {
            0 => H265NalUnitType::TrailN,
            1 => H265NalUnitType::TrailR,
            2 | 4 | 6 | 8 => H265NalUnitType::TrailN,
            3 | 5 | 7 | 9 => H265NalUnitType::TrailR,
            16 => H265NalUnitType::BlaWLp,
            17 => H265NalUnitType::BlaWRadl,
            18 => H265NalUnitType::BlaNLp,
            19 => H265NalUnitType::IdrWRadl,
            20 => H265NalUnitType::IdrNlp,
            21 => H265NalUnitType::Cra,
            32 => H265NalUnitType::Vps,
            33 => H265NalUnitType::Sps,
            34 => H265NalUnitType::Pps,
            35 => H265NalUnitType::Aud,
            36 => H265NalUnitType::EndOfSequence,
            37 => H265NalUnitType::EndOfStream,
            38 => H265NalUnitType::FillerData,
            39 => H265NalUnitType::SeiPrefix,
            40 => H265NalUnitType::Sei,
            48 => H265NalUnitType::Ap,
            49 => H265NalUnitType::Fu,
            _ => H265NalUnitType::Unspecified(t),
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            H265NalUnitType::TrailN => 0,
            H265NalUnitType::TrailR => 1,
            H265NalUnitType::BlaWLp => 16,
            H265NalUnitType::BlaWRadl => 17,
            H265NalUnitType::BlaNLp => 18,
            H265NalUnitType::IdrWRadl => 19,
            H265NalUnitType::IdrNlp => 20,
            H265NalUnitType::Cra => 21,
            H265NalUnitType::Vps => 32,
            H265NalUnitType::Sps => 33,
            H265NalUnitType::Pps => 34,
            H265NalUnitType::Aud => 35,
            H265NalUnitType::EndOfSequence => 36,
            H265NalUnitType::EndOfStream => 37,
            H265NalUnitType::FillerData => 38,
            H265NalUnitType::SeiPrefix => 39,
            H265NalUnitType::Sei => 40,
            H265NalUnitType::Ap => 48,
            H265NalUnitType::Fu => 49,
            H265NalUnitType::Unspecified(t) => *t,
        }
    }

    /// 判断是否关键帧 NAL 类型
    pub fn is_keyframe(&self) -> bool {
        matches!(
            self,
            H265NalUnitType::BlaWLp
                | H265NalUnitType::BlaWRadl
                | H265NalUnitType::BlaNLp
                | H265NalUnitType::IdrWRadl
                | H265NalUnitType::IdrNlp
                | H265NalUnitType::Cra
        )
    }

    /// 判断是否是参数集类型
    pub fn is_parameter_set(&self) -> bool {
        matches!(
            self,
            H265NalUnitType::Sps | H265NalUnitType::Pps | H265NalUnitType::Vps
        )
    }
}

/// 解析 H.265 NAL 单元头 (2 字节)
///
/// 格式: [Type(6bit) | LayerId(6bit) | TID(3bit)]
/// 实际字节布局:
///   Byte 0: [F(1bit)=0 | Type(6bit)]
///   Byte 1: [LayerId(1bit of 6) | TID(3bit)]
/// 完整 16-bit: [F(1) | Type(6) | LayerId(6) | TID(3)] - 实际上有 2 个保留位
///
/// 正确的 2 字节 NAL 头格式 (RFC 7798):
///   bits 0-1:   ForbiddenZeroBit (1 bit) + NalUnitType (6 bits) -> 实际上第一个字节是 [0, Type(6bit)]
///   bits 2-7:   NalUnitType (remaining 1 bit) + LayerId (6 bits) -> 第二个字节
///   bits 8-15:  TID (3 bits) + reserved (5 bits) -> 实际上不同
///
/// 我重新检查 RFC 7798:
/// 2-byte NAL header:
///   +---------------+---------------+
///   |F|   Type    |  LayerId  | TID |
///   +---------------+---------------+
///   F: 1 bit
///   Type: 6 bits (bits 1-6 of first byte)
///   LayerId: 6 bits (bits 7-8 of first byte + bits 0-3 of second byte)
///   TID: 3 bits (bits 4-7 of second byte)
pub fn parse_h265_nal_header(bytes: &[u8]) -> Option<(u8, H265NalUnitType, u16, u8)> {
    if bytes.len() < 2 {
        return None;
    }

    let byte0 = bytes[0];
    let byte1 = bytes[1];

    let forbidden = (byte0 & 0x80) >> 7;
    let nal_type = ((byte0 & 0x7E) >> 1) as u8;
    let layer_id = (((byte0 & 0x01) << 5) | ((byte1 & 0xF8) >> 3)) as u16;
    let tid = (byte1 & 0x07) as u8;

    let nal_unit_type = H265NalUnitType::from_u8(nal_type);

    Some((forbidden, nal_unit_type, layer_id, tid))
}

/// 聚合包 (AP) 头 (RFC 7798 Section 4.3.2)
///
/// AP 头的格式:
///   NAL header (2 bytes) + AP type = 48
///   然后每个 NAL 单元前有 16-bit 长度字段
pub struct ApHeader {
    /// NAL 头 (2 字节)
    pub nal_header: [u8; 2],
}

/// 分片单元 (FU) 头 (RFC 7798 Section 4.4.3)
///
/// FU 头的格式:
///   NAL header (2 bytes) + FU type = 49
///   FU header (1 byte): [S(1) | E(1) | 保留(6)]
#[derive(Debug, Clone)]
pub struct FuHeader {
    /// 起始位
    pub start: bool,
    /// 结束位
    pub end: bool,
    /// 保留位
    pub reserved: u8,
    /// NAL 单元类型 (原始的，在被分片之前)
    pub nal_type: u8,
}

impl FuHeader {
    pub fn from_byte(byte: u8) -> Self {
        Self {
            start: (byte & 0x80) != 0,
            end: (byte & 0x40) != 0,
            reserved: 0,
            nal_type: byte & 0x3F,
        }
    }

    pub fn to_byte(&self) -> u8 {
        let mut b = 0u8;
        if self.start {
            b |= 0x80;
        }
        if self.end {
            b |= 0x40;
        }
        b |= self.nal_type & 0x3F;
        b
    }
}

/// 一个完整的 H.265 访问单元 (AU)
#[derive(Debug, Clone)]
pub struct H265AccessUnit {
    /// RTP 时间戳
    pub timestamp: u32,
    /// 重组后的数据 (包含起始码 0x00000001 + NAL 单元)
    pub data: Bytes,
    /// 是否包含关键帧 (IDR / CRA)
    pub is_keyframe: bool,
}

/// H.265 RTP 重组器
pub struct H265Reassembler {
    /// 当前正在收集的 AU: timestamp -> AccessUnitBuilder
    current_aus: BTreeMap<u32, H265AccessUnitBuilder>,
    /// 已完成的 AU
    completed_aus: Vec<H265AccessUnit>,
    /// 最大缓存 AU 数
    max_aus: usize,
    /// 缓存的参数集 (VPS/SPS/PPS)
    vps: Option<Bytes>,
    sps: Option<Bytes>,
    pps: Option<Bytes>,
}

struct H265AccessUnitBuilder {
    /// 收集的所有 NAL 单元 (带起始码)
    nals: Vec<Bytes>,
    /// 是否包含关键帧
    is_keyframe: bool,
    /// FU 分片数据缓存
    fu_started: bool,
    /// 当前 FU 的 NAL 头 (用于重建)
    fu_nal_header: Option<[u8; 2]>,
    /// FU 分片数据
    fu_data: BytesMut,
}

impl H265Reassembler {
    pub fn new() -> Self {
        Self {
            current_aus: BTreeMap::new(),
            completed_aus: Vec::new(),
            max_aus: 50,
            vps: None,
            sps: None,
            pps: None,
        }
    }

    /// 处理一个 RTP 包，返回完成的 H.265 访问单元
    pub fn push_packet(&mut self, packet: &RtpPacket) -> Option<H265AccessUnit> {
        if packet.payload.is_empty() {
            return None;
        }

        let timestamp = packet.header.timestamp;

        // 解析 NAL 头，判断类型
        let (_, nal_type, _, _) = match parse_h265_nal_header(&packet.payload) {
            Some(info) => info,
            None => return None,
        };

        match nal_type {
            H265NalUnitType::Fu => {
                self.handle_fu(timestamp, &packet.payload)?;
            }
            H265NalUnitType::Ap => {
                self.handle_ap(timestamp, &packet.payload)?;
            }
            _ => {
                // 单个 NAL 单元模式
                self.handle_single_nal(timestamp, &packet.payload)?;
            }
        }

        // 检查是否收到 marker 位
        if packet.header.marker {
            if let Some(au) = self.finalize_au(timestamp) {
                return Some(au);
            }
        }

        None
    }

    /// 处理单个 NAL 单元
    fn handle_single_nal(&mut self, timestamp: u32, payload: &[u8]) -> Option<()> {
        if payload.len() < 2 {
            return None;
        }

        let (_, nal_type, _, _) = parse_h265_nal_header(payload)?;

        // 缓存参数集
        match nal_type {
            H265NalUnitType::Vps => {
                self.vps = Some(Bytes::copy_from_slice(payload));
            }
            H265NalUnitType::Sps => {
                self.sps = Some(Bytes::copy_from_slice(payload));
            }
            H265NalUnitType::Pps => {
                self.pps = Some(Bytes::copy_from_slice(payload));
            }
            _ => {}
        }

        let entry = self
            .current_aus
            .entry(timestamp)
            .or_insert_with(|| H265AccessUnitBuilder::new());

        // 添加起始码 + NAL 单元
        let mut nal_with_startcode = BytesMut::new();
        nal_with_startcode.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        nal_with_startcode.extend_from_slice(payload);

        entry.nals.push(nal_with_startcode.freeze());

        if nal_type.is_keyframe() {
            entry.is_keyframe = true;
        }

        Some(())
    }

    /// 处理分片单元 (FU)
    fn handle_fu(&mut self, timestamp: u32, payload: &[u8]) -> Option<()> {
        // FU 格式:
        //   [2-byte NAL header] (Type=49=FU)
        //   [1-byte FU header]
        //   [FU payload]
        if payload.len() < 3 {
            return None;
        }

        let fu_header_byte = payload[2];
        let fu_header = FuHeader::from_byte(fu_header_byte);

        let entry = self
            .current_aus
            .entry(timestamp)
            .or_insert_with(|| H265AccessUnitBuilder::new());

        if fu_header.start {
            // 第一个分片：重建原始 NAL 头
            // 原始 NAL 头 = (FU NAL header 的 F 位) | (FU header 中的 NAL type) | (LayerId) | (TID)
            let mut original_nal_header = [0u8; 2];
            // 从 FU 的 NAL header (payload[0..2]) 获取 F, LayerId, TID
            // 从 FU header 获取原始 NAL type
            // 有些实现使用不同的方式重建，这里采用标准方式:
            // 新 NAL 头的 Type 字段使用 FU header 中的 NAL type
            original_nal_header[0] = (payload[0] & 0x81) | ((fu_header.nal_type << 1) & 0x7E);
            original_nal_header[1] = payload[1];

            entry.fu_nal_header = Some(original_nal_header);
            entry.fu_started = true;
            entry.fu_data.clear();

            // 添加起始码
            entry.fu_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            entry.fu_data.extend_from_slice(&original_nal_header);
            entry.fu_data.extend_from_slice(&payload[3..]);

            if H265NalUnitType::from_u8(fu_header.nal_type).is_keyframe() {
                entry.is_keyframe = true;
            }
        } else if entry.fu_started {
            // 中间或结束分片
            entry.fu_data.extend_from_slice(&payload[3..]);
        } else {
            return None;
        }

        if fu_header.end && entry.fu_started {
            // 分片结束
            let complete_nal = std::mem::take(&mut entry.fu_data).freeze();
            entry.nals.push(complete_nal);
            entry.fu_started = false;
            entry.fu_nal_header = None;
        }

        Some(())
    }

    /// 处理聚合包 (AP)
    fn handle_ap(&mut self, timestamp: u32, payload: &[u8]) -> Option<()> {
        // AP 格式:
        //   [2-byte NAL header] (Type=48=AP)
        //   [16-bit length] for NAL 1
        //   [NAL 1 data]
        //   [16-bit length] for NAL 2
        //   [NAL 2 data]
        //   ...
        if payload.len() < 4 {
            return None;
        }

        let entry = self
            .current_aus
            .entry(timestamp)
            .or_insert_with(|| H265AccessUnitBuilder::new());

        // 跳过 AP NAL 头 (2 字节)
        let mut offset = 2;

        while offset + 2 <= payload.len() {
            // 16-bit 长度字段 (大端)
            let nal_len = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
            offset += 2;

            if offset + nal_len > payload.len() {
                break;
            }

            let nal_data = &payload[offset..offset + nal_len];

            if nal_data.len() >= 2 {
                let (_, nal_type, _, _) = parse_h265_nal_header(nal_data)?;

                // 缓存参数集
                match nal_type {
                    H265NalUnitType::Vps => {
                        self.vps = Some(Bytes::copy_from_slice(nal_data));
                    }
                    H265NalUnitType::Sps => {
                        self.sps = Some(Bytes::copy_from_slice(nal_data));
                    }
                    H265NalUnitType::Pps => {
                        self.pps = Some(Bytes::copy_from_slice(nal_data));
                    }
                    _ => {}
                }

                // 添加起始码 + NAL 单元
                let mut nal_with_startcode = BytesMut::new();
                nal_with_startcode.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                nal_with_startcode.extend_from_slice(nal_data);

                entry.nals.push(nal_with_startcode.freeze());

                if nal_type.is_keyframe() {
                    entry.is_keyframe = true;
                }
            }

            offset += nal_len;
        }

        Some(())
    }

    /// 完成一个访问单元的重组
    fn finalize_au(&mut self, timestamp: u32) -> Option<H265AccessUnit> {
        if let Some(builder) = self.current_aus.remove(&timestamp) {
            if builder.nals.is_empty() {
                return None;
            }

            // 拼接所有 NAL 单元
            let mut data = BytesMut::new();

            // 如果是关键帧，在前面插入 VPS/SPS/PPS (如果需要)
            if builder.is_keyframe {
                if let Some(ref vps) = self.vps {
                    let mut vps_with_sc = BytesMut::new();
                    vps_with_sc.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    vps_with_sc.extend_from_slice(vps);
                    data.extend_from_slice(&vps_with_sc);
                }
                if let Some(ref sps) = self.sps {
                    let mut sps_with_sc = BytesMut::new();
                    sps_with_sc.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    sps_with_sc.extend_from_slice(sps);
                    data.extend_from_slice(&sps_with_sc);
                }
                if let Some(ref pps) = self.pps {
                    let mut pps_with_sc = BytesMut::new();
                    pps_with_sc.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    pps_with_sc.extend_from_slice(pps);
                    data.extend_from_slice(&pps_with_sc);
                }
            }

            for nal in &builder.nals {
                data.extend_from_slice(nal);
            }

            let au = H265AccessUnit {
                timestamp,
                data: data.freeze(),
                is_keyframe: builder.is_keyframe,
            };

            self.completed_aus.push(au.clone());

            // 限制缓存大小
            if self.completed_aus.len() > self.max_aus {
                self.completed_aus.remove(0);
            }

            Some(au)
        } else {
            None
        }
    }

    /// 获取已完成的访问单元
    pub fn access_units(&self) -> &[H265AccessUnit] {
        &self.completed_aus
    }

    /// 清除已完成的访问单元
    pub fn clear_access_units(&mut self) {
        self.completed_aus.clear();
    }

    /// 设置 VPS
    pub fn set_vps(&mut self, vps: Bytes) {
        self.vps = Some(vps);
    }

    /// 设置 SPS
    pub fn set_sps(&mut self, sps: Bytes) {
        self.sps = Some(sps);
    }

    /// 设置 PPS
    pub fn set_pps(&mut self, pps: Bytes) {
        self.pps = Some(pps);
    }
}

impl H265AccessUnitBuilder {
    fn new() -> Self {
        Self {
            nals: Vec::new(),
            is_keyframe: false,
            fu_started: false,
            fu_nal_header: None,
            fu_data: BytesMut::new(),
        }
    }
}

/// 将 H265AccessUnit 转换为 MediaPacket
impl From<H265AccessUnit> for MediaPacket {
    fn from(au: H265AccessUnit) -> Self {
        MediaPacket {
            media_type: MediaType::Video,
            codec_type: CodecType::H265,
            pts: au.timestamp as i64,
            dts: au.timestamp as i64,
            keyframe: au.is_keyframe,
            data: au.data,
            backend: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtp::packet::RtpHeader;

    fn make_packet(payload: Vec<u8>, timestamp: u32, marker: bool) -> RtpPacket {
        RtpPacket {
            header: RtpHeader {
                version: 2,
                padding: false,
                extension: false,
                marker,
                payload_type: 96, // H.265 动态负载类型
                sequence_number: 0,
                timestamp,
                ssrc: 0x12345678,
                csrcs: vec![],
            },
            payload: Bytes::from(payload),
        }
    }

    #[test]
    fn test_parse_h265_nal_header() {
        // NAL 头: 0x40 0x01 = VPS (Type=32, TID=1)
        let result = parse_h265_nal_header(&[0x40, 0x01]);
        assert!(result.is_some());
        let (_, nal_type, layer_id, tid) = result.unwrap();
        assert_eq!(nal_type, H265NalUnitType::Vps);
        assert_eq!(layer_id, 0);
        assert_eq!(tid, 1);
        assert_eq!(H265NalUnitType::from_u8(19), H265NalUnitType::IdrWRadl);
        assert_eq!(H265NalUnitType::from_u8(20), H265NalUnitType::IdrNlp);
        assert_eq!(H265NalUnitType::from_u8(48), H265NalUnitType::Ap);
        assert_eq!(H265NalUnitType::from_u8(49), H265NalUnitType::Fu);
    }

    #[test]
    fn test_single_nal() {
        let mut reassembler = H265Reassembler::new();

        // 模拟一个 VPS NAL 单元 (Type=32)
        let vps_payload = vec![0x40, 0x01, 0x0C, 0x01, 0x02];
        let packet = make_packet(vps_payload, 1000, true);

        let au = reassembler.push_packet(&packet);
        assert!(au.is_some());

        let au = au.unwrap();
        assert!(au.data.len() > 4); // 起始码 + NAL 数据
        assert!(!au.is_keyframe);
    }

    #[test]
    fn test_fu_reassembly() {
        let mut reassembler = H265Reassembler::new();

        // 模拟一个被分片的 IDR NAL 单元
        // NAL header (FU): Type=49
        // FU header: S=1, E=0, NAL type=19 (IDR_W_RADL)
        let part1 = vec![0x62, 0x01, 0x93, 0x01, 0x02, 0x03];
        let mut pkt1 = make_packet(part1, 2000, false);
        pkt1.header.sequence_number = 1;

        // FU header: S=0, E=0
        let part2 = vec![0x62, 0x01, 0x13, 0x04, 0x05, 0x06];
        let mut pkt2 = make_packet(part2, 2000, false);
        pkt2.header.sequence_number = 2;

        // FU header: S=0, E=1
        let part3 = vec![0x62, 0x01, 0x53, 0x07, 0x08, 0x09];
        let mut pkt3 = make_packet(part3, 2000, true);
        pkt3.header.sequence_number = 3;

        reassembler.push_packet(&pkt1);
        reassembler.push_packet(&pkt2);
        let au = reassembler.push_packet(&pkt3);

        assert!(au.is_some());
        let au = au.unwrap();
        assert!(au.is_keyframe);
        assert_eq!(&au.data[4..6], &[0x26, 0x01]);
    }
}
