mod subnet_tree;
mod udp_listener;

use subnet_tree::{IPTree, OctetNode};

fn main() {
    let mut tree = IPTree::new();
    tree.add(&[2, 9, 18, 22]);
    tree.add(&[2, 9, 18, 21]);
    tree.add(&[2, 9, 18, 20]);
    tree.add(&[127, 0, 0, 1]);
    for j in 0 .. 255 {
        for i in 0 .. 255 {
            tree.add(&[172, 16, j, i]);
        }
        tree.add(&[172, 16, j, 255]);
    }
    println!("{:?}", tree.list_cidr());
}
