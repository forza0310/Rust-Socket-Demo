use std::fmt;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread::ThreadId;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MAX_PAYLOAD_LEN: usize = 255;
const BUFFER_SIZE: usize = 1024;

pub struct Pdu {
    /// Payload 长度 (最多255字节)
    pub length: u8,
    /// 实际数据内容
    pub payload: Vec<u8>,
}

impl fmt::Display for Pdu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PDU[length={}, payload=\"{}\"]",
               self.length,
               String::from_utf8_lossy(&self.payload))
    }
}

impl Pdu {
    /// 创建新的 PDU 实例
    pub fn new(data: &[u8]) -> Option<Self> {
        if data.len() > MAX_PAYLOAD_LEN {
            // 超过 u8 最大值 MAX_MSG_LEN，无法表示长度
            return None;
        }

        Some(Pdu {
            length: data.len() as u8,
            payload: data.to_vec(),
        })
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(1 + self.length as usize);
        vec.push(self.length);
        vec.extend_from_slice(&self.payload);
        vec
    }

    pub fn from_bytes(buffer: &[u8]) -> Option<Self> {
        if buffer.is_empty() {
            return None;
        }

        let length = buffer[0];
        if buffer.len() < (1 + length as usize) {
            // 缓冲区长度不足以容纳声明的 payload 长度
            return None;
        }

        Some(Pdu {
            length,
            payload: buffer[1..(1 + length as usize)].to_vec(),
        })
    }

    /// 检查缓冲区是否包含完整的 PDU 数据
    pub fn is_complete_pdu(buffer: &[u8]) -> bool {
        if buffer.is_empty() {
            return false;
        }

        buffer.len() >= (1 + buffer[0] as usize)
    }

    /// 获取完整 PDU 所需的总字节数（包括头部）
    pub fn payload_size(buffer: &[u8]) -> Option<usize> {
        if buffer.is_empty() {
            return None;
        }

        Some(1 + buffer[0] as usize)
    }
}

