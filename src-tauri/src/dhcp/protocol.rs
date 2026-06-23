//! DHCP 协议消息处理模块
//! 
//! 该模块负责处理 DHCP 协议相关的消息，包括：
//! - DHCP 消息的编码和解码
//! - 各种 DHCP 消息类型的处理
//! - DHCP 选项的处理

// protocol.rs 中暂时不使用日志宏
use std::net::Ipv4Addr;

/// DHCP 消息类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DhcpMessageType {
    /// DHCP DISCOVER - 客户端广播发现 DHCP 服务器
    Discover = 1,
    /// DHCP OFFER - 服务器响应 DISCOVER，提供 IP 地址
    Offer = 2,
    /// DHCP REQUEST - 客户端请求 IP 地址或续期
    Request = 3,
    /// DHCP DECLINE - 客户端拒绝 IP 地址
    Decline = 4,
    /// DHCP ACK - 服务器确认请求
    Ack = 5,
    /// DHCP NAK - 服务器拒绝请求
    Nak = 6,
    /// DHCP RELEASE - 客户端释放 IP 地址
    Release = 7,
    /// DHCP INFORM - 客户端请求配置信息
    Inform = 8,
}

/// DHCP 选项代码
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DhcpOptionCode {
    /// 填充选项（用于 32-bit 对齐）
    Pad = 0,
    /// 子网掩码
    SubnetMask = 1,
    /// 时间偏移
    TimeOffset = 2,
    /// 路由器（网关）
    Router = 3,
    /// 时间服务器
    TimeServer = 4,
    /// 名称服务器
    NameServer = 5,
    /// 域名服务器（DNS）
    DnsServer = 6,
    /// 日志服务器
    LogServer = 7,
    /// Cookie 服务器
    CookieServer = 8,
    /// LPR 服务器
    LprServer = 9,
    /// 主机名
    HostName = 12,
    /// 域名
    DomainName = 15,
    /// 根路径
    RootPath = 17,
    /// 请求 IP 地址
    RequestedIpAddress = 50,
    /// 租约时间
    LeaseTime = 51,
    /// 选项覆盖
    OptionOverload = 52,
    /// DHCP 消息类型
    DhcpMessageType = 53,
    /// 服务器标识符
    ServerIdentifier = 54,
    /// 参数请求列表
    ParameterRequestList = 55,
    /// 消息
    Message = 56,
    /// 最大 DHCP 消息大小
    MaxMessageSize = 57,
    /// 租约续期时间 (T1)
    RenewalTime = 58,
    /// 租约重绑定时间 (T2)
    RebindingTime = 59,
    /// 厂商类标识符
    VendorClassIdentifier = 60,
    /// 客户端标识符
    ClientIdentifier = 61,
    /// 结束标记
    End = 255,
}

/// DHCP 消息结构
#[derive(Debug, Clone)]
pub struct DhcpMessage {
    /// 操作码（1=请求，2=回复）
    pub op: u8,
    /// 硬件类型（1=Ethernet）
    pub htype: u8,
    /// 硬件地址长度
    pub hlen: u8,
    /// 跳数
    pub hops: u8,
    /// 事务 ID
    pub xid: u32,
    /// 秒数
    pub secs: u16,
    /// 标志位
    pub flags: u16,
    /// 客户端 IP 地址
    pub ciaddr: Ipv4Addr,
    /// 你的 IP 地址（分配给客户端的）
    pub yiaddr: Ipv4Addr,
    /// 服务器 IP 地址
    pub siaddr: Ipv4Addr,
    /// 网关 IP 地址
    pub giaddr: Ipv4Addr,
    /// 客户端硬件地址
    pub chaddr: [u8; 16],
    /// 服务器主机名
    pub sname: String,
    /// 启动文件名
    pub file: String,
    /// DHCP 选项
    pub options: Vec<DhcpOption>,
}

/// DHCP 选项
#[derive(Debug, Clone)]
pub struct DhcpOption {
    /// 选项代码
    pub code: u8,
    /// 选项数据
    pub data: Vec<u8>,
}

