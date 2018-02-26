use std::collections::HashMap;


pub trait OctetNode {
    fn add(&mut self, octets: &[u8]) -> ();

    fn expand(&mut self, octet: u8) -> ();

    fn contains(&self, octet: &u8) -> bool;

    fn get(&self) -> u8;

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>>;

    fn get_cumulative_subnet(&self) -> u8;

    fn is_subnet(&self) -> bool;
}

pub struct StandardNode {
    octet: u8,
    level: u8,
    subnets: [u64; 8],
    subnodes: HashMap<u8, Box<OctetNode>>,
}

// TODO create better names
impl StandardNode {
    pub fn new(octet: u8, level: u8) -> StandardNode {
        StandardNode {
            octet: octet,
            level: level,
            subnets: [0; 8],
            subnodes: HashMap::new()
        }
    }

    fn _has_subnet(&self, subnet: u16) -> bool {
        let (idx, bit) = to_position(subnet).unwrap();
        bit_set(self.subnets[idx], bit)
    }

    fn _set_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        self.subnets[idx] |= bit;
    }

    fn _unset_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        let inv_bit = !bit;
        self.subnets[idx] &= inv_bit;
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
            if bit_set(self.subnets[idx], bit) {
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

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>> {
        self.subnodes.get_mut(octet)
    }

    fn get(&self) -> u8 {
        self.octet
    }

    fn get_cumulative_subnet(&self) -> u8 {
        8u8
    }

    fn contains(&self, octet: &u8) -> bool {
        if self._is_subnet(octet.clone()) {
            return true;
        }
        self.subnodes.contains_key(octet)
    }

    fn is_subnet(&self) -> bool {
        // TODO this method is pretty much unoptimised
        for k in 0 .. 255 {
            if !self._is_subnet(k) {
                return false;
            }
        }
        self._is_subnet(255)
    }
}

fn get_subnetized_octets(subnets: &[u64; 8]) -> [u64; 4] {
    let mut octets: [u64; 8] = subnets.clone();
    for i in 0 .. 256 {
        let (parent_idx, parent_bit) = to_position(i).unwrap();
        if bit_set(subnets[parent_idx], parent_bit) {
            let (left_idx, left_bit) = to_position(i * 2).unwrap();
            let (right_idx, right_bit) = to_position(i * 2 + 1).unwrap();
            octets[left_idx] |= left_bit;
            octets[right_idx] |= right_bit;
        }
    }
    let mut result: [u64; 4] = [0; 4];
    result.copy_from_slice(&octets[4..]);
    result
}

fn invert_assigned_octets(octets: &[u64; 4]) -> [u64; 4] {
    [!octets[0], !octets[1], !octets[2], !octets[3]] 
}

#[derive(Debug)]
pub struct LastNode {
    octet: u8,
    subnets: [u64; 8]
}

impl LastNode {
    pub fn new(octet: u8) -> LastNode {
        LastNode {
            octet: octet,
            subnets: [0u64; 8]
        }
    }

    fn _has_subnet(&self, subnet: u16) -> bool {
        let (idx, bit) = to_position(subnet).unwrap();
        bit_set(self.subnets[idx], bit)
    }

    fn _set_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        self.subnets[idx] |= bit;
    }

    fn _unset_heap_bit(&mut self, subnet: u16) {
        let (idx, bit) = to_position(subnet).unwrap();
        let inv_bit = !bit;
        self.subnets[idx] &= inv_bit;
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
}

impl OctetNode for LastNode {
    fn add(&mut self, octets: &[u8]) {
        if octets.len() > 1 {
            return;
        }
        self.expand(octets[0]);
    }

    fn expand(&mut self, octet: u8) -> () {
        if self.contains(&octet) {
            return;
        }
        let (idx, bit) = to_position(octet as u16 + 256u16).unwrap();
        self.subnets[idx] |= bit;
        self._subnetize(octet as u16 + 256u16);
    }

    fn get(&self) -> u8 {
        self.octet
    }

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>> {
        None
    }

    fn get_cumulative_subnet(&self) -> u8 {
        8u8
    }

    fn contains(&self, octet: &u8) -> bool {
        let mut pos = octet.clone() as u16 + 256u16;
        loop {
            let (idx, bit) = to_position(pos).unwrap();
            if bit_set(self.subnets[idx], bit) {
                return true;
            }
            pos >>= 1;
            if pos < 1 {
                return false;
            }
        };
    }

    fn is_subnet(&self) -> bool {
        2 == self.subnets[0]
    }
}

fn to_position(octet: u16) -> Result<(usize, u64), &'static str> {
    if octet > 511 {
        return Err("Subnetized octet can not have value > 512!");
    }
    Ok(((octet / 64) as usize, (1u64 << (octet % 64)) as u64))
}

fn bit_set(bits: u64, flag: u64) -> bool {
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
}

impl OctetNode for IPTree {
    fn add(&mut self, octet: &[u8]) -> () {
        if octet.len() != 4 {
            return;
        }
        self.expand(octet[0]);
        self.octets.get_mut(&octet[0]).unwrap().add(&octet[1..]);
    }

    fn expand(&mut self, octet: u8) -> () {
        if self.octets.contains_key(&octet) {
            return;
        }
        self.octets.insert(octet, Box::new(StandardNode::new(octet, 1)));
    }

    fn get(&self) -> u8 {
        0u8
    }

    fn get_node(&mut self, octet: &u8) -> Option<&mut Box<OctetNode>> {
        self.octets.get_mut(&octet)
    }

