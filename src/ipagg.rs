use subnet_tree::{IPTree, OctetNode};
use udp::{UdpSender, simpl_formatter};
use listeners::udp::UdpServer;
use listeners::kafka::KafkaListener;
use listeners::{Listener, IpSender, listener_factory, ListenerCreds};
use parsers::{simpl_parser, StreamParser};
use std::thread::JoinHandle;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::time::Duration;
use SETTINGS;

pub struct IpAggregator {
    handles: Vec<JoinHandle<()>>,
    tree: Arc<Mutex<IPTree>>,
    show_stopper: Arc<Mutex<bool>>
}


impl IpAggregator {
    pub fn new() -> IpAggregator {
        IpAggregator {
            handles: Vec::new(),
            tree: Arc::new(Mutex::new(IPTree::new())),
            show_stopper: Arc::new(Mutex::new(false))
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

    fn _start_listener_thread(&mut self, sender: Sender<Vec<[u8;4]>>) {
        let bind_addr: String = SETTINGS.get_udp_bind_address().unwrap().clone();
        self.handles.push(thread::spawn(move || {
            let mut listener = listener_factory(ListenerCreds::UdpServer(bind_addr), simpl_parser, sender);
            listener.listen();
        }));
    }

    fn _start_tree_updater_thread(&mut self, receiver: Receiver<Vec<[u8; 4]>>) {
        let tree_ref_mutex = Arc::clone(&self.tree);
        let stoper_mutex = Arc::clone(&self.show_stopper);
        self.handles.push(thread::spawn(move || {
            loop {
                let stop: bool = {
                    *stoper_mutex.lock().unwrap()
                };
                if stop {
                    break;
                }
                match receiver.recv() {
                    Ok(data) => {
                        let mut tree_ref = tree_ref_mutex.lock().unwrap();
                        for ip_address in data {
                            (*tree_ref).add(&ip_address);
                        }
                    },
                    Err(e) => panic!("IpTree updater thread paniced: {}", e)
                }
            }
        }));
    }

    fn _start_tree_lister_thread(&mut self, sender: Sender<Vec<String>>) {
        let tree_ref_mutex = Arc::clone(&self.tree);
        let stoper_mutex = Arc::clone(&self.show_stopper);
        let sleep_dur: Duration = Duration::from_secs(SETTINGS.get_publish_timer() as u64);
        self.handles.push(thread::spawn(move ||{
            loop {
                let stop: bool = {
                    *stoper_mutex.lock().unwrap()
                };
                if stop {
                    break;
                }
                thread::sleep(sleep_dur);
                let aggregations: Vec<String> = {
                    (*tree_ref_mutex.lock().unwrap()).list_cidr()
                };
                sender.send(aggregations).unwrap();
            }
        }));
    }

    fn _start_push_result_thread(&mut self, receiver: Receiver<Vec<String>>) {
        let send_to = SETTINGS.get_udp_sender_to().unwrap();
        self.handles.push(thread::spawn(move ||{
            let sender = UdpSender::new(send_to.as_str(), simpl_formatter, receiver);
            sender.run_sender();
        }));
    
    }

    pub fn stop(&mut self) {
        let stop_mutex = Arc::clone(&self.show_stopper);
        let mut stop = stop_mutex.lock().unwrap();
        *stop = true;
    }
}
