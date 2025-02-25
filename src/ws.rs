use crate::structs::{AccountInformation, BalanceEvent, FundingPayment, Position};
use crate::url::URL;
use crate::{
    error::{self, Error, Result},
    rest::Client,
    structs::{Fill, FundingData, MarketSummary, OrderBook, OrderUpdate, Trade, BBO},
};
use futures_util::{stream::StreamExt, SinkExt};
use jsonrpsee_core::{params::ObjectParams, traits::ToRpcParams};
use jsonrpsee_types::{Notification, Response, ResponsePayload};
use log::{info, trace, warn};
use serde_json::Value;
use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::spawn,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::Uri},
    MaybeTlsStream, WebSocketStream,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identifier(u64);

#[derive(Debug, Clone)]
pub enum Message {
    //Control Messages
    Connected,
    Disconnected,
    Unsubscribed,
    Error(error::Error),

    //Public Channels
    BBO(BBO),
    MarketSummary(MarketSummary),
    OrderBook(OrderBook),
    OrderBookDeltas(OrderBook),
    Trades(Trade),
    FundingData(FundingData),

    //Private Channels
    Orders(OrderUpdate),
    Fills(Fill),
    Position(Position),
    Account(AccountInformation),
    BalanceEvent(BalanceEvent),
    FundingPayments(FundingPayment),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Channel {
    //Public Channels
    MarketSummary,
    OrderBook {
        market_symbol: String,
        refresh_rate: String,
        price_tick: Option<String>,
    },
    OrderBookDeltas {
        market_symbol: String,
    },
    BBO {
        market_symbol: String,
    },
    Trades {
        market_symbol: String,
    },
    FundingData {
        market_symbol: Option<String>,
    },

    //Private Channels
    Orders {
        market_symbol: Option<String>,
    },
    Fills {
        market_symbol: Option<String>,
    },
    Position,
    Account,
    BalanceEvents,
    FundingPayments {
        market_symbol: Option<String>,
    },
}

impl Channel {
    fn channel_name(&self) -> String {
        match self {
            Channel::MarketSummary => "markets_summary".into(),
            Channel::BBO { market_symbol } => format!("bbo.{market_symbol}"),
            Channel::Trades { market_symbol } => format!("trades.{market_symbol}"),
            Channel::OrderBook {
                market_symbol,
                refresh_rate,
                price_tick,
            } => format!(
                "order_book.{}.snapshot@15@{}{}",
                market_symbol,
                refresh_rate,
                if let Some(tick) = price_tick {
                    format!("@{}", tick)
                } else {
                    "".into()
                }
            ),
            Channel::OrderBookDeltas { market_symbol } => {
                format!("order_book.{}.deltas", market_symbol)
            }
            Channel::FundingData { market_symbol } => format!(
                "funding_data.{}",
                if let Some(s) = market_symbol {
                    s
                } else {
                    "ALL"
                }
            ),

            Channel::Orders { market_symbol } => format!(
                "orders.{}",
                if let Some(s) = market_symbol {
                    s
                } else {
                    "ALL"
                }
            ),
            Channel::Fills { market_symbol } => format!(
                "fills.{}",
                if let Some(s) = market_symbol {
                    s
                } else {
                    "ALL"
                }
            ),
            Channel::Position => "positions".into(),
            Channel::Account => "account".into(),
            Channel::BalanceEvents => "balance_events".into(),
            Channel::FundingPayments { market_symbol } => {
                format!(
                    "funding_payments.{}",
                    if let Some(s) = market_symbol {
                        s
                    } else {
                        "ALL"
                    }
                )
            }
        }
    }

    fn parse_notification<T: jsonrpsee_core::DeserializeOwned>(
        mut notification: Notification<Value>,
        function: impl Fn(T) -> Message,
    ) -> Message {
        if let Some(data) = notification.params.get_mut("data") {
            match serde_json::from_value::<T>(data.take()) {
                Ok(value) => function(value),
                Err(e) => Message::Error(error::Error::JsonParseError(e.to_string())),
            }
        } else {
            Message::Error(error::Error::JsonParseError(format!(
                "Notification missing data attribute {:?}",
                notification
            )))
        }
    }

