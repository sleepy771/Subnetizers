use SETTINGS;
use std::collections::{HashMap, LinkedList};
use std::iter::{Iterator, IntoIterator};
use std::ops::BitXor;


pub trait OctetNode: Send {
    fn add(&mut self, octets: &[u8]) -> ();

    fn contains(&self, octet: &u8) -> bool;

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>>;

    fn is_subnet(&self) -> bool;

    fn walk<'a>(&'a self, prefix: u32, mask: u8) -> Box<Iterator<Item=(u32, u8)> + 'a>;
}

pub struct StandardNode {
    octet: u8,
    level: u8,
    heap: [u64; 8],
    subnodes: HashMap<u8, Box<OctetNode>>,
}

// TODO create better names
// > @pastMe: Nah I'm too lazy
impl StandardNode {
    pub fn new(octet: u8, level: u8) -> StandardNode {
        StandardNode {
            octet,
            level,
            heap: [0; 8],
            subnodes: HashMap::new(),
        }
    }

    fn get_heap_ref<'a>(&'a self) -> &'a [u64; 8] {
        &self.heap
    }

    fn contains_subnet_in_heap(&self, subnet: u16) -> bool {
        let (idx, bit) = to_position(subnet).unwrap();
        is_flag_set(self.heap[idx], bit)
    }

    fn set_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        self.heap[idx] |= bit;
    }

    fn unset_heap_bit(&mut self, subnet: u16) {
        // or should I just call this upset_heap_bit?
        let (idx, bit) = to_position(subnet).unwrap();
        let inv_bit = !bit;
        self.heap[idx] &= inv_bit;
    }

    fn is_heap_empty(&self) -> bool {
        let mut heap = 0u64;
        for i in 0..8 {
            heap |= self.heap[i];
        }
        heap == 0u64
    }

    fn merge_subnets(&mut self, subnet: u16) {
        let mut current_subnet = subnet;
        loop {
            let parent = current_subnet >> 1;
            if self.contains_subnet_in_heap(current_subnet) && self.contains_subnet_in_heap(neighbor(current_subnet)) {
                self.set_heap_bit(parent);
                self.unset_heap_bit(current_subnet);
                self.unset_heap_bit(neighbor(current_subnet));
            } else {
                break;
            }
            current_subnet = parent;
            if parent < 1 {
                break;
            }
        }
    }

    fn is_part_of_aggregated_subnet(&self, octet: u8) -> bool {
        let mut pos = octet.clone() as u16 + 256u16;
        loop {
            let (idx, bit) = to_position(pos).unwrap();
            if is_flag_set(self.heap[idx], bit) {
                return true;
            }
            pos >>= 1;
            if pos < 1 {
                match self.subnodes.get(&octet) {
                    None => return false,
                    Some(child) => return child.is_subnet()
                }
            }
        };
    }

    fn expand(&mut self, octet: u8) -> () {
        if self.is_part_of_aggregated_subnet(octet) {
            return;
        }
        match self.subnodes.get(&octet) {
            Some(_) => {}
            None => {
                if self.level == 0 {
                    if SETTINGS.is_last_node_with_settings() {
                        self.subnodes.insert(octet,
                                             Box::new(
                                                 LastNode::new_with_opts(
                                                     octet,
                                                     SETTINGS.add_zeroed(),
                                                     SETTINGS.add_broadcast(),
                                                 )
                                             ));
                    } else {
                        self.subnodes.insert(octet, Box::new(LastNode::new(octet)));
                    }
                } else {
                    self.subnodes.insert(octet, Box::new(StandardNode::new(octet, self.level - 1)));
                }
            }
        }
    }
}

impl OctetNode for StandardNode {
    fn add(&mut self, octet: &[u8]) -> () {
        if octet.len() == 0 {
            return;
        }
        if self.is_part_of_aggregated_subnet(octet[0]) {
            return;
        }
        self.expand(octet[0]);
        self.subnodes.get_mut(&octet[0]).unwrap().add(&octet[1..]);

        if self.subnodes.get(&octet[0]).unwrap().is_subnet() {
            self.set_heap_bit(octet[0] as u16 + 256u16);
            if self.is_part_of_aggregated_subnet(octet[0]) && self.is_part_of_aggregated_subnet(neighbor(octet[0])) {
                self.merge_subnets(octet[0] as u16 + 256u16);
            }
            self.subnodes.remove(&octet[0]);
        }
    }

