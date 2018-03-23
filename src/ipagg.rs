use subnet_tree::{IPTree, OctetNode};
use udp::{UdpServer, simpl_parser};
use std::thread::JoinHandle;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver};
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

    }

    fn _start_listener_thread(&mut self, sender: Sender<Vec<[u8;4]>>) {
        let bind_addr: &str = SETTINGS.get_udp_bind_address().unwrap().as_str();
        self.handles.push(thread::spawn(move || {
            let listener = UdpServer::new(bind_addr, simpl_parser, sender).unwrap();
            listener.listen();
        }));
    }

    fn _start_tree_updater_thread(&mut self, receiver: Receiver<Vec<[u8; 4]>>) {
        let mut tree_ref_mutex = Arc::clone(&self.tree);
        self.handles.push(thread::spawn(move || {
            loop {
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

    fn _start_push_result_thread(&mut self, receiver: Sender<Vec<String>>) {
        self.handles.push(thread::spawn(||{
            
        }));
    
    }
}
