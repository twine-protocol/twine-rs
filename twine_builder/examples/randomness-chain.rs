//! An example of a randomness chain that regularly publishing pulses

// periodically create, serialize, and send pulses to a friend

use twine_core::twine::{Pulse, Chain};
use twine_core::josekit::jwk::{KeyPair, Jwk};

fn main() {
    // we create our chain
    let our_keys: dyn KeyPair = //
    let us: Chain = //

    // they create their chain
    let their_keys: dyn KeyPair = // 
    let them: Chain = //
        

    let channel = //
}