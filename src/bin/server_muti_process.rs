use std::collections::{HashSet};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use signal_hook::consts::{SIGINT, SIGCHLD};
use signal_hook::iterator::Signals;
use std::sync::atomic::{AtomicBool, Ordering};

use socket::network_handler::handle_client2;

// 使用原子变量作为全局sigint_flag，0表示未收到信号，1表示收到SIGINT信号
static SIGINT_FLAG: AtomicBool = AtomicBool::new(false);

fn main() {
    // 创建TCP监听器，绑定到指定地址和端口
    let listener = TcpListener::bind("0.0.0.0:8080").expect("无法绑定到地址");
    let local_addr = listener.local_addr().unwrap().to_string();
    println!("[srv] server[{}] is initializing!", local_addr);

    // 创建子进程ID存储器
    let child_pids: Arc<Mutex<HashSet<i32>>> = Arc::new(Mutex::new(HashSet::new()));
    let child_pids_clone = child_pids.clone();

    // 设置信号处理
    let mut signals = Signals::new(&[SIGINT, SIGCHLD]).expect("无法创建信号处理器");

    // 在单独的线程中处理信号，避免阻塞主线程
    std::thread::spawn(move || {
        // move 关键字使闭包获取外部变量的所有权，而不是借用
        // 在这个例子中，signals 和 local_addr 等变量需要被移动到新线程
        // 这确保了新线程可以独立访问这些变量，而不用担心生命周期
        for sig in signals.forever() {
            let mut child_pids = child_pids_clone.lock().unwrap();
            match sig {
                SIGINT => {
                    println!("[srv] SIGINT is coming!");
                    SIGINT_FLAG.store(true, Ordering::SeqCst);
                    println!("[srv] 主动连接一次本地地址以唤醒accept()");
                    let _ = TcpStream::connect(&local_addr);
                    println!("[srv] 向所有子进程发送SIGINT");
                    for pid in child_pids.iter() {
                        unsafe { libc::kill(*pid, SIGINT); }
                    }
                },
                SIGCHLD => {
                    println!("[srv] SIGCHLD is coming!");
                    unsafe {
                        let mut pid;
                        while {
                            pid = libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG);
                            pid > 0
                        } {
                            println!("[srv] 收到子进程[{}]的退出信号", pid);
                            child_pids.remove(&pid);
                        }
                    };
                },
                _ => unreachable!(),
            }
        }
    });

    // 多进程 处理客户端请求的大循环
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

                // 克隆需要传递给进程的变量
                let stream_clone = stream.try_clone().unwrap();

                // 创建子进程处理客户端请求
                unsafe {
                    match libc::fork() {
                        0 => {
                            // 子进程
                            // 关闭不需要的资源
                            drop(child_pids);
                            drop(listener);

                            let pid = std::process::id() as i32;
                            // signal-hook 库会包装原本的信号处理器、缓存收到的信号（缓存在Signals变量中）。
                            // 当 fork() 被调用时，子进程继承了包装后的信号处理器和Signals变量，但是没有专门的线程
                            // 去消耗缓存的信号，这会产生非预期的行为，因此必须要重置相关的信号处理器。
                            libc::signal(SIGINT, libc::SIG_DFL); // 重置信号处理器

                            // 创建子进程的信号处理器
                            let stream_clone2 = stream_clone.try_clone().unwrap();
                            let mut signals = Signals::new(&[SIGINT,]).expect("无法创建信号处理器");
                            std::thread::spawn(move || {
                                for sig in signals.forever() {
                                    match sig {
                                        SIGINT => {
                                            println!("[{}] SIGINT is coming!", pid);
                                            stream_clone2.shutdown(std::net::Shutdown::Both).expect("无法关闭连接");
                                            break; // 退出信号监听循环
                                        },
                                        _ => unreachable!(),
                                    }
                                }
                            });

                            handle_client2(stream_clone, peer_addr, pid);

                            // 子进程退出
                            println!("[{}] 子进程退出", pid);
                            libc::exit(0);
                        }
                        pid if pid > 0 => {
                            // 父进程
                            // 父进程不需要这个连接，关闭它
                            drop(stream_clone);
                            println!("[srv] 创建子进程[{}]", pid);
                            child_pids.lock().unwrap().insert(pid);
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

    // 等待所有进程结束
    println!("等待通信子线程退出...");
    while child_pids.lock().unwrap().len() > 0 {
       std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // 正常退出服务器
    println!("服务器关闭");
}
