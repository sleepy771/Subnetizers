use formatters::AggFormatter;
use std::net::Ipv4Addr;
use std::net::UdpSocket;
use std::str::FromStr;
use std::sync::mpsc::Receiver;

trait Sender {
    fn run_sender(&self) -> Result<(), String>;
}


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
                Ok(ip_vec) => {
                    if ip_vec == vec!["STOP!".to_string()] {
                        break;
                    }
                    for ip_string in (self.formatter)(ip_vec) {
                        self.socket.send_to(ip_string.as_bytes(), self.send_to.as_str()).unwrap();
                    }
                }
                Err(reason) => panic!("Receiver stopped working!")
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
        receiver: Receiver<Vec<String>>,
        topic: String,
    }

    impl KafkaProducer {
        pub fn new(hosts: Vec<String>, ack_timeout: Duration, topic: String, formatter: AggFormatter, receiver: Receiver<Vec<String>>)
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
        fn run_sender(&self) {
            loop {
                match self.receiver.recv() {
                    Ok(ip_vec) => {
                        if ip_vec == vec!["STOP!".to_string()] {
                            break;
                        }
                        for ip_string in (self.formatter)(ip_vec) {
                            self.producer.send(&Record::from_value(self.topic.as_ref(), ip_string.as_bytes())).unwrap();
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    #[test]
    fn test_UdpSender_run_sender() {
        use std::thread;
        use std::sync::mpsc::channel;
        use std::string::ToString;
        use formatters::simple_formatter;

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
                    udp_listener_tx.send(str::from_utf8(&buffer[0..len]).unwrap().to_string()).unwrap();
                }

                Err(e) => panic!("Error occured during receive udp datagram: {}", e)
            }
        }));

        thread::sleep_ms(3000);

        let (mut tx, mut rx) = channel();

        handles.push(thread::spawn(move || {
            let sender = UdpSender::new("127.0.0.1:13345", simple_formatter, rx);
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
