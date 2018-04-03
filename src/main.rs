extern crate argparse;
extern crate kafka;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use]
extern crate nom;
#[macro_use]
extern crate log;
extern crate log4rs;

mod subnet_tree;
mod senders;
mod config;
mod ipagg;
mod parsers;
mod listeners;
mod formatters;

use config::{load_from_default_location, load_from_file, Settings, read_cmd_line_args, default_log4rs_config};
use ipagg::IpAggregator;
use std::env::home_dir;
use std::path::PathBuf;


lazy_static! {
    pub static ref SETTINGS: Settings = {
        let overriding_settings = read_cmd_line_args();
        let paths = {
            let mut paths = Vec::new();
            if let Some(home_path) = home_dir() {
                paths.push(home_path.clone().join(PathBuf::from(".ipaggregator".to_owned())));
            }
            paths.push(PathBuf::from("/etc/ipaggregator".to_owned()));
            paths
        };
        if let Some(path) = overriding_settings.get_settings_path() {
            match load_from_file(&PathBuf::from(path)) {
                Ok(settings) => return settings.override_settings(overriding_settings),
                Err(reason) => {
                    warn!("Can not load file: {}", reason);
                }
            }
        }
        for path in paths {
            match load_from_default_location(&path) {
                Ok(settings) => return settings.override_settings(overriding_settings),
                Err(reason) => {
                    info!("Can not load file: {}", reason);
                }
            }
        }
        Settings::default().override_settings(overriding_settings)
    };
}

fn main() {
    match SETTINGS.get_logger_config() {
        Some(log4rs_cfg_path) => {
            log4rs::init_file(log4rs_cfg_path, Default::default()).unwrap();
        }
        None => {
            log4rs::init_config(default_log4rs_config()).unwrap();
        }
    }
    let mut aggregator = IpAggregator::new();
    aggregator.start();
}
