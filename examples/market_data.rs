use std::{fmt::Debug, time::Duration};

use log::{info, warn};
use paradex::url::URL;
use paradex::ws::{
    BboSubscription, ChannelEvent, FundingDataSubscription, MarketSummarySubscription,
    OrderBookDeltasSubscription, OrderBookSubscription, TradesSubscription, WebsocketManager,
};

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let symbol: String = "BTC-USD-PERP".into();
    let manager = WebsocketManager::new(URL::Testnet, None).await;

    let summary_id = manager
        .subscribe_typed(MarketSummarySubscription, |event| {
            log_channel_event("Market summary", event);
        })
        .await
        .unwrap();
    let bbo_id = manager
        .subscribe_typed(BboSubscription::new(symbol.clone()), |event| {
            log_channel_event("BBO", event);
        })
        .await
        .unwrap();
    let trades_id = manager
        .subscribe_typed(TradesSubscription::new(symbol.clone()), |event| {
            log_channel_event("Trades", event);
        })
        .await
        .unwrap();
    let orderbook_spec = OrderBookSubscription {
        market_symbol: symbol.clone(),
        channel_name: None,
        refresh_rate: "50ms".into(),
        price_tick: None,
    };
    let orderbook_id = manager
        .subscribe_typed(orderbook_spec, |event| {
            log_channel_event("Order book", event);
        })
        .await
        .unwrap();
    let orderbook_deltas_id = manager
        .subscribe_typed(OrderBookDeltasSubscription::new(symbol.clone()), |event| {
            log_channel_event("Order book deltas", event);
        })
        .await
        .unwrap();
    let funding_id = manager
        .subscribe_typed(FundingDataSubscription::all(), |event| {
            log_channel_event("Funding", event);
        })
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

fn log_channel_event<'a, T: Debug>(label: &str, event: ChannelEvent<'a, T>) {
    match event {
        ChannelEvent::Connected => info!("{label}: connected"),
        ChannelEvent::Disconnected => info!("{label}: disconnected"),
        ChannelEvent::Unsubscribed => info!("{label}: unsubscribed"),
        ChannelEvent::Error(err) => warn!("{label}: error {err:?}"),
        ChannelEvent::Data(payload) => info!("{label}: {payload:?}"),
    }
}
