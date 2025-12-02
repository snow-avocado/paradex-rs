use num_bigint::BigUint;
use num_traits::{Num, One};
use sha2::{Digest, Sha256};
use starknet_crypto::Felt;
use thiserror::Error;

/// STARK curve order (hex) — identique à StarkWare docs / JS implementation.
const STARK_EC_ORDER_HEX: &str = "0800000000000010ffffffffffffffffb781126dcae7b2321e66a241adc64d2f";

#[derive(Debug, Error)]
pub enum KeyDerivationError {
    #[error("signature too short, need at least 64 bytes (r||s)")]
    SigTooShort,
    #[error("internal bigint error")]
    BigIntError,
    #[error("counter overflowed")]
    CounterOverflow,
}

/// Grind a seed into a scalar < STARK_ORDER using SHA-256(seed || counter).
///
/// * `seed` can be 32 or 64 bytes (we allow variable but typical input is r||s (64 bytes)).
pub fn grind_key(seed: &[u8]) -> Result<[u8; 32], KeyDerivationError> {
    let order = BigUint::from_str_radix(STARK_EC_ORDER_HEX, 16)
        .map_err(|_| KeyDerivationError::BigIntError)?;

    let sha256_max = BigUint::one() << 256;
    let max_allowed = &sha256_max - (&sha256_max % &order);

    let mut counter: u32 = 0;
    loop {
        if counter == u32::MAX {
            return Err(KeyDerivationError::CounterOverflow);
        }

        let candidate = hash_with_index(seed, counter);

        if candidate < max_allowed {
            let reduced = candidate % &order;
            let mut out = [0u8; 32];
            let cand_be = reduced.to_bytes_be();
            if cand_be.len() > 32 {
                return Err(KeyDerivationError::BigIntError);
            }
            out[32 - cand_be.len()..].copy_from_slice(&cand_be);
            return Ok(out);
        }

        counter += 1;
    }
}

/// Derive Paradex / Stark private key (Felt) from an Ethereum signature bytes.
///
/// - `sig_bytes` : ECDSA signature bytes. We read the first 64 bytes as r||s (big-endian).
///                 Accepts 64 or 65-length sig (ignores v if present).
pub fn private_key_from_signature(sig_bytes: &[u8]) -> Result<Felt, KeyDerivationError> {
    if sig_bytes.len() < 64 {
        return Err(KeyDerivationError::SigTooShort);
    }

    // r component is the first 32 bytes
    let r: &[u8] = &sig_bytes[..32];

    // grind r to a valid private scalar
    let priv_bytes = grind_key(r)?;

    // convert to Felt (big-endian 32 bytes)
    let felt = Felt::from_bytes_be(&priv_bytes);

    Ok(felt)
}

fn hash_with_index(seed: &[u8], index: u32) -> BigUint {
    let index_bytes = encode_counter(index);
    let mut buf = Vec::with_capacity(seed.len() + index_bytes.len());
    buf.extend_from_slice(seed);
    buf.extend_from_slice(&index_bytes);

    let digest = Sha256::digest(&buf);
    BigUint::from_bytes_be(&digest)
}

fn encode_counter(counter: u32) -> Vec<u8> {
    if counter == 0 {
        return vec![0];
    }
    let mut value = counter;
    let mut bytes = Vec::new();
    while value > 0 {
        bytes.push((value & 0xff) as u8);
        value >>= 8;
    }
    bytes.reverse();
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn hex_to_bytes(hex: &str) -> Vec<u8> {
        assert!(hex.len() % 2 == 0, "hex length must be even");
        hex.as_bytes()
            .chunks(2)
            .map(|pair| {
                let hex_str = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(hex_str, 16).unwrap()
            })
            .collect()
    }

    // Example test vector: not a real MetaMask signature, just sanity test of flow.
    #[test]
    fn test_grind_and_private_key_from_sig() {
        // fake r||s (64 bytes) — here we use repeated pattern to test determinism
        let r_s_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeffedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
        let r_s_bytes = hex_to_bytes(r_s_hex);
        assert_eq!(r_s_bytes.len(), 64);

        let p = private_key_from_signature(&r_s_bytes).expect("derive");
        let b = p.to_bytes_be(); // Felt -> bytes (32)
        assert_eq!(b.len(), 32);
        // deterministic: calling twice yields same result
        let priv2 = private_key_from_signature(&r_s_bytes).expect("derive2");
        assert_eq!(p, priv2);
    }

    #[test]
    fn test_matches_starkware_vector() {
        // Signature produced from the sample key in docs (EIP-712 "STARK Key" message)
        let sig_hex = "7a0d778385e64317e5131bf967de6c3656216651833d7d1a370cd6ae02d65d7a67f7354309952a1a46a0ec3e5107d08381408ca5f58c94e5836c0c37ad06b7161c";
        let sig_bytes = hex_to_bytes(sig_hex);
        let derived = private_key_from_signature(&sig_bytes).expect("derive Paradex key");
        let expected =
            Felt::from_str("0x13110ffbd17e7a8121ff33f3a08cd1b944c3a3a2b04f33f8241472349fb5f03")
                .unwrap();
        assert_eq!(derived, expected);
    }
}