    fn contains(&self, octet: &u8) -> bool {
        if self.is_part_of_aggregated_subnet(octet.clone()) {
            return true;
        }
        self.subnodes.contains_key(octet)
    }

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>> {
        self.subnodes.get_mut(octet)
    }

    fn is_subnet(&self) -> bool {
        self.heap[0] == 2
    }

    fn walk<'a>(&'a self, prefix: u32, mask: u8) -> Box<Iterator<Item=(u32, u8)> + 'a> {
        let cur_prefix = prefix | ((self.octet as u32) << (24 - mask));
        let cur_mask = mask + 8;
        let mut node_iters: LinkedList<Box<Iterator<Item=(u32, u8)> + 'a>> = LinkedList::new();
        for node in self.subnodes.values() {
            node_iters.push_back(Box::new(node.walk(cur_prefix, cur_mask)));
        }
        Box::new(MoonWalker {
            heap: self.get_heap_ref(),
            idx: 0,
            prefix: cur_prefix,
            mask: cur_mask,
            stack: node_iters,
            node_iter: None,
        })
    }
}

fn calculate_partial_cidr(heap_bit: u16) -> (u8, u8) {
    let partial_mask: u8 = floor_log2(heap_bit as u64).unwrap();
    let mask_bit_complement = 1 << partial_mask;
    let cidr_padding = 256 >> partial_mask;
    let range_idx = heap_bit & (mask_bit_complement - 1);
    ((cidr_padding * range_idx) as u8, partial_mask)
}

struct MoonWalker<'a> {
    heap: &'a [u64; 8],
    idx: u16,
    prefix: u32,
    mask: u8,
    stack: LinkedList<Box<Iterator<Item=(u32, u8)> + 'a>>,
    node_iter: Option<Box<Iterator<Item=(u32, u8)> + 'a>>,
}

impl <'a>Iterator for MoonWalker<'a> {
    type Item = (u32, u8);

    fn next(&mut self) -> Option<(u32, u8)> {
        let mut match_ = false;
        while ! match_ && self.idx < 511 {
            self.idx += 1;
            let (idx, flag) = to_position(self.idx).unwrap();
            match_ = is_flag_set(self.heap[idx], flag);
        }
        if match_ {
            let (octet, p_mask) = calculate_partial_cidr(self.idx);
            let ip_address = self.prefix | (octet as u32) << (24 - self.mask);
            return Some((ip_address, self.mask + p_mask));
        }

        let mut reassign = false;
        match self.node_iter {
            Some(ref mut iter) => {
                match iter.next() {
                    Some((ip, mask)) => return Some((ip, mask)),
                    None => { reassign = true }
                }
            },
            None => { reassign = true }
        }

        if reassign {
            if self.stack.is_empty() {
                None
            } else {
                self.node_iter = self.stack.pop_back();
                self.next()
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct LastNode {
    octet: u8,
    heap: [u64; 8],
}

impl LastNode {
    pub fn new(octet: u8) -> LastNode {
        LastNode {
            octet,
            heap: [0u64; 8],
        }
    }

    pub fn new_with_opts(octet: u8, zeroed: bool, broadcast: bool) -> LastNode {
        let mut node = Self::new(octet);
        if zeroed {
            node.set_heap_bit(256u16 + 0);
        }
        if broadcast {
            node.set_heap_bit(256u16 + 255);
        }
        node
    }

    fn get_heap_ref<'a>(&'a self) -> &'a [u64; 8] {
        &self.heap
    }

    fn contains_subnet_in_heap(&self, subnet: u16) -> bool {
        let (idx, bit) = to_position(subnet).unwrap();
        is_flag_set(self.heap[idx], bit)
    }

    fn set_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        self.heap[idx] |= bit;
    }

    fn unset_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        let inv_bit = !bit;
        self.heap[idx] &= inv_bit;
    }

    fn subnetize(&mut self, subnet: u16) {
        let mut current_subnet = subnet;
        loop {
            let parent = current_subnet >> 1;
            if self.contains_subnet_in_heap(current_subnet) && self.contains_subnet_in_heap(neighbor(current_subnet)) {
                self.set_heap_bit(parent);
                self.unset_heap_bit(current_subnet);
                self.unset_heap_bit(neighbor(current_subnet));
            } else {
                break;
            }
            current_subnet = parent;
            if parent < 1 {
                break;
            }
        }
    }