pub fn handle_client(mut stream: TcpStream, peer_addr: SocketAddr, tid: ThreadId) {
    let mut buffer= [0_u8; BUFFER_SIZE];
    let mut received_data: Vec<u8> = Vec::new(); // 存储已接收但尚未构成完整PDU的数据
    let mut pdu: Option<Pdu> = None;
    // 收发业务数据的小循环
    loop {
        // 接收来自客户端的数据
        match stream.read(&mut buffer) {
            Ok(0) => {
                // 客户端正常关闭连接
                println!("[{:?}] client[{}] is closed!", tid, peer_addr);
                break;
            }
            Ok(size) => {
                println!("[{:?}] 从客户端 {} 接收到 {} 字节数据", tid, peer_addr, size);
                received_data.extend_from_slice(&buffer[..size]);
                // 解析自定义应用层协议PDU
                if !Pdu::is_complete_pdu(&received_data) {
                    continue; // 接收到的数据不完整，等待下一次接收
                }
                match Pdu::payload_size(&received_data) {
                    Some(expected_size) => {
                        pdu = Pdu::from_bytes(&received_data[..expected_size]);
                        received_data.drain(..expected_size); // 移除已处理的数据
                    }
                    None => { // 输入的buffer没有内容
                        continue;
                    }
                }

                println!("{}", pdu.as_ref().unwrap());
                // 将接收到的数据原样发送回客户端（实现echo功能）
                let vec = pdu.unwrap().to_vec();
                match stream.write_all(vec.as_slice()) {
                    Ok(_) => {
                        println!("[{:?}] 向客户端 {} 发送 {} 字节数据", tid, peer_addr, vec.len());
                    }
                    Err(e) => {
                        eprintln!("[{:?}] 写入客户端 {} 失败: {}", tid, peer_addr, e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("[{:?}] 读取客户端 {} 数据失败: {}", tid, peer_addr, e);
                break;
            }
        }
    }

    // std::thread::sleep(std::time::Duration::from_secs(5)); // 模拟子线程退出的延迟

    // 连接会在drop时自动关闭
    println!("[{:?}] 与客户端 {} 的连接已关闭", tid, peer_addr);
}

pub fn handle_client2(mut stream: TcpStream, peer_addr: SocketAddr, id: u32) {
    let mut buffer= [0_u8; BUFFER_SIZE];
    let mut received_data: Vec<u8> = Vec::new(); // 存储已接收但尚未构成完整PDU的数据
    let mut pdu: Option<Pdu> = None;
    // 收发业务数据的小循环
    loop {
        // 接收来自客户端的数据
        match stream.read(&mut buffer) {
            Ok(0) => {
                // 客户端正常关闭连接
                println!("[{:?}] client[{}] is closed!", id, peer_addr);
                break;
            }
            Ok(size) => {
                println!("[{:?}] 从客户端 {} 接收到 {} 字节数据", id, peer_addr, size);
                received_data.extend_from_slice(&buffer[..size]);
                // 解析自定义应用层协议PDU
                if !Pdu::is_complete_pdu(&received_data) {
                    continue; // 接收到的数据不完整，等待下一次接收
                }
                match Pdu::payload_size(&received_data) {
                    Some(expected_size) => {
                        pdu = Pdu::from_bytes(&received_data[..expected_size]);
                        received_data.drain(..expected_size); // 移除已处理的数据
                    }
                    None => { // 输入的buffer没有内容
                        continue;
                    }
                }

                println!("{}", pdu.as_ref().unwrap());
                // 将接收到的数据原样发送回客户端（实现echo功能）
                let vec = pdu.unwrap().to_vec();
                match stream.write_all(vec.as_slice()) {
                    Ok(_) => {
                        println!("[{:?}] 向客户端 {} 发送 {} 字节数据", id, peer_addr, vec.len());
                    }
                    Err(e) => {
                        eprintln!("[{:?}] 写入客户端 {} 失败: {}", id, peer_addr, e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("[{:?}] 读取客户端 {} 数据失败: {}", id, peer_addr, e);
                break;
            }
        }
    }

    // 连接会在drop时自动关闭
    println!("[{:?}] 与客户端 {} 的连接已关闭", id, peer_addr);
}

pub async fn handle_client_async(mut stream: tokio::net::TcpStream, peer_addr: SocketAddr, id: u32, shutdown_notify: &tokio::sync::Notify) {
    let mut buffer= [0_u8; BUFFER_SIZE];
    let mut received_data: Vec<u8> = Vec::new(); // 存储已接收但尚未构成完整PDU的数据
    let mut pdu: Option<Pdu> = None;

    loop {
        tokio::select! {
            result = stream.read(&mut buffer) => {
                match result {
                    Ok(0) => {
                        // 客户端正常关闭连接
                        println!("[{}] client[{}] is closed!", id, peer_addr);
                        break;
                    }
                    Ok(size) => {
                        println!("[{:?}] 从客户端 {} 接收到 {} 字节数据", id, peer_addr, size);
                        received_data.extend_from_slice(&buffer[..size]);
                        // 解析自定义应用层协议PDU
                        if !Pdu::is_complete_pdu(&received_data) {
                            continue; // 接收到的数据不完整，等待下一次接收
                        }
                        match Pdu::payload_size(&received_data) {
                            Some(expected_size) => {
                                pdu = Pdu::from_bytes(&received_data[..expected_size]);
                                received_data.drain(..expected_size); // 移除已处理的数据
                            }
                            None => { // 输入的buffer没有内容
                                continue;
                            }
                        }

                        println!("{}", pdu.as_ref().unwrap());
                        // 将接收到的数据原样发送回客户端（实现echo功能）
                        let vec = pdu.unwrap().to_vec();
                        match stream.write_all(vec.as_slice()).await {
                            Ok(_) => {
                                println!("[{:?}] 向客户端 {} 发送 {} 字节数据", id, peer_addr, vec.len());
                            }
                            Err(e) => {
                                eprintln!("[{:?}] 写入客户端 {} 失败: {}", id, peer_addr, e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}] 读取客户端 {} 数据失败: {}", id, peer_addr, e);
                        break;
                    }
                }
            }
            // 等待关闭通知
            _ = shutdown_notify.notified() => {
                println!("[async] 收到关闭通知，断开客户端 {}", peer_addr);
                break;
            }
        }
    }
}