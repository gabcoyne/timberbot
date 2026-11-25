use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("game not ready (409): {hint}")]
    GameNotReady { hint: String },

    #[error("unauthorized (401): {message}")]
    Unauthorized { message: String },

    #[error("api error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("websocket error: {0}")]
    WebSocket(#[from] Box<tokio_tungstenite::tungstenite::Error>),
}

impl From<tokio_tungstenite::tungstenite::Error> for ClientError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, ClientError>;
