#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;

#[macro_use]
extern crate lazy_static;

mod subnet_tree;
mod udp_listener;
mod config;

use subnet_tree::{IPTree, OctetNode};
use config::{Settings, load_from_file, load_from_default_location};
use std::env::home_dir;
use std::path::PathBuf;


lazy_static! {
    pub static ref SETTINGS: Settings = {
        let paths = {
            let mut paths = Vec::new();
            if let Some(home_path) = home_dir() {
                paths.push(home_path.clone());
            }
            paths.push(PathBuf::from("/etc".to_string()));
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

fn main() {
//    let mut tree = IPTree::new();
//    tree.add(&[2, 9, 18, 22]);
//    tree.add(&[2, 9, 18, 21]);
//    tree.add(&[2, 9, 18, 20]);
//    tree.add(&[127, 0, 0, 1]);
//    for k in 0 .. 255 {
//        for j in 0 .. 255 {
//            for i in 0 .. 255 {
//                tree.add(&[172, k, j, i]);
//            }
//        }
//        tree.add(&[172, k, 255, 255]);
//    }
//    println!("{:?}", tree.list_cidr());
    let settings = load_from_file(&PathBuf::from("settings_test.conf.yaml")).unwrap();
    println!("{:?}", settings);
    println!("{:?}", SETTINGS.add_zeroed());
}
