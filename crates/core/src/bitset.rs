//! A compact bitset over `u64` words, used as the precomputed filter index.
//! Words are serialized as little-endian bytes then standard base64.

use base64::{Engine as _, engine::general_purpose::STANDARD};

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

    /// Decode a base64 little-endian bitset.
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

    /// Encode each `u64` word as little-endian bytes, then standard base64 — the
    /// inverse of [`from_base64`].
    #[must_use]
    pub fn to_base64(&self) -> String {
        let mut bytes = Vec::with_capacity(self.words.len() * Self::BITS_PER_WORD / 8);
        for word in &self.words {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        STANDARD.encode(bytes)
    }

    /// The empty set: every membership test is `false`.
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

    /// Create an all-zero set wide enough for `num_words` 64-bit words.
    ///
    /// [`to_base64`] keeps every word, so a dimension stays fixed-width
    /// regardless of which bits are set.
    #[must_use]
    pub fn with_words(num_words: usize) -> Self {
        Self {
            words: vec![0u64; num_words],
        }
    }

    /// Set bit `i`.
    ///
    /// # Panics
    /// Panics if `i` is beyond the capacity reserved by [`with_words`].
    pub fn set(&mut self, i: usize) {
        self.words[i / Self::BITS_PER_WORD] |= 1u64 << (i % Self::BITS_PER_WORD);
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
    /// Truncated to the shorter operand; bits beyond it are zero in at least one
    /// operand anyway.
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
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    /// Base64-encode raw little-endian bytes for test fixtures.
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

    #[test]
    fn set_then_to_base64_matches_go_little_endian() {
        // bits 0 and 2 set in one word => byte 0x05, then seven zero bytes.
        let mut bs = BitSet::with_words(1);
        bs.set(0);
        bs.set(2);
        assert_eq!(bs.to_base64(), b64(&[5, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn to_base64_round_trips_through_from_base64() {
        let mut bs = BitSet::with_words(2);
        for i in [0usize, 2, 63, 64, 127] {
            bs.set(i);
        }
        let decoded = BitSet::from_base64(&bs.to_base64()).unwrap();
        assert_eq!(ones(&decoded), vec![0, 2, 63, 64, 127]);
    }

    #[test]
    fn with_words_is_fixed_width_even_when_empty() {
        // No bits set, but two reserved words => 16 little-endian bytes.
        let bs = BitSet::with_words(2);
        assert_eq!(bs.to_base64(), b64(&[0u8; 16]));
    }

    use proptest::prelude::*;
    use std::collections::BTreeSet;

    /// A bitset of `num_words` words with an arbitrary set of in-range bits, plus
    /// the word count so tests can build a matching `all_ones`.
    fn any_bitset() -> impl Strategy<Value = (BitSet, usize)> {
        (1usize..8).prop_flat_map(|nw| {
            let cap = nw * 64;
            prop::collection::vec(0..cap, 0..32).prop_map(move |positions| {
                let mut bs = BitSet::with_words(nw);
                for p in positions {
                    bs.set(p);
                }
                (bs, nw)
            })
        })
    }

    /// Two bitsets of the *same* width (so `and` is a clean intersection).
    fn two_same_width() -> impl Strategy<Value = (BitSet, BitSet)> {
        (1usize..8).prop_flat_map(|nw| {
            let cap = nw * 64;
            (
                prop::collection::vec(0..cap, 0..32),
                prop::collection::vec(0..cap, 0..32),
            )
                .prop_map(move |(pa, pb)| {
                    let mut a = BitSet::with_words(nw);
                    let mut b = BitSet::with_words(nw);
                    for p in pa {
                        a.set(p);
                    }
                    for p in pb {
                        b.set(p);
                    }
                    (a, b)
                })
        })
    }

    proptest! {
        /// base64 is a lossless round-trip, width included (the wire contract with
        /// the Go/JS consumer).
        #[test]
        fn base64_round_trips((bs, _nw) in any_bitset()) {
            let decoded = BitSet::from_base64(&bs.to_base64()).unwrap();
            prop_assert_eq!(decoded, bs);
        }

        /// The two "which bits are set" paths agree, and positions are ascending.
        #[test]
        fn count_matches_iter_and_is_sorted((bs, _nw) in any_bitset()) {
            let ones: Vec<usize> = bs.iter_ones().collect();
            prop_assert_eq!(bs.count_ones() as usize, ones.len());
            prop_assert!(ones.windows(2).all(|w| w[0] < w[1]));
        }

        /// AND is commutative, idempotent, and `all_ones` of the same width is its
        /// identity.
        #[test]
        fn and_is_commutative_idempotent_with_identity((bs, nw) in any_bitset()) {
            prop_assert_eq!(bs.and(&bs), bs.clone()); // idempotent
            let all = BitSet::all_ones(nw * 64);
            prop_assert_eq!(bs.and(&all), bs.clone()); // identity
            prop_assert_eq!(all.and(&bs), bs); // commutes with identity
        }

        /// AND equals set intersection of the two operands' positions.
        #[test]
        fn and_equals_intersection((a, b) in two_same_width()) {
            let got: Vec<usize> = a.and(&b).iter_ones().collect();
            let sa: BTreeSet<usize> = a.iter_ones().collect();
            let sb: BTreeSet<usize> = b.iter_ones().collect();
            let want: Vec<usize> = sa.intersection(&sb).copied().collect();
            prop_assert_eq!(got, want);
        }

        /// Decoding never panics on arbitrary input — valid base64 decodes, other
        /// strings return an error.
        #[test]
        fn from_base64_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..64), junk in ".*") {
            prop_assert!(BitSet::from_base64(&STANDARD.encode(&bytes)).is_ok());
            let _ = BitSet::from_base64(&junk);
        }
    }
}
