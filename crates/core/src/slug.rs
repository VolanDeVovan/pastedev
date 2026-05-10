use regex::Regex;
use std::sync::OnceLock;

pub const SLUG_LEN: usize = 7;
pub const SLUG_ALPHABET: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];

pub fn generate() -> String {
    nanoid::nanoid!(SLUG_LEN, SLUG_ALPHABET)
}

pub fn is_valid_slug(s: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9]{7}$").unwrap());
    re.is_match(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_a_seven_char_slug_in_alphabet() {
        for _ in 0..1_000 {
            let s = generate();
            assert_eq!(s.len(), SLUG_LEN);
            assert!(is_valid_slug(&s));
        }
    }

    #[test]
    fn invalid_slugs_rejected() {
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("123"));
        assert!(!is_valid_slug("abcdef-"));
        assert!(!is_valid_slug("../abcd"));
        assert!(!is_valid_slug("abc12345")); // 8 chars
    }
}
