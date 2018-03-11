mod subnet_tree;

use subnet_tree::{IPTree, OctetNode};

fn main() {
//    let mut tree = IPTree::new_ipv4_tree();
//    for k in 0 .. 8 {
//        for i in 0 .. 256 {
//            tree.add_ipv4([192, 168, k as u8, i as u8])
//        }
//        tree.add_ipv4([192, 168, k, 1]);
//    }
//
//    for (subnet, mask) in tree.get_subnets() {
//        let subnet_address:String = subnet.iter().fold("".to_string(), |st, &oct| {
//            if st == "".to_string() {
//                format!("{}", oct)
//            } else {
//                format!("{}.{}", st, oct)
//            }
//        });
//        println!("{}/{}", subnet_address, mask);
//    }
//    let mut node = IPTree::new();
//    for j in 0 .. 4 {
//        for i in 0 .. 255 {
//            node.add(&[192, 168, j, i]);
//        }
//        node.add(&[192, 168, j, 255]);
//    }
//    println!("{}", node.get_node(&192).unwrap().get_node(&168).unwrap().get_node(&1).unwrap().is_subnet());
//    println!("{}", (!1i64 | 1i64) + 1i64);
    let mut tree = IPTree::new();
    tree.add(&[2, 9, 18, 22]);
    tree.add(&[2, 9, 18, 21]);
    tree.add(&[2, 9, 18, 20]);
    tree.add(&[127, 0, 0, 1]);
    for i in 0 .. 255 {
        tree.add(&[172, 16, 10, i]);
    }
    tree.add(&[172, 16, 10, 255]);
    println!("{:?}", tree.list_cidr());
}
