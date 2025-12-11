use rand::{RngCore, rng};

use crate::{define_byte_value, define_key_constant, define_key_sized};

const SESSION_KEY_SIZE: usize = 40;
const PUBLIC_KEY_SIZE: usize = 32;
const PRIVATE_KEY_SIZE: usize = 32;
const SALT_SIZE: usize = 32;
const PASSWORD_VERIFIER_SIZE: usize = 32;
const LARGE_SAFE_PRIME_SIZE: usize = 32;
const S_KEY_SIZE: usize = 32;
const SHA_HASH_SIZE: usize = 20;
const XOR_HASH_SIZE: usize = 20;
const PROOF_SIZE: usize = 20;
const RECONNECT_SEED_SIZE: usize = 16;

define_key_constant!(
    LargeSafePrime,
    LARGE_SAFE_PRIME_SIZE,
    [
        0xb7, 0x9b, 0x3e, 0x2a, 0x87, 0x82, 0x3c, 0xab, 0x8f, 0x5e, 0xbf, 0xbf, 0x8e, 0xb1, 0x1,
        0x8, 0x53, 0x50, 0x6, 0x29, 0x8b, 0x5b, 0xad, 0xbd, 0x5b, 0x53, 0xe1, 0x89, 0x5e, 0x64,
        0x4b, 0x89,
    ]
);

define_key_constant!(
    XorHash,
    XOR_HASH_SIZE,
    [
        0xdd, 0x7b, 0xb0, 0x3a, 0x38, 0xac, 0x73, 0x11, 0x3, 0x98, 0x7c, 0x5a, 0x50, 0x6f, 0xca,
        0x96, 0x6c, 0x7b, 0xc2, 0xa7,
    ]
);

define_byte_value!(K, 3);
define_byte_value!(Generator, 7);
define_key_sized!(Sha1Hash, SHA_HASH_SIZE);
define_key_sized!(Salt, SALT_SIZE);

impl Default for Salt {
    fn default() -> Self {
        let mut key = [0u8; Self::SIZE];
        rng().fill_bytes(&mut key);

        Self::from_bytes_le(&key)
    }
}

impl Salt {
    pub fn randomized() -> Self {
        Self::default()
    }
}

define_key_sized!(PasswordVerifier, PASSWORD_VERIFIER_SIZE);
define_key_sized!(PublicKey, PUBLIC_KEY_SIZE);
define_key_sized!(PrivateKey, PRIVATE_KEY_SIZE);
define_key_sized!(ProofKey, PROOF_SIZE);
define_key_sized!(InterimSessionKey, S_KEY_SIZE);
define_key_sized!(SessionKey, SESSION_KEY_SIZE);
define_key_sized!(ReconnectSeed, RECONNECT_SEED_SIZE);
