//! H.264 RTP 解包器 (RFC 6184)
//!
//! 支持以下 H.264 RTP 负载格式：
//! - 单个 NAL 单元模式 (Single NALU)
//! - 分片单元模式 (FU-A, FU-B)
//! - 聚合包模式 (STAP-A, STAP-B)
//! - 多时间聚合包 (MTAP16, MTAP24)

use crate::rtp::packet::RtpPacket;
use crate::rtp::decoder::frame::MediaPacket;
use crate::rtp::decoder::types::{MediaType, CodecType};
use bytes::{Bytes, BytesMut};
use std::collections::BTreeMap;

/// H.264 NAL 单元类型 (RFC 6184 Section 5.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NalUnitType {
    /// 未指定
    Unspecified(u8),
    /// 非 IDR 图像中的片 (Slice)
    SliceNonIdr,
    /// 片数据分区 A
    SliceDataPartitionA,
    /// 片数据分区 B
    SliceDataPartitionB,
    /// 片数据分区 C
    SliceDataPartitionC,
    /// IDR 图像中的片 (关键帧)
    SliceIdr,
    /// 补充增强信息 (SEI)
    Sei,
    /// 序列参数集 (SPS)
    Sps,
    /// 图像参数集 (PPS)
    Pps,
    /// 访问单元分隔符
    Aud,
    /// 文件尾
    EndOfSequence,
    /// 文件头
    EndOfStream,
    /// 填充数据
    FillerData,
    /// 分片单元 A (FU-A)
    FuA,
    /// 分片单元 B (FU-B)
    FuB,
    /// 单时间聚合包 A (STAP-A)
    StapA,
    /// 单时间聚合包 B (STAP-B)
    StapB,
    /// 多时间聚合包 16-bit offset (MTAP16)
    Mtap16,
    /// 多时间聚合包 24-bit offset (MTAP24)
    Mtap24,
}

impl NalUnitType {
    pub fn from_u8(t: u8) -> Self {
        match t & 0x1F {
            0 => NalUnitType::Unspecified(t),
            1 => NalUnitType::SliceNonIdr,
            2 => NalUnitType::SliceDataPartitionA,
            3 => NalUnitType::SliceDataPartitionB,
            4 => NalUnitType::SliceDataPartitionC,
            5 => NalUnitType::SliceIdr,
            6 => NalUnitType::Sei,
            7 => NalUnitType::Sps,
            8 => NalUnitType::Pps,
            9 => NalUnitType::Aud,
            10 => NalUnitType::EndOfSequence,
            11 => NalUnitType::EndOfStream,
            12 => NalUnitType::FillerData,
            14 => NalUnitType::FuA,
            15 => NalUnitType::FuB,
            24 => NalUnitType::StapA,
            25 => NalUnitType::StapB,
            26 => NalUnitType::Mtap16,
            27 => NalUnitType::Mtap24,
            _ => NalUnitType::Unspecified(t),
        }
    }

    pub fn to_u8(&self) -> u8 {
        match self {
            NalUnitType::Unspecified(t) => *t,
            NalUnitType::SliceNonIdr => 1,
            NalUnitType::SliceDataPartitionA => 2,
            NalUnitType::SliceDataPartitionB => 3,
            NalUnitType::SliceDataPartitionC => 4,
            NalUnitType::SliceIdr => 5,
            NalUnitType::Sei => 6,
            NalUnitType::Sps => 7,
            NalUnitType::Pps => 8,
            NalUnitType::Aud => 9,
            NalUnitType::EndOfSequence => 10,
            NalUnitType::EndOfStream => 11,
            NalUnitType::FillerData => 12,
            NalUnitType::FuA => 14,
            NalUnitType::FuB => 15,
            NalUnitType::StapA => 24,
            NalUnitType::StapB => 25,
            NalUnitType::Mtap16 => 26,
            NalUnitType::Mtap24 => 27,
        }
    }
}

/// 解析 H.264 NAL 单元头 (1 字节)
///
/// 格式: [F(1bit) | NRI(2bit) | Type(5bit)]
pub fn parse_nal_header(byte: u8) -> (bool, u8, NalUnitType) {
    let forbidden = (byte & 0x80) != 0;
    let nri = (byte & 0x60) >> 5;
    let nal_type = NalUnitType::from_u8(byte & 0x1F);
    (forbidden, nri, nal_type)
}

