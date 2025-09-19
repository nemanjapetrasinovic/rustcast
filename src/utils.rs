use crate::error::{RustcastError, RustcastResult, NetworkError};
use url::Url;

pub fn validate_podcast_url(url: &str) -> RustcastResult<()> {
    if url.trim().is_empty() {
        return Err(RustcastError::Network(NetworkError::InvalidUrl("URL cannot be empty".to_string())));
    }

    match Url::parse(url) {
        Ok(parsed_url) => {
            if !matches!(parsed_url.scheme(), "http" | "https") {
                return Err(RustcastError::Network(NetworkError::InvalidUrl(
                    "URL must use HTTP or HTTPS protocol".to_string()
                )));
            }
            Ok(())
        }
        Err(_) => Err(RustcastError::Network(NetworkError::InvalidUrl(
            "Invalid URL format".to_string()
        )))
    }
}

pub fn safe_network_request(url: &str) -> RustcastResult<ureq::Response> {
    validate_podcast_url(url)?;

    match ureq::get(url)
        .timeout(std::time::Duration::from_secs(30))
        .call()
    {
        Ok(response) => Ok(response),
        Err(ureq::Error::Status(code, _)) if code == 404 => {
            Err(RustcastError::Network(NetworkError::RequestFailed(
                "Podcast feed not found (404)".to_string()
            )))
        },
        Err(ureq::Error::Status(code, _)) if code >= 500 => {
            Err(RustcastError::Network(NetworkError::RequestFailed(
                "Server error - please try again later".to_string()
            )))
        },
        Err(e) => Err(RustcastError::from(e))
    }
}

pub fn safe_rss_parse(content: &str) -> RustcastResult<rss::Channel> {
    if content.trim().is_empty() {
        return Err(RustcastError::Rss(crate::error::RssError::InvalidFeed(
            "Feed content is empty".to_string()
        )));
    }

    match rss::Channel::read_from(content.as_bytes()) {
        Ok(channel) => {
            // Basic validation
            if channel.title().trim().is_empty() {
                return Err(RustcastError::Rss(crate::error::RssError::MissingRequiredField(
                    "Feed title is missing".to_string()
                )));
            }
            Ok(channel)
        }
        Err(e) => Err(RustcastError::from(e))
    }
}

pub fn validate_podcast_data(title: &str, link: &str, description: &str) -> RustcastResult<()> {
    if title.trim().is_empty() {
        return Err(RustcastError::Rss(crate::error::RssError::MissingRequiredField(
            "Podcast title cannot be empty".to_string()
        )));
    }

    validate_podcast_url(link)?;

    if description.len() > 2000 {
        return Err(RustcastError::Rss(crate::error::RssError::InvalidFeed(
            "Description is too long (max 2000 characters)".to_string()
        )));
    }

    Ok(())
}