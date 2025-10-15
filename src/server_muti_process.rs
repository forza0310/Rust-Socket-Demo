use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use signal_hook::consts::{SIGINT, SIGPIPE,};
use signal_hook::iterator::Signals;
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};

// 使用原子变量作为全局sigint_flag，0表示未收到信号，1表示收到SIGINT信号
static SIGINT_FLAG: AtomicBool = AtomicBool::new(false);

type Connections = Arc<Mutex<HashMap<u32, TcpStream>>>;

const MAX_MSG_LEN: usize = 120;
const BUFFER_SIZE: usize = MAX_MSG_LEN + 2 + 18;

fn main() {
    let veri_code = rand::rng().random_range(10000..100000);
    // 创建TCP监听器，绑定到指定地址和端口
    let listener = TcpListener::bind("0.0.0.0:8080").expect("无法绑定到地址");
    let local_addr = listener.local_addr().unwrap().to_string();
    println!("[srv] server[{}] is initializing![{}]", local_addr, veri_code);

    // 创建连接管理器
    // Arc 提供了线程间安全的引用计数
    // Mutex 确保同一时间只有一个线程能访问 HashMap
    // connections.clone() 调用的是 Arc 的 clone 方法，它会：
    // - 增加内部引用计数
    // - 返回一个新的 Arc 实例
    // - 但这个新的 Arc 实例指向的是同一个 Mutex<HashMap<u32, TcpStream>>
    let connections: Connections = Arc::new(Mutex::new(HashMap::new()));
    let connections_clone = connections.clone(); // Arc 智能指针，它指向堆上的 HashMap

    // 创建子进程ID存储器
    let mut child_pids: Vec<i32> = Vec::new();

    // 设置信号处理
    let mut signals = Signals::new(&[SIGINT,]).expect("无法创建信号处理器");

    // 在单独的线程中处理信号，避免阻塞主线程
    let signal_handle = std::thread::spawn(move || {
        // move 关键字使闭包获取外部变量的所有权，而不是借用
        // 在这个例子中，signals 和 local_addr 等变量需要被移动到新线程
        // 这确保了新线程可以独立访问这些变量，而不用担心生命周期
        for sig in signals.forever() {
            match sig {
                // process handle SIGINT -p true -s false // 加到 ~/.lldbinit
                // process status
                // platform shell kill -INT <PID>
                SIGINT => {
                    println!("[srv] SIGINT is coming!");
                    SIGINT_FLAG.store(true, Ordering::SeqCst);

                    println!("[srv] 关闭所有活动连接");
                    let mut connections = connections_clone.lock().unwrap();
                    for (_, stream) in connections.iter() {
                        stream.shutdown(std::net::Shutdown::Both).expect("无法关闭连接");
                    }
                    connections.clear();

                    println!("[srv] 主动连接一次本地地址以唤醒accept()");
                    let _ = TcpStream::connect(local_addr);

                    break; // 退出信号监听循环
                },
                _ => unreachable!(),
            }
        }
    });

    // 多线程 处理客户端请求的大循环
    let mut connection_counter = 0u32;
    loop {
        // 检查信号标志
        if SIGINT_FLAG.load(Ordering::SeqCst) {
            println!("检测到SIGINT信号，准备退出...");
            break;
        }

        match listener.accept() { // 没有连接时会被阻塞，RUST中会自动恢复被信号中断的系统调用（library/std/src/sys/pal/unix/mod.rs::cvt_r() ）。
            Ok((stream, _)) => {
                let peer_addr = stream.peer_addr().unwrap();
                println!("[srv] client[{}] is accepted!", peer_addr);

                // 将新连接添加到连接管理器中
                connection_counter += 1;
                let connection_id = connection_counter;
                connections.lock().unwrap().insert(connection_id, stream.try_clone().unwrap());

                // 克隆需要传递给线程的变量
                let connections_clone = connections.clone();
                let stream_clone = stream.try_clone().unwrap();

                // 创建子进程处理客户端请求
                unsafe {
                    match libc::fork() {
                        0 => {
                            // 子进程
                            // 关闭不需要的资源
                            drop(connections);
                            drop(listener);
                            drop(signal_handle);

                            handle_client(stream_clone, peer_addr, veri_code);

                            // 从连接管理器中移除已处理的连接
                            connections_clone.lock().unwrap().remove(&connection_id);

                            // 子进程退出
                            println!("子进程退出");
                            libc::exit(0);
                        }
                        pid if pid > 0 => {
                            // 父进程
                            child_pids.push(pid);
                            println!("[srv] 创建子进程[{}]", pid);
                            // 父进程不需要这个连接，关闭它
                            drop(stream_clone);
                        }
                        _ => {
                            eprintln!("创建子进程失败");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("接受连接失败: {}", e);
            }
        }
    }

    // 等待所有子进程退出
    println!("等待所有子进程退出...");
    for pid in child_pids {
        let mut status = 0;
        unsafe { libc::waitpid(pid, &mut status, 0) };
    }

    // 等待所有子线程退出
    println!("等待子线程退出...");
    if let Err(e) = signal_handle.join() {
        eprintln!("线程等待出错: {:?}", e);
    }

    // 正常退出服务器
    println!("服务器关闭");
}

fn handle_client(mut stream: TcpStream, peer_addr: SocketAddr, veri_code: i32) {
    let mut buffer= [0_u8; BUFFER_SIZE];

    // 收发业务数据的小循环
    loop {
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
            }
            Err(e) => {
                eprintln!("读取客户端 {} 数据失败: {}", peer_addr, e);
                break;
            }
        }
    }

    // std::thread::sleep(std::time::Duration::from_secs(5)); // 模拟子线程退出的延迟

    // 连接会在drop时自动关闭
    println!("与客户端 {} 的连接已关闭", peer_addr);
}
