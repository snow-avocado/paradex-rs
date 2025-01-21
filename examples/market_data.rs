use std::time::Duration;

use log::info;

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let symbol: String = "BTC-USD-PERP".into();
    let manager = paradex::ws::WebsocketManager::new(paradex::url::URL::Testnet, None).await;
    let summary_id = manager
        .subscribe(
            paradex::ws::Channel::MarketSummary,
            Box::new(|message| info!("Received message {message:?}")),
        )
        .await
        .unwrap();
    let bbo_id = manager
        .subscribe(
            paradex::ws::Channel::BBO {
                market_symbol: symbol.clone(),
            },
            Box::new(|message| info!("Received message {message:?}")),
        )
        .await
        .unwrap();
    let trades_id = manager
        .subscribe(
            paradex::ws::Channel::Trades {
                market_symbol: symbol.clone(),
            },
            Box::new(|message| info!("Received message {message:?}")),
        )
        .await
        .unwrap();
    let orderbook_id = manager
        .subscribe(
            paradex::ws::Channel::OrderBook {
                market_symbol: symbol.clone(),
                refresh_rate: "50ms".into(),
                price_tick: None,
            },
            Box::new(|message| info!("Received message {message:?}")),
        )
        .await
        .unwrap();
    let orderbook_deltas_id = manager
        .subscribe(
            paradex::ws::Channel::OrderBookDeltas {
                market_symbol: symbol.clone(),
            },
            Box::new(|message| info!("Received message {message:?}")),
        )
        .await
        .unwrap();
    let funding_id = manager
        .subscribe(
            paradex::ws::Channel::FundingData {
                market_symbol: None,
            },
            Box::new(|message| info!("Received message {message:?}")),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(120)).await;

    for id in [
        summary_id,
        bbo_id,
        trades_id,
        orderbook_id,
        orderbook_deltas_id,
        funding_id,
    ] {
        manager.unsubscribe(id).await.unwrap();
    }

    tokio::time::sleep(Duration::from_secs(5)).await;
    manager.stop().await.unwrap();
}
