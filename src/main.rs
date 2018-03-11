mod subnet_tree;
mod udp_listener;
mod config;

use subnet_tree::{IPTree, OctetNode};

fn main() {
    let mut tree = IPTree::new();
    tree.add(&[2, 9, 18, 22]);
    tree.add(&[2, 9, 18, 21]);
    tree.add(&[2, 9, 18, 20]);
    tree.add(&[127, 0, 0, 1]);
    for k in 0 .. 255 {
        for j in 0 .. 255 {
            for i in 0 .. 255 {
                tree.add(&[172, k, j, i]);
            }
            tree.add(&[172, k, j, 255]);
        }
        tree.add(&[172, k, 255, 255]);
    }
    println!("{:?}", tree.list_cidr());
}
