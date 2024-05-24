use zeroize::Zeroize;

pub const KEY_LEN:        usize = 32;
pub const BIG_STATE_LEN:  usize = 64;

const BLOCK_OFFSET:   usize = 0;
const EXTRACT_START:  usize = (KEY_LEN + BIG_STATE_LEN)
                            .div_ceil(32) + BLOCK_OFFSET;

pub struct BigStateRNG {
    key:        [u8; KEY_LEN],
    monotonic:  u64,
    bigstate:   [u8; BIG_STATE_LEN],
}

impl BigStateRNG {
    pub fn new() -> Self {
        Self {
            key:        [0_u8; KEY_LEN],
            monotonic:  0_u64,
            bigstate:   [0_u8; BIG_STATE_LEN],
        }
    }

    // A convoluted way to reseed
    //   1. Hash the old bigstate and the new seed in separate
    //   2. Xor both, use as the key for keyed hash
    //   3. Hash the constant input, finalize as Extendable Output Function (XOF)
    //   4. Xor bigstate with the output of XOF
    pub fn reseed(&mut self, block: &[u8]) {
        let mut prev: [u8; 32] = blake3::hash(&self.bigstate).into();
        let mut next: [u8; 32] = blake3::hash(block).into();
        prev.iter_mut()
            .zip(next.iter())
            .for_each(|(a, b)| *a ^= *b);
        let mut mix = blake3::Hasher::new_keyed(&prev);
        mix.update(&self.monotonic.to_le_bytes());
        // Constant input
        mix.update(b"Big State Reseed");
        let mut xof = mix.finalize_xof();

        let mut state_update = [0u8; BIG_STATE_LEN];
        xof.fill(&mut state_update);
        self.bigstate.iter_mut()
                .zip(state_update.iter())
                .for_each(|(a, b)| *a ^= *b);

        self.monotonic_tick();
        state_update.zeroize();
        mix.zeroize();
        next.zeroize();
    }
    
    // Reseed: alternate mode
    // Consumes everything inside a regular hash, replace bigstate
    #[allow(unused)]
    pub fn alt_reseed(&mut self, block: &[u8]) {
        let mut mix = blake3::Hasher::new();
        mix.update(&self.monotonic.to_le_bytes());
        mix.update(b"Big State Reseed");
        mix.update(&self.bigstate);
        mix.update(block);

        let mut xof = mix.finalize_xof();
        xof.fill(&mut self.bigstate);

        self.monotonic_tick();
        mix.zeroize();
    }

    pub fn extract(&mut self, output: &mut [u8]) {
        let mut hasher = blake3::Hasher::new_keyed(&self.key);
        hasher.update(&self.monotonic.to_le_bytes());
        hasher.update(&self.bigstate);

        let mut xof = hasher.finalize_xof();
        xof.fill(&mut self.key);
        self.monotonic_tick();

        xof.set_position(EXTRACT_START.try_into().unwrap());
        xof.fill(output);
        hasher.zeroize();
    }

    fn monotonic_tick(&mut self) {
        self.monotonic = self.monotonic.wrapping_add(1);
    }
}
