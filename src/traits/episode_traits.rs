use sea_orm::ActiveValue;
use crate::entity::episode;
use crate::error::{RustcastError, RustcastResult};

impl episode::ActiveModel {
    pub fn try_from_rss_item(value: rss::Item) -> RustcastResult<Self> {
        let title = value.title()
            .ok_or_else(|| RustcastError::rss_missing_field("title"))?
            .to_string();

        let link = value.enclosure()
            .ok_or_else(|| RustcastError::rss_missing_field("enclosure"))?
            .url()
            .to_string();

        let description = value.description()
            .unwrap_or("No description available")
            .to_string();

        let guid = value.guid()
            .map(|g| g.value().to_string())
            .unwrap_or_else(|| format!("generated-{}", link));

        let pub_date = value.pub_date()
            .map(|d| d.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        Ok(episode::ActiveModel {
            title: ActiveValue::Set(Some(title)),
            link: ActiveValue::Set(Some(link)),
            description: ActiveValue::Set(Some(description)),
            guid: ActiveValue::Set(Some(guid)),
            pub_date: ActiveValue::Set(Some(pub_date)),
            ..Default::default()
        })
    }
}

// Keep the old implementation for backward compatibility, but log warnings
impl From<rss::Item> for episode::ActiveModel {
    fn from(value: rss::Item) -> Self {
        match Self::try_from_rss_item(value) {
            Ok(model) => model,
            Err(e) => {
                log::warn!("Failed to parse RSS item safely, using fallback: {}", e);
                episode::ActiveModel {
                    title: ActiveValue::Set(Some("Unknown Episode".to_string())),
                    link: ActiveValue::Set(Some("".to_string())),
                    description: ActiveValue::Set(Some("Failed to parse episode data".to_string())),
                    guid: ActiveValue::Set(Some("invalid".to_string())),
                    pub_date: ActiveValue::Set(Some("Unknown".to_string())),
                    ..Default::default()
                }
            }
        }
    }
}