impl DhcpMessage {
    /// 创建新的 DHCP 消息
    pub fn new() -> Self {
        Self {
            op: 1,
            htype: 1,
            hlen: 6,
            hops: 0,
            xid: 0,
            secs: 0,
            flags: 0,
            ciaddr: Ipv4Addr::new(0, 0, 0, 0),
            yiaddr: Ipv4Addr::new(0, 0, 0, 0),
            siaddr: Ipv4Addr::new(0, 0, 0, 0),
            giaddr: Ipv4Addr::new(0, 0, 0, 0),
            chaddr: [0; 16],
            sname: String::new(),
            file: String::new(),
            options: Vec::new(),
        }
    }
    
    /// 设置 DHCP 消息类型
    pub fn set_message_type(&mut self, msg_type: DhcpMessageType) {
        let option = DhcpOption {
            code: DhcpOptionCode::DhcpMessageType as u8,
            data: vec![msg_type as u8],
        };
        self.options.push(option);
    }
    
    /// 获取 DHCP 消息类型
    pub fn get_message_type(&self) -> Option<DhcpMessageType> {
        for option in &self.options {
            if option.code == DhcpOptionCode::DhcpMessageType as u8 {
                if option.data.len() > 0 {
                    return match option.data[0] {
                        1 => Some(DhcpMessageType::Discover),
                        2 => Some(DhcpMessageType::Offer),
                        3 => Some(DhcpMessageType::Request),
                        4 => Some(DhcpMessageType::Decline),
                        5 => Some(DhcpMessageType::Ack),
                        6 => Some(DhcpMessageType::Nak),
                        7 => Some(DhcpMessageType::Release),
                        8 => Some(DhcpMessageType::Inform),
                        _ => None,
                    };
                }
            }
        }
        None
    }
    
    /// 设置客户端 MAC 地址
    pub fn set_mac_address(&mut self, mac: &str) {
        let parts: Vec<&str> = mac.split(':').collect();
        if parts.len() == 6 {
            for (i, part) in parts.iter().enumerate() {
                if let Ok(byte) = u8::from_str_radix(part, 16) {
                    self.chaddr[i] = byte;
                }
            }
        }
    }
    
