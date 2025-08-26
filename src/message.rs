use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};
use crate::structs::{ModifyOrderRequest, OrderRequest};
use cached::SizedCache;
use cached::proc_macro::cached;
use reqwest::header::{HeaderMap, HeaderValue};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use starknet_core::crypto::compute_hash_on_elements;
use starknet_core::types::Felt;
use starknet_core::utils::{
    cairo_short_string_to_felt, get_contract_address, get_selector_from_name, starknet_keccak,
};
use starknet_crypto::{PedersenHasher, Signature};
use starknet_signers::SigningKey;

/*
Ideally we could just use logic similar to below for signing.
However the paradex StarkNetDomain specification does not follow SNIP-12 as the chainId is prior to version.
The starknet_core does not support this. As such, below we manually sign messages with the cryptographic primitives.

fn build_auth_message(chain_id : Felt, timestamp : u128, expiration : u128) -> Result<TypedData> {
    let types = serde_json::from_str::<Types>(r#"{
        "StarkNetDomain": [
            {"name": "name", "type": "felt"},
            {"name": "chainId", "type": "felt"},
            {"name": "version", "type": "felt"}
        ],
        "Request": [
            {"name": "method", "type": "felt"},
            {"name": "path", "type": "felt"},
            {"name": "body", "type": "felt"},
            {"name": "timestamp", "type": "felt"},
            {"name": "expiration", "type": "felt"}
        ]
    }"#).map_err(|e|Error::JsonParseError(e.to_string()))?;

    let domain = Domain {
        name : cairo_short_string_to_felt("Paradex").map_err(|e|Error::StarknetError(e.to_string()))?,
        chain_id,
        version : Felt::ONE,
        revision : Revision::V0
    };
    let primary_type= starknet_core::types::typed_data::InlineTypeReference::Custom("Request".into());
    let mut fields = IndexMap::new();
    fields.insert(String::from_str("method").map_err(|e|Error::TypeConversionError(e.to_string()))?,
                Value::String(String::from_str("POST").map_err(|e|Error::TypeConversionError(e.to_string()))?));
    fields.insert(String::from_str("path").map_err(|e|Error::TypeConversionError(e.to_string()))?,
                Value::String(String::from_str("/v1/auth").map_err(|e|Error::TypeConversionError(e.to_string()))?));
    fields.insert(String::from_str("body").map_err(|e|Error::TypeConversionError(e.to_string()))?,
                Value::String(String::from_str("").map_err(|e|Error::TypeConversionError(e.to_string()))?));
    fields.insert(String::from_str("timestamp").map_err(|e|Error::TypeConversionError(e.to_string()))?,
                Value::UnsignedInteger(timestamp));
    fields.insert(String::from_str("expiration").map_err(|e|Error::TypeConversionError(e.to_string()))?,
                Value::UnsignedInteger(expiration));
    let message= Value::Object(ObjectValue {
        fields
    });

    TypedData::new(
        types,
        domain,
        primary_type,
        message
    ).map_err(|e|crate::error::Error::StarknetError(e.to_string()))
}

*/

// short string encoding of 'StarkNet Message'
const STARKNET_MESSAGE_PREFIX: Felt = Felt::from_raw([
    257012186512350467,
    18446744073709551605,
    10480951322775611302,
    16156019428408348868,
]);

pub fn account_address(
    public_key: Felt,
    paraclear_account_proxy_hash: Felt,
    paraclear_account_hash: Felt,
) -> Result<Felt> {
    let calldata: [Felt; 5] = [
        paraclear_account_hash,
        get_selector_from_name("initialize").map_err(|e| Error::StarknetError(e.to_string()))?,
        Felt::TWO,
        public_key,
        Felt::ZERO,
    ];
    Ok(get_contract_address(
        public_key,
        paraclear_account_proxy_hash,
        &calldata,
        Felt::ZERO,
    ))
}

