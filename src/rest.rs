use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use log::trace;
use reqwest::header::{HeaderMap, HeaderValue};
use starknet_core::types::Felt;
use starknet_core::utils::cairo_short_string_to_felt;
use starknet_signers::SigningKey;

use crate::error::{Error, Result};
use crate::message::{account_address, auth_headers, sign_order};
use crate::structs::{
    AccountInformation, Balances, JWTToken, MarketSummaryStatic, OrderRequest, OrderUpdate, Positions, ResultsContainer, SystemConfig, BBO
};
use crate::url::URL;

pub struct Client {
    url: URL,
    client: reqwest::Client,
    l2_chain_private_key_account: Option<(Felt, SigningKey, Felt)>,
    jwt: Option<(SystemTime, String)>, // the current valid JWT and timestamp created
}

impl Client {
    pub async fn new(url: URL, l2_private_key_hex_str: Option<String>) -> Result<Self> {
        let mut new_client = Self {
            url,
            client: reqwest::Client::new(),
            l2_chain_private_key_account: None,
            jwt: None,
        };
        if let Some(hex_str) = l2_private_key_hex_str {
            let signing_key = SigningKey::from_secret_scalar(
                Felt::from_hex(hex_str.as_str())
                    .map_err(|e| Error::StarknetError(e.to_string()))?,
            );
            let public_key = signing_key.verifying_key();
            let system_config = new_client.system_config().await?;

            let account = account_address(
                public_key.scalar(),
                Felt::from_str(system_config.paraclear_account_proxy_hash.as_str())
                    .map_err(|e| Error::StarknetError(e.to_string()))?,
                Felt::from_str(system_config.paraclear_account_hash.as_str())
                    .map_err(|e| Error::StarknetError(e.to_string()))?,
            )
            .map_err(|e| Error::StarknetError(e.to_string()))?;

            let chain_id = cairo_short_string_to_felt(system_config.starknet_chain_id.as_str())
                .map_err(|e| Error::StarknetError(e.to_string()))?;

            new_client.l2_chain_private_key_account = Some((chain_id, signing_key, account));
        }
        Ok(new_client)
    }

    pub async fn system_config(&self) -> Result<SystemConfig> {
        self.request("/v1/system/config".into(), None::<String>, None).await
    }

    pub async fn markets(&self) -> Result<Vec<MarketSummaryStatic>> {
        self.request("/v1/markets".into(), None::<()>, None).await
        .map(|result_container : ResultsContainer<Vec<MarketSummaryStatic>> | result_container.results)
    }

    pub(crate) fn is_private(&self) -> bool {
        self.l2_chain_private_key_account.is_some()
    }

    pub async fn jwt(&mut self) -> Result<String> {
        if self.jwt.as_ref().is_none_or(|(ts, _jwt)| {
            SystemTime::now()
                .duration_since(*ts)
                .ok()
                .is_none_or(|duration| duration.as_secs() > 240)
        }) {
            let (l2_chain, signing_key, account) = self
                .l2_chain_private_key_account
                .as_ref()
                .ok_or(Error::MissingPrivateKey)?;
            let (timestamp, headers) = auth_headers(l2_chain, signing_key, account)?;
            trace!("Auth Headers {headers:?}");
            let token = self
                .request::<&'static str, JWTToken>("/v1/auth".into(), Some(""), Some(headers))
                .await
                .map(|s| s.jwt_token)?;
            self.jwt = Some((timestamp, token));
        }
        if let Some((_ts, jwt)) = &self.jwt {
            Ok(jwt.clone())
        } else {
            panic!("unexpected path")
        }
    }

    pub async fn bbo(&self, market_symbol: String) -> Result<BBO> {
        self.request(format!("/v1/bbo/{market_symbol}"), None::<String>, None)
            .await
    }

    pub async fn create_order(&mut self, order_request: OrderRequest) -> Result<OrderUpdate> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::TimeError(e.to_string()))?
            .as_millis();

        let (l2_chain, signing_key, account) = self
            .l2_chain_private_key_account
            .as_ref()
            .ok_or(Error::MissingPrivateKey)?;

        let order = sign_order(order_request, signing_key, timestamp, *l2_chain, *account)?;

        self.request_auth("/v1/orders".into(), Some(order)).await
    }

    pub async fn account_information(&mut self) -> Result<AccountInformation> {
        self.request_auth("/v1/account".into(), None::<()>).await
    }

    pub async fn balance(&mut self) -> Result<Balances> {
        self.request_auth("/v1/balance".into(), None::<()>).await
    }

    pub async fn positions(&mut self) -> Result<Positions> {
        self.request_auth("/v1/positions".into(), None::<()>).await
    }

    async fn request_auth<B: serde::Serialize, T: for<'de> serde::Deserialize<'de>>(
        &mut self,
        path: String,
        body: Option<B>,
    ) -> Result<T> {
        let jwt = self.jwt().await?;
        let mut header_map: HeaderMap<HeaderValue> = HeaderMap::with_capacity(1);
        header_map.insert("Authorization", format!("Bearer {jwt}").parse().unwrap());
        self.request(path, body, Some(header_map)).await
    }

    async fn request<B: serde::Serialize, T: for<'de> serde::Deserialize<'de>>(
        &self,
        path: String,
        body: Option<B>,
        additional_headers: Option<HeaderMap<HeaderValue>>,
    ) -> Result<T> {
        let url = format!("{}{path}", self.url.rest());
        let mut request = if let Some(body_object) = body {
            self.client.post(url).json(&body_object)
        } else {
            self.client.get(url)
        };

        request = request.header("Accept", "application/json");

        if let Some(headers) = additional_headers {
            request = request.headers(headers);
        }

        let result = request
            .send()
            .await
            .map_err(|e| Error::RestError(e.to_string()))?;
        let text = result
            .text()
            .await
            .map_err(|e| Error::RestError(e.to_string()))?;
        serde_json::from_str(&text)
            .map_err(|e| Error::DeserializationError(format!("Text: {text} Error: {e:?}")))
    }
}
