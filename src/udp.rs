use std::net::UdpSocket;
use std::sync::mpsc::{Sender, Receiver};
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
                Err(err) => {
                    panic!("UdpServer stoped working due to: {}", err);
                }
            }
        }
    }

    pub fn shutdown(&self) -> () {
        self.socket.send("STOP!".as_bytes()).unwrap();
    }
}

type AggFormatter = fn(Vec<String>) -> Vec<String>;

struct UdpSender {
    socket: UdpSocket,
    receiver: Receiver<Vec<String>>,
    formatter: AggFormatter,
    send_to: String,
}


impl UdpSender {
    pub fn new(send_to: &str, formatter: AggFormatter, receiver: Receiver<Vec<String>>) -> UdpSender {
        match UdpSocket::bind("127.0.0.1:43211") {
            Ok(socket) => {
                UdpSender {
                    socket: socket,
                    receiver: receiver,
                    formatter: formatter,
                    send_to: send_to.to_string(),
                }
            }
            Err(reason) => panic!("Can not bind sender to address: {}", reason)
        }
    }

    pub fn run_sender(&self) {
        loop {
            match self.receiver.recv() {
                Ok(ip_vec) => {
                    for ip_string in (self.formatter)(ip_vec) {
                        self.socket.send_to(ip_string.as_bytes(), self.send_to.as_str()).unwrap();
                    }
                },
                Err(reason) => panic!("Receiver stopped working!")
            }
        };
    }
}

pub fn simpl_formatter(cidrs: Vec<String>) -> Vec<String> {
    let mut from: usize = 0;
    let mut concated_msg: Vec<String> = Vec::new();
    while from < cidrs.len() - 1 {
        let (msg, idx) = _concat_to_size(&cidrs[from .. ], 508);
        from += idx;
        concated_msg.push(msg);
    }
    concated_msg
}

fn _concat_to_size(strings: &[String], max_size: usize) -> (String, usize) {
    let mut tmp_size: usize = 0;
    let mut chunk_last_idx: usize = 0;
    let mut use_entire_slice = true;
    for (idx, str_) in strings.iter().enumerate() {
        chunk_last_idx = idx;
        if tmp_size > 0 {
            tmp_size += 1;
        }
        if str_.len() + tmp_size > max_size {
            use_entire_slice = false;
            break;
        }
        tmp_size += str_.len();
    }
    if use_entire_slice {
        (strings.join(" "), strings.len())
    } else {
        (strings[..chunk_last_idx].join(" "), chunk_last_idx)
    }
}

pub fn simpl_parser(bytes: &[u8]) -> Result<Vec<[u8; 4]>, String> {
    let mut from: i64 = -1;
    let mut ip_vec: Vec<[u8; 4]> = Vec::new();
    for (i, &byte) in bytes.iter().enumerate() {
        if byte != b' ' && from < 0 {
            println!("Found non space");
            from = i as i64;
        } else if byte == b' ' && from >= 0 {
            ip_vec.push(parse_ip(&bytes[from as usize .. i]));
            from = -1;
        }
        
    }
    if from >= 0 {
        ip_vec.push(parse_ip(&bytes[from as usize ..]));
    }
    Ok(ip_vec)
}

fn parse_ip(address_str: &[u8]) -> [u8; 4] {
    Ipv4Addr::from_str(str::from_utf8(address_str).unwrap()).unwrap().octets()    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ip() {
        assert_eq!([192, 168, 1, 1], parse_ip(b"192.168.1.1"));
        assert_eq!([127, 0, 0, 1], parse_ip(b"127.0.0.1"));
    }

    #[test]
    fn test_simpl_parser() {
        let ips = b" 127.0.0.1   192.168.1.1";
        assert_eq!(Ok(vec![[127, 0, 0, 1], [192, 168, 1, 1]]), simpl_parser(ips));
    }

    #[test]
    fn test__concat_to_size() {
        let v = vec!["A".to_string(), "B".to_string(), "C".to_string(), "D".to_string()];
        assert_eq!(("A B".to_string(), 2), _concat_to_size(&v, 3));
        assert_eq!(("A B".to_string(), 2), _concat_to_size(&v, 4));
        assert_eq!(("A B C".to_string(), 3), _concat_to_size(&v, 5));
        assert_eq!(("A B C D".to_string(), 4), _concat_to_size(&v, 20));
    }
}
