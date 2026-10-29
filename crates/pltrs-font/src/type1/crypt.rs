//! Type 1 encryption (Adobe Type 1 Font Format, chapter 7): the `eexec`
//! encryption of the private part and the charstring encryption, with
//! matplotlib's handling of the discarded leading bytes.

use super::python::slice_from;

/// The two keys of Type 1 encryption.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Type1Cipher {
    /// The private dictionary after `eexec` (key 55665).
    Eexec,
    /// Charstrings and subroutines (key 4330).
    Charstring,
}

impl Type1Cipher {
    /// Decrypts `ciphertext` and drops the first `discard` bytes of the
    /// plaintext (matplotlib's `Type1Font._decrypt(ciphertext, key,
    /// ndiscard)`): a negative `discard` keeps only that many final bytes,
    /// as Python's slice does.
    pub fn decrypt(self, ciphertext: &[u8], discard: i64) -> Vec<u8> {
        decrypt(ciphertext, self, discard)
    }

    /// Encrypts `plaintext` after `discard` NUL bytes (matplotlib's
    /// `Type1Font._encrypt(plaintext, key, ndiscard)`).
    pub fn encrypt(self, plaintext: &[u8], discard: i64) -> Vec<u8> {
        encrypt(plaintext, self, discard)
    }

    const fn seed(self) -> u32 {
        match self {
            Self::Eexec => 55665,
            Self::Charstring => 4330,
        }
    }
}

/// The cipher's next key after the cipher byte `cipher`.
const fn next_key(key: u32, cipher: u8) -> u32 {
    ((key + cipher as u32) * 52845 + 22719) & 0xFFFF
}

/// Decrypts `ciphertext` and drops the first `discard` bytes of the
/// plaintext, with Python's slice semantics as matplotlib's `_decrypt`
/// has them: a negative `discard` keeps only that many final bytes.
pub(crate) fn decrypt(ciphertext: &[u8], key: Type1Cipher, discard: i64) -> Vec<u8> {
    let mut state = key.seed();
    let plain: Vec<u8> = ciphertext
        .iter()
        .map(|&cipher| {
            let byte = cipher ^ (state >> 8) as u8;
            state = next_key(state, cipher);
            byte
        })
        .collect();
    plain[slice_from(plain.len(), discard)..].to_vec()
}

/// Encrypts `plaintext` after `discard` NUL bytes (none when `discard` is
/// not positive), as matplotlib's `_encrypt` does for reproducibility
/// where the specification asks for random bytes.
pub(crate) fn encrypt(plaintext: &[u8], key: Type1Cipher, discard: i64) -> Vec<u8> {
    let padding = usize::try_from(discard).unwrap_or(0);
    let mut state = key.seed();
    std::iter::repeat_n(0, padding)
        .chain(plaintext.iter().copied())
        .map(|byte| {
            let cipher = byte ^ (state >> 8) as u8;
            state = next_key(state, cipher);
            cipher
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_with_matplotlibs_padding() {
        let data = b"this is my plaintext \0\x01\x02\x03";
        let encrypted = encrypt(data, Type1Cipher::Eexec, 4);
        assert_eq!(encrypted.len(), data.len() + 4);
        assert_ne!(&encrypted[4..], data.as_slice());
        assert_eq!(decrypt(&encrypted, Type1Cipher::Eexec, 4), data);
        let charstring = encrypt(b"\x0b", Type1Cipher::Charstring, 4);
        assert_eq!(decrypt(&charstring, Type1Cipher::Charstring, 4), b"\x0b");
    }

    #[test]
    fn discard_follows_python_slices() {
        let encrypted = encrypt(b"abcdef", Type1Cipher::Charstring, 0);
        assert_eq!(decrypt(&encrypted, Type1Cipher::Charstring, 0), b"abcdef");
        assert_eq!(decrypt(&encrypted, Type1Cipher::Charstring, -2), b"ef");
        assert_eq!(decrypt(&encrypted, Type1Cipher::Charstring, -10), b"abcdef");
        assert_eq!(decrypt(&encrypted, Type1Cipher::Charstring, 10), b"");
        assert_eq!(encrypt(b"x", Type1Cipher::Eexec, -1).len(), 1);
    }

    #[test]
    fn the_cipher_methods_take_the_discard() {
        let encrypted = Type1Cipher::Eexec.encrypt(b"abc", 2);
        assert_eq!(encrypted, encrypt(b"abc", Type1Cipher::Eexec, 2));
        assert_eq!(Type1Cipher::Eexec.decrypt(&encrypted, 2), b"abc");
        assert_eq!(Type1Cipher::Eexec.decrypt(&encrypted, -1), b"c");
    }
}