/// FU-A 头 (RFC 6184 Section 5.8)
///
/// 格式: [S(1bit) | E(1bit) | R(1bit) | Type(5bit)]
#[derive(Debug, Clone)]
pub struct FuHeader {
    /// 起始位 (Start)
    pub start: bool,
    /// 结束位 (End)
    pub end: bool,
    /// 保留位 (必须为 0)
    pub reserved: bool,
    /// NAL 单元类型
    pub nal_type: NalUnitType,
}

impl FuHeader {
    pub fn from_byte(byte: u8) -> Self {
        Self {
            start: (byte & 0x80) != 0,
            end: (byte & 0x40) != 0,
            reserved: (byte & 0x20) != 0,
            nal_type: NalUnitType::from_u8(byte & 0x1F),
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
        if self.reserved {
            b |= 0x20;
        }
        b |= self.nal_type.to_u8() & 0x1F;
        b
    }
}

/// 一个完整的 H.264 访问单元 (AU)
///
/// 包含一个或多个 NAL 单元，构成一个完整的可解码单元
#[derive(Debug, Clone)]
pub struct H264AccessUnit {
    /// RTP 时间戳
    pub timestamp: u32,
    /// 重组后的数据 (包含起始码 0x00000001 + NAL 单元)
    pub data: Bytes,
    /// 是否包含关键帧 (IDR)
    pub is_keyframe: bool,
}

/// H.264 RTP 重组器
///
/// 收集同一个时间戳的所有 RTP 包，重组为完整的 H.264 访问单元
pub struct H264Reassembler {
    /// 当前正在收集的 AU: timestamp -> Vec<NAL unit>
    current_aus: BTreeMap<u32, AccessUnitBuilder>,
    /// 已完成的 AU
    completed_aus: Vec<H264AccessUnit>,
    /// 最大缓存 AU 数
    max_aus: usize,
    /// 缓存的 SPS/PPS (用于在每个 IDR 帧前插入)
    sps: Option<Bytes>,
    pps: Option<Bytes>,
}

struct AccessUnitBuilder {
    /// 收集的所有 NAL 单元 (带起始码)
    nals: Vec<Bytes>,
    /// 是否包含关键帧
    is_keyframe: bool,
    /// 是否收到 FU-A 的结束包
    fu_started: bool,
    /// 当前 FU-A 的 NAL 头 (用于重建第一个字节)
    fu_nal_header: Option<u8>,
    /// FU-A 分片数据缓存
    fu_data: BytesMut,
}

impl H264Reassembler {
    pub fn new() -> Self {
        Self {
            current_aus: BTreeMap::new(),
            completed_aus: Vec::new(),
            max_aus: 50,
            sps: None,
            pps: None,
        }
    }

    /// 处理一个 RTP 包，返回完成的 H.264 访问单元
    pub fn push_packet(&mut self, packet: &RtpPacket) -> Option<H264AccessUnit> {
        if packet.payload.is_empty() {
            return None;
        }

        let timestamp = packet.header.timestamp;
        let nal_header = packet.payload[0];
        let (_, _, nal_type) = parse_nal_header(nal_header);

        match nal_type {
            // 分片单元 A
            NalUnitType::FuA => {
                self.handle_fu_a(timestamp, &packet.payload)?;
            }
            // 分片单元 B
            NalUnitType::FuB => {
                // FU-B 类似 FU-A 但包含 DON (解码顺序号)
                self.handle_fu_b(timestamp, &packet.payload)?;
            }
            // 单时间聚合包 A (多个 NAL 单元在同一个时间戳)
            NalUnitType::StapA => {
                self.handle_stap_a(timestamp, &packet.payload)?;
            }
            // 单时间聚合包 B
            NalUnitType::StapB => {
                self.handle_stap_b(timestamp, &packet.payload)?;
            }
            // 单个 NAL 单元模式
            _ => {
                self.handle_single_nal(timestamp, &packet.payload)?;
            }
        }

        // 检查是否收到 marker 位 (通常表示一帧结束)
        if packet.header.marker {
            if let Some(au) = self.finalize_au(timestamp) {
                return Some(au);
            }
        }

        None
    }

    /// 处理单个 NAL 单元
    fn handle_single_nal(&mut self, timestamp: u32, payload: &[u8]) -> Option<()> {
        let nal_type = NalUnitType::from_u8(payload[0] & 0x1F);

        // 缓存 SPS/PPS
        match nal_type {
            NalUnitType::Sps => {
                self.sps = Some(Bytes::copy_from_slice(payload));
            }
            NalUnitType::Pps => {
                self.pps = Some(Bytes::copy_from_slice(payload));
            }
            _ => {}
        }

        let entry = self.current_aus.entry(timestamp).or_insert_with(|| {
            AccessUnitBuilder::new()
        });

        // 添加起始码 + NAL 单元
        let mut nal_with_startcode = BytesMut::new();
        nal_with_startcode.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        nal_with_startcode.extend_from_slice(payload);

        entry.nals.push(nal_with_startcode.freeze());

        if matches!(nal_type, NalUnitType::SliceIdr) {
            entry.is_keyframe = true;
        }

        Some(())
    }

