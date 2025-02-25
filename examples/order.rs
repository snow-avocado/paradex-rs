use std::time::Duration;

use log::info;
use paradex::{
    rest::Client,
    structs::{OrderRequest, OrderType, Side},
    url::URL,
};
use rust_decimal::{prelude::FromPrimitive, Decimal};

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let url = URL::Testnet;
    let symbol: String = "BTC-USD-PERP".into();

    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY not set");
    let client_private = Client::new(url, Some(private_key.clone())).await.unwrap();

    info!(
        "Account Information {:?}",
        client_private.account_information().await
    );
    info!("Balance {:?}", client_private.balance().await);
    info!("Positions {:?}", client_private.positions().await);

    let manager = paradex::ws::WebsocketManager::new(
        paradex::url::URL::Testnet,
        Some(Client::new(url, Some(private_key)).await.unwrap()),
    )
    .await;
    let orders_id = manager
        .subscribe(
            paradex::ws::Channel::Orders {
                market_symbol: None,
            },
            Box::new(|message| info!("Received order update {message:?}")),
        )
        .await
        .unwrap();
    let fills_id = manager
        .subscribe(
            paradex::ws::Channel::Fills {
                market_symbol: None,
            },
            Box::new(|message| info!("Received fill {message:?}")),
        )
        .await
        .unwrap();
    let position_id = manager
        .subscribe(
            paradex::ws::Channel::Position,
            Box::new(|message| info!("Received position {message:?}")),
        )
        .await
        .unwrap();
    let account_id = manager
        .subscribe(
            paradex::ws::Channel::Account,
            Box::new(|message| info!("Received account {message:?}")),
        )
        .await
        .unwrap();
    let balance_id = manager
        .subscribe(
            paradex::ws::Channel::BalanceEvents,
            Box::new(|message| info!("Received balance event {message:?}")),
        )
        .await
        .unwrap();
    let funding_payments_id = manager
    .subscribe(
        paradex::ws::Channel::FundingPayments { market_symbol: None },
        Box::new(|message| info!("Received funding payment {message:?}")),
    )
    .await
    .unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let order_request = OrderRequest {
        instruction: paradex::structs::OrderInstruction::GTC,
        market: symbol,
        price: Decimal::from_f64(90000.0),
        side: Side::SELL,
        size: Decimal::from_f64(0.005).unwrap(),
        order_type: OrderType::LIMIT,
        client_id: Some("A".into()),
        flags: vec![],
        recv_window: None,
        stp: None,
        trigger_price: None,
    };
    info!("Sending order {order_request:?}");
    let result = client_private.create_order(order_request).await.unwrap();
    info!("Order result {result:?}");

    tokio::time::sleep(Duration::from_secs(30)).await;

    info!("Cancel Order Result {:?}", client_private.cancel_order(result.id.clone()).await);

    for id in [orders_id, fills_id, position_id, account_id, balance_id, funding_payments_id] {
        manager.unsubscribe(id).await.unwrap();
    }

    tokio::time::sleep(Duration::from_secs(5)).await;
    manager.stop().await.unwrap();
}
