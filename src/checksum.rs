pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc = 0xffffu16;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

pub fn crc32_iso(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

pub fn fletcher32(data: &[u8]) -> u32 {
    let mut sum1 = 0xffffu32;
    let mut sum2 = 0xffffu32;
    let mut chunks = data.chunks_exact(2);
    for chunk in &mut chunks {
        let word = u16::from_le_bytes([chunk[0], chunk[1]]) as u32;
        sum1 = (sum1 + word) % 0xffff;
        sum2 = (sum2 + sum1) % 0xffff;
    }
    let rem = chunks.remainder();
    if let Some(&byte) = rem.first() {
        sum1 = (sum1 + byte as u32) % 0xffff;
        sum2 = (sum2 + sum1) % 0xffff;
    }
    (sum2 << 16) | sum1
}

pub fn rolling_tag(seed: u32, data: &[u8]) -> u32 {
    let mut state = seed ^ 0x9e37_79b9;
    for (idx, &byte) in data.iter().enumerate() {
        state ^= (byte as u32).wrapping_add((idx as u32).rotate_left((idx % 31) as u32));
        state = state.rotate_left(5).wrapping_mul(0x85eb_ca6b);
        state ^= state >> 13;
    }
    state
}

pub fn parity8(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, byte| acc ^ byte)
}

pub fn weighted_sum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for (idx, &byte) in data.iter().enumerate() {
        let weight = ((idx as u32) % 251) + 1;
        sum = sum.wrapping_add(weight.wrapping_mul(byte as u32));
    }
    sum
}

pub fn verify_crc32(data: &[u8], expected: u32) -> bool {
    crc32_iso(data) == expected
}

pub fn verify_crc16(data: &[u8], expected: u16) -> bool {
    crc16_ccitt(data) == expected
}

#[derive(Debug, Clone)]
pub struct ChecksumWindow {
    seed: u32,
    bytes: Vec<u8>,
}

impl ChecksumWindow {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            bytes: Vec::new(),
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.bytes.extend_from_slice(data);
        if self.bytes.len() > 4096 {
            let drain = self.bytes.len() - 4096;
            self.bytes.drain(0..drain);
        }
    }

    pub fn tag(&self) -> u32 {
        rolling_tag(self.seed, &self.bytes)
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn reset(&mut self, seed: u32) {
        self.seed = seed;
        self.bytes.clear();
    }
}
