use std::collections::HashMap;


pub trait OctetNode {
    fn add(&mut self, octets: &[u8]) -> ();

    fn contains(&self, octet: &u8) -> bool;

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>>;

    fn is_subnet(&self) -> bool;

    fn recursive_list(&self, prefix: u32, prefix_length: u8) -> Vec<(u32, u8)>;
}

pub struct StandardNode {
    octet: u8,
    level: u8,
    heap: [u64; 8],
    subnodes: HashMap<u8, Box<OctetNode>>,
}

// TODO create better names
impl StandardNode {
    pub fn new(octet: u8, level: u8) -> StandardNode {
        StandardNode {
            octet: octet,
            level: level,
            heap: [0; 8],
            subnodes: HashMap::new()
        }
    }

    fn _has_subnet(&self, subnet: u16) -> bool {
        let (idx, bit) = to_position(subnet).unwrap();
        is_flag_set(self.heap[idx], bit)
    }

    fn _set_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        self.heap[idx] |= bit;
    }

    fn _unset_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        let inv_bit = !bit;
        self.heap[idx] &= inv_bit;
    }

    fn _has_empty_heap(&self) -> bool {
        let mut heap = 0u64;
        for i in 0 .. 8 {
            heap |= self.heap[i];
        }
        heap == 0u64
    }

    fn _subnetize(&mut self, subnet: u16) {
        let mut current_subnet = subnet;
        loop {
            let parent = current_subnet >> 1;
            if self._has_subnet(current_subnet) && self._has_subnet(neighbor(current_subnet)) {
                self._set_heap_bit(parent);
                self._unset_heap_bit(current_subnet);
                self._unset_heap_bit(neighbor(current_subnet));
            } else {
                break;
            }
            current_subnet = parent;
            if parent < 1 {
                break;
            }
        }
    }

    fn _is_subnet(&self, octet: u8) -> bool {
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
        if self._is_subnet(octet) {
            return;
        }
        match self.subnodes.get(&octet) {
            Some(_) => {},
            None => {
                if self.level == 0 {
                    self.subnodes.insert(octet, Box::new(LastNode::new(octet)));
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
        self.expand(octet[0]);
        self.subnodes.get_mut(&octet[0]).unwrap().add(&octet[1..]);

        if self.subnodes.get(&octet[0]).unwrap().is_subnet() {
            self._set_heap_bit(octet[0] as u16 + 256u16);
            if self._is_subnet(octet[0]) && self._is_subnet(neighbor(octet[0] as u16) as u8) {
                self._subnetize(octet[0] as u16 + 256u16);   
            }
            self.subnodes.remove(&octet[0]);
        }
    }

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>> {
        self.subnodes.get_mut(octet)
    }

    fn contains(&self, octet: &u8) -> bool {
        if self._is_subnet(octet.clone()) {
            return true;
        }
        self.subnodes.contains_key(octet)
    }

    fn is_subnet(&self) -> bool {
        self.heap[0] == 2
    }

    fn recursive_list(&self, prefix: u32, prefix_length: u8) -> Vec<(u32, u8)> {
        let mut prefix_vector: Vec<(u32, u8)> = Vec::new();
        let inner_prefix = prefix | ((self.octet as u32) << (32 - (prefix_length + 8)));
        for node in self.subnodes.values() {
            prefix_vector.append(&mut node.recursive_list(inner_prefix, prefix_length + 8));
        }

        if ! self._has_empty_heap() {
            prefix_vector.append(&mut make_cidr(inner_prefix, prefix_length + 8, &self.heap));
        }

        prefix_vector
    }
}

fn make_cidr(prefix: u32, prefix_length: u8, heap: &[u64; 8]) -> Vec<(u32, u8)> {
    let mut ips: Vec<(u32, u8)> = Vec::new();
    for i in 1 .. 511 {
        let (idx, bit) = to_position(i).unwrap();
        if is_flag_set(heap[idx], bit) {
            let (octet, p_mask) = _calculate_partial_cidr(i);
            let ip_address = prefix | (octet as u32) << (24 - prefix_length);
            ips.push((ip_address, prefix_length + p_mask));
        }
    }
    ips
}

fn _calculate_partial_cidr(cidr_bit: u16) -> (u8, u8) {
    let partial_mask: u8 = floor_log2(cidr_bit as u64).unwrap();
    let mask_bit_complement = 1 << partial_mask;
    let cidr_padding = 256 >> partial_mask;
    let range_idx = cidr_bit & (mask_bit_complement - 1);
    ((cidr_padding * range_idx) as u8, partial_mask)
}

#[derive(Debug)]
pub struct LastNode {
    octet: u8,
    heap: [u64; 8]
}

impl LastNode {
    pub fn new(octet: u8) -> LastNode {
        LastNode {
            octet: octet,
            heap: [0u64; 8]
        }
    }

    fn _has_subnet(&self, subnet: u16) -> bool {
        let (idx, bit) = to_position(subnet).unwrap();
        is_flag_set(self.heap[idx], bit)
    }

    fn _set_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        self.heap[idx] |= bit;
    }

    fn _unset_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        let inv_bit = !bit;
        self.heap[idx] &= inv_bit;
    }

    fn _subnetize(&mut self, subnet: u16) {
        let mut current_subnet = subnet;
        loop {
            let parent = current_subnet >> 1;
            if self._has_subnet(current_subnet) && self._has_subnet(neighbor(current_subnet)) {
                self._set_heap_bit(parent);
                self._unset_heap_bit(current_subnet);
                self._unset_heap_bit(neighbor(current_subnet));
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
        self._subnetize(octet as u16 + 256u16);
    }
}

impl OctetNode for LastNode {
    fn add(&mut self, octets: &[u8]) {
        if octets.len() > 1 {
            return;
        }
        self.expand(octets[0]);
    }

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>> {
        None
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

    fn is_subnet(&self) -> bool {
        2 == self.heap[0]
    }

    fn recursive_list(&self, prefix: u32, prefix_length: u8) -> Vec<(u32, u8)> {
        let inner_prefix: u32 = prefix | ((self.octet as u32) << (32 - (prefix_length + 8)));
        make_cidr(inner_prefix, prefix_length + 8, &self.heap)
    }
}

fn to_position(octet: u16) -> Result<(usize, u64), &'static str> {
    if octet > 511 {
        return Err("Subnetized octet can not have value > 512!");
    }
    Ok(((octet / 64) as usize, (1u64 << (octet % 64)) as u64))
}

fn is_flag_set(bits: u64, flag: u64) -> bool {
    bits & flag == flag
}

fn neighbor(subnet: u16) -> u16 {
    subnet ^ 1
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

    pub fn list_cidr(&self) -> Vec<String> {
        self.recursive_list(0, 0).iter().map(|&(ip, mask)| {
            format!("{}.{}.{}.{}/{}", ip >> 24, (ip >> 16) & 0xff, (ip >> 8) & 0xff, ip & 0xff, mask)
        }).collect()
    }
}

impl OctetNode for IPTree {
    fn add(&mut self, octet: &[u8]) -> () {
        if octet.len() != 4 {
            return;
        }
        self.expand(octet[0]);
        self.octets.get_mut(&octet[0]).unwrap().add(&octet[1..]);
    }

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>> {
        self.octets.get_mut(&octet)
    }

    fn contains(&self, octet: &u8) -> bool {
        self.octets.contains_key(&octet)
    }

    fn is_subnet(&self) -> bool {
        false
    }

    fn recursive_list(&self, prefix: u32, prefix_length: u8) -> Vec<(u32, u8)> {
        let mut cidrs: Vec<(u32, u8)> = Vec::new();

        for node in self.octets.values() {
            cidrs.append(&mut node.recursive_list(0, 0));
        }

        cidrs
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

    #[test]
    fn test_neighbor() {
        assert_eq!(neighbor(1), 0);
        assert_eq!(neighbor(255), 254);
        assert_eq!(neighbor(256 + 255), 256 + 254);
        assert_eq!(neighbor(256 + 0), 256 + 1);
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
            heap: [0, 0, 0, 0, 3, 0, 0, 0]
        };
        node._subnetize(256 + 1);
        assert_eq!(node.heap, [0, 0, 1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_LastNode__unset_heap_bit() {
        let mut node = LastNode {
            octet: 0,
            heap: [1, 0, 0, 0, 0, 0, 0, 0]
        };
        node._unset_heap_bit(0);
        assert_eq!(node.heap, [0; 8]);
    }

    #[test]
    fn test_LastNode__set_heap_bit() {
        let mut node = LastNode {
            octet: 0,
            heap: [0, 0, 0, 0, 0, 0, 0, 0]
        };
        node._set_heap_bit(0);
        assert_eq!(node.heap, [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_LastNode___has_subnet() {
        let mut node = LastNode {
            octet: 0,
            heap: [0, 0, 0, 0, 1, 0, 0, 0]
        };
        assert!(node._has_subnet(256 + 0));
    }

    #[test]
    fn test_LastNode_contains() {
        let mut node = LastNode {
            octet: 0,
            heap: [2, 0, 0, 0, 0, 0, 0, 0]
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
        for i in 0 .. 255 {
            node.expand(i);
        }
        assert!(!node.is_subnet());
        node.expand(255);
        assert!(node.is_subnet());
    }

    #[test]
    fn test_StandardNode__has_subnet() {
        let mut node = StandardNode {
            level: 0,
            heap: [2, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            octet: 0
        };
        assert!(node._has_subnet(1));
        assert!(!node._has_subnet(2));
    }

    #[test]
    fn test_StandardNode__unset_heap_bit() {
        let mut node = StandardNode {
            octet: 0,
            heap: [1, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node._unset_heap_bit(0);
        assert_eq!(node.heap, [0; 8]);
    }

    #[test]
    fn test_StandardNode__set_heap_bit() {
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node._set_heap_bit(0);
        assert_eq!(node.heap, [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_StandardNode__subnetize() {
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 3, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node._subnetize(256 + 1);
        assert_eq!(node.heap, [0, 0, 1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_StandardNode__is_subnet() {
        // Node has subnet bit set up
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 1, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        assert!(node._is_subnet(0));
        assert!(!node._is_subnet(1));
        
        // Node does not have subnet bit setup, but child already is subnet
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 1, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node.subnodes.insert(0, Box::new(LastNode {
            heap: [ 2, 0, 0, 0, 0, 0, 0, 0],
            octet: 0
        }));
        assert!(node._is_subnet(0));

        // Node has subnet bit setup, and child is still there with invalid data.
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 1, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node.subnodes.insert(0, Box::new(LastNode {
            heap: [ 0, 0, 0, 0, 0, 0, 0, 0],
            octet: 0
        }));
        assert!(node._is_subnet(0));

        // Node does not have subnet bit set and child is not a subnet.
        let mut node = StandardNode {
            octet: 0,
            heap: [0, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node.subnodes.insert(0, Box::new(LastNode {
            heap: [ 0, 0, 0, 0, 1, 0, 0, 0],
            octet: 0
        }));
        assert!(!node._is_subnet(0));

        // Entire node is subnet, so children have to be also
        let mut node = StandardNode {
            octet: 0,
            heap: [2, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        assert!(node._is_subnet(23));
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
            heap: [2, 0, 0, 0, 0, 0, 0, 0]
        };
        assert!(node.contains(&2));
    }

    #[test]
    fn test_StandardNode_is_subnet() {
        // simple test wihtout heap
        let mut node = StandardNode::new(0,0);
        assert!(!node.is_subnet());

        // not all octets are heap yet
        for j in 0 .. 255 {
            for i in 0 .. 255 {
                node.add(&[j, i]);
            }
            node.add(&[j, 255]);
        };
        assert!(!node.is_subnet());

        for i in 0 .. 255 {
            node.add(&[255, i]);
        }
        node.add(&[255, 255]);
        // all octets are heap
        assert!(node.is_subnet());

        // node is subnet
        node = StandardNode {
            octet: 0,
            level: 0,
            subnodes: HashMap::new(),
            heap: [2, 0, 0, 0, 0, 0, 0, 0]
        };
        assert!(node.is_subnet());

        // TODO create some advanced (mixed) test.
        // TODO split tests to multiple functions, so fixing would be easier.
    }

    #[test]
    fn test_StandardNode_add () {
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

        for i in 1 .. 255 {
            node.add(&[0, i]);
        }
        assert_eq!(node.heap, [0, 0, 0, 0, 1, 0, 0, 0]);
        assert!(!node.subnodes.contains_key(&0));
    }

    #[test]
    fn test_StandardNode_add_multipl_heap() {
        let mut node = StandardNode::new(0, 0);
        for j in 0 .. 4 {
            node.add(&[j, 0]);
            node.add(&[j, 255]);

            for i in 1 .. 255 {
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
    fn test_floor_log2() {
        assert_eq!(floor_log2(2), Ok(1));
        assert_eq!(floor_log2(4), Ok(2));
        assert_eq!(floor_log2(1), Ok(0));
        assert_eq!(floor_log2(9), Ok(3));
    }

    #[test]
    fn test_calculate_subnet() {
        assert_eq!(_calculate_partial_cidr(1), (0, 0));
        assert_eq!(_calculate_partial_cidr(511), (255, 8)); 
        assert_eq!(_calculate_partial_cidr(256), (0, 8)); 
    }

    #[test]
    fn test__calculate_partial_cidr() {
        assert_eq!((0u8, 0u8), _calculate_partial_cidr(1u16));
        assert_eq!((0u8, 1u8), _calculate_partial_cidr(2u16));
        assert_eq!((0u8, 8u8), _calculate_partial_cidr(256u16));
        assert_eq!((255u8, 8u8), _calculate_partial_cidr(511u16));
    }

    #[test]
    fn test_make_cidr() {
        let prefix: u32 = 192 << 24 | 168 << 16 | 1 << 8;
        let mask: u8 = 24;
        assert_eq!(vec![(prefix, 24)], make_cidr(prefix, mask, &[2, 0, 0, 0, 0, 0, 0, 0]));
        assert_eq!(vec![(prefix, 32)], make_cidr(prefix, mask, &[0, 0, 0, 0, 1, 0, 0, 0]));
        assert_eq!(vec![(prefix | 64u32, 32)], make_cidr(prefix, mask, &[0, 0, 0, 0, 0, 1, 0, 0]));
        assert_eq!(
            vec![(prefix | 64, 32), (prefix | 66, 32)],
            make_cidr(prefix, mask, &[0, 0, 0, 0, 0, 5, 0, 0])
        );
    }
}
