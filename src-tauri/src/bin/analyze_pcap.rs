//! 分析 pcapng 文件，提取 RTP 包并检查是否包含 MJPEG 数据
//!
//! 使用方法：cargo run --bin analyze_pcap -- --file rtp.pcapng

use std::env;
use std::fs::File;
use std::io::Read;

/// 简单的 pcapng 解析器
struct PcapNgReader {
    data: Vec<u8>,
}

impl PcapNgReader {
    fn new(filepath: &str) -> Result<Self, String> {
        let mut file = File::open(filepath).map_err(|e| e.to_string())?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| e.to_string())?;
        Ok(Self { data })
    }

    /// 解析 pcapng 文件，提取数据包
    fn parse(&self) -> Result<Vec<Vec<u8>>, String> {
        let mut packets = Vec::new();
        let mut pos = 0;

        // 检查 Section Header Block
        if pos + 8 > self.data.len() {
            return Err("File too short".into());
        }

        // Block Type 总是以大端格式存储
        let block_type = u32::from_be_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]);
        
        if block_type != 0x0A0D0D0A {
            // 可能不是 pcapng，尝试作为 pcap 文件处理
            return self.parse_pcap();
        }

        // 确定字节序 - 读取 Byte Order Magic (偏移 8-11)
        let byte_order_magic = u32::from_be_bytes([self.data[8], self.data[9], self.data[10], self.data[11]]);
        let big_endian = byte_order_magic == 0x1A2B3C4D;

        eprintln!("Debug: pcapng format, big_endian={}", big_endian);

        while pos < self.data.len() {
            if pos + 8 > self.data.len() {
                break;
            }

            let block_type = if big_endian {
                u32::from_be_bytes([self.data[pos], self.data[pos+1], self.data[pos+2], self.data[pos+3]])
            } else {
                u32::from_le_bytes([self.data[pos], self.data[pos+1], self.data[pos+2], self.data[pos+3]])
            };

            let total_length = if big_endian {
                u32::from_be_bytes([self.data[pos+4], self.data[pos+5], self.data[pos+6], self.data[pos+7]]) as usize
            } else {
                u32::from_le_bytes([self.data[pos+4], self.data[pos+5], self.data[pos+6], self.data[pos+7]]) as usize
            };

            if total_length < 8 || pos + total_length > self.data.len() {
                eprintln!("Debug: invalid block at pos {}, type={:08X}, length={}", pos, block_type, total_length);
                break;
            }

            eprintln!("Debug: block at pos {}, type={:08X}, length={}", pos, block_type, total_length);

            match block_type {
                0x0A0D0D0A => {
                    // Section Header Block - 跳过
                    pos += total_length;
                }
                1 => {
                    // Interface Description Block - 跳过
                    pos += total_length;
                }
                2 | 6 => {
                    // Enhanced Packet Block (2) 或 Packet Block (6, 已弃用)
                    if let Some(packet_data) = self.parse_enhanced_packet_block(pos, total_length, big_endian) {
                        packets.push(packet_data);
                    }
                    pos += total_length;
                }
                3 => {
                    // Simple Packet Block
                    if let Some(packet_data) = self.parse_simple_packet_block(pos, total_length, big_endian) {
                        packets.push(packet_data);
                    }
                    pos += total_length;
                }
                _ => {
                    // 跳过未知块
                    pos += total_length;
                }
            }

            // 4 字节对齐
            pos = (pos + 3) & !3;
        }

        Ok(packets)
    }

    /// 解析 pcap 格式文件 (不是 pcapng)
    fn parse_pcap(&self) -> Result<Vec<Vec<u8>>, String> {
        let mut packets = Vec::new();
        
        if self.data.len() < 24 {
            return Err("File too short for pcap header".into());
        }

        // 检查 magic number
        let magic = u32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]);
        let big_endian = magic == 0xA1B2C3D4;
        
        eprintln!("Debug: pcap format, magic={:08X}, big_endian={}", magic, big_endian);

        let mut pos = 24; // 跳过 pcap header

        while pos < self.data.len() {
            if pos + 16 > self.data.len() {
                break;
            }

            // Packet header: ts_sec, ts_usec, incl_len, orig_len
            let ts_sec = if big_endian {
                u32::from_be_bytes([self.data[pos], self.data[pos+1], self.data[pos+2], self.data[pos+3]])
            } else {
                u32::from_le_bytes([self.data[pos], self.data[pos+1], self.data[pos+2], self.data[pos+3]])
            };

            let ts_usec = if big_endian {
                u32::from_be_bytes([self.data[pos+4], self.data[pos+5], self.data[pos+6], self.data[pos+7]])
            } else {
                u32::from_le_bytes([self.data[pos+4], self.data[pos+5], self.data[pos+6], self.data[pos+7]])
            };

            let incl_len = if big_endian {
                u32::from_be_bytes([self.data[pos+8], self.data[pos+9], self.data[pos+10], self.data[pos+11]]) as usize
            } else {
                u32::from_le_bytes([self.data[pos+8], self.data[pos+9], self.data[pos+10], self.data[pos+11]]) as usize
            };

            pos += 16;

            if pos + incl_len > self.data.len() {
                break;
            }

            let packet_data = self.data[pos..pos + incl_len].to_vec();
            packets.push(packet_data);

            pos += incl_len;
            pos = (pos + 3) & !3;
        }

        Ok(packets)
    }

    fn parse_enhanced_packet_block(&self, block_start: usize, total_length: usize, big_endian: bool) -> Option<Vec<u8>> {
        // Block Type (4) + Total Length (4) + Interface ID (4) + Timestamp High (4) + Timestamp Low (4) + Captured Len (4) + Packet Len (4)
        let header_size = 28;
        
        if block_start + header_size > self.data.len() {
            return None;
        }

        let captured_len = if big_endian {
            u32::from_be_bytes([
                self.data[block_start + 20],
                self.data[block_start + 21],
                self.data[block_start + 22],
                self.data[block_start + 23],
            ]) as usize
        } else {
            u32::from_le_bytes([
                self.data[block_start + 20],
                self.data[block_start + 21],
                self.data[block_start + 22],
                self.data[block_start + 23],
            ]) as usize
        };

        let data_start = block_start + header_size;
        let data_end = data_start + captured_len;

        if data_end > self.data.len() {
            return None;
        }

        Some(self.data[data_start..data_end].to_vec())
    }

    fn parse_simple_packet_block(&self, block_start: usize, total_length: usize, big_endian: bool) -> Option<Vec<u8>> {
        // Block Type (4) + Total Length (4) + Packet Len (4)
        let header_size = 12;
        
        if block_start + header_size > self.data.len() {
            return None;
        }

        let packet_len = if big_endian {
            u32::from_be_bytes([
                self.data[block_start + 8],
                self.data[block_start + 9],
                self.data[block_start + 10],
                self.data[block_start + 11],
            ]) as usize
        } else {
            u32::from_le_bytes([
                self.data[block_start + 8],
                self.data[block_start + 9],
                self.data[block_start + 10],
                self.data[block_start + 11],
            ]) as usize
        };

        let data_start = block_start + header_size;
        let data_end = data_start + packet_len.min(total_length - header_size - 4);

        if data_end > self.data.len() {
            return None;
        }

        Some(self.data[data_start..data_end.min(self.data.len())].to_vec())
    }
}

