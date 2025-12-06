use log::info;

#[tokio::main]
async fn main() {
    // Initialize logging
    simple_logger::init_with_level(log::Level::Info).unwrap();

    // Create a new client connected to testnet
    let client = paradex::rest::Client::new(paradex::url::URL::Testnet, None)
        .await
        .unwrap();

    // Get and print all available markets
    let markets = client.markets().await.unwrap();
    info!("Markets: {:#?}", markets);
    let markets_summary = client.markets_summary("USDC".to_string()).await.unwrap();
    info!("Markets Summary USDC: {:#?}", markets_summary);
    let markets_summary = client
        .markets_summary("BTC-USD-PERP".to_string())
        .await
        .unwrap();
    info!("Markets Summary BTC-USD-PERP: {:#?}", markets_summary);
    // Test an option to see IV fields
    let markets_summary = client
        .markets_summary("BTC-USD-100000-C".to_string())
        .await
        .unwrap();
    info!("Markets Summary BTC-USD-100000-C: {:#?}", markets_summary);
}
