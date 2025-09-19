use std::fmt;

#[derive(Debug, Clone)]
pub enum RustcastError {
    Network(NetworkError),
    Database(DatabaseError),
    Rss(RssError),
    Player(PlayerError),
    Ui(UiError),
}

#[derive(Debug, Clone)]
pub enum NetworkError {
    RequestFailed(String),
    InvalidUrl(String),
    ConnectionTimeout,
    InvalidResponse(String),
}

#[derive(Debug, Clone)]
pub enum DatabaseError {
    ConnectionFailed(String),
    QueryFailed(String),
    DataNotFound(String),
    ConstraintViolation(String),
}

#[derive(Debug, Clone)]
pub enum RssError {
    ParseFailed(String),
    InvalidFeed(String),
    MissingRequiredField(String),
    UnsupportedFormat(String),
}

#[derive(Debug, Clone)]
pub enum PlayerError {
    OpenFailed(String),
    PlaybackFailed(String),
    SeekFailed(String),
    UnsupportedFormat(String),
}

#[derive(Debug, Clone)]
pub enum UiError {
    InvalidState(String),
    ComponentError(String),
}

impl fmt::Display for RustcastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RustcastError::Network(e) => write!(f, "Network error: {}", e),
            RustcastError::Database(e) => write!(f, "Database error: {}", e),
            RustcastError::Rss(e) => write!(f, "RSS error: {}", e),
            RustcastError::Player(e) => write!(f, "Player error: {}", e),
            RustcastError::Ui(e) => write!(f, "UI error: {}", e),
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            NetworkError::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
            NetworkError::ConnectionTimeout => write!(f, "Connection timeout"),
            NetworkError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => write!(f, "Database connection failed: {}", msg),
            DatabaseError::QueryFailed(msg) => write!(f, "Database query failed: {}", msg),
            DatabaseError::DataNotFound(item) => write!(f, "Data not found: {}", item),
            DatabaseError::ConstraintViolation(msg) => write!(f, "Constraint violation: {}", msg),
        }
    }
}

impl fmt::Display for RssError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RssError::ParseFailed(msg) => write!(f, "RSS parse failed: {}", msg),
            RssError::InvalidFeed(msg) => write!(f, "Invalid RSS feed: {}", msg),
            RssError::MissingRequiredField(field) => write!(f, "Missing required field: {}", field),
            RssError::UnsupportedFormat(format) => write!(f, "Unsupported format: {}", format),
        }
    }
}

impl fmt::Display for PlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayerError::OpenFailed(msg) => write!(f, "Failed to open media: {}", msg),
            PlayerError::PlaybackFailed(msg) => write!(f, "Playback failed: {}", msg),
            PlayerError::SeekFailed(msg) => write!(f, "Seek failed: {}", msg),
            PlayerError::UnsupportedFormat(format) => write!(f, "Unsupported format: {}", format),
        }
    }
}

impl fmt::Display for UiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiError::InvalidState(msg) => write!(f, "Invalid UI state: {}", msg),
            UiError::ComponentError(msg) => write!(f, "Component error: {}", msg),
        }
    }
}

impl std::error::Error for RustcastError {}
impl std::error::Error for NetworkError {}
impl std::error::Error for DatabaseError {}
impl std::error::Error for RssError {}
impl std::error::Error for PlayerError {}
impl std::error::Error for UiError {}

// Conversion traits for better ergonomics
impl From<sea_orm::DbErr> for RustcastError {
    fn from(err: sea_orm::DbErr) -> Self {
        RustcastError::Database(DatabaseError::QueryFailed(err.to_string()))
    }
}

impl From<ureq::Error> for RustcastError {
    fn from(err: ureq::Error) -> Self {
        match err {
            ureq::Error::Status(code, response) => {
                RustcastError::Network(NetworkError::RequestFailed(
                    format!("HTTP {}: {}", code, response.status_text())
                ))
            },
            ureq::Error::Transport(transport) => {
                RustcastError::Network(NetworkError::RequestFailed(transport.to_string()))
            },
        }
    }
}

impl From<rss::Error> for RustcastError {
    fn from(err: rss::Error) -> Self {
        RustcastError::Rss(RssError::ParseFailed(err.to_string()))
    }
}

pub type RustcastResult<T> = Result<T, RustcastError>;

// Helper functions for creating specific errors
impl RustcastError {
    pub fn network_invalid_url(url: &str) -> Self {
        RustcastError::Network(NetworkError::InvalidUrl(url.to_string()))
    }

    pub fn rss_missing_field(field: &str) -> Self {
        RustcastError::Rss(RssError::MissingRequiredField(field.to_string()))
    }

    pub fn player_open_failed(msg: &str) -> Self {
        RustcastError::Player(PlayerError::OpenFailed(msg.to_string()))
    }

    pub fn ui_invalid_state(msg: &str) -> Self {
        RustcastError::Ui(UiError::InvalidState(msg.to_string()))
    }

    pub fn user_friendly_message(&self) -> String {
        match self {
            RustcastError::Network(NetworkError::RequestFailed(_)) =>
                "Failed to connect to the podcast server. Please check your internet connection.".to_string(),
            RustcastError::Network(NetworkError::InvalidUrl(_)) =>
                "The podcast URL is invalid. Please check the URL and try again.".to_string(),
            RustcastError::Network(NetworkError::ConnectionTimeout) =>
                "Connection timed out. Please try again later.".to_string(),
            RustcastError::Rss(RssError::ParseFailed(_)) =>
                "Failed to parse the podcast feed. The feed may be malformed.".to_string(),
            RustcastError::Rss(RssError::MissingRequiredField(field)) =>
                format!("The podcast feed is missing required information: {}", field),
            RustcastError::Database(DatabaseError::ConnectionFailed(_)) =>
                "Database connection failed. Please restart the application.".to_string(),
            RustcastError::Player(PlayerError::OpenFailed(_)) =>
                "Failed to open the audio file. The file may be corrupted or in an unsupported format.".to_string(),
            RustcastError::Player(PlayerError::PlaybackFailed(_)) =>
                "Playback failed. Please try again.".to_string(),
            _ => self.to_string(),
        }
    }
}