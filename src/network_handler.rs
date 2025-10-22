use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread::ThreadId;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const MAX_MSG_LEN: usize = 120;
const BUFFER_SIZE: usize = MAX_MSG_LEN + 2 + 18;

pub fn handle_client(mut stream: TcpStream, peer_addr: SocketAddr, tid: ThreadId) {
    let mut buffer= [0_u8; BUFFER_SIZE];

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

                // 解析自定义应用层协议PDU（在这里只是简单地回显）
                // 实际应用中可以在此处添加业务逻辑处理
                let response = format!("({:?}){}", tid, String::from_utf8_lossy(&buffer[..size]));
                println!("{}", response);

                // 将接收到的数据原样发送回客户端（实现echo功能）
                match stream.write_all(response.as_bytes()) {
                    Ok(_) => {
                        println!("[{:?}] 向客户端 {} 发送 {} 字节数据", tid, peer_addr, size);
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

    // 收发业务数据的小循环
    loop {
        // 接收来自客户端的数据
        match stream.read(&mut buffer) {
            Ok(0) => {
                // 客户端正常关闭连接
                println!("[{}] client[{}] is closed!", id, peer_addr);
                break;
            }
            Ok(size) => {
                println!("从客户端 {} 接收到 {} 字节数据", peer_addr, size);

                // 解析自定义应用层协议PDU（在这里只是简单地回显）
                // 实际应用中可以在此处添加业务逻辑处理
                let response = format!("({}){}", id, String::from_utf8_lossy(&buffer[..size]));
                println!("{}", response);

                // 将接收到的数据原样发送回客户端（实现echo功能）
                match stream.write_all(response.as_bytes()) {
                    Ok(_) => {
                        println!("[{}] 向客户端 {} 发送 {} 字节数据", id, peer_addr, size);
                    }
                    Err(e) => {
                        eprintln!("[{}] 写入客户端 {} 失败: {}", id, peer_addr, e);
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

    // std::thread::sleep(std::time::Duration::from_secs(5)); // 模拟子线程退出的延迟

    // 连接会在drop时自动关闭
    println!("[{}] 与客户端 {} 的连接已关闭", id, peer_addr);
}

pub async fn handle_client_async(mut stream: tokio::net::TcpStream, peer_addr: SocketAddr, id: u32, shutdown_notify: &tokio::sync::Notify) {
    let mut buffer= [0_u8; BUFFER_SIZE];

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
                        println!("[{}] 从客户端 {} 接收到 {} 字节数据", id, peer_addr, size);
                        
                        // 解析自定义应用层协议PDU（在这里只是简单地回显）
                        let response = format!("({}){}", id, String::from_utf8_lossy(&buffer[..size]));
                        println!("{}", response);
                        
                        // 将接收到的数据原样发送回客户端（实现echo功能）
                        if let Err(e) = stream.write_all(response.as_bytes()).await {
                            eprintln!("[{}] 写入客户端 {} 失败: {}", id, peer_addr, e);
                            break;
                        }
                        println!("[{}] 向客户端 {} 发送 {} 字节数据", id, peer_addr, size);
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