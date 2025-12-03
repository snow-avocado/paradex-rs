use crate::error::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_tuple::{Deserialize_tuple, Serialize_tuple};
use serde_with::{DisplayFromStr, serde_as};
use starknet_core::utils::cairo_short_string_to_felt;
use starknet_crypto::Felt;
use std::str::FromStr;

fn deserialize_string_to_f64<'de, D>(deserializer: D) -> std::result::Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(f64::NAN)
    } else {
        f64::from_str(&s).map_err(serde::de::Error::custom)
    }
}

fn deserialize_optional_string_to_f64<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    // First deserialize to an Option<String>
    let opt_str = Option::<String>::deserialize(deserializer)?;

    // Handle the Option
    match opt_str {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => f64::from_str(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

fn serialize_f64_as_string<S>(value: &f64, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn serialize_optional_f64_as_string<S>(
    value: &Option<f64>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        None => Ok(serializer.serialize_unit())?,
        Some(float) => serializer.serialize_str(&float.to_string()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultsContainer<T> {
    pub results: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BridgedToken {
    pub decimals: u32,
    pub l1_bridge_address: String,
    pub l1_token_address: String,
    pub l2_bridge_address: String,
    pub l2_token_address: String,
    pub name: String,
    pub symbol: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemConfig {
    pub block_explorer_url: String,
    pub bridged_tokens: Vec<BridgedToken>,
    pub environment: String,
    pub l1_chain_id: String,
    pub l1_core_contract_address: String,
    pub l1_operator_address: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub liquidation_fee: f64,
    pub oracle_address: String,
    pub paraclear_account_hash: String,
    pub paraclear_account_proxy_hash: String,
    pub paraclear_address: String,
    pub paraclear_decimals: u32,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub partial_liquidation_buffer: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub partial_liquidation_share_increment: f64,
    pub starknet_chain_id: String,
    pub starknet_fullnode_rpc_url: String,
    pub starknet_gateway_url: String,
    pub universal_deployer_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SystemStatus {
    Ok,
    Maintenance,
    CancelOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemState {
    pub status: SystemStatus,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemTimeResponse {
    #[serde_as(as = "DisplayFromStr")]
    pub server_time: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JWTToken {
    pub jwt_token: String,
}

#[cfg(feature = "onboarding")]
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct OnboardingUtm {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[cfg(feature = "onboarding")]
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct OnboardingRequest {
    pub public_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketing_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm: Option<OnboardingUtm>,
}

#[cfg(feature = "onboarding")]
impl OnboardingRequest {
    pub fn new(public_key_hex: impl Into<String>) -> Self {
        Self {
            public_key: public_key_hex.into(),
            marketing_code: None,
            referral_code: None,
            utm: None,
        }
    }

    pub fn with_marketing_code(mut self, code: impl Into<String>) -> Self {
        self.marketing_code = Some(code.into());
        self
    }

    pub fn with_referral_code(mut self, code: impl Into<String>) -> Self {
        self.referral_code = Some(code.into());
        self
    }

    pub fn with_utm(mut self, utm: OnboardingUtm) -> Self {
        self.utm = Some(utm);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketSummary {
    pub symbol: String,
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub mark_price: f64,
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub last_traded_price: f64,
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub bid: f64,
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub ask: f64,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_to_f64",
        serialize_with = "serialize_optional_f64_as_string"
    )]
    pub volume_24: Option<f64>,
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub total_volume: f64,
    pub created_at: u64,
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub underlying_price: f64,
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub open_interest: f64,
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub funding_rate: f64,
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub price_change_rate_24h: f64,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_to_f64",
        serialize_with = "serialize_optional_f64_as_string"
    )]
    pub bid_iv: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_to_f64",
        serialize_with = "serialize_optional_f64_as_string"
    )]
    pub ask_iv: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_to_f64",
        serialize_with = "serialize_optional_f64_as_string"
    )]
    pub last_iv: Option<f64>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_to_f64",
        serialize_with = "serialize_optional_f64_as_string"
    )]
    pub delta: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptionType {
    CALL,
    PUT,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Delta1CrossMarginParams {
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub imf_base: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub imf_factor: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub imf_shift: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub mmf_factor: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum KlineResolution {
    Min1 = 1,
    Min3 = 3,
    Min5 = 5,
    Min15 = 15,
    Min30 = 30,
    Hour1 = 60,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KlinePriceKind {
    Last,
    Mark,
    Underlying,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct KlineParams {
    /// Start time in UTC timestamp (milliseconds since epoch)
    pub start_at: u64,
    /// End time in UTC timestamp (milliseconds since epoch)
    pub end_at: u64,
    pub symbol: String,
    pub resolution: KlineResolution,
    pub price_kind: Option<KlinePriceKind>,
}

impl From<KlineParams> for Vec<(String, String)> {
    fn from(params: KlineParams) -> Self {
        let mut vec = vec![
            ("start_at".to_string(), params.start_at.to_string()),
            ("end_at".to_string(), params.end_at.to_string()),
            ("symbol".to_string(), params.symbol.clone()),
            (
                "resolution".to_string(),
                (params.resolution as u32).to_string(),
            ),
        ];
        if let Some(price_kind) = &params.price_kind {
            vec.push((
                "price_kind".to_string(),
                format!("{:?}", price_kind).to_lowercase(),
            ));
        }
        vec
    }
}

#[derive(Clone, Debug, Serialize_tuple, Deserialize_tuple, PartialEq)]
pub struct Kline {
    pub timestamp_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderBookParams {
    /// Defaults to 20
    pub depth: Option<u16>,
    /// Price tick for aggregation
    pub price_tick: Option<String>,
}

impl From<OrderBookParams> for Vec<(String, String)> {
    fn from(params: OrderBookParams) -> Self {
        let mut vec = Vec::new();
        if let Some(depth) = params.depth {
            vec.push(("depth".to_string(), depth.to_string()));
        }
        if let Some(price_tick) = params.price_tick {
            vec.push(("price_tick".to_string(), price_tick));
        }
        vec
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderBookResponse {
    /// List of Ask sizes and prices
    pub asks: Vec<(String, String)>,
    /// List of Bid sizes and prices
    pub bids: Vec<(String, String)>,
    /// Last update to the orderbook in milliseconds
    pub last_updated_at: u64,
    /// Market name
    pub market: String,
    /// Sequence number of the orderbook
    pub seq_no: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderBookInteractiveResponse {
    /// List of Ask sizes and prices
    pub asks: Vec<(String, String)>,
    /// Size on the best bid from API (excluding RPI)
    pub best_bid_api: (String, String),
    /// Last update to the orderbook in milliseconds
    pub last_updated_at: u64,
    /// Market name
    pub market: String,
    /// Sequence number of the orderbook
    pub seq_no: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MarketSummaryStatic {
    pub asset_kind: String,
    pub base_currency: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub clamp_rate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta1_cross_margin_params: Option<Delta1CrossMarginParams>,
    pub expiry_at: i64,
    pub funding_period_hours: u16,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub interest_rate: f64,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_to_f64",
        serialize_with = "serialize_optional_f64_as_string"
    )]
    pub iv_bands_width: Option<f64>,
    pub market_kind: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub max_funding_rate: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub max_funding_rate_change: f64,
    pub max_open_orders: i64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub max_order_size: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub max_tob_spread: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub min_notional: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_type: Option<OptionType>,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub oracle_ewma_factor: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub order_size_increment: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub position_limit: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub price_bands_width: f64,
    pub price_feed_id: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub price_tick_size: f64,
    pub quote_currency: String,
    pub settlement_currency: String,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string_to_f64",
        serialize_with = "serialize_optional_f64_as_string"
    )]
    pub strike_price: Option<f64>,
    pub symbol: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BBO {
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub bid: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub bid_size: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub ask: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub ask_size: f64,

    pub market: String,
    pub last_updated_at: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Side {
    BUY,
    SELL,
}

impl Side {
    pub fn felt(&self) -> Felt {
        match self {
            Side::BUY => Felt::ONE,
            Side::SELL => Felt::TWO,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TradeType {
    FILL,
    LIQUIDATION,
    RPI,
    TRANSFER,
    SETTLE_MARKET,
    BLOCK_TRADE,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub created_at: u64,
    pub id: String,
    pub market: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub price: f64,
    pub side: Side,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub size: f64,
    pub trade_type: TradeType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Level {
    pub side: Side,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub price: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub size: f64,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderBookUpdateType {
    #[serde(rename = "s")]
    Snapshot,
    #[serde(rename = "d")]
    Delta,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderBook {
    pub seq_no: u64,
    pub market: String,
    pub last_updated_at: u64,
    pub update_type: OrderBookUpdateType,
    pub deletes: Vec<Level>,
    pub inserts: Vec<Level>,
    pub updates: Vec<Level>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderInstruction {
    GTC,
    IOC,
    POST_ONLY,
    RPI,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    NEW,
    OPEN,
    CLOSED,
    UNTRIGGERED,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType {
    MARKET,
    LIMIT,
    STOP_MARKET,
    STOP_LIMIT,
    TAKE_PROFIT_LIMIT,
    TAKE_PROFIT_MARKET,
    STOP_LOSS_MARKET,
    STOP_LOSS_LIMIT,
}

impl OrderType {
    pub fn felt(&self) -> Result<Felt> {
        match self {
            OrderType::MARKET => cairo_short_string_to_felt("MARKET"),
            OrderType::LIMIT => cairo_short_string_to_felt("LIMIT"),
            OrderType::STOP_MARKET => cairo_short_string_to_felt("STOP_MARKET"),
            OrderType::STOP_LIMIT => cairo_short_string_to_felt("STOP_LIMIT"),
            OrderType::TAKE_PROFIT_LIMIT => cairo_short_string_to_felt("TAKE_PROFIT_LIMIT"),
            OrderType::TAKE_PROFIT_MARKET => cairo_short_string_to_felt("TAKE_PROFIT_MARKET"),
            OrderType::STOP_LOSS_MARKET => cairo_short_string_to_felt("STOP_LOSS_MARKET"),
            OrderType::STOP_LOSS_LIMIT => cairo_short_string_to_felt("STOP_LOSS_LIMIT"),
        }
        .map_err(|e| Error::StarknetError(e.to_string()))
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderFlags {
    REDUCE_ONLY,
    STOP_CONDITION_BELOW_TRIGGER,
    STOP_CONDITION_ABOVE_TRIGGER,
    INTERACTIVE,
    TARGET_STRATEGY_VWAP,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum STPType {
    EXPIRE_MAKER,
    EXPIRE_TAKER,
    EXPIRE_BOTH,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderRequest {
    pub instruction: OrderInstruction,
    pub market: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    pub side: Side,
    pub size: Decimal,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    pub flags: Vec<OrderFlags>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recv_window: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stp: Option<STPType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<Decimal>,
}

impl OrderRequest {
    pub(crate) fn into_order(self, signature: [Felt; 2], signature_timestamp: u128) -> Order {
        Order {
            instruction: self.instruction,
            market: self.market,
            price: self.price,
            side: self.side,
            size: self.size,
            order_type: self.order_type,
            client_id: self.client_id,
            flags: self.flags,
            recv_window: self.recv_window,
            stp: self.stp,
            trigger_price: self.trigger_price,
            signature,
            signature_timestamp,
        }
    }
}

fn serialize_signature_as_string<S>(
    value: &[Felt; 2],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!(
        r#"["{}","{}"]"#,
        value[0].to_bigint(),
        value[1].to_bigint()
    ))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Order {
    pub instruction: OrderInstruction,
    pub market: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    pub side: Side,
    #[serde(serialize_with = "serialize_signature_as_string")]
    pub signature: [Felt; 2],
    pub signature_timestamp: u128,
    pub size: Decimal,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    pub flags: Vec<OrderFlags>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recv_window: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stp: Option<STPType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<Decimal>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModifyOrderRequest {
    pub id: String,
    pub market: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    pub side: Side,
    pub size: Decimal,
    #[serde(rename = "type")]
    pub order_type: OrderType,
}

impl ModifyOrderRequest {
    pub(crate) fn into_modify_order(
        self,
        signature: [Felt; 2],
        signature_timestamp: u128,
    ) -> ModifyOrder {
        ModifyOrder {
            id: self.id,
            market: self.market,
            price: self.price,
            side: self.side,
            signature,
            signature_timestamp,
            size: self.size,
            order_type: self.order_type,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModifyOrder {
    pub id: String,
    pub market: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    pub side: Side,
    #[serde(serialize_with = "serialize_signature_as_string")]
    pub signature: [Felt; 2],
    pub signature_timestamp: u128,
    pub size: Decimal,
    #[serde(rename = "type")]
    pub order_type: OrderType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderUpdate {
    pub account: String,
    pub cancel_reason: String,
    pub client_id: String,
    pub created_at: u64,
    pub id: String,
    pub instruction: OrderInstruction,
    pub last_updated_at: u64,
    pub market: String,
    pub price: Option<Decimal>,
    pub remaining_size: Decimal,
    pub side: Side,
    pub size: Decimal,
    pub status: OrderStatus,
    pub timestamp: u64,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub seq_no: u64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub avg_fill_price: f64,
    pub received_at: u64,
    pub published_at: u64,
    pub flags: Vec<OrderFlags>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_price: Option<Decimal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderUpdates {
    pub results: Vec<OrderUpdate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FillLiquidity {
    TAKER,
    MAKER,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FillType {
    FILL,
    LIQUIDATION,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fill {
    pub client_id: String,
    pub created_at: u64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub fee: f64,
    pub fee_currency: String,
    pub id: String,
    pub liquidity: FillLiquidity,
    pub market: String,
    pub order_id: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub price: f64,
    pub side: Side,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub size: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub remaining_size: f64,
    //pub seq_no : u64, //in paradex documentation, but does not appear to be sent.
    pub fill_type: FillType,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub realized_pnl: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferStatus {
    PENDING,
    AVAILABLE,
    COMPLETED,
    FAILED,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferBridge {
    STARKGATE,
    LAYERSWAP,
    RHINOFI,
    HYPERLANE,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferDirection {
    IN,
    OUT,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferKind {
    DEPOSIT,
    WITHDRAWAL,
    UNWINDING,
    VAULT_DEPOSIT,
    VAULT_WITHDRAWAL,
    AUTO_WITHDRAWAL,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transfer {
    pub account: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub amount: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub auto_withdrawal_fee: f64,
    pub bridge: TransferBridge,
    pub counterparty: String,
    pub created_at: u64,
    pub direction: TransferDirection,
    pub external_account: String,
    pub external_chain: String,
    pub external_txn_hash: String,
    pub failure_reason: String,
    pub id: String,
    pub kind: TransferKind,
    pub last_updated_at: u64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub socialized_loss_factor: f64,
    pub status: TransferStatus,
    pub token: String,
    pub txn_hash: String,
    pub vault_address: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub vault_unwind_completion_percentage: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FundingPayment {
    pub id: String,
    pub market: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub payment: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub index: f64,
    pub fill_id: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FundingData {
    pub market: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub funding_index: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub funding_premium: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub funding_rate: f64,
    pub created_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountStatus {
    ACTIVE,
    LIQUIDATION,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountInformation {
    pub account: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub account_value: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub free_collateral: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub initial_margin_requirement: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub maintenance_margin_requirement: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub margin_cushion: f64,
    pub seq_no: u64,
    pub settlement_asset: String,
    pub status: AccountStatus,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub total_collateral: f64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarginConfig {
    pub market: String,
    pub leverage: u64,
    pub margin_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolated_margin_leverage: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountMarginConfigurations {
    pub account: String,
    pub configs: Vec<MarginConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountMarginUpdate {
    pub leverage: u64,
    pub margin_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountMarginUpdateResponse {
    pub account: String,
    pub leverage: u64,
    pub margin_type: String,
    pub market: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceEvent {
    pub fill_id: String,
    pub market: String,
    pub status: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub settlement_asset_balance_before: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub settlement_asset_balance_after: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub settlement_asset_price: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub funding_index: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub realized_pnl: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub fees: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub realized_funding: f64,
    pub created_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Balance {
    pub token: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub size: f64,
    pub last_updated_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Balances {
    pub results: Vec<Balance>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PositionStatus {
    OPEN,
    CLOSED,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PositionSide {
    SHORT,
    LONG,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub average_entry_price: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub average_entry_price_usd: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub cached_funding_index: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub cost: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub cost_usd: f64,
    pub id: String,
    pub last_fill_id: String,
    pub last_updated_at: u64,
    pub leverage: String,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub liquidation_price: f64,
    pub market: String,
    pub seq_no: u64,
    pub side: PositionSide,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub size: f64,
    pub status: PositionStatus,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub unrealized_funding_pnl: f64,
    #[serde(
        serialize_with = "serialize_f64_as_string",
        deserialize_with = "deserialize_string_to_f64"
    )]
    pub unrealized_pnl: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Positions {
    pub results: Vec<Position>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CancelByMarketResponse {
    pub market: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RestError {
    pub error: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorResult<T> {
    pub next: Option<String>,
    pub prev: Option<String>,
    pub results: Vec<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_error() {
        let text = r#"{"message":"rate limit exceeded"}"#;
        let error = serde_json::from_str::<RestError>(text).unwrap();
        assert_eq!(error.message, "rate limit exceeded");
        assert!(error.error.is_none());
    }
}
