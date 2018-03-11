
const THRITY_SECONDS: u32 = 30 * 60 * 1000;


pub struct Settings {
    receiver_address: String,
    publish_address: String,
    use_udp: bool,
    publish_timer: u32,
    auto_add_broadcast: bool,
    auto_add_zeroed: bool,
}

impl Settings {
    pub fn defualt() -> Settings {
        Settings {
            receiver_address: "127.0.0.1:6788".to_string(),
            publish_address: "127.0.0.1:6789".to_string(),
            use_udp: true,
            publish_timer: THRITY_SECONDS,
            auto_add_zeroed: true,
            auto_add_broadcast: true
        }
    }
}

pub fn load_from_file(filepath: String) -> Settings {
    Settings::defualt()
}
