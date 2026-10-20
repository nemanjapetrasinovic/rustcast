use crate::error::{RustcastError, RustcastResult, NetworkError};
use crate::podcasts_model::PodcastSearchResult;
use url::Url;
use serde::Deserialize;

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

#[derive(Deserialize)]
struct ItunesSearchResponse {
    results: Vec<ItunesPodcast>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItunesPodcast {
    #[serde(default)]
    collection_name: String,
    #[serde(default)]
    feed_url: String,
    #[serde(default)]
    artist_name: String,
    #[serde(default)]
    artwork_url100: String,
}

pub fn search_podcasts(query: &str) -> RustcastResult<Vec<PodcastSearchResult>> {
    let url = format!(
        "https://itunes.apple.com/search?term={}&media=podcast&limit=20",
        urlencoded(query)
    );

    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| RustcastError::Network(NetworkError::RequestFailed(e.to_string())))?;

    let body = response.into_string()
        .map_err(|e| RustcastError::Network(NetworkError::InvalidResponse(e.to_string())))?;

    let parsed: ItunesSearchResponse = serde_json::from_str(&body)
        .map_err(|e| RustcastError::Network(NetworkError::InvalidResponse(e.to_string())))?;

    Ok(parsed.results
        .into_iter()
        .filter(|p| !p.feed_url.is_empty())
        .map(|p| PodcastSearchResult {
            title: p.collection_name,
            feed_url: p.feed_url,
            artist: p.artist_name,
            artwork_url: p.artwork_url100,
        })
        .collect())
}

fn urlencoded(s: &str) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("", s)
        .finish()
        .trim_start_matches('=')
        .to_string()
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