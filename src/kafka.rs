extern crate kafka;

use kafka::kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use std::sync::mpsc::Sender;
use udp::IpSender;
use parsers::StreamParser;

struct KafkaListener {
    consumer: Consumer,
    value_parser: StreamParser,
    sender: IpSender,
}


impl KafkaListener {
    pub fn new(hosts: Vec<String>, topic: &str, group: &str,
               value_parser: StreamParser, sender: IpSender) -> KafkaListener {
        KafkaListener {
            consumer: Consumer::from_hosts(hosts)
                .with_topic_partitions(topic.to_owned(), &[0, 1])
                .with_fallback_offset(FetchOffset::Earliest)
                .with_group(group.to_owned())
                .with_offset_storage(GroupOffsetStorage::Kafka)
                .create()
                .unwrap(),
            value_parser,
            sender
        }
    }

    pub fn listen(&mut self) -> () {
        for ms in self.consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                let messages: Vec<[u8; 4]> = (self.value_parser)(m.value).unwrap();
                self.sender.send(messages).unwrap();
            }
        }
    }
}
