use formatters::AggFormatter;
use std::net::Ipv4Addr;
use std::net::UdpSocket;
use std::str::FromStr;
use std::sync::mpsc::Receiver;

trait Sender {
    fn run_sender(&mut self);
}


pub struct UdpSender {
    socket: UdpSocket,
    receiver: Receiver<Vec<(u32, u8)>>,
    formatter: AggFormatter,
    send_to: String,
}


impl UdpSender {
    pub fn new(send_to: &str, formatter: AggFormatter, receiver: Receiver<Vec<(u32, u8)>>) -> UdpSender {
        match UdpSocket::bind("127.0.0.1:43211") {
            Ok(socket) => {
                UdpSender {
                    socket,
                    receiver,
                    formatter,
                    send_to: send_to.to_string(),
                }
            }
            Err(reason) => panic!("Can not bind sender to address: {}", reason)
        }
    }

    pub fn run_sender(&self) {
        loop {
            match self.receiver.recv() {
                Ok(cidr_vec) => {
                    if cidr_vec == vec![(0, 32)] {
                        break;
                    }
                    for ip_string in (self.formatter)(cidr_vec) {
                        self.socket.send_to(ip_string.as_bytes(), self.send_to.as_str()).unwrap();
                    }
                }
                Err(reason) => panic!("Receiver stopped working: {}", reason)
            }
        };
    }
}

pub mod kafka {
    use super::*;
    use kafka::producer::{Producer, Record, RequiredAcks};
    use std::time::Duration;

    pub struct KafkaProducer {
        producer: Producer,
        formatter: AggFormatter,
        receiver: Receiver<Vec<(u32, u8)>>,
        topic: String,
    }

    impl KafkaProducer {
        pub fn new(hosts: Vec<String>, ack_timeout: Duration, topic: String, formatter: AggFormatter, receiver: Receiver<Vec<(u32, u8)>>)
            -> KafkaProducer {
            KafkaProducer {
                producer: Producer::from_hosts(hosts).with_ack_timeout(ack_timeout).with_required_acks(RequiredAcks::One).create().unwrap(),
                formatter,
                receiver,
                topic,
            }
        }
    }

    impl Sender for KafkaProducer {
        fn run_sender(&mut self) {
            loop {
                match self.receiver.recv() {
                    Ok(ip_vec) => {
                        for ip_string in (self.formatter)(ip_vec) {
                            self.producer.send(&Record::from_value(self.topic.as_ref(), ip_string.as_bytes())).unwrap();
                        }
                    }
                    Err(e) => panic!("Something happend: {}", e)
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
    fn test_UdpSender_run_sender() {
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

                Err(e) => panic!("Error occured during receive udp datagram: {}", e)
            }
        }));

        // blocking until udp socket is not bound
        udp_listener_rx.recv().unwrap();

        let (mut tx, mut rx) = channel();

        handles.push(thread::spawn(move || {
            let sender = UdpSender::new("127.0.0.1:13345", simple_formatter, rx);
            sender.run_sender();
        }));

        tx.send(data).unwrap();
        tx.send(vec![(0_u32, 32_u8)]).unwrap();
        let recv_data = udp_listener_rx.recv().unwrap();

        assert_eq!("192.168.2.1/32 172.16.100.1/24 10.10.1.1/16".to_string(), recv_data);

        drop(tx);
        drop(udp_listener_rx);


        for handle in handles {
            handle.join().unwrap();
        }
    }
}