    fn to_message(&self, notification: Notification<Value>) -> Message {
        match self {
            Channel::MarketSummary => {
                Self::parse_notification::<MarketSummary>(notification, Message::MarketSummary)
            }
            Channel::BBO { .. } => Self::parse_notification::<BBO>(notification, Message::BBO),
            Channel::Trades { .. } => {
                Self::parse_notification::<Trade>(notification, Message::Trades)
            }
            Channel::OrderBook { .. } => {
                Self::parse_notification::<OrderBook>(notification, Message::OrderBook)
            }
            Channel::OrderBookDeltas { .. } => {
                Self::parse_notification::<OrderBook>(notification, Message::OrderBookDeltas)
            }
            Channel::FundingData { .. } => {
                Self::parse_notification::<FundingData>(notification, Message::FundingData)
            }

            Channel::Orders { .. } => {
                Self::parse_notification::<OrderUpdate>(notification, Message::Orders)
            }
            Channel::Fills { .. } => Self::parse_notification::<Fill>(notification, Message::Fills),
            Channel::Position => {
                Self::parse_notification::<Position>(notification, Message::Position)
            }
            Channel::Account => {
                Self::parse_notification::<AccountInformation>(notification, Message::Account)
            }
            Channel::BalanceEvents => {
                Self::parse_notification::<BalanceEvent>(notification, Message::BalanceEvent)
            }
            Channel::FundingPayments { .. } => {
                Self::parse_notification::<FundingPayment>(notification, Message::FundingPayments)
            }
        }
    }
}

enum WebsocketOperation {
    Subscribe(Channel, CallbackFn, Identifier),
    Unsubscribe(Identifier),
    Stop,
}

#[derive(Clone)]
pub struct WebsocketManager {
    current_id: Arc<AtomicU64>,
    sub_sender: UnboundedSender<WebsocketOperation>,
}

type CallbackFn = Box<dyn Fn(&Message) + Send + 'static>;

impl WebsocketManager {
    pub async fn new(url: URL, rest_client: Option<Client>) -> Self {
        let (sub_sender, sub_receiver) =
            tokio::sync::mpsc::unbounded_channel::<WebsocketOperation>();
        spawn(Self::_reader(url, rest_client, sub_receiver));
        Self {
            current_id: Arc::new(AtomicU64::new(0)),
            sub_sender,
        }
    }

