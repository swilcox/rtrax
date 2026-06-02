//! Tiny non-cryptographic PRNG (xorshift64*) used only to shuffle play order.
//! We don't pull in the `rand` crate for this one job — shuffle quality here is
//! cosmetic, not security-sensitive.

pub struct Rng(u64);

impl Rng {
    /// Seed from the wall clock. Good enough to give a different shuffle each
    /// run; never used where reproducibility or security matters.
    pub fn from_clock() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15);
        Self::new(nanos)
    }

    pub fn new(seed: u64) -> Self {
        // xorshift's state must be non-zero.
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Uniform-ish integer in `0..n` (n > 0). Modulo bias is irrelevant here.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % n as u64) as usize
    }
}

/// A Fisher–Yates permutation of `0..len`.
pub fn permutation(len: usize, rng: &mut Rng) -> Vec<usize> {
    let mut v: Vec<usize> = (0..len).collect();
    for i in (1..len).rev() {
        let j = rng.below(i + 1);
        v.swap(i, j);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permutation_is_a_bijection_of_the_range() {
        let mut rng = Rng::new(42);
        let mut p = permutation(64, &mut rng);
        assert_eq!(p.len(), 64);
        p.sort_unstable();
        assert_eq!(p, (0..64).collect::<Vec<_>>());
    }

    #[test]
    fn permutation_handles_degenerate_sizes() {
        let mut rng = Rng::new(1);
        assert!(permutation(0, &mut rng).is_empty());
        assert_eq!(permutation(1, &mut rng), vec![0]);
    }

    #[test]
    fn distinct_seeds_generally_differ() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        assert_ne!(permutation(32, &mut a), permutation(32, &mut b));
    }
}