/// 解析以太网帧，提取 IP 和 UDP，然后提取 RTP
fn parse_ethernet(frame: &[u8]) -> Option<(u8, u32, u32, &[u8])> {
    if frame.len() < 14 {
        return None;
    }

    let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
    
    if ethertype == 0x0800 {
        // IPv4
        parse_ipv4(&frame[14..])
    } else if ethertype == 0x0806 || ethertype == 0x86DD {
        // ARP or IPv6 - skip
        None
    } else {
        // 可能是自定义 EtherType 或无 Ethernet 头
        // 尝试直接解析为 IPv4
        parse_ipv4(frame)
    }
}

/// 解析 IPv4 包
fn parse_ipv4(packet: &[u8]) -> Option<(u8, u32, u32, &[u8])> {
    if packet.len() < 20 {
        return None;
    }

    let version_ihl = packet[0];
    let version = (version_ihl >> 4) & 0x0F;
    let ihl = (version_ihl & 0x0F) * 4;

    if version != 4 {
        return None;
    }

    let protocol = packet[9];
    let src_ip = u32::from_be_bytes([packet[12], packet[13], packet[14], packet[15]]);
    let dst_ip = u32::from_be_bytes([packet[16], packet[17], packet[18], packet[19]]);

    if protocol == 17 {
        // UDP
        if packet.len() < ihl as usize + 8 {
            return None;
        }
        let udp_data = &packet[ihl as usize + 8..];
        Some((17, src_ip, dst_ip, udp_data))
    } else {
        None
    }
}

