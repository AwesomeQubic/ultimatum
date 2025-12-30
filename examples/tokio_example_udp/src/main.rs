use std::{net::{Ipv4Addr, SocketAddr, SocketAddrV4}, sync::Arc, thread::available_parallelism};
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {
    let socket = UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(127, 0, 0, 1),
        6664,
    ))).await.unwrap();

    let socket = Arc::new(socket);
    let mut handles = Vec::new();

    for _ in 0..available_parallelism().map(|x| x.get()).unwrap_or(4) {
        handles.push(tokio::spawn(handler(socket.clone())));
    }
    
    for ele in handles {
        let _ = ele.await;
    }
}

async fn handler(socket: Arc<UdpSocket>) {
    let mut read: [u8; 4096] = [0; 4096];
    loop {
        let addr = socket.recv_from(&mut read).await.unwrap().1;
        socket.send_to(&mut read, addr).await.unwrap();
    }
}