use log::info;
use paradex::structs::KlineParams;

#[tokio::main]
async fn main() {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Info).unwrap();

    // Create a new client connected to testnet
    let client = paradex::rest::Client::new(paradex::url::URL::Testnet, None)
        .await
        .unwrap();

    // Get and print all available klines for BTC-USD-PERP
    let now = chrono::Utc::now();
    let start = now - chrono::Duration::weeks(10);
    let start_ms = start.timestamp_millis() as u64;
    let end_ms = now.timestamp_millis() as u64;
    let klines = client
        .klines(KlineParams {
            start_at: start_ms,
            end_at: end_ms,
            symbol: "BTC-USD-PERP".into(),
            price_kind: Some(paradex::structs::KlinePriceKind::Mark),
            resolution: paradex::structs::KlineResolution::Min30,
        })
        .await
        .unwrap();
    info!("klines: {:#?}", klines);
}
