use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream, UdpSocket},
    thread,
};

fn main() {
    let socket = TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(127, 0, 0, 1),
        6664,
    )))
    .unwrap();
    loop {
        let out = socket.accept().unwrap();
        println!("NEW CONNECTION");
        thread::spawn(|| handler(out.0));
    }
}

fn handler(mut stream: TcpStream) {
    let mut read: [u8; 4096] = [0; 4096];
    loop {
        stream.read(&mut read).unwrap();
        stream.write(&mut read).unwrap();
    }
}
