use std::path::Path;
use std::convert::AsRef;
use std::ffi::OsStr;
use std::fs::File;
use serde_yaml;

const DEFAUL_PATH: &'static str = ".ip_aggregator/settings.yaml";


#[derive(PartialEq,Eq,Serialize,Deserialize,Debug,Clone)]
pub struct UdpSettings {
    #[serde(default = "default_udp_receiver")]
    receiver_address: Option<String>,
    #[serde(default = "default_udp_sender")]
    send_to: Option<String>,
}

#[derive(Deserialize,PartialEq,Eq,Debug,Clone)]
pub struct Settings {
    udp: Option<UdpSettings>,
    #[serde(default = "thrity_seconds")]
    publish_timer: u32,

    #[serde(default = "default_add_broadcast")]
    auto_add_broadcast: bool,
    #[serde(default = "default_add_zeroed")]
    auto_add_zeroed: bool,
}

impl Settings {
    pub fn defualt() -> Settings {
        Settings {
            udp: Some(UdpSettings {
                receiver_address: default_udp_receiver(),
                send_to: default_udp_sender()
            }),
            publish_timer: thrity_seconds(),
            auto_add_zeroed: default_add_zeroed(),
            auto_add_broadcast: default_add_broadcast()
        }
    }

    pub fn is_using_udp(&self) -> bool {
        match self.udp {
            Some(_) => true,
            None => false
        }
    }

    pub fn is_last_node_with_settings(&self) -> bool {
        self.auto_add_broadcast || self.auto_add_zeroed
    }

    pub fn add_zeroed(&self) -> bool {
        self.auto_add_zeroed
    }

    pub fn add_boradcast(&self) -> bool {
        self.auto_add_broadcast
    }

    pub fn get_udp_bind_address(&self) -> Option<&String> {
        match self.udp {
            Some(ref udp_settings) => {
                match udp_settings.receiver_address {
                    Some(ref addr) => Some(addr),
                    None => None
                }
            },
            None => None
        }
    }

    pub fn get_udp_sender_to(&self) -> Option<&String> {
        match self.udp {
            Some(ref udp_settings) => {
                match udp_settings.send_to {
                    Some(ref addr) => Some(addr),
                    None => None
                }
            },
            None => None
        }
    }
    
    pub fn get_publish_timer(&self) -> u32 {
        self.publish_timer
    }
}

pub fn load_from_default_location(root: &Path) -> Result<Settings, String> {
    load_from_file(&root.join(&Path::new(DEFAUL_PATH)))
}

pub fn load_from_file(path: &Path) -> Result<Settings, String> {
    println!("Attempted load of settings from `{}`", path.display());
    if ! path.exists() {
        return Err(format!("File `{}` does not exist!", path.display()));
    }
    
    match File::open(&path) {
        Err(reason) => {
            Err(format!("File `{}` can not be open: {}", path.display(), reason))
        },
        Ok(file) => {
            match serde_yaml::from_reader(file) {
                Err(reason) => Err(format!("Parsing settings file `{}` failed: {}", path.display(), reason)),
                Ok(settings) => Ok(settings)
            }
        }
    }
}

pub fn settings_path_exists(root: &Path) -> bool {
    root.join(&Path::new(DEFAUL_PATH)).exists()
}

fn default_add_broadcast() -> bool {
    true
}

fn default_add_zeroed() -> bool {
    true
}

fn default_udp_receiver() -> Option<String> {
    Some("127.0.0.1:6788".to_string())
}

fn default_udp_sender() -> Option<String> {
    Some("127.0.0.1:6789".to_string())
}

fn thrity_seconds() -> u32 {
    30 * 60 * 1000
}