#[cached(
    ty = "SizedCache<Felt, Result<Felt>>",
    create = "{ SizedCache::with_size(100) }"
)]
fn domain_hash(chain_id: Felt) -> Result<Felt> {
    //chainId should be after version according to SNIP-12. However paradex has the order swapped.
    let domain_name_hash =
        starknet_keccak("StarkNetDomain(name:felt,chainId:felt,version:felt)".as_bytes());
    Ok(compute_hash_on_elements(&[
        domain_name_hash,
        cairo_short_string_to_felt("Paradex").map_err(|e| Error::StarknetError(e.to_string()))?,
        chain_id,
        Felt::ONE,
    ]))
}

static REQUEST_TYPE_HASH: LazyLock<Felt> = LazyLock::new(|| {
    starknet_keccak(
        "Request(method:felt,path:felt,body:felt,timestamp:felt,expiration:felt)".as_bytes(),
    )
});

pub fn auth_message_hash(
    chain_id: Felt,
    timestamp: u128,
    expiration: u128,
    address: Felt,
) -> Result<Felt> {
    let request_hash = compute_hash_on_elements(&[
        *REQUEST_TYPE_HASH,
        cairo_short_string_to_felt("POST").map_err(|e| Error::StarknetError(e.to_string()))?,
        cairo_short_string_to_felt("/v1/auth").map_err(|e| Error::StarknetError(e.to_string()))?,
        cairo_short_string_to_felt("").map_err(|e| Error::StarknetError(e.to_string()))?,
        timestamp.into(),
        expiration.into(),
    ]);

    let mut hasher = PedersenHasher::default();
    hasher.update(STARKNET_MESSAGE_PREFIX);
    hasher.update(domain_hash(chain_id)?);
    hasher.update(address);
    hasher.update(request_hash);

    Ok(hasher.finalize())
}

pub fn auth_headers(
    l2_chain: &Felt,
    signing_key: &SigningKey,
    account: &Felt,
) -> Result<(SystemTime, HeaderMap)> {
    let system_timestamp = SystemTime::now();
    let timestamp: u128 = system_timestamp
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::TimeError(e.to_string()))?
        .as_secs()
        .into();

    let expiration = timestamp + 60 * 60;
    let message_hash =
        crate::message::auth_message_hash(*l2_chain, timestamp, expiration, *account)?;
    let signature = signing_key
        .sign(&message_hash)
        .map_err(|e| Error::StarknetError(e.to_string()))?;

    let account_str = account.to_hex_string();
    let signature_str = format!(r#"["{}","{}"]"#, signature.r, signature.s);

    let mut header_map: HeaderMap<HeaderValue> = HeaderMap::with_capacity(4);
    header_map.insert("PARADEX-STARKNET-ACCOUNT", account_str.parse().unwrap());
    header_map.insert("PARADEX-STARKNET-SIGNATURE", signature_str.parse().unwrap());
    header_map.insert("PARADEX-TIMESTAMP", timestamp.to_string().parse().unwrap());
    header_map.insert(
        "PARADEX-SIGNATURE-EXPIRATION",
        expiration.to_string().parse().unwrap(),
    );
    Ok((system_timestamp, header_map))
}

static ORDER_TYPE_HASH: LazyLock<Felt> = LazyLock::new(|| {
    starknet_keccak(
        "Order(timestamp:felt,market:felt,side:felt,orderType:felt,size:felt,price:felt)"
            .as_bytes(),
    )
});

