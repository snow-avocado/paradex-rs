use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use paradex::{
    message::sign_order,
    structs::{OrderRequest, OrderType, Side},
};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use starknet_crypto::Felt;
use starknet_signers::SigningKey;

use mimalloc::MiMalloc;

//10-15% performance improvement with mimalloc vs default allocator
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub fn order_benchmark(c: &mut Criterion) {
    let order_request = OrderRequest {
        instruction: paradex::structs::OrderInstruction::IOC,
        market: "BTC-USD-PERP".into(),
        price: None,
        side: Side::BUY,
        size: Decimal::from_f64(0.001).unwrap(),
        order_type: OrderType::MARKET,
        client_id: Some("A".into()),
        flags: vec![],
        recv_window: None,
        stp: None,
        trigger_price: None,
    };

    let signing_key: SigningKey = SigningKey::from_random();
    let signature_timestamp_ms: u128 = 1737256670821;
    let chain_id = Felt::from_hex("0x505249564154455f534e5f504f54435f5345504f4c4941").unwrap();
    let address = Felt::THREE;

    c.bench_with_input(
        BenchmarkId::new("sign order", 0),
        &(
            order_request,
            signing_key,
            signature_timestamp_ms,
            chain_id,
            address,
        ),
        |b, s| b.iter(|| sign_order(&s.0, &s.1, s.2, s.3, s.4)),
    );
}

criterion_group!(benches, order_benchmark);
criterion_main!(benches);
