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

    let private_key = "<private key hex string>";
    let mut client_private = Client::new(url, Some(private_key.into())).await.unwrap();

    info!("JWT {:?}", client_private.jwt().await);
}
