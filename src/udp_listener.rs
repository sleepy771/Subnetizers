use std::net::UdpSocket;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::str;
use std::str::FromStr;
use std::net::Ipv4Addr;


type StreamParser = fn(&[u8]) -> Result<Vec<[u8; 4]>, String>;


pub struct UdpServer {
    socket: UdpSocket,
    sender: Sender<Vec<[u8; 4]>>,
    parser: StreamParser
}


impl UdpServer {
    pub fn new(address: &str, parser: StreamParser, sender: Sender<Vec<[u8; 4]>>) -> Result<UdpServer, String> {
        match UdpSocket::bind(address) {
            Ok(socket) => {
                Ok(UdpServer { socket: socket, sender: sender, parser: parser })
            },
            Err(err) => Err(format!("Can not start UdpServer, reason: {}", err))
        }
    }

    pub fn listen(&self) {
        let mut buffer: [u8; 2048] = [0; 2048];

        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((size, addr)) => {
                    if &buffer[0 .. size] == "STOP!".as_bytes() {
                        println!("Stop listening");
                        break;
                    }
                    let data = (self.parser)(&buffer[0 .. size]).unwrap();
                    self.sender.send(data).unwrap();
                },
                Err(err) => panic!("UdpServer stoped working due to: {}", err)
            }
        }
    }

    pub fn shutdown(&self) -> () {
        self.socket.send("STOP!".as_bytes()).unwrap();
    }
}


pub fn simpl_parser(bytes: &[u8]) -> Result<Vec<[u8; 4]>, String> {
    let mut from: i64 = -1;
    let mut ip_vec: Vec<[u8; 4]> = Vec::new();
    for (i, &byte) in bytes.iter().enumerate() {
        if byte != (' ' as u8) && from < 0 {
            from = i as i64;
        } else if byte == (' ' as u8) && from > 0 {
            ip_vec.push(parse_ip(&bytes[from as usize .. i]));
            from = -1;
        }
        
    }
    Ok(ip_vec)
}

fn parse_ip(address_str: &[u8]) -> [u8; 4] {
    Ipv4Addr::from_str(str::from_utf8(address_str).unwrap()).unwrap().octets()    
}
