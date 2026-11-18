//! Invite codes and seat tokens. The invite admits one peer; the token is
//! the only way back in. Neither is a password for a central account.

use getrandom::fill;

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Crockford base32, case- and dash-insensitive. Returns `XXXX-XXXX`.
pub fn normalize_invite(code: &str) -> Option<String> {
    let raw: String = code
        .chars()
        .filter(|c| !matches!(c, '-' | ' '))
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        })
        .collect();
    let valid = raw.len() == 8
        && raw
            .chars()
            .all(|c| c.is_ascii_digit() || (c.is_ascii_uppercase() && c != 'U'));
    valid.then(|| format!("{}-{}", &raw[..4], &raw[4..]))
}

/// Sixteen hex digits, dashes and case ignored. Returns `XXXXXXXX-XXXXXXXX`.
pub fn normalize_fingerprint(raw: &str) -> Option<String> {
    let compact: String = raw
        .chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .flat_map(|c| c.to_uppercase())
        .collect();
    if compact.len() == 16 && compact.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("{}-{}", &compact[..8], &compact[8..]))
    } else {
        None
    }
}

/// 40 bits from the OS CSPRNG, Crockford-encoded.
pub fn new_invite() -> String {
    let mut bytes = [0u8; 5];
    fill(&mut bytes).expect("OS random");
    let mut n = bytes.iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b));
    let mut chars = [0u8; 8];
    for slot in chars.iter_mut().rev() {
        *slot = ALPHABET[(n & 31) as usize];
        n >>= 5;
    }
    let text = std::str::from_utf8(&chars).expect("alphabet is ascii");
    format!("{}-{}", &text[..4], &text[4..])
}

/// 128-bit token, hex encoded. Never log or render this.
pub fn new_token() -> String {
    let mut bytes = [0u8; 16];
    fill(&mut bytes).expect("OS random");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn clean_name(name: &str, fallback: &str) -> String {
    let cleaned: String = name.chars().filter(|c| !c.is_control()).take(24).collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned.to_string()
    }
}
