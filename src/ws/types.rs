use crate::error;
use crate::structs::{
    AccountInformation, BBO, BalanceEvent, Fill, FundingData, FundingPayment, MarketSummary,
    OrderBook, OrderUpdate, Position, Trade,
};
use jsonrpsee_types::Notification;
use serde_json::Value;
use std::string::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identifier(pub(crate) u64);

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
        channel_name: Option<String>,
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
    pub fn channel_name(&self) -> String {
        match self {
            Channel::MarketSummary => "markets_summary".into(),
            Channel::BBO { market_symbol } => format!("bbo.{market_symbol}"),
            Channel::Trades { market_symbol } => format!("trades.{market_symbol}"),
            Channel::OrderBook {
                market_symbol,
                channel_name,
                refresh_rate,
                price_tick,
            } => format!(
                "order_book.{}.{}@15@{}{}",
                market_symbol,
                channel_name
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("snapshot"),
                refresh_rate,
                if let Some(tick) = price_tick {
                    format!("@{tick}")
                } else {
                    String::new()
                }
            ),
            Channel::OrderBookDeltas { market_symbol } => {
                format!("order_book.{market_symbol}.deltas")
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

    pub fn to_message(&self, notification: Notification<Value>) -> Message {
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
