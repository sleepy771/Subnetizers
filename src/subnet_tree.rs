

struct IPAddress {
    octets: [u8, 4];
}

impl IPAddress {
    pub fn new(adress: u32) -> IPAddress {
        IPAddress { octets: u32to_octets(address) }
    }
}

fn u32to_octets(adress: u32) -> [u8; 4] {
    let mut octets: [u8; 4] = [0,0,0,0];
    octets[0] = ((address >> 24) & 0xff) as u8;
    octets[1] = ((address >> 16) & 0xff) as u8;
    octets[2] = ((address >> 8) & 0xff) as u8;
    octets[3] = (address & 0xff) as u8;
    octets
}

struct IPOctet {
    number: u8,
    suboctets: HashMap<u8, IPOctet>,
    heap: [u64; 8],
    depth: u8
}

impl IPOctet {
    pub fn new(octet: u8, depth: u8) -> IPOctet {
        IPOctet {number: octet, suboctets: HashMap::new(), heap: [0,0,0,0,0,0,0,0], depth: depth}
    }

    pub fn add_octet(&mut self, octet: IPOctet) {
        self._add_heap_index(calculate_heap_bit((256 + octet.number) as u16));
        self.suboctets.insert(&octet.number, octet);
        if self.depth == 0 || self.is_subnet() {
            self._subnetize(octet.number);
        }
    }

    fn _add_heap_index(&mut self, idx: usize, bit_up: u64) {
        self.number[idx] |= bit_up;
    }

    fn _subnetize(&mut self, octet: u8) -> u16
    {
        let mut subnet = (octet.copy() + 255) as u16;

        while self._has_neighbor(subnet) {
            subnet /= 2;
            self._add_heap_index(subnet);
        }

        subnet
    }

    fn _has_neighbor(&self, octet: u8) {
        let neighbor = octet ^ 1;
        self._has_octet(neighbor)
    }

    fn _has_octet(&self, octet: u8) -> bool {
        let (idx, bit_up) = calculate_heap_bit(octet);
        self.heap[idx] & bit_up == bit_up
    }

    pub fn is_subnet(&self) -> bool {
        self.heap[0] & 1 == 1
    }

    fn _on_subnet(&mut self)
    {
        self.suboctets.clear();
        self.depth = 0 as u8;
    }
}

fn calculate_heap_bit(subnet: u16) -> (usize, u64)
{
    let idx: usize = subnet/64;
    let bit_up: u64 = 1 << (subnet & 0x3f);
    (idx, bit_up)
}

struct IPTree {
    octet_chains: HashMap<u8, IPOctet>
}
