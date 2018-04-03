use serde_yaml;
use std::fs::File;
use std::path::Path;
use argparse::{ArgumentParser, StoreOption, Collect};
use log4rs::config::{Config, Appender, Root};
use log4rs::append::console::ConsoleAppender;
use log::LevelFilter;

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

impl KafkaReceiver {
    fn extend_kafka_hosts(&mut self, hosts: &[String]) {
        for host in hosts {
            self.hosts.push(host.to_owned());
        }
    }

    pub fn get_hosts(&self) -> Vec<String> {
        self.hosts.clone()
    }

    pub fn get_topic(&self) -> String {
        self.topic.clone()
    }

    pub fn get_group(&self) -> String {
        self.group.clone()
    }
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

impl KafkaSender {
    fn extend_kafka_hosts(&mut self, hosts: &[String]) {
        hosts.into_iter().for_each(|host| {self.hosts.push(host.to_owned())});
    }

    pub fn get_hosts(&self) -> Vec<String> {
        self.hosts.clone()
    }

    pub fn get_topic(&self) -> String {
        self.topic.clone()
    }

    pub fn get_ack_duration_seconds(&self) -> u64 {
        self.ack_duration_seconds
    }
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
    #[serde(default = "default_logger_settings_path")]
    log4rs_settings: Option<String>,
}

impl Settings {
    pub fn default() -> Settings {
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
            log4rs_settings: default_logger_settings_path()
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

    pub fn get_udp_bind_address(&self) -> Option<String> {
        self.receiver.udp_address.clone()
    }

    pub fn get_udp_send_to(&self) -> Option<String> {
        self.sender.udp_address.clone()
    }

    pub fn get_publish_timer(&self) -> u32 {
        self.publish_timer
    }

    pub fn get_logger_config(&self) -> Option<String> {
        self.log4rs_settings.clone()
    }

    pub fn get_receiver_type(&self) -> &str {
        self.receiver.receiver.as_ref()
    }

    pub fn get_publisher_type(&self) -> &str {
        self.sender.sender.as_ref()
    }

    pub fn get_kafka_receiver_credentials(&self) -> Option<KafkaReceiver> {
        self.receiver.kafka.clone()
    }

    pub fn override_settings(mut self, settings: OverrideSettings) -> Settings {
        if let Some(ref kafka_hosts) = settings.get_kafka_hosts() {
            match self.sender.kafka {
                Some(ref mut kafka) => {
                    kafka.extend_kafka_hosts(kafka_hosts);
                },
                None => {
                    self.sender.kafka = Some(KafkaSender {
                        hosts: kafka_hosts.iter().map(|host| {host.to_owned()}).collect(),
                        topic: String::new(),
                        ack_duration_seconds: default_ack_duration(),
                    })
                }
            }
            match self.receiver.kafka {
                Some(ref mut kafka) => {
                    kafka.extend_kafka_hosts(kafka_hosts);
                },
                None => {
                    self.receiver.kafka = Some(KafkaReceiver {
                        hosts: kafka_hosts.iter().map(|host| {host.to_owned()}).collect(),
                        topic: String::new(),
                        group: String::new(),
                    });
                }
            }
        }
        if let Some(ref kafka_inbound_topic) = settings.get_kafka_inbound_topic() {
            match self.receiver.kafka {
                Some(ref mut kafka) => {
                    kafka.topic = kafka_inbound_topic.to_owned();
                }
                None => {
                    self.receiver.kafka = Some(KafkaReceiver {
                        hosts: Vec::new(),
                        topic: kafka_inbound_topic.to_owned(),
                        group: String::new(),
                    });
                }
            }
        }
        if let Some(ref kafka_outbound_topic) = settings.get_kafka_outbound_topic() {
            match self.sender.kafka {
                Some(ref mut kafka) => kafka.topic = kafka_outbound_topic.to_owned(),
                None => {
                    self.sender.kafka = Some(KafkaSender {
                        hosts: Vec::new(),
                        topic: kafka_outbound_topic.to_owned(),
                        ack_duration_seconds: default_ack_duration(),
                    });
                }
            }
        }
        if let Some(ref kafka_group) = settings.get_kafka_group() {
            match self.receiver.kafka {
                Some(ref mut kafka) => kafka.group = kafka_group.to_owned(),
                None => {
                    self.receiver.kafka = Some(KafkaReceiver {
                        hosts: Vec::new(),
                        topic: String::new(),
                        group: kafka_group.to_owned(),
                    });
                }
            }
        }
        if let Some(log4rs_path) = settings.get_log4rs_path() {
            self.log4rs_settings = Some(log4rs_path);
        }
        if let Some(sender_type) = settings.get_sender_type() {
            self.sender.sender = sender_type;
        }
        if let Some(receiver_type) = settings.get_receiver_type() {
            self.receiver.receiver = receiver_type;
        }
        if let Some(udp_recv_host) = settings.get_udp_recv_host() {
            self.receiver.udp_address = Some(udp_recv_host);
        }
        if let Some(udp_send_to_host) = settings.get_udp_send_to_host() {
            self.sender.udp_address = Some(udp_send_to_host);
        }
        self
    }
}

pub fn load_from_default_location(root: &Path) -> Result<Settings, String> {
    load_from_file(&root.join(&Path::new(SETTINGS_FILE_NAME)))
}

pub fn load_from_file(path: &Path) -> Result<Settings, String> {
    info!("Attempted load of settings from `{}`", path.display());
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

fn default_logger_settings_path() -> Option<String> {
    None
}

pub struct OverrideSettings {
    settings_path: Option<String>,
    logger_config: Option<String>,
    kafka_hosts: Vec<String>,
    kafka_inboud_topic: Option<String>,
    kafka_outbound_topic: Option<String>,
    kafka_group: Option<String>,
    udp_recv_host: Option<String>,
    udp_send_to_host: Option<String>,
    receiver: Option<String>,
    sender: Option<String>
}

impl OverrideSettings {
    pub fn default() -> OverrideSettings {
        OverrideSettings {
            settings_path: None,
            logger_config: None,
            kafka_hosts: Vec::new(),
            kafka_inboud_topic: None,
            kafka_outbound_topic: None,
            kafka_group: None,
            udp_recv_host: None,
            udp_send_to_host: None,
            receiver: None,
            sender: None,
        }
    }

    pub fn get_settings_path(&self) -> Option<String> {
        self.settings_path.clone()
    }

    pub fn get_log4rs_path(&self) -> Option<String> {
        self.logger_config.clone()
    }

    pub fn get_kafka_hosts(&self) -> Option<&[String]> {
        if self.kafka_hosts.is_empty() {
            None
        } else {
            Some(&self.kafka_hosts)
        }
    }

    pub fn get_receiver_type(&self) -> Option<String> {
        self.receiver.clone()
    }

    pub fn get_sender_type(&self) -> Option<String> {
        self.sender.clone()
    }

    pub fn get_udp_recv_host(&self) -> Option<String> {
        self.udp_recv_host.clone()
    }

    pub fn get_udp_send_to_host(&self) -> Option<String> {
        self.udp_send_to_host.clone()
    }

    pub fn get_kafka_inbound_topic(&self) -> Option<String> {
        self.kafka_inboud_topic.clone()
    }

    pub fn get_kafka_outbound_topic(&self) -> Option<String> {
        self.kafka_outbound_topic.clone()
    }

    pub fn get_kafka_group(&self) -> Option<String> {
        self.kafka_group.clone()
    }
}

pub fn read_cmd_line_args() -> OverrideSettings {
    let mut cmd_settings: OverrideSettings = OverrideSettings::default();
    {
        let mut ap: ArgumentParser = ArgumentParser::new();
        ap.set_description("Small uService for IPv4 Addresses aggregation in standard CIDR format.");
        ap.refer(&mut cmd_settings.settings_path).add_option(&["-c", "--config-path"], StoreOption, "Alternative config file path.");
        ap.refer(&mut cmd_settings.receiver).add_option(&["-r", "--receiver"], StoreOption, "Receiver type. Defaults to `udp`. Possible options are [`udp`, `kafka`].");
        ap.refer(&mut cmd_settings.sender).add_option(&["-s", "--sender"], StoreOption, "Sender type. Defaults to `udp`. Possible options are [`udp`, `kafka`]");
        ap.refer(&mut cmd_settings.kafka_hosts).add_option(&["--kafka-hosts"], Collect, "Kafka hosts, if kafka option is specified.");
        ap.refer(&mut cmd_settings.kafka_inboud_topic).add_option(&["--kafka-inbound-topic"], StoreOption, "Kafka consumer topic.");
        ap.refer(&mut cmd_settings.kafka_outbound_topic).add_option(&["--kafka-outbound-topic"], StoreOption, "Kafka send_to topic");
        ap.refer(&mut cmd_settings.kafka_group).add_option(&["--kafka-receiver-group"], StoreOption, "Kafka group.");
        ap.refer(&mut cmd_settings.udp_recv_host).add_option(&["--udp-receiver-host"], StoreOption, "Udp receiver host.");
        ap.refer(&mut cmd_settings.udp_send_to_host).add_option(&["--udp-send-to-host"], StoreOption, "Udp send to host.");
        ap.refer(&mut cmd_settings.logger_config).add_option(&["-l", "--log4rs-config"], StoreOption, "log4rs configuration file path");
        ap.parse_args_or_exit();
    };
    cmd_settings
}


pub fn default_log4rs_config() -> Config {
    let stdout = ConsoleAppender::builder().build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info)).unwrap();
    config
}