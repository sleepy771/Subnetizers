use formatters::{AggFormatter, simple_formatter};
use listeners::{IpSender, Listener, listener_factory, ListenerCredentials};
use listeners::kafka::KafkaListener;
use listeners::udp::UdpServer;
use parsers::{simple_parser, StreamParser, nom_ip_parser};
use SETTINGS;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use subnet_tree::{IPTree, OctetNode};
use senders::{Publisher, create_publisher, PublisherCredentials};

pub struct IpAggregator {
    handles: Vec<JoinHandle<()>>,
    show_stopper: Arc<Mutex<bool>>,
}


impl IpAggregator {
    pub fn new() -> IpAggregator {
        IpAggregator {
            handles: Vec::new(),
            show_stopper: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&mut self) -> () {
        let (octet_tx, octet_rx) = channel();
        let timer_tx = octet_tx.clone();
        let (cidr_tx, cidr_rx) = channel();
        self.start_listener_thread(octet_tx);
        self.start_tree_event_listener(octet_rx, cidr_tx);
        self.start_dump_timer(timer_tx);
        self.start_push_result_thread(cidr_rx);

        while !self.handles.is_empty() {
            let handle = self.handles.pop().unwrap();
            handle.join().unwrap();
        }
    }

    fn start_listener_thread(&mut self, sender: Sender<AggEvent>) {
        let bind_addr: String = SETTINGS.get_udp_bind_address().unwrap().clone();
        self.handles.push(thread::spawn(move || {
            match listener_factory(ListenerCredentials::UdpServer(bind_addr), nom_ip_parser, sender) {
                Ok(ref mut listener) => {
                    match listener.listen() {
                        Err(e) => {
                            error!("Listener stopped listening; Cause: {}", e);
                            panic!();
                        },
                        _ => {}
                    }
                },
                Err(e) => {
                    error!("Could not create listener; Cause: {}", e);
                    panic!();
                }
            }
        }));
    }

    fn start_tree_event_listener(&mut self, receiver: Receiver<AggEvent>, sender: Sender<Vec<(u32, u8)>>) {
        self.handles.push(thread::spawn(move || {
            let mut tree = IPTree::new();
            loop {
                match receiver.recv() {
                    Ok(event) => {
                        match event {
                            AggEvent::ADD(data) => {
                                data.into_iter().for_each(|octet| {tree.add(&octet)});
                            },
                            AggEvent::DUMP => {
                                let mut ip_tree_iter = tree.walk();
                                loop {
                                    let ipvec: Vec<(u32, u8)> = (&mut ip_tree_iter).take(1000).collect();
                                    let vec_len = ipvec.len();
                                    sender.send(ipvec).unwrap();
                                    if vec_len < 1000 {
                                        break;
                                    }
                                }
                            },
                            AggEvent::TERMINATE => {
                                drop(sender);
                                break;
                            }
                        }
                    },
                    Err(e) => {
                        error!("IPTree thread panicked: {}", e);
                        panic!();
                    }
                }
            }
        }));
    }

    fn start_dump_timer(&mut self, sender: Sender<AggEvent>) {
        let sleep_dur = Duration::from_secs(SETTINGS.get_publish_timer() as u64);
        self.handles.push(thread::spawn(move || {
            loop {
                thread::sleep(sleep_dur);
                sender.send(AggEvent::DUMP).unwrap();
            }
        }));
    }

    fn start_push_result_thread(&mut self, receiver: Receiver<Vec<(u32, u8)>>) {
        let send_to = SETTINGS.get_udp_send_to().unwrap();
        let creds = PublisherCredentials::Udp(send_to.to_owned());
        self.handles.push(thread::spawn(move || {
            let mut sender = create_publisher(creds, simple_formatter, receiver).unwrap();
            sender.run_sender();
        }));
    }

    pub fn stop(&mut self) {
        let stop_mutex = Arc::clone(&self.show_stopper);
        let mut stop = stop_mutex.lock().unwrap();
        *stop = true;
    }
}

pub enum AggEvent {
    ADD(Vec<[u8;4]>),
    DUMP,
    TERMINATE,
}