    fn expand(&mut self, octet: u8) -> () {
        if self.contains(&octet) {
            return;
        }
        let (idx, bit) = to_position(octet as u16 + 256u16).unwrap();
        self.heap[idx] |= bit;
        self.subnetize(octet as u16 + 256u16);
    }
}

impl OctetNode for LastNode {
    fn add(&mut self, octets: &[u8]) {
        if octets.len() > 1 {
            return;
        }
        self.expand(octets[0]);
    }

    fn contains(&self, octet: &u8) -> bool {
        let mut pos = octet.clone() as u16 + 256u16;
        loop {
            let (idx, bit) = to_position(pos).unwrap();
            if is_flag_set(self.heap[idx], bit) {
                return true;
            }
            pos >>= 1;
            if pos < 1 {
                return false;
            }
        };
    }

    fn get_node(&mut self, _: &u8) -> Option<&mut Box<OctetNode>> {
        None
    }

    fn is_subnet(&self) -> bool {
        2 == self.heap[0]
    }

    fn walk<'a>(&'a self, prefix: u32, mask: u8) -> Box<Iterator<Item=(u32, u8)> + 'a> {
        let cur_prefix: u32 = prefix | (self.octet as u32) << (24 - mask);
        let cur_mask: u8 = mask + 8;
        Box::new(LastNodeIterator {
            heap: self.get_heap_ref(),
            idx: 0,
            prefix: cur_prefix,
            mask: cur_mask,
        })
    }
}

struct LastNodeIterator<'a> {
    heap: &'a [u64; 8],
    idx: u16,
    prefix: u32,
    mask: u8,
}

impl <'a>Iterator for LastNodeIterator<'a> {
    type Item = (u32, u8);

    fn next(&mut self) -> Option<(u32, u8)> {
        let mut match_found = false;
        while ! match_found && self.idx < 511 {
            self.idx += 1;
            let (idx, flag) = to_position(self.idx).unwrap();
            match_found = is_flag_set(self.heap[idx], flag);
        }
        if match_found {
            let (octet, p_mask) = calculate_partial_cidr(self.idx);
            let ip_address = self.prefix | (octet as u32) << (24 - self.mask);
            return Some((ip_address, self.mask + p_mask));
        }
        None
    }
}

fn to_position(octet: u16) -> Result<(usize, u64), &'static str> {
    if octet > 511 {
        return Err("Subnetized octet can not have value > 512!");
    }
    // because heap is split to eight 64bit arrays
    Ok(((octet / 64) as usize, (1u64 << (octet % 64)) as u64))
}

fn is_flag_set(bits: u64, flag: u64) -> bool {
    bits & flag == flag
}

fn neighbor<T: Neighboring>(subnet: T) -> T {
    subnet ^ T::one()
}

trait Neighboring: BitXor<Output=Self> + Copy {
    fn one() -> Self;
}

impl Neighboring for u8 {
    fn one() -> u8 { 1 }
}

impl Neighboring for u16 {
    fn one() -> u16 { 1 }
}

pub struct IPTree {
    octets: HashMap<u8, Box<OctetNode>>
}

impl IPTree {
    pub fn new() -> IPTree {
        IPTree {
            octets: HashMap::new()
        }
    }

    fn expand(&mut self, octet: u8) -> () {
        if self.octets.contains_key(&octet) {
            return;
        }
        self.octets.insert(octet, Box::new(StandardNode::new(octet, 1)));
    }

    fn clear(&mut self) -> () {
        self.octets = HashMap::new(); // just curious if this will be enough
    }

    pub fn add(&mut self, octet: &[u8]) -> () {
        if octet.len() != 4 {
            return;
        }
        self.expand(octet[0]);
        self.octets.get_mut(&octet[0]).unwrap().add(&octet[1..]);
    }

    pub fn contains(&self, octet: u8) -> bool {
        self.octets.contains_key(&octet)
    }

    pub fn get_node(&mut self, octet: u8) -> Option<&mut Box<OctetNode>> {
        self.octets.get_mut(&octet)
    }

    pub fn walk<'a>(&'a self) -> Box<Iterator<Item=(u32, u8)> + 'a> {
        let mut iter_stack: LinkedList< Box< Iterator<Item=(u32, u8)> + 'a>> = LinkedList::new();
        for node in self.octets.values() {
            iter_stack.push_back(node.walk(0, 0));
        }
        Box::new(TreeIter {
            iter_stack,
            cursor: None
        })
    }
}

struct TreeIter<'a> {
    iter_stack: LinkedList<Box<Iterator<Item=(u32, u8)> + 'a>>,
    cursor: Option<Box<Iterator<Item=(u32, u8)> + 'a>>
}

