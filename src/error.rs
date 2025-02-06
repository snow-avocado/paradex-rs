use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("Websocket Send Error: {0:?}")]
    WebSocketSend(String),
    #[error("Parse Error: {0:?}")]
    JsonParseError(String),
    #[error("Rest Error: {0:?}")]
    RestError(String),
    #[error("Rest Empty Response")]
    RestEmptyResponse,
    #[error("Deserialization Error: {0:?}")]
    DeserializationError(String),
    #[error("Starknet Error: {0:?}")]
    StarknetError(String),
    #[error("Type Conversion Error: {0:?}")]
    TypeConversionError(String),
    #[error("Time Error: {0:?}")]
    TimeError(String),
    #[error("Missing Private Key")]
    MissingPrivateKey,
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
