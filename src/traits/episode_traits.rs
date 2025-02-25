use sea_orm::ActiveValue;

use crate::entity::episode;

impl From<rss::Item> for episode::ActiveModel {
    fn from(value: rss::Item) -> Self {
        episode::ActiveModel {
            title: ActiveValue::set(Some(value.title().unwrap().to_string())),
            link: ActiveValue::set(Some(value.enclosure().unwrap().url().to_string())),
            description: ActiveValue::set(Some(value.description().unwrap().to_string())),
            guid: ActiveValue::set(Some(value.guid().unwrap().value().to_string())),
            pub_date: ActiveValue::set(Some(value.pub_date().unwrap().to_string())),
            ..Default::default()
        }
    }
}