/// 解析 RTP 包头
#[derive(Debug)]
struct RtpHeader {
    version: u8,
    padding: bool,
    extension: bool,
    csrc_count: u8,
    marker: bool,
    payload_type: u8,
    sequence_number: u16,
    timestamp: u32,
    ssrc: u32,
}

fn parse_rtp(data: &[u8]) -> Option<(RtpHeader, &[u8])> {
    if data.len() < 12 {
        return None;
    }

    let b0 = data[0];
    let version = (b0 >> 6) & 0x03;
    let padding = ((b0 >> 5) & 0x01) != 0;
    let extension = ((b0 >> 4) & 0x01) != 0;
    let csrc_count = b0 & 0x0F;

    let b1 = data[1];
    let marker = ((b1 >> 7) & 0x01) != 0;
    let payload_type = b1 & 0x7F;

    let sequence_number = u16::from_be_bytes([data[2], data[3]]);
    let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

    let mut offset = 12 + csrc_count as usize * 4;

    if extension {
        if data.len() < offset + 4 {
            return None;
        }
        let ext_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize * 4;
        offset += 4 + ext_len;
    }

    if offset >= data.len() {
        return None;
    }

    let header = RtpHeader {
        version,
        padding,
        extension,
        csrc_count,
        marker,
        payload_type,
        sequence_number,
        timestamp,
        ssrc,
    };

    let payload = &data[offset..];
    Some((header, payload))
}

