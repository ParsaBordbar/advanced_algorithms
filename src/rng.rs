#![allow(dead_code)]
use std::time::{SystemTime, UNIX_EPOCH};

pub struct EpochRng {
    state: u64,
}

impl EpochRng {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_nanos() as u64;
        Self { state: seed }
    }
    #[inline(always)]
    pub fn from_seed(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    #[inline(always)]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    #[inline(always)]
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9007199254740992.0)
    }

    #[inline(always)]
    pub fn gen_range(&mut self, low: f64, high: f64) -> f64 {
        assert!(low < high, "low must be less than high");
        low + (high - low) * self.next_f64()
    }

    #[inline(always)]
    pub fn gen_range_i32(&mut self, low: i32, high: i32) -> i32 {
        assert!(low < high, "low must be less than high");
        low + (self.next_u32() % (high - low) as u32) as i32
    }

    #[inline(always)]
    pub fn gen_range_usize(&mut self, low: usize, high: usize) -> usize {
        assert!(low < high, "low must be less than high");
        low + (self.next_u64() as usize % (high - low))
    }

    #[inline(always)]
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.gen_range_usize(0, i + 1);
            slice.swap(i, j);
        }
    }
}

impl Default for EpochRng {
    fn default() -> Self {
        Self::new()
    }
}


pub struct LcgRng {
    state: u64,
    a: u64,
    b: u64,
    m: u64,
}


impl LcgRng {

    #[inline(always)]
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_nanos() as u64;
        Self::from_seed(seed)
    }

    #[inline(always)]
    pub fn from_seed(seed: u64) -> Self {
        Self {
            state: seed,
            a: 1103515245,
            b: 12345,
            m: 1u64 << 32,
        }
    }

    #[inline(always)]
    pub fn with_params(seed: u64, a: u64, b: u64, m: u64) -> Self {
        Self { state: seed, a, b, m }
    }

    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        self.state = (self.a.wrapping_mul(self.state).wrapping_add(self.b)) % self.m;
        self.state
    }

    #[inline(always)]
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    #[inline(always)]
    pub fn next_f64(&mut self) -> f64 {
        self.next_u64() as f64 / self.m as f64
    }

    #[inline(always)]
    pub fn gen_range(&mut self, low: f64, high: f64) -> f64 {
        assert!(low < high, "low must be less than high");
        low + (high - low) * self.next_f64()
    }

    #[inline(always)]
    pub fn gen_range_i32(&mut self, low: i32, high: i32) -> i32 {
        assert!(low < high, "low must be less than high");
        low + (self.next_u32() % (high - low) as u32) as i32
    }

    #[inline(always)]
    pub fn gen_range_usize(&mut self, low: usize, high: usize) -> usize {
        assert!(low < high, "low must be less than high");
        low + (self.next_u64() as usize % (high - low))
    }
}

impl Default for LcgRng {
    fn default() -> Self {
        Self::new()
    }
}

pub struct XorShift {
    pub state: u64,
}

impl XorShift {
    #[inline(always)]
    pub fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 1 } else { seed } }
    }

    #[inline(always)]
    pub fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    #[inline(always)]
    pub fn rand_tenure(&mut self, max_val: u64) -> usize {
        if max_val == 0 { return 1; }
        (1 + (self.next() % max_val)) as usize
    }
}