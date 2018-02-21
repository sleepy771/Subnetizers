use std::net::IpAddr;
use std::collections::HashMap;


pub trait OctetNode {
    fn add(&mut self, octet: u8) -> ();

    fn get(&self) -> u8;

    fn get_node(&mut self, octet: u8) -> Option<&mut Box<OctetNode>>;

    fn get_cumulative_subnet(&self) -> u8;

    fn contains(&self, octet: u8) -> bool;

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
        match self.subnodes.get(&octet) {
            Some(node) => node.is_subnet(),
            None => {
                self._has_subnet(octet as u16 + 256u16)
            }
        }
    }
}


impl OctetNode for StandardNode {
    fn add(&mut self, octet: u8) -> () {
        if self._has_subnet(octet as u16 + 256u16) {
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
                self._set_heap_bit(octet as u16 + 256u16);
                if self._is_subnet(octet) && self._is_subnet(neighbor(octet as u16) as u8) {
                    self._subnetize(octet as u16 + 256u16);   
                }
            }
        }
    }

    fn get_node(&mut self, octet: u8) -> Option<&mut Box<OctetNode>> {
        self.subnodes.get_mut(&octet)
    }

    fn get(&self) -> u8 {
        self.octet
    }

    fn get_cumulative_subnet(&self) -> u8 {
        8u8
    }

    fn contains(&self, octet: u8) -> bool {
        self.subnodes.contains_key(&octet)
    }

    fn is_subnet(&self) -> bool {
        for k in 0 .. 255 {
            if !self.subnodes.contains_key(&k) {
                return false;
            }
        }
        true
    }
}


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
    fn add(&mut self, octet: u8) {
        if self.contains(octet) {
            return;
        }
        let (idx, bit) = to_position(octet as u16 + 256u16).unwrap();
        self.subnets[idx] |= bit;
        self._subnetize(octet as u16 + 256u16);
    }

    fn get(&self) -> u8 {
        self.octet
    }

    fn get_node(&mut self, octet: u8) -> Option<&mut Box<OctetNode>> {
        None
    }

    fn get_cumulative_subnet(&self) -> u8 {
        8u8
    }

    fn contains(&self, octet: u8) -> bool {
        let mut pos = octet as u16 + 256u16;
        loop {
            let (idx, bit) = to_position(pos).unwrap();
            if bit_set(self.subnets[idx], bit) {
                return true;
            }
            pos <<= 1;
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
    if octet > 512 {
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


fn empty_vec<T>(size: usize) -> Vec<Option<T>> {
    let mut v = Vec::with_capacity(size);
    for _ in 0 .. size {
        v.push(None);
    }
    v
}


struct IPTree {
    octets: HashMap<u8, Box<OctetNode>>
}

impl IPTree {
    pub fn add(&mut self, octet: u8) -> () {
        if self.octets.contains_key(&octet) {
            return;
        }
        self.octets.insert(octet, Box::new(StandardNode::new(octet, 2)));
    }

    pub fn get_node(&mut self, octet: u8) -> Option<&mut Box<OctetNode>> {
        self.octets.get_mut(&octet)
    }
}
