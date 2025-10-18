# Rust Socket 项目

这是一个使用 Rust 编写的 TCP Socket 通信示例项目，包含了服务器和客户端的实现。该项目演示了多种处理并发连接的方式，包括单线程、多线程和多进程模型。

## 项目结构

- [src/bin/server.rs] - 单线程 TCP 服务器实现
- [src/bin/server_muti_thread.rs] - 多线程 TCP 服务器实现
- [src/bin/server_muti_process.rs] - 多进程 TCP 服务器实现
- [src/bin/client.rs] - TCP 客户端实现
- [src/network_handler.rs] - 网络连接处理逻辑

## 功能特点

1. **TCP Echo 服务器**: 服务器接收客户端发送的数据，并将其原样返回
2. **多种并发模型**:
   - 单线程模型: 一次只能处理一个客户端连接
   - 多线程模型: 为每个客户端连接创建一个线程
   - 多进程模型: 为每个客户端连接创建一个进程
3. **信号处理**: 优雅地处理 SIGINT (Ctrl+C) 信号来关闭服务器
4. **连接管理**: 跟踪和管理活动连接