    /// 获取客户端 MAC 地址
    pub fn get_mac_address(&self) -> String {
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.chaddr[0],
            self.chaddr[1],
            self.chaddr[2],
            self.chaddr[3],
            self.chaddr[4],
            self.chaddr[5]
        )
    }
    
    /// 添加 DHCP 选项
    pub fn add_option(&mut self, code: u8, data: Vec<u8>) {
        let option = DhcpOption { code, data };
        self.options.push(option);
    }
    
    /// 获取 DHCP 选项
    pub fn get_option(&self, code: u8) -> Option<&DhcpOption> {
        self.options.iter().find(|opt| opt.code == code)
    }
    
    /// 编码 DHCP 消息为字节流
    /// 
    /// 注意：这是一个简化版本，实际 DHCP 协议需要更完整的实现
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(300);
        
        // 基本头部字段
        buffer.push(self.op);
        buffer.push(self.htype);
        buffer.push(self.hlen);
        buffer.push(self.hops);
        buffer.extend_from_slice(&self.xid.to_be_bytes());
        buffer.extend_from_slice(&self.secs.to_be_bytes());
        buffer.extend_from_slice(&self.flags.to_be_bytes());
        buffer.extend_from_slice(&self.ciaddr.octets());
        buffer.extend_from_slice(&self.yiaddr.octets());
        buffer.extend_from_slice(&self.siaddr.octets());
        buffer.extend_from_slice(&self.giaddr.octets());
        buffer.extend_from_slice(&self.chaddr);
        
        // 服务器主机名（64 字节）
        let sname_bytes = self.sname.as_bytes();
        let sname_len = sname_bytes.len().min(63);
        buffer.extend_from_slice(&sname_bytes[..sname_len]);
        buffer.resize(buffer.len() + 64 - sname_len, 0);
        
        // 启动文件名（128 字节）
        let file_bytes = self.file.as_bytes();
        let file_len = file_bytes.len().min(127);
        buffer.extend_from_slice(&file_bytes[..file_len]);
        buffer.resize(buffer.len() + 128 - file_len, 0);
        
        // DHCP 魔术 Cookie
        buffer.extend_from_slice(&[99, 130, 83, 99]);
        
        // DHCP 选项
        for option in &self.options {
            buffer.push(option.code);
            buffer.push(option.data.len() as u8);
            buffer.extend_from_slice(&option.data);
        }
        
        // 结束标记
        buffer.push(DhcpOptionCode::End as u8);
        
        buffer
    }
    
    /// 解码字节流为 DHCP 消息
    /// 
    /// 注意：这是一个简化版本，实际 DHCP 协议需要更完整的实现
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 240 {
            return None;
        }
        
        let mut msg = Self::new();
        
        // 解析基本头部字段
        msg.op = data[0];
        msg.htype = data[1];
        msg.hlen = data[2];
        msg.hops = data[3];
        msg.xid = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        msg.secs = u16::from_be_bytes([data[8], data[9]]);
        msg.flags = u16::from_be_bytes([data[10], data[11]]);
        
        msg.ciaddr = Ipv4Addr::new(data[12], data[13], data[14], data[15]);
        msg.yiaddr = Ipv4Addr::new(data[16], data[17], data[18], data[19]);
        msg.siaddr = Ipv4Addr::new(data[20], data[21], data[22], data[23]);
        msg.giaddr = Ipv4Addr::new(data[24], data[25], data[26], data[27]);
        
        // 客户端硬件地址
        msg.chaddr.copy_from_slice(&data[28..44]);
        
        // 服务器主机名
        let sname_end = data[44..108].iter().position(|&b| b == 0).unwrap_or(64);
        msg.sname = String::from_utf8_lossy(&data[44..44 + sname_end]).to_string();
        
        // 启动文件名
        let file_end = data[108..236].iter().position(|&b| b == 0).unwrap_or(128);
        msg.file = String::from_utf8_lossy(&data[108..108 + file_end]).to_string();
        
        // 解析 DHCP 选项
        if data.len() > 240 && &data[236..240] == [99, 130, 83, 99] {
            let mut pos = 240;
            while pos < data.len() {
                let code = data[pos];
                
                // 处理 Pad Option (0) - 用于 32-bit 对齐
                if code == DhcpOptionCode::Pad as u8 {
                    pos += 1;
                    continue;
                }
                
                if code == DhcpOptionCode::End as u8 {
                    break;
                }
                
                if pos + 1 >= data.len() {
                    break;
                }
                let len = data[pos + 1] as usize;
                if len == 0 || pos + 2 + len > data.len() {
                    break;
                }
                let option_data = data[pos + 2..pos + 2 + len].to_vec();
                msg.options.push(DhcpOption { code, data: option_data });
                pos += 2 + len;
            }
        }
        
        Some(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mac_address() {
        let mut msg = DhcpMessage::new();
        msg.set_mac_address("00:11:22:33:44:55");
        let mac = msg.get_mac_address();
        assert_eq!(mac, "00:11:22:33:44:55");
    }
    
    #[test]
    fn test_message_type() {
        let mut msg = DhcpMessage::new();
        msg.set_message_type(DhcpMessageType::Discover);
        let msg_type = msg.get_message_type();
        assert_eq!(msg_type, Some(DhcpMessageType::Discover));
    }
    
    #[test]
    fn test_encode_decode() {
        let mut msg = DhcpMessage::new();
        msg.xid = 12345;
        msg.set_mac_address("00:11:22:33:44:55");
        msg.set_message_type(DhcpMessageType::Discover);
        
        let encoded = msg.encode();
        let decoded = DhcpMessage::decode(&encoded);
        
        assert!(decoded.is_some());
        let decoded_msg = decoded.unwrap();
        assert_eq!(decoded_msg.xid, 12345);
        assert_eq!(decoded_msg.get_mac_address(), "00:11:22:33:44:55");
    }
}
