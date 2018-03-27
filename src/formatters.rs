pub type AggFormatter = fn(Vec<(u32, u8)>) -> Vec<String>;

pub fn simple_formatter(cidrs: Vec<(u32, u8)>) -> Vec<String> {
    let mut from: usize = 0;
    let mut concated_msg: Vec<String> = Vec::new();
    while from < cidrs.len() - 1 {
        let (msg, idx) = _concat_to_size(&cidrs[from..], 508);
        from += idx;
        concated_msg.push(msg);
    }
    concated_msg
}

fn _concat_to_size(strings: &[(u32, u8)], max_size: usize) -> (String, usize) {
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

    #[test]
    fn test__concat_to_size() {
        let v = vec!["A".to_string(), "B".to_string(), "C".to_string(), "D".to_string()];
        assert_eq!(("A B".to_string(), 2), _concat_to_size(&v, 3));
        assert_eq!(("A B".to_string(), 2), _concat_to_size(&v, 4));
        assert_eq!(("A B C".to_string(), 3), _concat_to_size(&v, 5));
        assert_eq!(("A B C D".to_string(), 4), _concat_to_size(&v, 20));
    }
}