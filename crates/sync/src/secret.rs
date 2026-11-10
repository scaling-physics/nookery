use zeroize::Zeroize;

/// The 128-bit pairing secret, shown to the user as a 26-character code.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(pub(crate) [u8; 16]);

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(..)")
    }
}

const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

impl Secret {
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut b = [0u8; 16];
        getrandom::fill(&mut b)?;
        Ok(Self(b))
    }

    /// `ABCD-EFGH-IJKL-MNOP-QRST-UV`: base32, in groups of four to read and type.
    pub fn to_code(&self) -> String {
        let mut bits = 0u32;
        let mut have = 0;
        let mut out = String::new();
        for &byte in &self.0 {
            bits = (bits << 8) | u32::from(byte);
            have += 8;
            while have >= 5 {
                have -= 5;
                out.push(ALPHABET[((bits >> have) & 31) as usize] as char);
            }
        }
        if have > 0 {
            out.push(ALPHABET[((bits << (5 - have)) & 31) as usize] as char);
        }
        let chars: Vec<char> = out.chars().collect();
        chars
            .chunks(4)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Accepts the code in any case, with or without dashes and spaces.
    pub fn from_code(code: &str) -> Option<Self> {
        let digits: Vec<u8> = code
            .chars()
            .filter(|c| !matches!(c, '-' | ' '))
            .map(|c| {
                let c = c.to_ascii_uppercase();
                ALPHABET
                    .iter()
                    .position(|&a| a as char == c)
                    .map(|p| p as u8)
            })
            .collect::<Option<_>>()?;
        if digits.len() != 26 {
            return None;
        }
        let (mut bits, mut have) = (0u32, 0);
        let mut out = Vec::with_capacity(17);
        for d in digits {
            bits = (bits << 5) | u32::from(d);
            have += 5;
            if have >= 8 {
                have -= 8;
                out.push(((bits >> have) & 0xff) as u8);
            }
        }
        // 26 x 5 = 130 bits: the last two are padding and must be zero.
        if out.len() != 16 || (bits & ((1 << have) - 1)) != 0 {
            return None;
        }
        let mut b = [0u8; 16];
        b.copy_from_slice(&out);
        Some(Self(b))
    }

    /// For storing in the settings table.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 32 || !s.is_ascii() {
            return None;
        }
        let mut b = [0u8; 16];
        for (i, slot) in b.iter_mut().enumerate() {
            *slot = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
        }
        Some(Self(b))
    }
}
