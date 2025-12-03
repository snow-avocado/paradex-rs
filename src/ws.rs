use crate::url::URL;
use crate::{
    error::{Error, Result},
    rest::Client,
};
use futures_util::{SinkExt, stream::StreamExt};
use jsonrpsee_core::{params::ObjectParams, traits::ToRpcParams};
use jsonrpsee_types::{Notification, Response, ResponsePayload};
use log::{info, trace, warn};
use serde_json::Value;
use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::Entry},
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::spawn,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{client::IntoClientRequest, http::Uri},
};

mod subscription;
mod types;

pub use subscription::{
    AccountSubscription, BalanceEventsSubscription, BboSubscription, ChannelEvent,
    FillsSubscription, FundingDataSubscription, FundingPaymentsSubscription,
    MarketSummarySubscription, OrderBookDeltasSubscription, OrderBookSubscription,
    OrdersSubscription, PositionSubscription, SubscriptionSpec, TradesSubscription,
};
pub use types::{Channel, Identifier, Message};

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

type CallbackFn = Arc<dyn Fn(&Message) + Send + Sync + 'static>;

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

    pub async fn subscribe_typed<S, F>(&self, spec: S, callback: F) -> Result<Identifier>
    where
        S: SubscriptionSpec,
        F: for<'a> Fn(ChannelEvent<'a, S::Payload>) + Send + Sync + 'static,
    {
        let channel = spec.into_channel();
        let handler: CallbackFn = Arc::new(move |message: &Message| match message {
            Message::Connected => callback(ChannelEvent::Connected),
            Message::Disconnected => callback(ChannelEvent::Disconnected),
            Message::Unsubscribed => callback(ChannelEvent::Unsubscribed),
            Message::Error(err) => callback(ChannelEvent::Error(err)),
            _ => {
                if let Some(data) = S::extract(message) {
                    callback(ChannelEvent::Data(data));
                }
            }
        });

        self.subscribe(channel, handler).await
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
            match connect_async_with_config(request, None, true).await {
                Ok((mut connection, _response)) => {
                    if let Some(client) = rest_client.as_mut()
                        && client.is_private()
                    {
                        match client.jwt().await {
                            Ok(token) => {
                                let mut params = ObjectParams::new();
                                params.insert("bearer", token).unwrap();
                                let request =
                                    Self::request("auth", jsonrpsee_types::Id::Number(0), params);
                                let request_str = serde_json::to_string(&request).unwrap();
                                if let Err(e) = connection
                                    .send(tokio_tungstenite::tungstenite::protocol::Message::text(
                                        request_str,
                                    ))
                                    .await
                                {
                                    log::error!(
                                        "Error sending auth request {request:?} error {e:?}"
                                    );
                                }
                            }
                            Err(e) => {
                                log::error!("Could not retrieve jwt auth token {}", e);
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

        // Ping/pong configuration (hard-coded for now)
        // Change these constants here to adjust behavior.
        const PING_INTERVAL: Duration = Duration::from_secs(30);
        const MAX_MISSED_PONGS: u32 = 3;

        let mut missed_pongs: u32 = 0;
        let mut ping_ticker = tokio::time::interval(PING_INTERVAL);

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
                                            if let Some(channel_entry) = notification.params.get("channel")
                                                && let Some(channel_name) = channel_entry.as_str()
                                                    && let Some( (_connected, data) ) = subscriptions_by_channel.get(&Cow::Borrowed(channel_name))
                                                        && let Some( (channel, _, _) ) = data.first() {
                                                            let channel_message = channel.to_message(notification.clone());
                                                            for (_,_,callback) in data.iter() {
                                                                callback(&channel_message)
                                                            }
                                                        }

                                        }
                                        else if let Ok(response) = serde_json::from_str::<Response<Value>>(text.as_str()) {
                                            match response.payload {
                                                ResponsePayload::Success(result) => {
                                                    if let Some(channel_object) = result.get("channel")
                                                        && let Some(channel_name) = channel_object.as_str()
                                                            && let Some(value) = subscriptions_by_channel.get_mut(&Cow::Owned(channel_name.to_string())) {
                                                                value.0=true;
                                                                for (_channel, _id, callback) in &value.1 {
                                                                    callback(&Message::Connected);
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
                                    tokio_tungstenite::tungstenite::Message::Ping(_) => {
                                        // incoming ping from server - respond is automatic at tungstenite level, or ignore
                                        trace!("Received ping from server");
                                    },
                                    tokio_tungstenite::tungstenite::Message::Pong(_) => {
                                        // received pong from server -> reset missed pong counter
                                        missed_pongs = 0;
                                        info!("Received pong from server, resetting missed_pongs to 0");
                                    }
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

                        missed_pongs = 0;
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
                                        value.1.push( (channel, identifier, Arc::clone(&callback)) );
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
                                            if let Some( (_, elem_id, _) ) = vec.get(idx) && *elem_id == identifier {
                                                elem_index = Some(idx);
                                                break;
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

                _ = ping_ticker.tick() => {
                    // Send a ping periodically. If we already missed too many pongs, force a reconnect by closing.
                    if missed_pongs >= MAX_MISSED_PONGS {
                        warn!("Missed {} pongs (threshold {}), closing connection to reconnect", missed_pongs, MAX_MISSED_PONGS);
                        if let Err(e) = connection.close(None).await {
                            warn!("Error closing websocket after missed pongs: {:?}", e);
                        }
                        // let the connection drop and the existing reconnection logic handle resubscribe
                        continue;
                    }

                    match connection.send(tokio_tungstenite::tungstenite::protocol::Message::Ping(Vec::new().into())).await {
                        Ok(_) => {
                            missed_pongs = missed_pongs.saturating_add(1);
                            info!("Sent ping to websocket; missed_pongs={}", missed_pongs);
                        }
                        Err(e) => {
                            warn!("Error sending ping: {:?}. Closing connection to reconnect", e);
                            let _ = connection.close(None).await;
                        }
                    }
                }

            }
        }
        info!("Exiting websocket read loop");
    }
}
