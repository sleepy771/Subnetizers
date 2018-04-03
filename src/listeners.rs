use parsers::StreamParser;
use std::sync::mpsc::Sender;
use ipagg::AggEvent;
use config::Settings;

pub type IpSender = Sender<AggEvent>;

pub trait Listener {
    fn listen(&mut self) -> Result<(), String>;
}


pub enum ListenerCredentials {
    Kafka(Vec<String>, String, String),
    UdpServer(String),
}

pub fn get_credentials_from_settings(settings: &Settings) -> Result<ListenerCredentials, String> {
    match settings.get_receiver_type() {
        "udp" => {
            match settings.get_udp_bind_address() {
                Some(ref address) => Ok(ListenerCredentials::UdpServer(address.to_owned())),
                None => Err("Expected udp receiver, but no address to bind specified".to_owned())
            }
        }
        "kafka" => {
            match settings.get_kafka_receiver_credentials() {
                Some(ref kafka) => Ok(ListenerCredentials::Kafka(kafka.get_hosts(),
                                                                 kafka.get_topic(),
                                                                 kafka.get_group())),
                None => Err("Expected kafka receiver, but no kafka settings specified".to_owned())
            }
        }
        receiver => Err(format!("Unknown receiver type `{}` specified!", receiver))
    }
}

pub fn listener_factory(creds: ListenerCredentials, parser: StreamParser, sender: IpSender)
                        -> Result<Box<Listener + 'static>, String>
{
    match creds {
        ListenerCredentials::Kafka(hosts, topic, group) => {
            match kafka::KafkaListener::new(hosts, topic, group, parser, sender) {
                Ok(listener) => Ok(Box::new(listener)),
                Err(e) => Err(e)
            }
        }
        ListenerCredentials::UdpServer(host) => {
            match udp::UdpServer::new(host.as_str(), parser, sender) {
                Ok(listener) => Ok(Box::new(listener)),
                Err(e) => Err(e)
            }
        }
    }
}

pub mod kafka {
    use kafka::consumer::{Consumer, GroupOffsetStorage};
    use super::{IpSender, Listener, StreamParser, AggEvent};

    pub struct KafkaListener {
        consumer: Consumer,
        value_parser: StreamParser,
        sender: IpSender,
    }


    impl KafkaListener {
        pub fn new(hosts: Vec<String>, topic: String, group: String,
                   value_parser: StreamParser, sender: IpSender) -> Result<KafkaListener, String> {
            match Consumer::from_hosts(hosts)
                .with_topic_partitions(topic, &[0, 1])
                .with_group(group)
                .with_offset_storage(GroupOffsetStorage::Kafka)
                .create() {
                Ok(consumer) => Ok(KafkaListener {consumer, value_parser, sender}),
                Err(e) => Err(format!("Kafka couldn't create consumer; Cause: {}", e))
            }
        }
    }

    impl Listener for KafkaListener {
        fn listen(&mut self) -> Result<(), String> {
            for ms in self.consumer.poll().unwrap().iter() {
                for m in ms.messages() {
                    let messages: Vec<[u8; 4]> = match (self.value_parser)(m.value) {
                        Ok(msg_vec) => msg_vec,
                        Err(e) => {
                            warn!("Parsing of message `{:?}` failed; Cause: {}. Skipping ...", m.value, e);
                            continue;
                        }
                    };
                    match self.sender.send(AggEvent::ADD(messages)) {
                        Ok(()) => {},
                        Err(e) => {
                            return Err(format!("Can not send Aggregator event via event queue; Cause: {}", e))
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

pub mod udp {
    use std::net::UdpSocket;
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
                Err(err) => Err(format!("Can not start UdpServer; Cause: {}", err))
            }
        }

        pub fn shutdown(&self) -> () {
            self.socket.send("STOP!".as_bytes()).unwrap();
        }
    }

    impl Listener for UdpServer {
        fn listen(&mut self) -> Result<(), String> {
            let mut buffer: [u8; 2048] = [0; 2048];

            loop {
                match self.socket.recv_from(&mut buffer) {
                    Ok((size, _)) => {
                        if &buffer[0..size] == "STOP!".as_bytes() {
                            return Ok(())
                        }
                        let data = match (self.parser)(&buffer[0..size]) {
                            Ok(ips) => ips,
                            Err(e) => {
                                warn!("Parsing of message {:?} failed. Skipping ...; Cause: {}", &buffer[0..size], e);
                                continue;
                            }
                        };
                        match self.sender.send(AggEvent::ADD(data)) {
                            Err(e) => return Err(format!("Can not send Aggregator event via event queue; Cause: {}", e)),
                            _ => {}
                        }
                    }
                    Err(err) => {
                        return Err(format!("UdpServer stopped working due to: {}", err));
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
        fn test_udp_server_listener() {
            use std::thread;
            use std::sync::mpsc::channel;

            let (tx, rx) = channel();
            let (lock_tx, lock_rx) = channel();

            let mut handles = Vec::new();

            handles.push(thread::spawn(move || {
                let mut serv = UdpServer::new("127.0.0.1:12345", simple_parser, tx).unwrap();
                lock_tx.send("".to_owned()).unwrap();
                serv.listen().unwrap();
            }));

            lock_rx.recv().unwrap();

            handles.push(thread::spawn(move || {
                let mut socket = UdpSocket::bind("127.0.0.1:12341").unwrap();
                let addresses = b"192.168.1.1 127.0.0.1 172.16.100.10";
                socket.send_to(addresses, "127.0.0.1:12345").unwrap();
                socket.send_to(b"STOP!", "127.0.0.1:12345");
                drop(socket);
            }));

            let data: Vec<[u8; 4]> = match rx.recv().unwrap() {
                AggEvent::ADD(data) => data,
                _ => panic!("This shouldn't happened!")
            };

            for handle in handles {
                handle.join().unwrap();
            }
            assert_eq!(vec![[192, 168, 1, 1], [127, 0, 0, 1], [172, 16, 100, 10]], data);
        }
    }
}