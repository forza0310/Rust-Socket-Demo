use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use signal_hook::consts::{SIGINT,};
use signal_hook::iterator::Signals;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, ThreadId};

use socket::network_handler::{handle_client};

// 使用原子变量作为全局sigint_flag，0表示未收到信号，1表示收到SIGINT信号
static SIGINT_FLAG: AtomicBool = AtomicBool::new(false);

type Connections = Arc<Mutex<HashMap<ThreadId, TcpStream>>>;  // 为了唤醒阻塞中的线程

const MAX_MSG_LEN: usize = 120;
const BUFFER_SIZE: usize = MAX_MSG_LEN + 2 + 18;

fn main() {
    // 创建TCP监听器，绑定到指定地址和端口
    let listener = TcpListener::bind("0.0.0.0:8080").expect("无法绑定到地址");
    let local_addr = listener.local_addr().unwrap().to_string();
    println!("[srv] server[{}] is initializing!", local_addr);

    // 创建连接管理器
    // Arc 提供了线程间安全的引用计数
    // Mutex 确保同一时间只有一个线程能访问 HashMap
    // connections.clone() 调用的是 Arc 的 clone 方法，它会：
    // - 增加内部引用计数
    // - 返回一个新的 Arc 实例
    // - 但这个新的 Arc 实例指向的是同一个 Mutex<HashMap<u32, TcpStream>>
    let connections: Connections = Arc::new(Mutex::new(HashMap::new()));
    let connections_clone = connections.clone(); // Arc 智能指针，它指向堆上的 HashMap

    // 创建线程句柄存储器
    let mut thread_handles: HashMap<ThreadId, JoinHandle<()>> = HashMap::new();

    // 设置信号处理
    let mut signals = Signals::new(&[SIGINT,]).expect("无法创建信号处理器");

    // 在单独的线程中处理信号，避免阻塞主线程
    std::thread::spawn(move || {
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
                    let _ = TcpStream::connect(&local_addr);
                },
                _ => unreachable!(),
            }
        }
    });

    // 多线程 处理客户端请求的大循环
    loop {
        // 检查信号标志
        if SIGINT_FLAG.load(Ordering::SeqCst) {
            println!("检测到SIGINT信号，准备退出...");
            break;
        }

        // 定期清理已完成的线程
        println!("清理通信子线程...");
        let finished_threads: Vec<_> = thread_handles
            .iter()
            .filter(|(_, handle)| handle.is_finished())
            .map(|(tid, _)| *tid)
            .collect();

        for tid in finished_threads {
            if let Some(handle) = thread_handles.remove(&tid) {
                match handle.join() {
                    Ok(_) => {
                        println!("[srv] 子线程[{:?}]成功join", tid);
                    }
                    Err(e) => {
                        eprintln!("[srv] 子线程[{:?}] join失败: {:?}", tid, e);
                    }
                }
            }
        }

        match listener.accept() { // 没有连接时会被阻塞，RUST会自动恢复被信号中断的accept()慢系统调用（std/src/sys/pal/unix/mod.rs::cvt_r() ）。
            Ok((stream, _)) => {
                let peer_addr = stream.peer_addr().unwrap();
                println!("[srv] client[{}] is accepted!", peer_addr);

                // 将新连接添加到连接管理器中
                connections.lock().unwrap().insert(std::thread::current().id(), stream.try_clone().unwrap());

                // 克隆需要传递给线程的变量
                let connections_clone = connections.clone();
                let stream_clone = stream.try_clone().unwrap(); 

                // 创建新线程处理客户端请求
                let handle = std::thread::spawn(move || {
                    handle_client(stream_clone, peer_addr, std::thread::current().id());
                    // 从连接管理器中移除已处理的连接
                    connections_clone.lock().unwrap().remove(&std::thread::current().id());
                });
                thread_handles.insert(handle.thread().id(), handle);
            }
            Err(e) => {
                eprintln!("接受连接失败: {}", e);
            }
        }
    }

    // 等待所有子线程退出
    println!("等待所通信子线程退出...");
    for (tid, handle) in thread_handles {
        if let Err(e) = handle.join() {
            eprintln!("线程等待出错: {:?}", e);
        }
        println!("[srv] 子线程[{:?}]成功join", tid);
    }

    // 正常退出服务器
    println!("服务器关闭");
}