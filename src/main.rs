#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

#[macro_use]
extern crate lazy_static;

extern crate argparse;

mod subnet_tree;
mod udp;
mod config;
mod ipagg;

use subnet_tree::{IPTree, OctetNode};
use config::{Settings, load_from_file, load_from_default_location};
use std::env::home_dir;
use std::path::PathBuf;
use argparse::{ArgumentParser, StoreTrue, Store, StoreOption};


lazy_static! {
    pub static ref SETTINGS: Settings = {
        let paths = {
            let mut paths = Vec::new();
            if let Some(home_path) = home_dir() {
                paths.push(home_path.clone().join(PathBuf::from(".ipaggregator".to_string())));
            }
            paths.push(PathBuf::from("/etc/ipaggregator".to_string()));
            paths
        };
        for path in paths {
            match load_from_default_location(&path) {
                Ok(settings) => return settings,
                Err(reason) => println!("Can not load file: {}", reason)
            }
        }
        Settings::defualt()
    };
}

fn start_up() {
    let mut settings_path: Option<String> = None;
    let mut start_udp: bool = false;
    {
        let mut ap: ArgumentParser = ArgumentParser::new();
        ap.set_description("Small uService for IPv4 Addresses aggregation in standard CIDR format.");
        ap.refer(&mut settings_path).add_option(&["-c", "--config-path"], StoreOption, "Alternative config file path.");
        ap.refer(&mut start_udp).add_option(&["-u", "--udp"], StoreTrue, "Start as UDP server");
    };
}

fn main() {

}