    pub async fn subscribe(&self, channel: Channel, callback: CallbackFn) -> Result<Identifier> {
        let identifier = Identifier(
            self.current_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        self.sub_sender
            .send(WebsocketOperation::Subscribe(channel, callback, identifier))
            .map_err(|e| Error::WebSocketSend(e.to_string()))?;
        Ok(identifier)
    }

    pub async fn unsubscribe(&self, identifier: Identifier) -> Result<()> {
        self.sub_sender
            .send(WebsocketOperation::Unsubscribe(identifier))
            .map_err(|e| Error::WebSocketSend(e.to_string()))?;
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        self.sub_sender
            .send(WebsocketOperation::Stop)
            .map_err(|e| Error::WebSocketSend(e.to_string()))?;
        Ok(())
    }

    async fn _connect(
        url: URL,
        rest_client: &mut Option<Client>,
    ) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
        loop {
            let request = url
                .websocket()
                .parse::<Uri>()
                .unwrap()
                .into_client_request()
                .unwrap();
            match connect_async(request).await {
                Ok((mut connection, _response)) => {
                    if let Some(client) = rest_client.as_mut() {
                        if client.is_private() {
                            match client.jwt().await {
                                Ok(token) => {
                                    let mut params = ObjectParams::new();
                                    params.insert("bearer", token).unwrap();
                                    let request = Self::request(
                                        "auth",
                                        jsonrpsee_types::Id::Number(0),
                                        params,
                                    );
                                    let request_str = serde_json::to_string(&request).unwrap();
                                    if let Err(e) = connection
                                        .send(
                                            tokio_tungstenite::tungstenite::protocol::Message::text(
                                                request_str,
                                            ),
                                        )
                                        .await
                                    {
                                        log::error!(
                                            "Error sending auth request {request:?} error {e:?}"
                                        );
                                    }
                                }
                                Err(e) => {
                                    log::error!(
                                        "Could not retrieve jwt auth token {}",
                                        e.to_string()
                                    );
                                }
                            }
                        }
                    }
                    return connection;
                }
                Err(e) => {
                    warn!("Error connecting to websocket {e:?}");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    fn request(
        method: &'static str,
        identifier: jsonrpsee_types::Id<'static>,
        object_params: ObjectParams,
    ) -> jsonrpsee_types::RequestSer<'static> {
        jsonrpsee_types::RequestSer::owned(
            identifier,
            method,
            object_params.to_rpc_params().ok().unwrap(),
        )
    }

    fn request_channel(
        method: &'static str,
        channel_name: String,
        identifier: Identifier,
    ) -> jsonrpsee_types::RequestSer<'static> {
        let mut params = ObjectParams::new();
        params.insert("channel", channel_name).unwrap();
        Self::request(method, jsonrpsee_types::Id::Number(identifier.0), params)
    }

    #[allow(clippy::type_complexity)]
    async fn _reader(
        url: URL,
        mut rest_client: Option<Client>,
        mut receiver: UnboundedReceiver<WebsocketOperation>,
    ) {
        let mut subscriptions_by_id: HashMap<Identifier, Cow<'_, str>> = HashMap::new();
        let mut subscriptions_by_channel: HashMap<
            Cow<'_, str>,
            (bool, Vec<(Channel, Identifier, CallbackFn)>),
        > = HashMap::new();
        let mut connection = Self::_connect(url, &mut rest_client).await;
        loop {
            tokio::select! {
                biased;

                message = connection.next() => {
                    if let Some(data) = message {
                        match data {
                            Ok(valid_message) => {
                                trace!("Received websocket message {valid_message:?}");
                                match valid_message {
                                    tokio_tungstenite::tungstenite::Message::Text(text) => {
                                        if let Ok(notification) = serde_json::from_str::<Notification<Value>>(text.as_str()) {
                                            if let Some(channel_entry) = notification.params.get("channel") {
                                                if let Some(channel_name) = channel_entry.as_str() {
                                                    if let Some( (_connected, data) ) = subscriptions_by_channel.get(&Cow::Borrowed(channel_name)) {
                                                        if let Some( (channel, _, _) ) = data.first() {
                                                            let channel_message = channel.to_message(notification.clone());
                                                            for (_,_,callback) in data.iter() {
                                                                callback(&channel_message)
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                        }
                                        else if let Ok(response) = serde_json::from_str::<Response<Value>>(text.as_str()) {
                                            match response.payload {
                                                ResponsePayload::Success(result) => {
                                                    if let Some(channel_object) = result.get("channel") {
                                                        if let Some(channel_name) = channel_object.as_str() {
                                                            if let Some(value) = subscriptions_by_channel.get_mut(&Cow::Owned(channel_name.to_string())) {
                                                                value.0=true;
                                                                for (_channel, _id, callback) in &value.1 {
                                                                    callback(&Message::Connected);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                ResponsePayload::Error(e) => {
                                                    warn!("Received error response {e:?} message {text:?} ");
                                                }
                                            }
                                        }
                                        else {
                                            warn!("Could not parse message {text:?}");
                                        }
                                    }
                                    tokio_tungstenite::tungstenite::Message::Ping(_) => {},
                                    _ => {warn!("Unexpected websocket message {valid_message}")},
                                }

                            }
                            Err(e) => {
                                warn!("Error in received message {e}");
                            }
                        }

                    }

                    else {
                        warn!("Websocket Disconnected");

                        for value in subscriptions_by_channel.values_mut() {
                            for (_channel, _id, callback) in &value.1 {
                                callback(&Message::Disconnected);
                            }
                        }

                        connection = Self::_connect(url, &mut rest_client).await;
                        let requests : Vec<jsonrpsee_types::RequestSer<'static>> = subscriptions_by_channel.iter()
                            .filter_map( |entry| if let Some( (_, identifier, _)) = entry.1.1.first() { Some(Self::request_channel("subscribe", entry.0.to_string(), *identifier))} else {None})
                            .collect();
                        for request in requests {
                            if let Err(e) = connection.send(tokio_tungstenite::tungstenite::protocol::Message::text(serde_json::to_string(&request).unwrap())).await {
                                log::error!("Error sending resubscribe request {e:?}");
                            }
                        }
                    }
                }

                operation = receiver.recv() => {
                    if let Some(action) = operation {
                        match action {
                            WebsocketOperation::Subscribe(channel, callback, identifier) => {
                                let channel_name = channel.channel_name();

                                subscriptions_by_id.insert(identifier, Cow::Owned(channel_name.clone()));
                                let entry = subscriptions_by_channel.entry(Cow::Owned(channel_name.clone()));
                                match entry {
                                    Entry::Occupied(mut occupied_entry) => {
                                        let value = occupied_entry.get_mut();
                                        if value.0 {
                                            callback(&Message::Connected);
                                        }
                                        value.1.push( (channel, identifier, callback) );
                                    }
                                    Entry::Vacant(vacant_entry) => {
                                        let request = Self::request_channel("subscribe", channel_name.clone(), identifier);
                                        if let Err(e) = connection.send(tokio_tungstenite::tungstenite::protocol::Message::text(serde_json::to_string(&request).unwrap())).await {
                                            log::error!("Error sending subscription request {request:?} error {e:?}");
                                        }
                                        vacant_entry.insert( (false, vec![(channel, identifier, callback)]) );
                                    }
                                }
                            },
                            WebsocketOperation::Unsubscribe(identifier) => {
                                if let Some(channel_name) = subscriptions_by_id.remove(&identifier) {
                                    if let Some((_,vec)) = subscriptions_by_channel.get_mut(&channel_name) {
                                        let mut elem_index = None;
                                        for idx in 0..vec.len() {
                                            if let Some( (_, elem_id, _) ) = vec.get(idx) {
                                                if *elem_id == identifier {
                                                    elem_index = Some(idx);
                                                    break;
                                                }
                                            }
                                        }
                                        if let Some(idx) = elem_index {
                                            let (_, _, callback) = vec.remove(idx);
                                            if vec.is_empty() {
                                                let request = Self::request_channel("unsubscribe", channel_name.to_string(), identifier);
                                                if let Err(e) = connection.send(tokio_tungstenite::tungstenite::protocol::Message::text(serde_json::to_string(&request).unwrap())).await {
                                                    log::error!("Error sending unsubscribe request {request:?} error {e:?}");
                                                }
                                                subscriptions_by_channel.remove(&channel_name);
                                            }
                                            callback(&Message::Unsubscribed);
                                        }
                                        else {
                                            warn!("Could not find {identifier:?} in subscriptions_by_channel");
                                        }
                                    }
                                    else {
                                        warn!("could not find subscription to remove {identifier:?}");
                                    }

                                }
                                else {
                                    warn!("Received unsubscribe request for {identifier:?} but could not locate subscription");
                                }
                            }
                            WebsocketOperation::Stop => {
                                warn!("Received websocket stop request. Stopping websocket read task");
                                break;
                            },
                        }
                    }
                    else { //senders closed. Should we exit?
                    }
                }

            }
        }
        info!("Exiting websocket read loop");
    }
}
