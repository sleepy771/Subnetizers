use std::net::UdpSocket;
use std::sync::mpsc::{Sender, Receiver};
use std::str;
use std::str::FromStr;
use std::net::Ipv4Addr;
use parsers::{StreamParser, simpl_parser};


pub type IpSender = Sender<Vec<[u8; 4]>>;


pub struct UdpServer {
    socket: UdpSocket,
    sender: Sender<Vec<[u8; 4]>>,
    parser: StreamParser
}


impl UdpServer {
    pub fn new(address: &str, parser: StreamParser, sender: Sender<Vec<[u8; 4]>>) -> Result<UdpServer, String> {
        match UdpSocket::bind(address) {
            Ok(socket) => {
                Ok(UdpServer { socket, sender, parser })
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

pub struct UdpSender {
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
                    if ip_vec == vec!["STOP!".to_string()] {
                        break;
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test__concat_to_size() {
        let v = vec!["A".to_string(), "B".to_string(), "C".to_string(), "D".to_string()];
        assert_eq!(("A B".to_string(), 2), _concat_to_size(&v, 3));
        assert_eq!(("A B".to_string(), 2), _concat_to_size(&v, 4));
        assert_eq!(("A B C".to_string(), 3), _concat_to_size(&v, 5));
        assert_eq!(("A B C D".to_string(), 4), _concat_to_size(&v, 20));
    }

    #[test]
    fn test_UdpServer_listener() {
        use std::thread;
        use std::sync::mpsc::channel;

        let (tx, rx) = channel();

        let mut handles = Vec::new();

        handles.push(thread::spawn(move || {
            let serv = UdpServer::new("127.0.0.1:12345", simpl_parser, tx).unwrap();
            serv.listen();
        }));
        
        thread::sleep_ms(3000);

        handles.push(thread::spawn(move || {
            let mut socket = UdpSocket::bind("127.0.0.1:12341").unwrap();
            let addresses = b"192.168.1.1 127.0.0.1 172.16.100.10";
            socket.send_to(addresses, "127.0.0.1:12345").unwrap();
            thread::sleep_ms(2000);
            socket.send_to(b"STOP!", "127.0.0.1:12345");
            drop(socket);
        }));

        let data: Vec<[u8; 4]> = rx.recv().unwrap();
        
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(vec![[192, 168, 1, 1], [127, 0, 0, 1], [172, 16, 100, 10]], data);

    }

    #[test]
    fn test_UdpSender_run_sender() {
        use std::thread;
        use std::sync::mpsc::channel;
        use std::string::ToString;



        let data = vec![
            "192.168.2.1/32".to_string(),
            "172.16.100.1/24".to_string(),
            "10.10.1.1/16".to_string()];
        
        let mut handles = Vec::new();

        let (mut udp_listener_tx, mut udp_listener_rx) = channel();
        handles.push(thread::spawn(move || {
            let mut socket = UdpSocket::bind("127.0.0.1:13345").unwrap();
            
            let mut buffer: [u8; 2048] = [0; 2048];

            match socket.recv_from(&mut buffer) {
                Ok((len, addr)) => {
                    udp_listener_tx.send(str::from_utf8(&buffer[0 .. len]).unwrap().to_string()).unwrap();
                }

                Err(e) => panic!("Error occured during receive udp datagram: {}", e)
            }
        }));

        thread::sleep_ms(3000);

        let (mut tx, mut rx) = channel();

        handles.push(thread::spawn(move || {
            let sender = UdpSender::new("127.0.0.1:13345", simpl_formatter, rx);
            sender.run_sender();
        }));

        tx.send(data).unwrap();
        tx.send(vec!["STOP!".to_string()]);
        thread::sleep_ms(5000);
        drop(tx);

        let recv_data = udp_listener_rx.recv().unwrap();
        drop(udp_listener_rx);

        assert_eq!("192.168.2.1/32 172.16.100.1/24 10.10.1.1/16".to_string(), recv_data);

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