impl <'a>Iterator for TreeIter<'a> {
    type Item = (u32, u8);

    fn next(&mut self) -> Option<(u32, u8)> {
        let mut reassign;
        match self.cursor {
            Some(ref mut iter) => {
                match iter.next() {
                    Some((ip, prefix)) => return Some((ip, prefix)),
                    None => reassign = true
                }
            }
            None => reassign = true
        }
        if reassign {
            if self.iter_stack.is_empty() {
                None
            } else {
                self.cursor = self.iter_stack.pop_back();
                self.next()
            }
        } else {
            None
        }
    }
}

fn floor_log2(number: u64) -> Result<u8, &'static str> {
    match number {
        0 => Err("Undefined log2 of `0` called."),
        n => Ok((63 - n.leading_zeros()) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_prefix(ip: [u8; 4]) -> u32 {
        ((ip[0] as u32) << 24) | ((ip[1] as u32) << 16) | ((ip[2] as u32) << 8) | ip[3] as u32
    }

    #[test]
    fn test_neighbor() {
        assert_eq!(neighbor::<u16>(1), 0);
        assert_eq!(neighbor::<u16>(255), 254);
        assert_eq!(neighbor::<u16>(256 + 255), 256 + 254);
        assert_eq!(neighbor::<u16>(256 + 0), 256 + 1);
    }

    #[test]
    fn test_is_flag_set() {
        assert!(is_flag_set(1, 1));
        assert!(is_flag_set(5, 1));
        assert!(is_flag_set(5, 4));
        assert!(!is_flag_set(5, 2));
    }

    #[test]
    fn test_to_position() {
        assert_eq!(to_position(0).unwrap(), (0, 1));
        assert!(to_position(512).is_err());
        assert_eq!(to_position(255).unwrap(), (3, 1 << 63));
    }

    #[test]
    fn test_LastNode_add() {
        let mut node = LastNode::new(0);
        assert!(!node.contains(&1));
        node.add(&[1]);
        assert!(node.contains(&1));
    }

    #[test]
    fn test_LastNode__subnetize() {
        let mut node = LastNode {
            octet: 0,
            heap: [0, 0, 0, 0, 3, 0, 0, 0],
        };
        node.subnetize(256 + 1);
        assert_eq!(node.heap, [0, 0, 1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_LastNode__unset_heap_bit() {
        let mut node = LastNode {
            octet: 0,
            heap: [1, 0, 0, 0, 0, 0, 0, 0],
        };
        node.unset_heap_bit(0);
        assert_eq!(node.heap, [0; 8]);
    }

    #[test]
    fn test_LastNode__set_heap_bit() {
        let mut node = LastNode {
            octet: 0,
            heap: [0, 0, 0, 0, 0, 0, 0, 0],
        };
        node.set_heap_bit(0);
        assert_eq!(node.heap, [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_LastNode___has_subnet() {
        let mut node = LastNode {
            octet: 0,
            heap: [0, 0, 0, 0, 1, 0, 0, 0],
        };
        assert!(node.contains_subnet_in_heap(256 + 0));
    }

    #[test]
    fn test_LastNode_contains() {
        let mut node = LastNode {
            octet: 0,
            heap: [2, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(node.contains(&128));
        assert!(node.contains(&2));
    }

    #[test]
    fn test_LastNode_expand() {
        let mut node = LastNode::new(0);
        assert!(!node.contains(&128));
        node.expand(128);
        assert!(node.contains(&128));
    }

    #[test]
    fn test_LastNode_is_subnet() {
        let mut node = LastNode::new(0);
        for i in 0..255 {
            node.expand(i);
        }
        assert!(!node.is_subnet());
        node.expand(255);
        assert!(node.is_subnet());
    }

    #[test]
    fn test_last_node_walk_empty() {
        let node = LastNode {
            octet: 0,
            heap: [0, 0, 0, 0, 0, 0, 0, 0],
        };
        let mut iter = node.walk(0, 16);
        assert_eq!(None, iter.next())
    }

    #[test]
    fn test_last_node_walk_single_element() {
        let node = LastNode {
            octet: 0,
            heap: [0, 0, 0, 0, 2, 0, 0, 0],
        };
        let mut iter = node.walk(0, 16);
        assert_eq!(Some((1, 32)), iter.next());
        assert_eq!(None, iter.next())
    }

    #[test]
    fn test_last_node_walk_multiple() {
        let node = LastNode {
            octet: 1,
            heap: [0, 0, 0, 0, 2 | 8 | 32, 0, 0, 0]
        };
        let mut iter = node.walk(make_prefix([192, 168, 0, 0]), 16);
        assert_eq!(Some((make_prefix([192, 168, 1, 1]), 32)), iter.next());
        assert_eq!(Some((make_prefix([192, 168, 1, 3]), 32)), iter.next());
        assert_eq!(Some((make_prefix([192, 168, 1, 5]), 32)), iter.next());
        assert_eq!(None, iter.next())
    }

    #[test]
    fn test_StandardNode__has_subnet() {
        let mut node = StandardNode {
            level: 0,
            heap: [2, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            octet: 0,
        };
        assert!(node.contains_subnet_in_heap(1));
        assert!(!node.contains_subnet_in_heap(2));
    }

    #[test]
    fn test_StandardNode__unset_heap_bit() {
        let mut node = StandardNode {
            octet: 0,
            heap: [1, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0,
        };
        node.unset_heap_bit(0);
        assert_eq!(node.heap, [0; 8]);
    }

    #[test]
    fn test_StandardNode__set_heap_bit() {
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0,
        };
        node.set_heap_bit(0);
        assert_eq!(node.heap, [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_StandardNode__subnetize() {
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 3, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0,
        };
        node.merge_subnets(256 + 1);
        assert_eq!(node.heap, [0, 0, 1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_StandardNode__is_subnet() {
        // Node has subnet bit set up
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 1, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0,
        };
        assert!(node.is_part_of_aggregated_subnet(0));
        assert!(!node.is_part_of_aggregated_subnet(1));

        // Node does not have subnet bit setup, but child already is subnet
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 1, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0,
        };
        node.subnodes.insert(0, Box::new(LastNode {
            heap: [2, 0, 0, 0, 0, 0, 0, 0],
            octet: 0,
        }));
        assert!(node.is_part_of_aggregated_subnet(0));

        // Node has subnet bit setup, and child is still there with invalid data.
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 1, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0,
        };
        node.subnodes.insert(0, Box::new(LastNode {
            heap: [0, 0, 0, 0, 0, 0, 0, 0],
            octet: 0,
        }));
        assert!(node.is_part_of_aggregated_subnet(0));

        // Node does not have subnet bit set and child is not a subnet.
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0,
        };
        node.subnodes.insert(0, Box::new(LastNode {
            heap: [0, 0, 0, 0, 1, 0, 0, 0],
            octet: 0,
        }));
        assert!(!node.is_part_of_aggregated_subnet(0));

        // Entire node is subnet, so children have to be also
        let mut node = StandardNode {
            octet: 0,
            heap: [2, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0,
        };
        assert!(node.is_part_of_aggregated_subnet(23));
    }

    #[test]
    fn test_StandardNode_expand() {
        let mut node = StandardNode::new(0, 0);
        assert!(!node.subnodes.contains_key(&1));
        node.expand(1);
        assert!(node.contains(&1));
        assert_eq!(node.heap, [0; 8]);
        assert!(node.subnodes.contains_key(&1));

        // test node that is in subnet
        let mut node = StandardNode {
            octet: 0,
            level: 0,
            heap: [2, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
        };
        assert!(!node.subnodes.contains_key(&1));
        node.expand(1);
        assert_eq!(node.heap, [2, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!node.subnodes.contains_key(&1));
    }

    #[test]
    fn test_StandardNode_contains() {
        // test empty
        let mut node = StandardNode::new(0, 0);
        assert!(!node.contains(&1));

        // test after insertion
        node.subnodes.insert(2, Box::new(LastNode::new(2)));
        assert!(node.contains(&2));

        // test subnetized
        node = StandardNode {
            octet: 0,
            level: 0,
            subnodes: HashMap::new(),
            heap: [2, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(node.contains(&2));
    }

    #[test]
    fn test_StandardNode_is_subnet() {
        // simple test wihtout heap
        let mut node = StandardNode::new(0, 0);
        assert!(!node.is_subnet());

        // not all octets are heap yet
        for j in 0..255 {
            for i in 0..255 {
                node.add(&[j, i]);
            }
            node.add(&[j, 255]);
        };
        assert!(!node.is_subnet());

        for i in 0..255 {
            node.add(&[255, i]);
        }
        // all octets are heap
        assert!(node.is_subnet());

        // node is subnet
        node = StandardNode {
            octet: 0,
            level: 0,
            subnodes: HashMap::new(),
            heap: [2, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(node.is_subnet());

        // TODO create some advanced (mixed) test.
        // TODO split tests to multiple functions, so fixing would be easier.
    }

    #[test]
    fn test_StandardNode_add() {
        // Most important test.
        // Test add 2 octets in tree.
        let mut node = StandardNode::new(0, 0);
        node.add(&[0, 0]);
        assert!(node.subnodes.contains_key(&0));
        assert!(node.subnodes.get(&0).unwrap().contains(&0));
    }

    #[test]
    fn test_StandardNode_add_subnet_carry_over() {
        let mut node = StandardNode::new(0, 0);
        node.add(&[0, 0]);
        node.add(&[0, 255]);

        for i in 1..255 {
            node.add(&[0, i]);
        }
        assert_eq!(node.heap, [0, 0, 0, 0, 1, 0, 0, 0]);
        assert!(!node.subnodes.contains_key(&0));
    }

    #[test]
    fn test_StandardNode_add_multipl_heap() {
        let mut node = StandardNode::new(0, 0);
        for j in 0..4 {
            node.add(&[j, 0]);
            node.add(&[j, 255]);

            for i in 1..255 {
                node.add(&[j, i]);
            }
        }
        assert_eq!(node.heap, [0, 1, 0, 0, 0, 0, 0, 0]);
        assert!(!node.subnodes.contains_key(&0));
        assert!(!node.subnodes.contains_key(&1));
        assert!(!node.subnodes.contains_key(&2));
        assert!(!node.subnodes.contains_key(&3));
    }

    #[test]
    fn test_standard_node_walk_empty() {
        let node = StandardNode {
            octet: 0,
            level: 0,
            heap: [0; 8],
            subnodes: HashMap::new(),
        };
        let mut iter = node.walk(0, 0);
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_standard_node_walk_single_heap() {
        let node = StandardNode {
            octet: 168,
            level: 0, // No love for level
            heap: [0, 0, 0, 0, 2, 0, 0, 0],
            subnodes: HashMap::new(),
        };
        let mut iter = node.walk(make_prefix([192, 0, 0, 0]), 8);
        assert_eq!(Some((make_prefix([192, 168, 1, 0]), 24)), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_standard_node_walk_multiple_heap() {
        let node = StandardNode {
            octet: 168,
            level: 0, // No love for level
            heap: [0, 0, 0, 0, 2 | 8 | 32, 0, 0, 0],
            subnodes: HashMap::new(),
        };
        let mut iter = node.walk(make_prefix([192, 0, 0, 0]), 8);
        assert_eq!(Some((make_prefix([192, 168, 1, 0]), 24)), iter.next());
        assert_eq!(Some((make_prefix([192, 168, 3, 0]), 24)), iter.next());
        assert_eq!(Some((make_prefix([192, 168, 5, 0]), 24)), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_standard_node_walk_single_nested() {
        let subnodes = {
            let mut nodes: HashMap<u8, Box<OctetNode>> = HashMap::new();
            nodes.insert(1_u8, Box::new(LastNode {
                octet: 1,
                heap: [0, 0, 0, 0, 2, 0, 0, 0],
            }));
            nodes
        };
        let node = StandardNode {
            octet: 168,
            level: 0, // No love for level
            heap: [0; 8],
            subnodes,
        };
        let mut iter = node.walk(make_prefix([192, 0, 0, 0]), 8);
        assert_eq!(Some((make_prefix([192, 168, 1, 1]), 32)), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_floor_log2() {
        assert_eq!(floor_log2(2), Ok(1));
        assert_eq!(floor_log2(4), Ok(2));
        assert_eq!(floor_log2(1), Ok(0));
        assert_eq!(floor_log2(9), Ok(3));
    }

    #[test]
    fn test_calculate_subnet() {
        assert_eq!(calculate_partial_cidr(1), (0, 0));
        assert_eq!(calculate_partial_cidr(511), (255, 8));
        assert_eq!(calculate_partial_cidr(256), (0, 8));
    }

    #[test]
    fn test__calculate_partial_cidr() {
        assert_eq!((0u8, 0u8), calculate_partial_cidr(1u16));
        assert_eq!((0u8, 1u8), calculate_partial_cidr(2u16));
        assert_eq!((0u8, 8u8), calculate_partial_cidr(256u16));
        assert_eq!((255u8, 8u8), calculate_partial_cidr(511u16));
    }
}
