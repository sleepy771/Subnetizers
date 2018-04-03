use formatters::AggFormatter;
use std::net::UdpSocket;
use std::sync::mpsc::Receiver;
use std::time::Duration;

pub enum PublisherCredentials {
    Udp(String),
    Kafka(Vec<String>, Duration, String),
}


pub fn create_publisher(credentials: PublisherCredentials,
                        formatter: AggFormatter,
                        receiver: Receiver<Vec<(u32, u8)>>)
                        -> Result<Box<Publisher + 'static>, String> {
    match credentials {
        PublisherCredentials::Udp(host) => {
            match UdpSender::new(host.as_ref(), formatter, receiver) {
                Ok(sender) => Ok(Box::new(sender)),
                Err(e) => Err(e)
            }
        }
        PublisherCredentials::Kafka(hosts, ack_timeout, topic) => {
            match kafka::KafkaProducer::new(hosts, ack_timeout, topic, formatter, receiver) {
                Ok(publisher) => Ok(Box::new(publisher)),
                Err(e) => Err(e)
            }
        }
    }
}

pub trait Publisher {
    fn run_sender(&mut self);
}


pub struct UdpSender {
    socket: UdpSocket,
    receiver: Receiver<Vec<(u32, u8)>>,
    formatter: AggFormatter,
    send_to: String,
}


impl UdpSender {
    pub fn new(send_to: &str, formatter: AggFormatter, receiver: Receiver<Vec<(u32, u8)>>) -> Result<UdpSender, String> {
        match UdpSocket::bind("127.0.0.1:43211") {
            Ok(socket) => {
                Ok(UdpSender { socket, receiver, formatter, send_to: send_to.to_string() })
            }
            Err(reason) => Err(format!("Can not bind sender to address: {}", reason))
        }
    }
}

impl Publisher for UdpSender {
    fn run_sender(&mut self) {
        loop {
            match self.receiver.recv() {
                Ok(cidr_vec) => {
                    if cidr_vec == vec![(0, 33)] {
                        break;
                    }
                    for ip_string in (self.formatter)(cidr_vec) {
                        self.socket.send_to(ip_string.as_bytes(), self.send_to.as_str()).unwrap();
                    }
                }
                Err(reason) => panic!("UdpSender::run_sender panicked; Cause: {}", reason)
            }
        };
    }
}

pub mod kafka {
    use super::*;
    use kafka::producer::{Producer, Record, RequiredAcks};

    pub struct KafkaProducer {
        producer: Producer,
        formatter: AggFormatter,
        receiver: Receiver<Vec<(u32, u8)>>,
        topic: String,
    }

    impl KafkaProducer {
        pub fn new(hosts: Vec<String>, ack_timeout: Duration, topic: String, formatter: AggFormatter, receiver: Receiver<Vec<(u32, u8)>>)
                   -> Result<KafkaProducer, String> {
            match Producer::from_hosts(hosts).with_ack_timeout(ack_timeout).with_required_acks(RequiredAcks::One).create() {
                Ok(producer) => {
                    Ok(KafkaProducer { producer, formatter, receiver, topic })
                }
                Err(e) => Err(format!("Creation of new KafkaProducer failed; Reason: {}", e))
            }
        }
    }

    impl Publisher for KafkaProducer {
        fn run_sender(&mut self) {
            loop {
                match self.receiver.recv() {
                    Ok(ip_vec) => {
                        for ip_string in (self.formatter)(ip_vec) {
                            self.producer.send(&Record::from_value(self.topic.as_ref(), ip_string.as_bytes())).unwrap();
                        }
                    }
                    Err(e) => panic!("KafkaProducer::run_sender panicked; Cause: {}", e)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    fn make_prefix(octets: [u8; 4]) -> u32 {
        (octets[0] as u32) << 24 | (octets[1] as u32) << 16 | (octets[2] as u32) << 8 | octets[3] as u32
    }

    #[test]
    fn test_udp_sender_run_sender() {
        use std::thread;
        use std::sync::mpsc::channel;
        use std::string::ToString;
        use formatters::simple_formatter;

        let data = vec![
            (make_prefix([192, 168, 2, 1]), 32),
            (make_prefix([172, 16, 100, 1]), 24),
            (make_prefix([10, 10, 1, 1]), 16)];

        let mut handles = Vec::new();

        let (mut udp_listener_tx, mut udp_listener_rx) = channel();
        handles.push(thread::spawn(move || {
            let mut socket = UdpSocket::bind("127.0.0.1:13345").unwrap();

            // bind lock
            udp_listener_tx.send("".to_owned()).unwrap();

            let mut buffer: [u8; 2048] = [0; 2048];

            match socket.recv_from(&mut buffer) {
                Ok((len, addr)) => {
                    udp_listener_tx.send(str::from_utf8(&buffer[0..len]).unwrap().to_string()).unwrap();
                }

                Err(e) => panic!("Error occurred during receive udp datagram: {}", e)
            }
        }));

        // blocking until udp socket is not bound
        udp_listener_rx.recv().unwrap();

        let (mut tx, mut rx) = channel();

        handles.push(thread::spawn(move || {
            let mut sender = UdpSender::new("127.0.0.1:13345", simple_formatter, rx).unwrap();
            sender.run_sender();
        }));

        tx.send(data).unwrap();
        tx.send(vec![(0_u32, 33_u8)]).unwrap();
        let recv_data = udp_listener_rx.recv().unwrap();

        assert_eq!("192.168.2.1/32 172.16.100.1/24 10.10.1.1/16".to_string(), recv_data);

        drop(tx);
        drop(udp_listener_rx);


        for handle in handles {
            handle.join().unwrap();
        }
    }
}
