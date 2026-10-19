//! Deterministic RNG with no external dependencies, so replays cannot drift
//! with a dependency upgrade. PCG-XSH-RR 64/32 (O'Neill), state serialized.

use serde::{Deserialize, Serialize};

const MULT: u64 = 6364136223846793005;
pub const DEFAULT_INC: u64 = 0xda3e_39cb_94b9_5bdb;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64) -> Self {
        let mut rng = Pcg32 {
            state: 0,
            inc: DEFAULT_INC,
        };
        rng.step();
        rng.state = rng.state.wrapping_add(seed);
        rng.step();
        rng
    }

    fn step(&mut self) {
        self.state = self.state.wrapping_mul(MULT).wrapping_add(self.inc | 1);
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.step();
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in `0..n` via rejection sampling (unbiased).
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "below(0) is undefined");
        let threshold = n.wrapping_neg() % n;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % n;
            }
        }
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        if items.len() < 2 {
            return;
        }
        for i in (1..items.len()).rev() {
            let j = self.below((i + 1) as u32) as usize;
            items.swap(i, j);
        }
    }

    /// Serializable state, for digest and save/load round-trips.
    pub fn raw_state(&self) -> (u64, u64) {
        (self.state, self.inc)
    }
}