pub fn sign_order(
    order_request: &OrderRequest,
    signing_key: &SigningKey,
    signature_timestamp_ms: u128,
    chain_id: Felt,
    address: Felt,
) -> Result<Signature> {
    const QUANTIZE_FACTOR: rust_decimal::Result<Decimal> = Decimal::try_new(10_i64.pow(8), 0);
    let quantize_factor = QUANTIZE_FACTOR.unwrap();
    let price_scaled = if let Some(value) = &order_request.price {
        (value * quantize_factor).to_i64().ok_or_else(|| {
            Error::TypeConversionError(format!(
                "Could not convert order price {:?} to i64 ",
                order_request.price
            ))
        })?
    } else {
        0
    };
    let size_scaled = (order_request.size * quantize_factor)
        .to_i64()
        .ok_or_else(|| {
            Error::TypeConversionError(format!(
                "Could not convert order size {} to i64 ",
                order_request.size
            ))
        })?;

    let order_hash = compute_hash_on_elements(&[
        *ORDER_TYPE_HASH,
        signature_timestamp_ms.into(),
        cairo_short_string_to_felt(order_request.market.as_str())
            .map_err(|e| Error::StarknetError(e.to_string()))?,
        order_request.side.felt(),
        order_request.order_type.felt()?,
        size_scaled.into(),
        price_scaled.into(),
    ]);

    let mut hasher = PedersenHasher::default();
    hasher.update(STARKNET_MESSAGE_PREFIX);
    hasher.update(domain_hash(chain_id)?);
    hasher.update(address);
    hasher.update(order_hash);

    let hash = hasher.finalize();
    signing_key
        .sign(&hash)
        .map_err(|e| Error::StarknetError(e.to_string()))
}

static MODIFY_ORDER_TYPE_HASH: std::sync::LazyLock<Felt> = std::sync::LazyLock::new(|| {
    starknet_core::utils::starknet_keccak(
        "ModifyOrder(timestamp:felt,market:felt,side:felt,orderType:felt,size:felt,price:felt,id:felt)"
            .as_bytes(),
    )
});

fn str_to_felt(s: &str) -> Result<Felt> {
    if s.chars().all(|c| c.is_ascii_digit()) {
        Ok(Felt::from_dec_str(s).map_err(|e| Error::StarknetError(e.to_string()))?)
    } else {
        Ok(cairo_short_string_to_felt(s).map_err(|e| Error::StarknetError(e.to_string()))?)
    }
}

