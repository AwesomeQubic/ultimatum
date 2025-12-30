use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}};

#[tokio::main]
async fn main() {
    let socket = TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(127, 0, 0, 1),
        6664,
    ))).await.unwrap();
    loop {
        let out = socket.accept().await.unwrap();
        tokio::spawn(handler(out.0));
    }
}

async fn handler(mut stream: TcpStream) {
    let mut read: [u8; 4096] = [0; 4096];
    loop {
        stream.read(&mut read).await.unwrap();
        stream.write(&mut read).await.unwrap();
    }
}