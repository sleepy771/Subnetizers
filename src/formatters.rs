pub type AggFormatter = fn(Vec<(u32, u8)>) -> Vec<String>;
const MAX_UDP_DATAGRAM_PAYLOAD_SIZE: usize = 508;

pub fn simple_formatter(cidrs: Vec<(u32, u8)>) -> Vec<String> {
    let mut from: usize = 0;
    let mut concated_msg: Vec<String> = Vec::new();
    while from < cidrs.len() - 1 {
        let (msg, idx) = concat_to_size(&cidrs[from..], MAX_UDP_DATAGRAM_PAYLOAD_SIZE);
        from += idx;
        concated_msg.push(msg);
    }
    concated_msg
}

fn concat_to_size(strings: &[(u32, u8)], max_size: usize) -> (String, usize) {
    let mut tmp_size: usize = 0;
    let mut chunk_last_idx: usize = 0;
    let mut use_entire_slice = true;
    for (idx, cidr) in strings.iter().enumerate() {
        let cidr_str = make_cidr_ip_string(cidr);
        chunk_last_idx = idx;
        if tmp_size > 0 {
            tmp_size += 1;
        }
        if cidr_str.len() + tmp_size > max_size {
            use_entire_slice = false;
            break;
        }
        tmp_size += cidr_str.len();
    }
    if use_entire_slice {
        let cidr_ips: Vec<String> = strings.iter().map(|cidr| {make_cidr_ip_string(cidr)}).collect();
        (cidr_ips.join(" "), strings.len())
    } else {
        let cidr_ips: Vec<String> = strings[..chunk_last_idx].iter().map(|cidr|{make_cidr_ip_string(cidr)}).collect();
        (cidr_ips.join(" "), chunk_last_idx)
    }
}

fn make_cidr_ip_string(cidr: &(u32, u8)) -> String {
    format!("{}.{}.{}.{}/{}", cidr.0 >> 24, (cidr.0 >> 16) & 0xff, (cidr.0 >> 8) & 0xff, cidr.0 & 0xff, cidr.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ip(a: u8, b: u8, c: u8, d: u8) -> u32 {
        (a as u32) << 24 | (b as u32) << 16 | (c as u32) << 8 | d as u32
    }

    #[test]
    fn test_concat_to_size() {
        let addresses = vec![
            (make_ip(192, 168, 1, 1), 32_u8),
            (make_ip(172, 16, 100, 0), 24_u8),
            (make_ip(10, 10, 0, 0), 16_u8),
            (make_ip(20, 0, 0, 0), 8_u8)
        ];
        assert_eq!(("192.168.1.1/32".to_owned(), 1), concat_to_size(&addresses, 20));
    }

    #[test]
    fn test_concat_to_size_with_exact_len() {
        let addresses = vec![
            (make_ip(192, 168, 1, 1), 32_u8),
            (make_ip(172, 16, 100, 0), 24_u8),
            (make_ip(10, 10, 0, 0), 16_u8),
            (make_ip(20, 0, 0, 0), 8_u8)
        ];
        assert_eq!(("192.168.1.1/32 172.16.100.0/24".to_owned(), 2), concat_to_size(&addresses, 30));
    }

    #[test]
    fn test_concat_to_size_too_short() {
        let addresses = vec![
            (make_ip(192, 168, 1, 1), 32_u8),
            (make_ip(172, 16, 100, 0), 24_u8),
            (make_ip(10, 10, 0, 0), 16_u8),
            (make_ip(20, 0, 0, 0), 8_u8)
        ];
        assert_eq!(("".to_owned(), 0), concat_to_size(&addresses, 5));
    }
}