    fn get_cumulative_subnet(&self) -> u8 {
        0u8
    }

    fn contains(&self, octet: &u8) -> bool {
        self.octets.contains_key(&octet)
    }

    fn is_subnet(&self) -> bool {
        false
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
    fn test_bit_set() {
        assert!(bit_set(1, 1));
        assert!(bit_set(5, 1));
        assert!(bit_set(5, 4));
        assert!(!bit_set(5, 2));
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
            subnets: [0, 0, 0, 0, 3, 0, 0, 0]
        };
        node._subnetize(256 + 1);
        assert_eq!(node.subnets, [0, 0, 1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_LastNode__unset_heap_bit() {
        let mut node = LastNode {
            octet: 0,
            subnets: [1, 0, 0, 0, 0, 0, 0, 0]
        };
        node._unset_heap_bit(0);
        assert_eq!(node.subnets, [0; 8]);
    }

    #[test]
    fn test_LastNode__set_heap_bit() {
        let mut node = LastNode {
            octet: 0,
            subnets: [0, 0, 0, 0, 0, 0, 0, 0]
        };
        node._set_heap_bit(0);
        assert_eq!(node.subnets, [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_LastNode___has_subnet() {
        let mut node = LastNode {
            octet: 0,
            subnets: [0, 0, 0, 0, 1, 0, 0, 0]
        };
        assert!(node._has_subnet(256 + 0));
    }

    #[test]
    fn test_LastNode_contains() {
        let mut node = LastNode {
            octet: 0,
            subnets: [2, 0, 0, 0, 0, 0, 0, 0]
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
            subnets: [2, 0, 0, 0, 0, 0, 0, 0],
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
            subnets: [1, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node._unset_heap_bit(0);
        assert_eq!(node.subnets, [0; 8]);
    }

    #[test]
    fn test_StandardNode__set_heap_bit() {
        let mut node = StandardNode {
            octet: 0,
            subnets: [0, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node._set_heap_bit(0);
        assert_eq!(node.subnets, [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_StandardNode__subnetize() {
        let mut node = StandardNode {
            octet: 0,
            subnets: [0, 0, 0, 0, 3, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node._subnetize(256 + 1);
        assert_eq!(node.subnets, [0, 0, 1, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_StandardNode__is_subnet() {
        // Node has subnet bit set up
        let mut node = StandardNode {
            octet: 0,
            subnets: [0, 0, 0, 0, 1, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        assert!(node._is_subnet(0));
        assert!(!node._is_subnet(1));
        
        // Node does not have subnet bit setup, but child already is subnet
        let mut node = StandardNode {
            octet: 0,
            subnets: [0, 0, 0, 0, 1, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node.subnodes.insert(0, Box::new(LastNode {
            subnets: [ 2, 0, 0, 0, 0, 0, 0, 0],
            octet: 0
        }));
        assert!(node._is_subnet(0));

        // Node has subnet bit setup, and child is still there with invalid data.
        let mut node = StandardNode {
            octet: 0,
            subnets: [0, 0, 0, 0, 1, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node.subnodes.insert(0, Box::new(LastNode {
            subnets: [ 0, 0, 0, 0, 0, 0, 0, 0],
            octet: 0
        }));
        assert!(node._is_subnet(0));

        // Node does not have subnet bit set and child is not a subnet.
        let mut node = StandardNode {
            octet: 0,
            subnets: [0, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
            level: 0
        };
        node.subnodes.insert(0, Box::new(LastNode {
            subnets: [ 0, 0, 0, 0, 1, 0, 0, 0],
            octet: 0
        }));
        assert!(!node._is_subnet(0));

        // Entire node is subnet, so children have to be also
        let mut node = StandardNode {
            octet: 0,
            subnets: [2, 0, 0, 0, 0, 0, 0, 0],
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
        assert_eq!(node.subnets, [0; 8]);
        assert!(node.subnodes.contains_key(&1));

        // test node that is in subnet
        let mut node = StandardNode {
            octet: 0,
            level: 0,
            subnets: [2, 0, 0, 0, 0, 0, 0, 0],
            subnodes: HashMap::new(),
        };
        assert!(!node.subnodes.contains_key(&1));
        node.expand(1);
        assert_eq!(node.subnets, [2, 0, 0, 0, 0, 0, 0, 0]);
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
            subnets: [2, 0, 0, 0, 0, 0, 0, 0]
        };
        assert!(node.contains(&2));
    }

    #[test]
    fn test_StandardNode_is_subnet() {
        // simple test wihtout subnets
        let mut node = StandardNode::new(0,0);
        assert!(!node.is_subnet());

        // not all octets are subnets yet
        for i in 0 .. 255 {
            node.subnodes.insert(i, Box::new(LastNode {
                octet: i,
                subnets: [2, 0, 0, 0, 0, 0, 0, 0]
            }));
        };
        assert!(!node.is_subnet());

        // all octets are subnets
        node.subnodes.insert(255, Box::new(LastNode {
            octet: 255,
            subnets: [2, 0, 0, 0, 0, 0, 0, 0]
        }));
        assert!(node.is_subnet());

        // node is subnet
        node = StandardNode {
            octet: 0,
            level: 0,
            subnodes: HashMap::new(),
            subnets: [2, 0, 0, 0, 0, 0, 0, 0]
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
        assert_eq!(node.subnets, [0, 0, 0, 0, 1, 0, 0, 0]);
        assert!(!node.subnodes.contains_key(&0));

    }
}