pub fn sign_modify_order(
    order_request: &ModifyOrderRequest,
    signing_key: &SigningKey,
    signature_timestamp_ms: u128,
    chain_id: Felt,
    address: Felt,
) -> Result<Signature> {
    const QUANTIZE_FACTOR: rust_decimal::Result<Decimal> = Decimal::try_new(10_i64.pow(8), 0);
    let quantize_factor = QUANTIZE_FACTOR.unwrap();
    let price_scaled = if let Some(value) = &order_request.price {
        (value * quantize_factor).to_i64().ok_or_else(|| {
            Error::TypeConversionError(format!(
                "Could not convert order price {:?} to i64 ",
                order_request.price
            ))
        })?
    } else {
        0
    };
    let size_scaled = (order_request.size * quantize_factor)
        .to_i64()
        .ok_or_else(|| {
            Error::TypeConversionError(format!(
                "Could not convert order size {} to i64 ",
                order_request.size
            ))
        })?;

    let order_hash = compute_hash_on_elements(&[
        *MODIFY_ORDER_TYPE_HASH,
        signature_timestamp_ms.into(),
        cairo_short_string_to_felt(order_request.market.as_str())
            .map_err(|e| Error::StarknetError(e.to_string()))?,
        order_request.side.felt(),
        order_request.order_type.felt()?,
        size_scaled.into(),
        price_scaled.into(),
        str_to_felt(order_request.id.as_str())?,
    ]);

    let mut hasher = PedersenHasher::default();
    hasher.update(STARKNET_MESSAGE_PREFIX);
    hasher.update(domain_hash(chain_id)?);
    hasher.update(address);
    hasher.update(order_hash);

    let hash = hasher.finalize();
    signing_key
        .sign(&hash)
        .map_err(|e| Error::StarknetError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::structs::{Order, OrderInstruction, OrderRequest, OrderType, Side};
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use starknet_core::types::Felt;
    use starknet_signers::SigningKey;

    #[test]
    fn test_domain_hash() {
        let chain_id = cairo_short_string_to_felt("PRIVATE_SN_PARACLEAR_MAINNET").unwrap();
        assert_eq!(
            domain_hash(chain_id).unwrap(),
            Felt::from_hex_unchecked(
                "0x6f74f207280b65cf663fb8d7763fac1e7398cd6d7da5d7681dc300ee4278a0a"
            )
        );
    }

    #[test]
    fn test_account_address() {
        let public_key = Felt::from_raw([1, 2, 3, 4]);
        let paraclear_account_proxy_hash = Felt::from_hex_unchecked(
            "0x3530cc4759d78042f1b543bf797f5f3d647cde0388c33734cf91b7f7b9314a9",
        );
        let paraclear_account_hash = Felt::from_hex_unchecked(
            "0x41cb0280ebadaa75f996d8d92c6f265f6d040bb3ba442e5f86a554f1765244e",
        );

        let address = account_address(
            public_key,
            paraclear_account_proxy_hash,
            paraclear_account_hash,
        )
        .unwrap();
        assert_eq!(
            address,
            Felt::from_hex_unchecked(
                "0x7dea1662f9eb5be9da7df7b0a6cf8c1ad042aed3a28e126aa9b9a31592934f6"
            )
        );
    }

    #[test]
    fn test_auth_message_hash() {
        let chain_id = cairo_short_string_to_felt("PRIVATE_SN_PARACLEAR_MAINNET").unwrap();
        let timestamp = 1737473412;
        let expiration = timestamp + 60 * 60;
        let address = Felt::from_raw([5, 6, 7, 8]);

        let result = auth_message_hash(chain_id, timestamp, expiration, address);
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(
            hash,
            Felt::from_hex_unchecked(
                "0x66ac7ec0cecb995894928c2046ab6ff914e315b2fd6f267e5dde15215af6d9c"
            )
        );
    }

    #[test]
    fn test_sign_order() {
        let order_request = OrderRequest {
            instruction: OrderInstruction::IOC,
            market: "BTC-USD-PERP".into(),
            price: Decimal::from_f64(100000.),
            side: Side::BUY,
            size: Decimal::from_f64(0.001).unwrap(),
            order_type: OrderType::LIMIT,
            client_id: Some("A".into()),
            flags: vec![],
            recv_window: None,
            stp: None,
            trigger_price: None,
        };
        let signing_key = SigningKey::from_secret_scalar(Felt::from_raw([1, 2, 3, 4]));
        let signature_timestamp_ms = 123456789;
        let chain_id = Felt::from_raw([5, 6, 7, 8]);
        let address = Felt::from_raw([9, 10, 11, 12]);

        let result = sign_order(
            &order_request,
            &signing_key,
            signature_timestamp_ms,
            chain_id,
            address,
        );
        assert!(result.is_ok());
        let signature = result.unwrap();
        let order = order_request.into_order([signature.r, signature.s], signature_timestamp_ms);
        assert_eq!(
            order,
            Order {
                instruction: OrderInstruction::IOC,
                market: "BTC-USD-PERP".into(),
                price: Decimal::from_f64(100000.),
                side: Side::BUY,
                size: Decimal::from_f64(0.001).unwrap(),
                order_type: OrderType::LIMIT,
                client_id: Some("A".into()),
                flags: vec![],
                recv_window: None,
                stp: None,
                trigger_price: None,
                signature_timestamp: signature_timestamp_ms,
                signature: [
                    Felt::from_hex_unchecked(
                        "0x208ef0213a190f14b118a0becef75eedfb15f07b9d2b2ed7a03488ed02d07e1"
                    ),
                    Felt::from_hex_unchecked(
                        "0x7fc8c4600708096f0231dcbbdbb0b699b46c149e0d4a81c49e163ec913b9fe2"
                    )
                ],
            }
        );
    }
}
