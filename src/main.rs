// BAD CODE AHEAD
// DO NOT USE PLS

mod state;

use std::io::{self, Read, Write};
use crate::state::{BigStateRNG, BIG_STATE_LEN};

// Filter mode, get any from stdin, outputs good things
fn main() -> io::Result<()> {
    let mut buf = [0u8; BIG_STATE_LEN];
    let mut rng = BigStateRNG::new();

    // TODO: Replace this. What's the point if we have to rely on OS service
    getrandom::getrandom(&mut buf).unwrap();

    rng.reseed(&buf);
    rng.extract(&mut buf);

    let mut input  = vec![0u8; 65536];
    let mut output = vec![0u8; 1048576];
    loop {
        let _ = io::stdin().read(&mut input)?;
        //rng.reseed(&input);
        rng.alt_reseed(&input);
        rng.extract(&mut output);
        io::stdout().write_all(&output)?;
    }
}