    /// 处理 FU-A 分片
    fn handle_fu_a(&mut self, timestamp: u32, payload: &[u8]) -> Option<()> {
        if payload.len() < 2 {
            return None;
        }

        let fu_header = FuHeader::from_byte(payload[1]);
        let entry = self.current_aus.entry(timestamp).or_insert_with(|| {
            AccessUnitBuilder::new()
        });

        if fu_header.start {
            // 第一个分片：重建 NAL 头
            // NAL 头 = (原始 F 和 NRI from FU indicator) | (NAL type from FU header)
            let nal_header = (payload[0] & 0xE0) | (payload[1] & 0x1F);
            entry.fu_nal_header = Some(nal_header);
            entry.fu_started = true;
            entry.fu_data.clear();

            // 添加起始码
            entry.fu_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            entry.fu_data.extend_from_slice(&[nal_header]);
            entry.fu_data.extend_from_slice(&payload[2..]);

            entry.is_keyframe = matches!(fu_header.nal_type, NalUnitType::SliceIdr);
        } else if entry.fu_started {
            // 中间或结束分片
            entry.fu_data.extend_from_slice(&payload[2..]);
        }

        if fu_header.end {
            // 分片结束，将完整 NAL 单元添加到 AU
            let complete_nal = std::mem::take(&mut entry.fu_data).freeze();
            entry.nals.push(complete_nal);
            entry.fu_started = false;
        }

        Some(())
    }

    /// 处理 FU-B 分片
    fn handle_fu_b(&mut self, timestamp: u32, payload: &[u8]) -> Option<()> {
        // FU-B 与 FU-A 类似，但包含 DON (2 字节)
        if payload.len() < 4 {
            return None;
        }
        // 跳过 DON (字节 2-3)，其余与 FU-A 相同
        let fu_payload = &payload[4..];
        let mut modified = Vec::with_capacity(payload.len() - 2);
        modified.push(payload[0]); // FU indicator
        modified.push(payload[1]); // FU header
        modified.extend_from_slice(fu_payload);

        self.handle_fu_a(timestamp, &modified)
    }

    /// 处理 STAP-A (单时间聚合包)
    fn handle_stap_a(&mut self, timestamp: u32, payload: &[u8]) -> Option<()> {
        if payload.len() < 3 {
            return None;
        }

        let entry = self.current_aus.entry(timestamp).or_insert_with(|| {
            AccessUnitBuilder::new()
        });

        // 跳过 STAP-A 头 (1 字节)
        let mut offset = 1;
        while offset + 2 <= payload.len() {
            // 每个 NAL 单元前有一个 16-bit 长度字段
            let nal_len = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
            offset += 2;

            if offset + nal_len > payload.len() {
                break;
            }

            let nal_data = &payload[offset..offset + nal_len];
            let nal_type = NalUnitType::from_u8(nal_data[0] & 0x1F);

            // 缓存 SPS/PPS
            match nal_type {
                NalUnitType::Sps => {
                    self.sps = Some(Bytes::copy_from_slice(nal_data));
                }
                NalUnitType::Pps => {
                    self.pps = Some(Bytes::copy_from_slice(nal_data));
                }
                _ => {}
            }

            // 添加起始码 + NAL 单元
            let mut nal_with_startcode = BytesMut::new();
            nal_with_startcode.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
            nal_with_startcode.extend_from_slice(nal_data);

            entry.nals.push(nal_with_startcode.freeze());

            if matches!(nal_type, NalUnitType::SliceIdr) {
                entry.is_keyframe = true;
            }

            offset += nal_len;
        }

        Some(())
    }

    /// 处理 STAP-B (单时间聚合包，带 DON)
    fn handle_stap_b(&mut self, timestamp: u32, payload: &[u8]) -> Option<()> {
        // STAP-B 与 STAP-A 类似，但包含 DON (前 2 字节)
        if payload.len() < 3 {
            return None;
        }
        // 跳过 STAP-B 头 (1 字节) 和 DON (2 字节)
        let stap_a_payload = &payload[2..];
        self.handle_stap_a(timestamp, stap_a_payload)
    }

