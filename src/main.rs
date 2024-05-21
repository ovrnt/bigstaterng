// BAD CODE AHEAD
// DO NOT USE PLS

use std::io::{self, Read, Write};
use zeroize::Zeroize;

const KEY_LEN:        usize = 32;
const BIG_STATE_LEN:  usize = 64;
const BLOCK_OFFSET:   usize = 0;
const EXTRACT_START:  usize = (KEY_LEN + BIG_STATE_LEN)
                            .div_ceil(32) + BLOCK_OFFSET;
struct BigStateRNG {
    key:        [u8; KEY_LEN],
    monotonic:  u64,
    bigstate:   [u8; BIG_STATE_LEN],
}

impl BigStateRNG {
    fn new() -> Self {
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
    //   4. Replace bigstate with the output of XOF
    fn reseed(&mut self, block: &[u8]) {
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

        self._monotonic_tick();
        state_update.zeroize();
        mix.zeroize();
        next.zeroize();
    }

    fn extract(&mut self, output: &mut [u8]) {
        let mut hasher = blake3::Hasher::new_keyed(&self.key);
        hasher.update(&self.monotonic.to_le_bytes());
        hasher.update(&self.bigstate);

        let mut xof = hasher.finalize_xof();
        xof.fill(&mut self.key);
        self._monotonic_tick();

        xof.set_position(EXTRACT_START.try_into().unwrap());
        xof.fill(output);
        hasher.zeroize();
    }

    fn _monotonic_tick(&mut self) {
        self.monotonic = self.monotonic.wrapping_add(1);
    }
}

// Filter mode, get any from stdin, outputs good things
fn main() -> io::Result<()>{
    let mut buf = [0u8; BIG_STATE_LEN];
    let mut rng = BigStateRNG::new();

    // TODO: Replace this. What's the point if we have to rely on OS service
    getrandom::getrandom(&mut buf).unwrap();

    rng.reseed(&buf);
    rng.extract(&mut buf);

    // TODO: Try wider input buffer?
    //let mut input = [0u8; BIG_STATE_LEN];
    let mut input  = vec![0u8; 65536];
    let mut output = vec![0u8; 1048576];
    loop {
        let _ = io::stdin().read(&mut input)?;
        rng.reseed(&input);
        rng.extract(&mut output);
        io::stdout().write_all(&output)?;
    }
}
