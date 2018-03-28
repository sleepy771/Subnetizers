use parsers::StreamParser;
use std::sync::mpsc::Sender;
use ipagg::AggEvent;

pub type IpSender = Sender<AggEvent>;

pub trait Listener {
    fn listen(&mut self);
}


pub enum ListenerCredentials {
    Kafka(Vec<String>, String, String),
    UdpServer(String),
}

pub fn listener_factory(creds: ListenerCredentials, parser: StreamParser, sender: IpSender)
                        -> Box<Listener + 'static>
{
    match creds {
        ListenerCredentials::Kafka(hosts, topic, group) => {
            Box::new(kafka::KafkaListener::new(hosts, topic, group, parser, sender))
        }
        ListenerCredentials::UdpServer(host) => {
            Box::new(udp::UdpServer::new(host.as_str(), parser, sender).unwrap())
        }
    }
}

pub mod kafka {
    use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
    use std::sync::mpsc::Sender;
    use super::{IpSender, Listener, StreamParser, AggEvent};

    pub struct KafkaListener {
        consumer: Consumer,
        value_parser: StreamParser,
        sender: IpSender,
    }


    impl KafkaListener {
        pub fn new(hosts: Vec<String>, topic: String, group: String,
                   value_parser: StreamParser, sender: IpSender) -> KafkaListener {
            KafkaListener {
                consumer: Consumer::from_hosts(hosts)
                    .with_topic_partitions(topic, &[0, 1])
                    .with_fallback_offset(FetchOffset::Earliest)
                    .with_group(group)
                    .with_offset_storage(GroupOffsetStorage::Kafka)
                    .create()
                    .unwrap(),
                value_parser,
                sender,
            }
        }
    }

    impl Listener for KafkaListener {
        fn listen(&mut self) -> () {
            for ms in self.consumer.poll().unwrap().iter() {
                for m in ms.messages() {
                    let messages: Vec<[u8; 4]> = (self.value_parser)(m.value).unwrap();
                    self.sender.send(AggEvent::ADD(messages)).unwrap();
                }
            }
        }
    }
}

pub mod udp {
    use std::net::UdpSocket;
    use std::sync::mpsc::Sender;
    use super::{IpSender, Listener, StreamParser, AggEvent};

    pub struct UdpServer {
        socket: UdpSocket,
        sender: IpSender,
        parser: StreamParser,
    }


    impl UdpServer {
        pub fn new(address: &str, parser: StreamParser, sender: IpSender) -> Result<UdpServer, String> {
            match UdpSocket::bind(address) {
                Ok(socket) => {
                    Ok(UdpServer { socket, sender, parser })
                }
                Err(err) => Err(format!("Can not start UdpServer, reason: {}", err))
            }
        }

        pub fn shutdown(&self) -> () {
            self.socket.send("STOP!".as_bytes()).unwrap();
        }
    }

    impl Listener for UdpServer {
        fn listen(&mut self) {
            let mut buffer: [u8; 2048] = [0; 2048];

            loop {
                match self.socket.recv_from(&mut buffer) {
                    Ok((size, addr)) => {
                        if &buffer[0..size] == "STOP!".as_bytes() {
                            break;
                        }
                        let data = (self.parser)(&buffer[0..size]).unwrap();
                        self.sender.send(AggEvent::ADD(data)).unwrap();
                    }
                    Err(err) => {
                        panic!("UdpServer stoped working due to: {}", err);
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use parsers::simple_parser;
        use super::*;

        #[test]
        fn test_UdpServer_listener() {
            use std::thread;
            use std::sync::mpsc::channel;

            let (tx, rx) = channel();
            let (lock_tx, lock_rx) = channel();

            let mut handles = Vec::new();

            handles.push(thread::spawn(move || {
                let mut serv = UdpServer::new("127.0.0.1:12345", simple_parser, tx::send).unwrap();
                lock_tx.send("".to_owned()).unwrap();
                serv.listen();
            }));

            lock_rx.recv().unwrap();

            handles.push(thread::spawn(move || {
                let mut socket = UdpSocket::bind("127.0.0.1:12341").unwrap();
                let addresses = b"192.168.1.1 127.0.0.1 172.16.100.10";
                socket.send_to(addresses, "127.0.0.1:12345").unwrap();
                socket.send_to(b"STOP!", "127.0.0.1:12345");
                drop(socket);
            }));

            let data: Vec<[u8; 4]> = rx.recv().unwrap();

            for handle in handles {
                handle.join().unwrap();
            }
            assert_eq!(vec![[192, 168, 1, 1], [127, 0, 0, 1], [172, 16, 100, 10]], data);
        }
    }
}