    /// 完成一个访问单元的重组
    fn finalize_au(&mut self, timestamp: u32) -> Option<H264AccessUnit> {
        if let Some(builder) = self.current_aus.remove(&timestamp) {
            if builder.nals.is_empty() {
                return None;
            }

            // 拼接所有 NAL 单元
            let mut data = BytesMut::new();
            for nal in &builder.nals {
                data.extend_from_slice(nal);
            }

            // 如果是关键帧，在前面插入 SPS/PPS (如果需要)
            let final_data = if builder.is_keyframe {
                let mut with_sps_pps = BytesMut::new();
                if let Some(ref sps) = self.sps {
                    let mut sps_with_sc = BytesMut::new();
                    sps_with_sc.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    sps_with_sc.extend_from_slice(sps);
                    with_sps_pps.extend_from_slice(&sps_with_sc);
                }
                if let Some(ref pps) = self.pps {
                    let mut pps_with_sc = BytesMut::new();
                    pps_with_sc.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    pps_with_sc.extend_from_slice(pps);
                    with_sps_pps.extend_from_slice(&pps_with_sc);
                }
                with_sps_pps.extend_from_slice(&data);
                with_sps_pps.freeze()
            } else {
                data.freeze()
            };

            let au = H264AccessUnit {
                timestamp,
                data: final_data,
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
    pub fn access_units(&self) -> &[H264AccessUnit] {
        &self.completed_aus
    }

    /// 清除已完成的访问单元
    pub fn clear_access_units(&mut self) {
        self.completed_aus.clear();
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

impl AccessUnitBuilder {
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

/// 将 H264AccessUnit 转换为 MediaPacket
impl From<H264AccessUnit> for MediaPacket {
    fn from(au: H264AccessUnit) -> Self {
        MediaPacket {
            media_type: MediaType::Video,
            codec_type: CodecType::H264,
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
                csrc_count: 0,
                marker,
                payload_type: 96, // H.264 动态负载类型
                sequence: 0,
                timestamp,
                ssrc: 0x12345678,
            },
            payload: Bytes::from(payload),
        }
    }

    #[test]
    fn test_parse_nal_header() {
        // NAL 头: 0x67 = SPS (forbidden=0, nri=3, type=7)
        let (forbidden, nri, nal_type) = parse_nal_header(0x67);
        assert!(!forbidden);
        assert_eq!(nri, 3);
        assert_eq!(nal_type, NalUnitType::Sps);
    }

    #[test]
    fn test_single_nal() {
        let mut reassembler = H264Reassembler::new();

        // 模拟一个 SPS NAL 单元
        let sps_payload = vec![0x67, 0x42, 0x00, 0x1E, 0xAB, 0xCD];
        let packet = make_packet(sps_payload, 1000, true);

        let au = reassembler.push_packet(&packet);
        assert!(au.is_some());

        let au = au.unwrap();
        assert!(au.data.len() > 4); // 起始码 + NAL 数据
        assert!(!au.is_keyframe);
    }

    #[test]
    fn test_fu_a_reassembly() {
        let mut reassembler = H264Reassembler::new();

        // 模拟一个被分片的 NAL 单元 (IDR slice)
        // FU indicator: 0x7C (F=0, NRI=3, Type=28=FU-A)
        // FU header: 0x85 (S=1, E=0, R=0, Type=5=IDR)
        let part1 = vec![0x7C, 0x85, 0x01, 0x02, 0x03];
        let mut pkt1 = make_packet(part1, 2000, false);
        pkt1.header.sequence = 1;

        // FU header: 0x05 (S=0, E=0, R=0, Type=5=IDR)
        let part2 = vec![0x7C, 0x05, 0x04, 0x05, 0x06];
        let mut pkt2 = make_packet(part2, 2000, false);
        pkt2.header.sequence = 2;

        // FU header: 0x45 (S=0, E=1, R=0, Type=5=IDR)
        let part3 = vec![0x7C, 0x45, 0x07, 0x08, 0x09];
        let mut pkt3 = make_packet(part3, 2000, true);
        pkt3.header.sequence = 3;

        reassembler.push_packet(&pkt1);
        reassembler.push_packet(&pkt2);
        let au = reassembler.push_packet(&pkt3);

        assert!(au.is_some());
        let au = au.unwrap();
        assert!(au.is_keyframe);
        // 数据应包含重建的 NAL 头 + 分片数据
        assert!(au.data.len() > 4 + 3 + 3 + 3);
    }
}
