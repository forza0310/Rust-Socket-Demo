use std::net::{Shutdown, TcpStream};
use std::io::{stdin, stdout, Read, Write};

const MAX_MSG_LEN: usize = 120;
const BUFFER_SIZE: usize = MAX_MSG_LEN + 2 + 18;

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
            match stream.write_all(input_buffer.as_bytes()) {
                Ok(_) => {
                    // 消息已发送
                }
                Err(e) => {
                    eprintln!("发送消息到服务器失败: {}", e);
                    break;
                }
            }
        }

        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("服务器已关闭连接");
                break;
            }
            Ok(size) => {
                let message = String::from_utf8_lossy(&buffer[..size]);
                print!("[ECH_REP] {}", message);
                stdout().flush().unwrap();
            }
            Err(e) => {
                eprintln!("读取服务器消息失败: {}", e);
                break;
            }
        }
    }


    println!("[cli] client is to return!");
}