/// MJPEG/RTP 解析 (RFC 2435)
/// 返回 (type, q, width, height, quant_tables, scan_data)
fn parse_jpeg_rtp(payload: &[u8]) -> Option<(u8, u8, u16, u16, Vec<Vec<u8>>, &[u8])> {
    if payload.len() < 8 {
        return None;
    }

    let type_spec = payload[0];
    let offset = u32::from_be_bytes([0, payload[1], payload[2], payload[3]]);
    let _type = (type_spec >> 3) & 0x07;
    let q = type_spec & 0x07;
    let width = u16::from_be_bytes([payload[4], payload[5]]);
    let height = u16::from_be_bytes([payload[6], payload[7]]);

    let rest = &payload[8..];

    // 如果 type 指定使用量化表，它们会在 rest 中
    let mut quant_tables = Vec::new();
    let scan_data = rest;

    Some((_type, q, width, height, quant_tables, scan_data))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filepath = if args.len() > 2 && args[1] == "--file" {
        &args[2]
    } else if args.len() > 1 {
        &args[1]
    } else {
        "rtp.pcapng"
    };

    eprintln!("分析文件: {}", filepath);

    let reader = match PcapNgReader::new(filepath) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("打开文件失败: {}", e);
            return;
        }
    };

    let packets = match reader.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("解析失败: {}", e);
            return;
        }
    };

    eprintln!("总共解析到 {} 个数据包", packets.len());

    let mut rtp_packets = 0;
    let mut rtp_by_pt: std::collections::HashMap<u8, u32> = std::collections::HashMap::new();
    let mut ssrcs: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut jpeg_packets = 0;
    let mut jpeg_payloads: Vec<Vec<u8>> = Vec::new();

    for packet_data in &packets {
        // 尝试解析为以太网帧
        if let Some((proto, _, _, udp_data)) = parse_ethernet(packet_data) {
            if proto == 17 {
                // UDP
                if let Some((header, payload)) = parse_rtp(udp_data) {
                    rtp_packets += 1;
                    *rtp_by_pt.entry(header.payload_type).or_insert(0) += 1;
                    ssrcs.insert(header.ssrc);

                    // 检查是否是 JPEG (payload type 26 或动态类型)
                    if header.payload_type == 26 {
                        jpeg_packets += 1;
                        jpeg_payloads.push(payload.to_vec());
                    } else if header.payload_type >= 96 {
                        // 动态类型，可能是 MJPEG
                        // 检查 RTP payload 是否以 JPEG 头开始 (RFC 2435)
                        if payload.len() >= 8 {
                            jpeg_packets += 1;
                            jpeg_payloads.push(payload.to_vec());
                        }
                    }
                }
            }
        } else {
            // 如果没有以太网头，尝试直接解析为 RTP
            if let Some((header, payload)) = parse_rtp(packet_data) {
                rtp_packets += 1;
                *rtp_by_pt.entry(header.payload_type).or_insert(0) += 1;
                ssrcs.insert(header.ssrc);

                if header.payload_type == 26 || header.payload_type >= 96 {
                    jpeg_packets += 1;
                    jpeg_payloads.push(payload.to_vec());
                }
            }
        }
    }

    println!("\n=== RTP 分析结果 ===");
    println!("RTP 包总数: {}", rtp_packets);
    println!("SSRC 数量: {}", ssrcs.len());
    println!("Payload Type 分布:");
    for (pt, count) in &rtp_by_pt {
        let name = match pt {
            0 => "PCMU",
            8 => "PCMA",
            26 => "JPEG",
            33 => "MP2T",
            34 => "H263",
            96..=127 => "Dynamic",
            _ => "Unknown",
        };
        println!("  PT={} ({}) : {} 个包", pt, name, count);
    }

    println!("\n=== MJPEG 分析 ===");
    println!("可能的 JPEG RTP 包数量: {}", jpeg_packets);

    if !jpeg_payloads.is_empty() {
        println!("\n尝试重组 MJPEG...");
        
        // 检查第一个 JPEG RTP 包的格式
        let first_payload = &jpeg_payloads[0];
        if first_payload.len() >= 8 {
            let type_spec = first_payload[0];
            let offset = u32::from_be_bytes([0, first_payload[1], first_payload[2], first_payload[3]]);
            let width = u16::from_be_bytes([first_payload[4], first_payload[5]]);
            let height = u16::from_be_bytes([first_payload[6], first_payload[7]]);
            
            println!("JPEG RTP 头:");
            println!("  Type spec: {}", type_spec);
            println!("  Offset: {}", offset);
            println!("  Width: {}", width);
            println!("  Height: {}", height);
            println!("  Payload 大小: {}", first_payload.len());

            // 尝试提取 JPEG 数据
            let jpeg_data_start = 8;
            if first_payload.len() > jpeg_data_start {
                let jpeg_header = &first_payload[jpeg_data_start..];
                // 检查是否是 JPEG 文件头 (FF D8 FF)
                if jpeg_header.len() >= 3 && jpeg_header[0] == 0xFF && jpeg_header[1] == 0xD8 {
                    println!("\n检测到 JPEG 文件头 (FF D8 FF)！");
                    println!("可以提取 MJPEG 帧");
                } else {
                    println!("\n未检测到 JPEG 文件头");
                    println!("前 16 字节: {:02X?}", &jpeg_header[..16.min(jpeg_header.len())]);
                }
            }
        }

        // 尝试将所有 RTP payload 重组为完整的 JPEG
        let mut reassembled = Vec::new();
        for payload in &jpeg_payloads {
            if payload.len() > 8 {
                let jpeg_data = &payload[8..];
                reassembled.extend_from_slice(jpeg_data);
            }
        }

        if reassembled.len() > 0 {
            println!("\n重组后的数据大小: {} 字节", reassembled.len());
            
            // 检查是否是有效的 JPEG
            if reassembled.len() >= 3 && reassembled[0] == 0xFF && reassembled[1] == 0xD8 {
                println!("检测到 JPEG SOI 标记");
                
                // 保存为 JPEG 文件
                let output_path = "output_mjpeg.jpg";
                std::fs::write(output_path, &reassembled).unwrap();
                println!("已保存为: {}", output_path);
            } else {
                println!("未检测到有效的 JPEG 头");
                println!("前 32 字节: {:02X?}", &reassembled[..32.min(reassembled.len())]);
            }
        }
    }

    println!("\n=== 结论 ===");
    if jpeg_packets > 0 {
        println!("抓包文件中包含 MJPEG RTP 流！");
        println!("现有 RTP 模块可以解析 RTP 包头，但需要额外的 MJPEG 重组逻辑。");
    } else {
        println!("未检测到 MJPEG RTP 流。");
        println!("可能的原因:");
        println!("  1. Payload type 不是标准的 26");
        println!("  2. 使用了动态 payload type (96-127)");
        println!("  3. 不是 MJPEG 格式");
    }
}
