
pub type AggFormatter = fn(Vec<String>) -> Vec<String>;

pub fn simpl_formatter(cidrs: Vec<String>) -> Vec<String> {
    let mut from: usize = 0;
    let mut concated_msg: Vec<String> = Vec::new();
    while from < cidrs.len() - 1 {
        let (msg, idx) = _concat_to_size(&cidrs[from .. ], 508);
        from += idx;
        concated_msg.push(msg);
    }
    concated_msg
}

fn _concat_to_size(strings: &[String], max_size: usize) -> (String, usize) {
    let mut tmp_size: usize = 0;
    let mut chunk_last_idx: usize = 0;
    let mut use_entire_slice = true;
    for (idx, str_) in strings.iter().enumerate() {
        chunk_last_idx = idx;
        if tmp_size > 0 {
            tmp_size += 1;
        }
        if str_.len() + tmp_size > max_size {
            use_entire_slice = false;
            break;
        }
        tmp_size += str_.len();
    }
    if use_entire_slice {
        (strings.join(" "), strings.len())
    } else {
        (strings[..chunk_last_idx].join(" "), chunk_last_idx)
    }
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