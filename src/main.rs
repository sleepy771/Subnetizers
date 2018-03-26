extern crate argparse;
extern crate kafka;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

mod subnet_tree;
mod senders;
mod config;
mod ipagg;
mod parsers;
mod listeners;
mod formatters;

use argparse::{ArgumentParser, StoreOption, StoreTrue};
use config::{load_from_default_location, load_from_file, Settings};
use ipagg::IpAggregator;
use std::env::home_dir;
use std::path::PathBuf;


lazy_static! {
    pub static ref SETTINGS: Settings = {
        let (optional_path, _) = read_cmd_line_args();
        let paths = {
            let mut paths = Vec::new();
            if let Some(home_path) = home_dir() {
                paths.push(home_path.clone().join(PathBuf::from(".ipaggregator".to_string())));
            }
            paths.push(PathBuf::from("/etc/ipaggregator".to_string()));
            paths
        };
        if let Some(path) = optional_path {
            match load_from_file(&PathBuf::from(path)) {
                Ok(settings) => return settings,
                Err(e) => println!("Can not load file: {}", e)
            }
        }
        for path in paths {
            match load_from_default_location(&path) {
                Ok(settings) => return settings,
                Err(reason) => println!("Can not load file: {}", reason)
            }
        }
        Settings::defualt()
    };
}

fn read_cmd_line_args() -> (Option<String>, bool) {
    let mut settings_path: Option<String> = None;
    let mut start_udp: bool = false;
    {
        let mut ap: ArgumentParser = ArgumentParser::new();
        ap.set_description("Small uService for IPv4 Addresses aggregation in standard CIDR format.");
        ap.refer(&mut settings_path).add_option(&["-c", "--config-path"], StoreOption, "Alternative config file path.");
        ap.refer(&mut start_udp).add_option(&["-u", "--udp"], StoreTrue, "Start as UDP server");
        ap.parse_args_or_exit();
    };
    (settings_path, start_udp)
}

fn main() {
    let mut aggregator = IpAggregator::new();
    aggregator.start();
}
