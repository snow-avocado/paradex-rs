use log::info;
use paradex::{rest::Client, url::URL};

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let symbol: String = "BTC-USD-PERP".into();

    let url = URL::Testnet;
    let client = Client::new(url, None).await.unwrap();
    info!("system_config {:?}", client.system_config().await);
    info!("BBO {:?}", client.bbo(symbol).await);
    info!("markets_static {:?}", client.markets().await);

    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY not set");
    let client_private = Client::new(url, Some(private_key)).await.unwrap();

    info!("JWT {:?}", client_private.jwt().await);
    info!("Open Orders {:?}", client_private.open_orders().await);
    info!("Fills {:?}", client_private.fills(Some("BTC-USD-PERP".to_string()), Some(chrono::Utc::now() - chrono::Duration::days(2)), Some(chrono::Utc::now())).await.unwrap().len());
    info!("Funding {:?}", client_private.funding_payments(None, None, None).await.unwrap());
    info!("Margin Config for BTC-USD-PERP {:?}", client_private.account_margin_configuration("BTC-USD-PERP".to_string()).await.unwrap());
}
