use formatters::{AggFormatter, simple_formatter};
use listeners::{IpSender, Listener, listener_factory, ListenerCredentials};
use listeners::kafka::KafkaListener;
use listeners::udp::UdpServer;
use parsers::{simple_parser, StreamParser};
use SETTINGS;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use subnet_tree::{IPTree, OctetNode};
use senders::UdpSender;

pub struct IpAggregator {
    handles: Vec<JoinHandle<()>>,
    tree: Arc<Mutex<IPTree>>,
    show_stopper: Arc<Mutex<bool>>,
}


impl IpAggregator {
    pub fn new() -> IpAggregator {
        IpAggregator {
            handles: Vec::new(),
            tree: Arc::new(Mutex::new(IPTree::new())),
            show_stopper: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&mut self) -> () {
        let (octet_tx, octet_rx) = channel();
        self._start_listener_thread(octet_tx);
        self._start_tree_updater_thread(octet_rx);
        let (cidr_tx, cidr_rx) = channel();
        self._start_tree_lister_thread(cidr_tx);
        self._start_push_result_thread(cidr_rx);

        while !self.handles.is_empty() {
            let handle = self.handles.pop().unwrap();
            handle.join().unwrap();
        }
    }

    fn _start_listener_thread(&mut self, sender: Sender<Vec<[u8; 4]>>) {
        let bind_addr: String = SETTINGS.get_udp_bind_address().unwrap().clone();
        self.handles.push(thread::spawn(move || {
            let mut listener = listener_factory(ListenerCredentials::UdpServer(bind_addr), simple_parser, sender);
            listener.listen();
        }));
    }

    fn _start_tree_updater_thread(&mut self, receiver: Receiver<Vec<[u8; 4]>>) {
        let tree_ref_mutex = Arc::clone(&self.tree);
        self.handles.push(thread::spawn(move || {
            loop {
                match receiver.recv() {
                    Ok(data) => {
                        let mut tree_ref = tree_ref_mutex.lock().unwrap();
                        data.into_iter().for_each(move |octet| { (*tree_ref).add(&octet) });
                    }
                    Err(e) => panic!("IpTree updater thread paniced: {}", e)
                }
            }
        }));
    }

    fn _start_tree_lister_thread(&mut self, sender: Sender<Vec<(u32, u8)>>) {
        let tree_ref_mutex = Arc::clone(&self.tree);
        let stoper_mutex = Arc::clone(&self.show_stopper);
        let sleep_dur: Duration = Duration::from_secs(SETTINGS.get_publish_timer() as u64);
        self.handles.push(thread::spawn(move || {
            loop {
                let stop: bool = {
                    *stoper_mutex.lock().unwrap()
                };
                if stop {
                    break;
                }
                thread::sleep(sleep_dur);
                {
                    let tree_ptr = (tree_ref_mutex.lock().unwrap());
                    let mut tree_iter = tree_ptr.walk();
                    loop {
                        let ipvec: Vec<(u32, u8)> = (&mut tree_iter).take(1000).collect();
                        let vec_len = ipvec.len();
                        sender.send(ipvec).unwrap();
                        if vec_len < 1000 {
                            break;
                        }
                    }
                };
            }
        }));
    }

    fn _start_push_result_thread(&mut self, receiver: Receiver<Vec<(u32, u8)>>) {
        let send_to = SETTINGS.get_udp_send_to().unwrap();
        self.handles.push(thread::spawn(move || {
            let sender = UdpSender::new(send_to.as_str(), simple_formatter, receiver);
            sender.run_sender();
        }));
    }

    pub fn stop(&mut self) {
        let stop_mutex = Arc::clone(&self.show_stopper);
        let mut stop = stop_mutex.lock().unwrap();
        *stop = true;
    }
}
