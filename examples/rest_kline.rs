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

    // Get and print all available markets
    let markets = client
        .klines(KlineParams {
            start_at: chrono::Utc::now() - chrono::Duration::weeks(10),
            end_at: chrono::Utc::now(),
            symbol: "BTC-USD-PERP".into(),
            price_kind: Some(paradex::structs::KlinePriceKind::Mark),
            resolution: paradex::structs::KlineResolution::Min30,
        })
        .await
        .unwrap();
    info!("Markets: {:#?}", markets);
}
