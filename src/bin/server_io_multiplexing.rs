use tokio::net::{TcpListener};
use tokio::sync::Notify;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::{HashMap};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use signal_hook::consts::SIGINT;
use signal_hook::iterator::Signals;

use socket::network_handler::{handle_client_async};

// 创建一个通知机制来处理关闭信号
static SHUTDOWN_NOTIFY: Notify = Notify::const_new();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建TCP监听器，绑定到指定地址和端口
    let listener = TcpListener::bind("0.0.0.0:8080").await.expect("无法绑定到地址");
    let local_addr = listener.local_addr().unwrap().to_string();
    println!("[srv] server[{}] is initializing!", local_addr);

    // 创建线程句柄存储器
    let mut task_handles: HashMap<u32, JoinHandle<()>> = HashMap::new();

    // 设置信号处理
    let mut signals = Signals::new(&[SIGINT,]).expect("无法创建信号处理器");

    // 在单独的线程中处理信号，避免阻塞主线程
    std::thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    println!("[srv] SIGINT is coming!");

                    println!("[srv] 关闭所有活动连接");
                    SHUTDOWN_NOTIFY.notify_waiters();
                },
                _ => unreachable!(),
            }
        }
    });

    let mut connection_id: u32 = 0;

    // 异步处理客户端请求的大循环
    loop {
        // 定期清理已完成的任务
        println!("清理已完成的任务...");
        let mut completed_tasks = Vec::new();
        for (id, handle) in task_handles.iter() {
            if handle.is_finished() {
                completed_tasks.push(*id);
            }
        }
        if !completed_tasks.is_empty() {
            for id in completed_tasks {
                if let Some(handle) = task_handles.remove(&id) {
                    // 等待任务完成并处理可能的错误
                    let _ = handle.await;
                    println!("[srv] 任务[{}]成功清理", id);
                }
            }
        }

        tokio::select! {
            // 监听新的连接
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let peer_addr = stream.peer_addr().unwrap();
                        println!("[srv] client[{}] is accepted!", peer_addr);

                        connection_id += 1;

                        let handle = tokio::spawn(async move {
                            handle_client_async(stream, peer_addr, connection_id, &SHUTDOWN_NOTIFY).await;
                        });

                        // 将任务句柄存储到连接管理器中
                        task_handles.insert(connection_id, handle);
                    }
                    Err(e) => {
                        eprintln!("接受连接失败: {}", e);
                    }
                }
            }
            // 等待关闭通知
            _ = SHUTDOWN_NOTIFY.notified() => {
                println!("[srv] select 收到关闭通知");
                break;
            }
        }
    }

    // 等待所有任务退出
    println!("等待所有通信任务退出...");
    for (id, handle) in task_handles {
        println!("[srv] 等待任务[{}]退出", id);
        handle.abort(); // 取消任务
        println!("[srv] 任务[{}]已退出", id)
    }

    // 正常退出服务器
    println!("服务器关闭");
    return Ok(())
}