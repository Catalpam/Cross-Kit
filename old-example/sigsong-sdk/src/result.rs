use uniffi::deps::anyhow;
pub type SDKResult<T> = Result<T, SDKError>;

#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum SDKError {
    #[error("server error {reason}: {message}")]
    ServerError { reason: i32, message: String },

    #[error("network error while {context}: {message}")]
    Network { context: String, message: String },

    #[error("database error while {context}: {message}")]
    Database { context: String, message: String },

    #[error("failed to decode {entity}: {message}")]
    Decode { entity: String, message: String },

    #[error("unexpected empty response from server")]
    EmptyResponse,

    #[error("unexpected response variant")]
    ResponseTypeMismatch,

    #[error("task join failed: {message}")]
    Join { message: String },

    #[error("io error: {message}")]
    Io { message: String },

    #[error("{message}")]
    Other { message: String },
}

impl SDKError {
    pub fn network(context: impl Into<String>, source: reqwest::Error) -> Self {
        Self::Network { context: context.into(), message: source.to_string() }
    }

    pub fn database(context: impl Into<String>, source: sqlx::Error) -> Self {
        Self::Database { context: context.into(), message: source.to_string() }
    }

    pub fn decode(entity: impl Into<String>, source: prost::DecodeError) -> Self {
        Self::Decode { entity: entity.into(), message: source.to_string() }
    }
}

impl From<anyhow::Error> for SDKError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other { message: err.to_string() }
    }
}

impl From<std::io::Error> for SDKError {
    fn from(err: std::io::Error) -> Self {
        Self::Io { message: err.to_string() }
    }
}

impl From<tokio::task::JoinError> for SDKError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Join { message: err.to_string() }
    }
}
