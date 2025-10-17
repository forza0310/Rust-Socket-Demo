use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use signal_hook::consts::{SIGINT, SIGPIPE,};
use signal_hook::iterator::Signals;
use rand::Rng;

// 使用原子变量作为全局sigint_flag，0表示未收到信号，1表示收到SIGINT信号
static SIGINT_FLAG: AtomicBool = AtomicBool::new(false);

// 用于跟踪所有活动连接的结构
type Connections = Arc<Mutex<HashMap<u32, TcpStream>>>;

const MAX_MSG_LEN: usize = 120;
const BUFFER_SIZE: usize = MAX_MSG_LEN + 2 + 18;

// rust 中捕获SIGPIPE信号是一个unstable的功能
// https://github.com/rust-lang/rust/pull/13158 native: Ignore SIGPIPE by default
// https://dev-doc.rust-lang.org/beta/unstable-book/language-features/unix-sigpipe.html#unix_sigpipe
// https://github.com/rust-lang/rust/issues/62569
// https://github.com/rust-lang/rust/pull/124480
fn main() {
    let veri_code = rand::rng().random_range(10000..100000);
    // 创建TCP监听器，绑定到指定地址和端口
    let listener = TcpListener::bind("0.0.0.0:8080").expect("无法绑定到地址");
    listener.set_nonblocking(true).expect("无法设置非阻塞模式");

    let local_addr = listener.local_addr().unwrap().to_string();
    println!("[srv] server[{}] is initializing![{}]", local_addr, veri_code);

    // 设置信号处理
    // let mut signals = Signals::new(&[SIGINT, SIGPIPE]).expect("无法创建信号处理器");
    let mut signals = Signals::new(&[SIGINT,]).expect("无法创建信号处理器");

    // 处理客户端请求的大循环
    loop {
        // 非阻塞检查 pending 信号
        if let Some(sig) = signals.pending().next() {
            match sig {
                SIGINT => {
                    println!("[srv] SIGINT received!");
                    SIGINT_FLAG.store(true, Ordering::SeqCst);
                    break;
                }
                _ => {}
            }
        }
        if SIGINT_FLAG.load(Ordering::SeqCst) {
            println!("检测到SIGINT信号，准备退出...");
            break;
        }

        match listener.accept() {
            Ok((stream, _)) => {
                let peer_addr = stream.peer_addr().unwrap();
                println!("[srv] client[{}] is accepted!", peer_addr);
                handle_client(stream, peer_addr, veri_code, &mut signals);
            }
            Err(e) => {
                eprintln!("接受连接失败: {}", e);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("sleep 1s")
    }

    // 正常退出服务器
    println!("服务器关闭");
}

fn handle_client(mut stream: TcpStream, peer_addr: SocketAddr, veri_code: i32, mut signals: &mut Signals) {
    let mut buffer= [0_u8; BUFFER_SIZE];

    // 收发业务数据的小循环
    loop {
        // 非阻塞检查 pending 信号
        if let Some(sig) = signals.pending().next() {
            match sig {
                SIGINT => {
                    println!("[srv] SIGINT received!");
                    SIGINT_FLAG.store(true, Ordering::SeqCst);
                    break;
                }
                _ => {}
            }
        }

        // 接收来自客户端的数据
        match stream.read(&mut buffer) {
            Ok(0) => {
                // 客户端正常关闭连接
                println!("[srv] client[{}] is closed!", peer_addr);
                break;
            }
            Ok(size) => {
                println!("从客户端 {} 接收到 {} 字节数据", peer_addr, size);

                // 解析自定义应用层协议PDU（在这里只是简单地回显）
                // 实际应用中可以在此处添加业务逻辑处理
                let response = format!("({}){}", veri_code, String::from_utf8_lossy(&buffer[..size]));
                println!("{}", response);

                // 将接收到的数据原样发送回客户端（实现echo功能）
                match stream.write_all(response.as_bytes()) {
                    Ok(_) => {
                        println!("向客户端 {} 发送 {} 字节数据",peer_addr, size);
                    }
                    Err(e) => {
                        eprintln!("写入客户端 {} 失败: {}", peer_addr, e);
                        break;
                    }
                }
                // // 循环发送数据，直到发送成功或者发生错误（用来测试broken pipe）
                // loop {
                //     match stream.write_all(response.as_bytes()) {
                //         Ok(_) => {
                //             println!("向客户端 {} 发送 {} 字节数据",peer_addr, size);
                //         }
                //         Err(e) => {
                //             eprintln!("写入客户端 {} 失败: {}", peer_addr, e);
                //             break;
                //         }
                //     }
                // }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // 非阻塞模式下没有数据可读，继续执行其他任务
                // 可以在这里添加短暂的sleep避免忙等待
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            Err(e) => {
                eprintln!("读取客户端 {} 数据失败: {}", peer_addr, e);
                break;
            }
        }
    }

    // 连接会在drop时自动关闭
    println!("与客户端 {} 的连接已关闭", peer_addr);
}
