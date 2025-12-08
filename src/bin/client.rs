use std::net::{TcpStream};
use std::io::{stdin, Read, Write};

use socket::network_handler::Pdu;

const MAX_MSG_LEN: usize = 255;
const BUFFER_SIZE: usize = MAX_MSG_LEN + 1;

fn main() {
    // 连接到服务器
    let mut stream = TcpStream::connect("127.0.0.1:8080").expect("无法连接到服务器");
    println!("[cli] server[{}] is connected!", stream.peer_addr().unwrap());

    let stdin = stdin();
    let mut input_buffer = String::new();
    let mut buffer = [0_u8; BUFFER_SIZE];

    println!("请输入要发送到服务器的消息（输入 'EXIT' 退出）:");

    loop {
        input_buffer.clear();
        stdin.read_line(&mut input_buffer).expect("读取输入失败");

        if input_buffer.trim() == "EXIT" {
            // 关闭TCP连接
            match stream.shutdown(std::net::Shutdown::Both) { // 关闭连接的读写两端
                Ok(_) => println!("[cli] stream is closed!"),
                Err(e) => eprintln!("关闭连接时出错: {}", e),
            }
            break;
        }

        if !input_buffer.is_empty() {
            println!("[ECH_RQT]{}", input_buffer);
            let pdu = Pdu::new(input_buffer.as_bytes());
            match stream.write_all(pdu.unwrap().to_vec().as_slice()) {
                Ok(_) => {
                    // 消息已发送
                }
                Err(e) => {
                    eprintln!("发送消息到服务器失败: {}", e);
                    break;
                }
            }
        }

        let mut received_data: Vec<u8> = Vec::new(); // 存储已接收但尚未构成完整PDU的数据
        let mut pdu: Option<Pdu> = None;
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => {
                    println!("服务器已关闭连接");
                    break;
                }
                Ok(size) => {
                    println!("从服务器接收到 {} 字节数据", size);

                    received_data.extend_from_slice(&buffer[..size]);
                    // 解析自定义应用层协议PDU
                    if !Pdu::is_complete_pdu(&received_data) {
                        continue; // 接收到的数据不完整，等待下一次接收
                    }
                    match Pdu::payload_size(&received_data) {
                        Some(expected_size) => {
                            pdu = Pdu::from_bytes(&received_data[..expected_size]);
                            received_data.clear();
                        }
                        None => { // 输入的buffer没有内容
                            continue;
                        }
                    }

                    println!("收到PDU: {}", pdu.as_ref().unwrap());
                    break;
                }
                Err(e) => {
                    eprintln!("读取服务器消息失败: {}", e);
                    break;
                }
            }
        }
    }

    println!("[cli] client is to return!");
}