use std::net::UdpSocket;
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct UdpClient {
    socket: UdpSocket,
    sender: Sender<String>,
}

impl UdpClient {
    fn new(addr: &str) -> Result<(Self, Arc<Mutex<Receiver<String>>>), std::io::Error> {
        let socket = UdpSocket::bind(addr)?;
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        Ok((UdpClient { socket, sender }, receiver))
    }

    fn start_receiving(&self) {
        let mut buf = [0; 1024];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((size, _)) => {
                    if let Ok(message) = String::from_utf8(buf[..size].to_vec()) {
                        self.sender.send(message).unwrap();
                    }
                }
                Err(e) => eprintln!("Error receiving data: {}", e),
            }
        }
    }
}

fn monitor_queue_size(receiver: &Arc<Mutex<Receiver<String>>>) -> usize {
    if let Ok(rx) = receiver.lock() {
        rx.try_iter().count()
    } else {
        0
    }
}

fn main() {
    // 初始化UDP客户端
    let (client, receiver) = UdpClient::new("127.0.0.1:44444").unwrap();

    // 启动消息接收线程
    let receiver_thread = thread::spawn(move || {
        client.start_receiving();
    });

    // 启动队列监控线程
    let monitor_receiver = receiver.clone();
    let monitor_thread = thread::spawn(move || {
        loop {
            let queue_size = monitor_queue_size(&monitor_receiver);
            println!("Current queue size: {}", queue_size);
            thread::sleep(Duration::from_secs(1));
        }
    });

    // 启动消费者线程
    let consumer_thread = thread::spawn(move || {
        loop {
            if let Ok(rx) = receiver.lock() {
                if let Ok(message) = rx.recv() {
                    println!("Received message: {}", message);
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    // 等待线程完成
    receiver_thread.join().unwrap();
    monitor_thread.join().unwrap();
    consumer_thread.join().unwrap();
}