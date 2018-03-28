use serde_yaml;
use std::fs::File;
use std::path::Path;

const SETTINGS_FILE_NAME: &'static str = "settings.yaml";

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct Receivers {
    #[serde(default = "default_receiver")]
    receiver: String,
    #[serde(default = "default_udp_receiver")]
    udp_address: Option<String>,
    kafka: Option<KafkaReceiver>
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct KafkaReceiver {
    hosts: Vec<String>,
    topic: String,
    group: String
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct Senders {
    #[serde(default = "default_sender")]
    sender: String,
    #[serde(default = "default_udp_sender")]
    udp_address: Option<String>,
    kafka: Option<KafkaSender>
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone)]
pub struct KafkaSender {
    hosts: Vec<String>,
    topic: String,
    #[serde(default = "default_ack_duration")]
    ack_duration_seconds: u64
}

#[derive(Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Settings {
    receiver: Receivers,
    sender: Senders,
    #[serde(default = "thirty_seconds")]
    publish_timer: u32,

    #[serde(default = "default_add_broadcast")]
    auto_add_broadcast: bool,
    #[serde(default = "default_add_zeroed")]
    auto_add_zeroed: bool,
}

impl Settings {
    pub fn defualt() -> Settings {
        Settings {
            receiver: Receivers {
                receiver: default_receiver(),
                udp_address: default_udp_receiver(),
                kafka: None
            },
            sender: Senders {
                sender: default_sender(),
                udp_address: default_udp_sender(),
                kafka: None
            },
            publish_timer: thirty_seconds(),
            auto_add_zeroed: default_add_zeroed(),
            auto_add_broadcast: default_add_broadcast(),
        }
    }

    pub fn is_last_node_with_settings(&self) -> bool {
        self.auto_add_broadcast || self.auto_add_zeroed
    }

    pub fn add_zeroed(&self) -> bool {
        self.auto_add_zeroed
    }

    pub fn add_broadcast(&self) -> bool {
        self.auto_add_broadcast
    }

    pub fn get_udp_bind_address(&self) -> Option<&String> {
        self.receiver.udp_address.as_ref()
    }

    pub fn get_udp_send_to(&self) -> Option<&String> {
        self.sender.udp_address.as_ref()
    }

    pub fn get_publish_timer(&self) -> u32 {
        self.publish_timer
    }
}

pub fn load_from_default_location(root: &Path) -> Result<Settings, String> {
    load_from_file(&root.join(&Path::new(SETTINGS_FILE_NAME)))
}

pub fn load_from_file(path: &Path) -> Result<Settings, String> {
    println!("Attempted load of settings from `{}`", path.display());
    if !path.exists() {
        return Err(format!("File `{}` does not exist!", path.display()));
    }

    match File::open(&path) {
        Err(reason) => {
            Err(format!("File `{}` can not be open: {}", path.display(), reason))
        }
        Ok(file) => {
            match serde_yaml::from_reader(file) {
                Err(reason) => Err(format!("Parsing settings file `{}` failed: {}", path.display(), reason)),
                Ok(settings) => Ok(settings)
            }
        }
    }
}

pub fn settings_path_exists(root: &Path) -> bool {
    root.join(&Path::new(SETTINGS_FILE_NAME)).exists()
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

fn thirty_seconds() -> u32 {
    30
}

fn default_sender() -> String {
    "udp".to_owned()
}

fn default_receiver() -> String {
    "udp".to_owned()
}

fn default_ack_duration() -> u64 {
    1_u64
}
