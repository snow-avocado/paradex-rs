use alloy_primitives::{B256, U256};
use alloy_signer::SignerSync;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{Eip712Domain, SolStruct, sol};
use starknet_crypto::Felt;

mod key_derivation;
use key_derivation::private_key_from_signature;

sol! {
    struct Constant {
        string action;
    }
}

pub fn get_paradex_private_key(eth_signer: &PrivateKeySigner) -> Felt {
    let domain = Eip712Domain::new(
        Some("Paradex".into()),
        Some("1".into()),
        Some(U256::from(1u64)),
        None,
        None,
    );

    let message = Constant {
        action: "STARK Key".into(),
    };

    let digest: B256 = message.eip712_signing_hash(&domain);

    let sig = eth_signer
        .sign_hash_sync(&digest)
        .expect("failed to sign EIP-712 digest");

    let sig_bytes = sig.as_bytes();
    private_key_from_signature(&sig_bytes).expect("failed to derive Paradex private key")
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use alloy_signer_local::LocalSigner;

    #[test]
    fn test_get_paradex_account() {
        let eth_signer = LocalSigner::from_str(
            "0x58d27b1d66da0dee9193105c848855b43eeceb14844f2b1de00cdcb1bdce3643",
        )
        .expect("Failed to create signer");
        let paradex_account = get_paradex_private_key(&eth_signer);
        let expected_account =
            Felt::from_str("0x549aa9cb8328a12b1394f99f9430ba2dbc2b5c26b8a4c3b9d2b3ca3765669b2")
                .expect("Failed to parse expected account");
        assert_eq!(paradex_account, expected_account);
    }
}
