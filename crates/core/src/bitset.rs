//! A compact bitset over `u64` words, used as the precomputed filter index.
//!
//! The producer (Go `encodeBitsets` in `internal/transform/v2.go`) serializes
//! `[]uint64` as little-endian bytes and base64 (`StdEncoding`). We decode the
//! exact same bytes back into `u64` words, so the bit semantics match the Go
//! producer and the former TS consumer (`web/src/lib/bitset.ts`).

use base64::{engine::general_purpose::STANDARD, Engine as _};

/// Error returned when a base64-encoded bitset cannot be decoded.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("invalid base64 bitset: {0}")]
    Base64(#[from] base64::DecodeError),
}

/// An immutable set of bit positions, backed by little-endian `u64` words.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    const BITS_PER_WORD: usize = u64::BITS as usize;

    /// Decode a base64 little-endian bitset produced by the Go pipeline.
    ///
    /// # Errors
    /// Returns [`DecodeError`] if `encoded` is not valid standard base64.
    pub fn from_base64(encoded: &str) -> Result<Self, DecodeError> {
        let bytes = STANDARD.decode(encoded)?;
        let words = bytes
            .chunks(8)
            .map(|chunk| {
                let mut buf = [0u8; 8];
                buf[..chunk.len()].copy_from_slice(chunk);
                u64::from_le_bytes(buf)
            })
            .collect();
        Ok(Self { words })
    }

    /// The empty set — no words, so every membership test is `false` and an
    /// [`and`](Self::and) against it collapses to empty.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a bitset with bits `0..n` all set.
    #[must_use]
    pub fn all_ones(n: usize) -> Self {
        let mut words = vec![0u64; n.div_ceil(Self::BITS_PER_WORD)];
        let full = n / Self::BITS_PER_WORD;
        for word in &mut words[..full] {
            *word = u64::MAX;
        }
        let remainder = n % Self::BITS_PER_WORD;
        if remainder > 0 {
            words[full] = (1u64 << remainder) - 1;
        }
        Self { words }
    }

    /// Test whether bit `i` is set.
    #[must_use]
    pub fn has(&self, i: usize) -> bool {
        let word = i / Self::BITS_PER_WORD;
        self.words
            .get(word)
            .is_some_and(|w| (w >> (i % Self::BITS_PER_WORD)) & 1 == 1)
    }

    /// Return a new bitset that is the bitwise AND of `self` and `other`.
    ///
    /// Like the original TS implementation, the result is truncated to the
    /// shorter operand; bits beyond it are zero in at least one operand anyway.
    #[must_use]
    pub fn and(&self, other: &Self) -> Self {
        let words = self
            .words
            .iter()
            .zip(&other.words)
            .map(|(a, b)| a & b)
            .collect();
        Self { words }
    }

    /// Iterate over the set bit positions in ascending order.
    pub fn iter_ones(&self) -> impl Iterator<Item = usize> + '_ {
        self.words
            .iter()
            .enumerate()
            .flat_map(|(w, &word)| OneBits {
                word,
                base: w * Self::BITS_PER_WORD,
            })
    }

    /// Count the number of set bits.
    #[must_use]
    pub fn count_ones(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }
}

/// Iterator over the set bits of a single word, lowest first.
struct OneBits {
    word: u64,
    base: usize,
}

impl Iterator for OneBits {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.word == 0 {
            return None;
        }
        let bit = self.word.trailing_zeros() as usize;
        self.word &= self.word - 1; // clear lowest set bit
        Some(self.base + bit)
    }
}

#[cfg(test)]
mod tests {
    use super::BitSet;
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    /// Encode raw little-endian bytes the way the Go pipeline does, for tests.
    fn b64(bytes: &[u8]) -> String {
        STANDARD.encode(bytes)
    }

    fn ones(bs: &BitSet) -> Vec<usize> {
        bs.iter_ones().collect()
    }

    #[test]
    fn creates_from_base64() {
        // 0x05 = bits 0 and 2 set.
        let bs = BitSet::from_base64(&b64(&[5, 0, 0, 0, 0, 0, 0, 0])).unwrap();
        assert!(bs.has(0));
        assert!(!bs.has(1));
        assert!(bs.has(2));
        assert!(!bs.has(3));
    }

    #[test]
    fn creates_all_ones() {
        let bs = BitSet::all_ones(10);
        for i in 0..10 {
            assert!(bs.has(i), "bit {i} should be set");
        }
        assert!(!bs.has(10));
    }

    #[test]
    fn and_operation() {
        let a = BitSet::from_base64(&b64(&[7, 0, 0, 0, 0, 0, 0, 0])).unwrap(); // 0,1,2
        let b = BitSet::from_base64(&b64(&[14, 0, 0, 0, 0, 0, 0, 0])).unwrap(); // 1,2,3
        let result = a.and(&b);
        assert!(!result.has(0));
        assert!(result.has(1));
        assert!(result.has(2));
        assert!(!result.has(3));
    }

    #[test]
    fn iter_ones_returns_positions() {
        // bits 0, 2, 5 = 0b100101 = 37
        let bs = BitSet::from_base64(&b64(&[37, 0, 0, 0, 0, 0, 0, 0])).unwrap();
        assert_eq!(ones(&bs), vec![0, 2, 5]);
    }

    #[test]
    fn count_ones_returns_number_of_set_bits() {
        let bs = BitSet::from_base64(&b64(&[37, 0, 0, 0, 0, 0, 0, 0])).unwrap();
        assert_eq!(bs.count_ones(), 3);
    }

    #[test]
    fn handles_empty_bitset() {
        let bs = BitSet::from_base64(&b64(&[0, 0, 0, 0, 0, 0, 0, 0])).unwrap();
        assert_eq!(ones(&bs), Vec::<usize>::new());
        assert_eq!(bs.count_ones(), 0);
    }

    #[test]
    fn handles_multi_word_bitset() {
        // Two u64 words; set bit 0 of the second word => global bit 64.
        let mut bytes = [0u8; 16];
        bytes[8] = 1;
        let bs = BitSet::from_base64(&b64(&bytes)).unwrap();
        assert!(bs.has(64));
        assert!(!bs.has(63));
        assert!(!bs.has(65));
    }

    #[test]
    fn and_with_all_ones_returns_original() {
        let bs = BitSet::from_base64(&b64(&[42, 0, 0, 0, 0, 0, 0, 0])).unwrap();
        let all = BitSet::all_ones(64);
        assert_eq!(ones(&bs.and(&all)), ones(&bs));
    }
}
