use formatters::{simple_formatter};
use listeners::{listener_factory, get_credentials_from_settings};
use parsers::{nom_ip_parser};
use SETTINGS;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use subnet_tree::IPTree;
use senders::{create_publisher, get_publisher_credentials};

pub struct IpAggregator {
    handles: Vec<JoinHandle<()>>,
}


impl IpAggregator {
    pub fn new() -> IpAggregator {
        IpAggregator {
            handles: Vec::new(),
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
        self.handles.push(thread::spawn(move || {
            let credentials = match get_credentials_from_settings(&SETTINGS) {
                Ok(creds) => creds,
                Err(e) => {
                    error!("Could not get valid credentials; Cause: {}", e);
                    panic!();
                }
            };
            match listener_factory(credentials, nom_ip_parser, sender) {
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
        self.handles.push(thread::spawn(move || {
            let creds = match get_publisher_credentials(&SETTINGS) {
                Ok(creds) => creds,
                Err(e) => {
                    error!("Couldn't obtain publisher settings; Cause: {}", e);
                    panic!();
                }
            };
            let mut sender = create_publisher(creds, simple_formatter, receiver).unwrap();
            sender.run_sender();
        }));
    }
}

pub enum AggEvent {
    ADD(Vec<[u8;4]>),
    DUMP,
    TERMINATE,
}
