
use std::str;
use std::net::Ipv4Addr;
use std::str::FromStr;

pub type StreamParser = fn(&[u8]) -> Result<Vec<[u8; 4]>, String>;

pub fn simpl_parser(bytes: &[u8]) -> Result<Vec<[u8; 4]>, String> {
    let mut from: i64 = -1;
    let mut ip_vec: Vec<[u8; 4]> = Vec::new();
    for (i, &byte) in bytes.iter().enumerate() {
        if byte != b' ' && from < 0 {
            from = i as i64;
        } else if byte == b' ' && from >= 0 {
            ip_vec.push(parse_ip(&bytes[from as usize .. i]));
            from = -1;
        }

    }
    if from >= 0 {
        ip_vec.push(parse_ip(&bytes[from as usize ..]));
    }
    Ok(ip_vec)
}

fn parse_ip(address_str: &[u8]) -> [u8; 4] {
    Ipv4Addr::from_str(str::from_utf8(address_str).unwrap()).unwrap().octets()
}

#[cfg(tests)]
mod tests {
    use super::*;


    #[test]
    fn test_parse_ip() {
        assert_eq!([192, 168, 1, 1], parse_ip(b"192.168.1.1"));
        assert_eq!([127, 0, 0, 1], parse_ip(b"127.0.0.1"));
    }

    #[test]
    fn test_simpl_parser() {
        let ips = b" 127.0.0.1   192.168.1.1";
        assert_eq!(Ok(vec![[127, 0, 0, 1], [192, 168, 1, 1]]), simpl_parser(ips));
    }

}