use super::types::{Channel, Message};
use crate::error;
use crate::structs::{
    AccountInformation, BBO, BalanceEvent, Fill, FundingData, FundingPayment, MarketSummary,
    OrderBook, OrderUpdate, Position, Trade,
};

/// High-level events surfaced to typed websocket callbacks.
pub enum ChannelEvent<'a, T> {
    Connected,
    Disconnected,
    Unsubscribed,
    Error(&'a error::Error),
    Data(&'a T),
}

/// Trait describing a typed subscription along with its payload.
pub trait SubscriptionSpec: Send + 'static {
    type Payload: Send + Sync + 'static;

    /// Build the websocket channel for this subscription.
    fn into_channel(self) -> Channel;

    /// Extract a typed payload from a raw message when it matches this subscription.
    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload>;
}

#[derive(Debug, Clone, Default)]
pub struct MarketSummarySubscription;

impl SubscriptionSpec for MarketSummarySubscription {
    type Payload = MarketSummary;

    fn into_channel(self) -> Channel {
        Channel::MarketSummary
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::MarketSummary(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct BboSubscription {
    pub market_symbol: String,
}

impl BboSubscription {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            market_symbol: symbol.into(),
        }
    }
}

impl SubscriptionSpec for BboSubscription {
    type Payload = BBO;

    fn into_channel(self) -> Channel {
        Channel::BBO {
            market_symbol: self.market_symbol,
        }
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::BBO(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct TradesSubscription {
    pub market_symbol: String,
}

impl TradesSubscription {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            market_symbol: symbol.into(),
        }
    }
}

impl SubscriptionSpec for TradesSubscription {
    type Payload = Trade;

    fn into_channel(self) -> Channel {
        Channel::Trades {
            market_symbol: self.market_symbol,
        }
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::Trades(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderBookSubscription {
    pub market_symbol: String,
    pub channel_name: Option<String>,
    pub refresh_rate: String,
    pub price_tick: Option<String>,
}

impl OrderBookSubscription {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            market_symbol: symbol.into(),
            channel_name: None,
            refresh_rate: "50ms".into(),
            price_tick: None,
        }
    }
}

impl SubscriptionSpec for OrderBookSubscription {
    type Payload = OrderBook;

    fn into_channel(self) -> Channel {
        Channel::OrderBook {
            market_symbol: self.market_symbol,
            channel_name: self.channel_name,
            refresh_rate: self.refresh_rate,
            price_tick: self.price_tick,
        }
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::OrderBook(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderBookDeltasSubscription {
    pub market_symbol: String,
}

impl OrderBookDeltasSubscription {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            market_symbol: symbol.into(),
        }
    }
}

impl SubscriptionSpec for OrderBookDeltasSubscription {
    type Payload = OrderBook;

    fn into_channel(self) -> Channel {
        Channel::OrderBookDeltas {
            market_symbol: self.market_symbol,
        }
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::OrderBookDeltas(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct FundingDataSubscription {
    pub market_symbol: Option<String>,
}

impl FundingDataSubscription {
    pub fn all() -> Self {
        Self {
            market_symbol: None,
        }
    }

    pub fn market(symbol: impl Into<String>) -> Self {
        Self {
            market_symbol: Some(symbol.into()),
        }
    }
}

impl SubscriptionSpec for FundingDataSubscription {
    type Payload = FundingData;

    fn into_channel(self) -> Channel {
        Channel::FundingData {
            market_symbol: self.market_symbol,
        }
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::FundingData(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrdersSubscription {
    pub market_symbol: Option<String>,
}

impl OrdersSubscription {
    pub fn all() -> Self {
        Self {
            market_symbol: None,
        }
    }

    pub fn market(symbol: impl Into<String>) -> Self {
        Self {
            market_symbol: Some(symbol.into()),
        }
    }
}

impl SubscriptionSpec for OrdersSubscription {
    type Payload = OrderUpdate;

    fn into_channel(self) -> Channel {
        Channel::Orders {
            market_symbol: self.market_symbol,
        }
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::Orders(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct FillsSubscription {
    pub market_symbol: Option<String>,
}

impl FillsSubscription {
    pub fn all() -> Self {
        Self {
            market_symbol: None,
        }
    }

    pub fn market(symbol: impl Into<String>) -> Self {
        Self {
            market_symbol: Some(symbol.into()),
        }
    }
}

impl SubscriptionSpec for FillsSubscription {
    type Payload = Fill;

    fn into_channel(self) -> Channel {
        Channel::Fills {
            market_symbol: self.market_symbol,
        }
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::Fills(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PositionSubscription;

impl SubscriptionSpec for PositionSubscription {
    type Payload = Position;

    fn into_channel(self) -> Channel {
        Channel::Position
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::Position(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AccountSubscription;

impl SubscriptionSpec for AccountSubscription {
    type Payload = AccountInformation;

    fn into_channel(self) -> Channel {
        Channel::Account
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::Account(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BalanceEventsSubscription;

impl SubscriptionSpec for BalanceEventsSubscription {
    type Payload = BalanceEvent;

    fn into_channel(self) -> Channel {
        Channel::BalanceEvents
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::BalanceEvent(data) = message {
            Some(data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct FundingPaymentsSubscription {
    pub market_symbol: Option<String>,
}

impl FundingPaymentsSubscription {
    pub fn all() -> Self {
        Self {
            market_symbol: None,
        }
    }

    pub fn market(symbol: impl Into<String>) -> Self {
        Self {
            market_symbol: Some(symbol.into()),
        }
    }
}

impl SubscriptionSpec for FundingPaymentsSubscription {
    type Payload = FundingPayment;

    fn into_channel(self) -> Channel {
        Channel::FundingPayments {
            market_symbol: self.market_symbol,
        }
    }

    fn extract<'a>(message: &'a Message) -> Option<&'a Self::Payload> {
        if let Message::FundingPayments(data) = message {
            Some(data)
        } else {
            None
        }
    }
}
