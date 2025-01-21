#[derive(Debug, Clone, Copy)]
pub enum URL {
    Production,
    Testnet,
}

impl URL {
    pub fn rest(&self) -> &str {
        match self {
            URL::Production => "https://api.prod.paradex.trade",
            URL::Testnet => "	https://api.testnet.paradex.trade",
        }
    }

    pub fn websocket(&self) -> &str {
        match self {
            URL::Production => "wss://ws.api.prod.paradex.trade/v1",
            URL::Testnet => "wss://ws.api.testnet.paradex.trade/v1",
        }
    }
}
