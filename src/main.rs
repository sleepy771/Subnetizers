mod subnet_tree;

use std::collections::HashMap;

use subnet_tree::{IPTree, OctetNode};


#[derive(Debug, Clone)]
struct IPOctet {
    number: u8,
    subnet: HashMap<u8, IPOctet>,
    heap: [u64; 8],
    depth: u8,
}

impl IPOctet {

    fn _subnetize(&mut self, subnet: u16)
    {
        if self.depth > 1 && ! self.is_subnet() {
            println!("Return subnetize; cuase {}:{}", self.depth, self.is_subnet());
            println!("{:?}", self.heap);
            return ()
        }
        if self._has_neighbor(subnet) && self._has_subnet(subnet) {
            let parent = subnet >> 1;
            self._set_heap_bit(parent);
            self._unset_heap_bit(subnet);
            self._unset_heap_bit(_calculate_neighbor(subnet));
            if parent >= 1 {
                self._subnetize(parent);
            }
        }
    }

    fn _subnet(&mut self, subnets: &[u16]) -> bool
    {
        let can_subnetize = match subnets.len() {
            0 => return false,
            1 => {
                true
            }
            _ => {
                match self.subnet.get_mut(&(subnets[1] as u8)) {
                    Some(child) => {
                        child._subnet(&subnets[1..])
                    },
                    None => false
                }
            }
        };
        if can_subnetize {
            println!("Subnetize {}", subnets[0]);
            self._subnetize(subnets[0]);
        }
        self.is_subnet()
    }

    fn _unset_heap_bit(&mut self, subnet: u16)
    {
        let (idx, bit) = _heap_index_unsafe(subnet);
        self.heap[idx] &= !bit;
    }

    pub fn get_subnets(&self) -> Vec<(u8, u8)>
    {
        let mut subnet_vec: Vec<(u8, u8)> = Vec::new();

        for i in 1 .. 512 {
            if self._has_subnet(i) {
                let (ip_octet, partial_mask) = _partially_calculate_subnet(i);
                subnet_vec.push((ip_octet, partial_mask))
            }
        }
        subnet_vec
    }

    pub fn get_subnets_rec(&self, parent: &Vec<u8>, mask: u8) -> Vec<([u8;4], u8)>
    {
        let octet_subnets = self.get_subnets();
        let mut prepared = parent.clone();
        prepared.push(self.number);
        let mut res: Vec<([u8;4], u8)> = Vec::new();
        for (octet, p_mask) in octet_subnets {
            if self.depth > 1 && p_mask == 8 {
                let ip_octet = self.subnet.get(&octet).unwrap();
                let mut subnet_res = ip_octet.get_subnets_rec(&prepared.clone(), mask + p_mask);
                res.append(&mut subnet_res);
            } else {
                let mut ip_address: [u8;4] = [0; 4];
                let mut k: usize = 0;
                for poctet in prepared.clone() {
                    ip_address[k] = poctet;
                    k += 1;
                }
                ip_address[k] = octet;
                k += 1;
                for l in k .. 4 {
                    ip_address[l] = 0;
                }
                res.push((ip_address, mask + p_mask));
            }
        }
        res
    }

    pub fn is_subnet(&self) -> bool
    {
        // This is some advanced code (expanding mind).
        return self.heap[0] & 2 == 2
    }

    fn _set_heap_bit(&mut self, subnet: u16)
    {
        let (idx, bit) = _heap_index_unsafe(subnet);
        self.heap[idx] |= bit;
    }

    fn _has_neighbor(&self, subnet: u16) -> bool
    {
        self._has_subnet(_calculate_neighbor(subnet))
    }

    fn _has_subnet(&self, subnet: u16) -> bool
    {
        let (idx, bit) = _heap_index_unsafe(subnet);
        (self.heap[idx] & bit) == bit
    }
}


fn _heap_index_unsafe(octet: u16) -> (usize, u64)
{
    // because `something / 64` & `something % 64` is too mainstream
    let idx = (octet >> 6) as usize;
    let bit = 1 << (octet & 0x3f);
    (idx, bit)
}

fn _partially_calculate_subnet(subnet_bit: u16) -> (u8, u8)
{
    // This is actually madness.
    let partial_mask: u8 = (_floor_log2(subnet_bit as u64) - 1) as u8;
    let highest_bounded_power_of_2 = 1 << partial_mask;
    let mult = 256 >> partial_mask; // 256 / mask
    let k = subnet_bit & (highest_bounded_power_of_2 - 1); // i % mask
    let ip_octet = (mult * k) as u8;
    (ip_octet, partial_mask)
}

fn _floor_log2(number: u64) -> u32
{
    return 64 - number.leading_zeros()
}

fn _calculate_neighbor(subnet: u16) -> u16
{
    subnet ^ 1
}

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
    println!("{:?}", tree.list_cidr());
}
