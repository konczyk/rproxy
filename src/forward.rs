use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn forward(buf: &mut [u8], from: &mut TcpStream, to: &mut TcpStream) -> io::Result<usize> {
    from.read(buf).and_then(|bytes| {
        to.write_all(&buf[..bytes]).map(|_| bytes)
    })
}
