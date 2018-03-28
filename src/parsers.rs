use std::net::Ipv4Addr;
use std::str;
use std::str::FromStr;
use nom;

pub type StreamParser = fn(&[u8]) -> Result<Vec<[u8; 4]>, String>;

pub fn simple_parser(bytes: &[u8]) -> Result<Vec<[u8; 4]>, String> {
    let mut from: i64 = -1;
    let mut ip_vec: Vec<[u8; 4]> = Vec::new();
    for (i, &byte) in bytes.iter().enumerate() {
        if byte != b' ' && from < 0 {
            from = i as i64;
        } else if byte == b' ' && from >= 0 {
            ip_vec.push(parse_ip(&bytes[from as usize..i]));
            from = -1;
        }
    }
    if from >= 0 {
        ip_vec.push(parse_ip(&bytes[from as usize..]));
    }
    Ok(ip_vec)
}

fn parse_ip(address_str: &[u8]) -> [u8; 4] {
    Ipv4Addr::from_str(str::from_utf8(address_str).unwrap()).unwrap().octets()
}

named!(nom_parse_ip<&[u8], Vec<[u8; 4]>>, many0!(ws!(map!(is_a!("0123456789."), parse_ip) )));

pub fn nom_ip_parser(stream: &[u8]) -> Result<Vec<[u8; 4]>, String> {
    match nom_parse_ip(stream) {
        nom::IResult::Done(_, octets) => Ok(octets),
        nom::IResult::Error(e) => Err(format!("Error occurred during parsing: {}", e)),
        nom::IResult::Incomplete(_) => Err("Octet stream is incomplete".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::IResult;

    #[test]
    fn test_parse_ip() {
        assert_eq!([192, 168, 1, 1], parse_ip(b"192.168.1.1"));
        assert_eq!([127, 0, 0, 1], parse_ip(b"127.0.0.1"));
    }

    #[test]
    fn test_simpl_parser() {
        let ips = b" 127.0.0.1   192.168.1.1 ";
        assert_eq!(Ok(vec![[127, 0, 0, 1], [192, 168, 1, 1]]), simple_parser(ips));
    }

    #[test]
    fn test_nom_ip_parser() {
        let ips = b" 127.0.0.1  192.168.1.1 ";
        assert_eq!(IResult::Done(&b""[..], vec![[127, 0, 0, 1], [192, 168, 1, 1]]), nom_parse_ip(&ips[..]));
    }
}