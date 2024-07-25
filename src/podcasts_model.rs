use crate::{data_provider, entity::podcast};
use sea_orm::*;

#[derive(Default, PartialEq, Debug, Clone)]
pub struct Podcast {
    pub title: String,
    pub link: String,
    pub description: String
}

pub struct PodcastsModel {
    pub podcasts: Vec<Podcast>,
    pub current_podcast: Podcast,
    pub new_podcast: Podcast
}

impl PodcastsModel {
    pub fn new() -> Self {
        PodcastsModel {
            podcasts: Default::default(),
            current_podcast: Default::default(),
            new_podcast: Default::default()
        }
    }
}

impl From<Podcast> for podcast::ActiveModel {
    fn from(p : Podcast) -> Self {
        podcast::ActiveModel {
            title: ActiveValue::set(Some(p.title.to_owned())),
            link: ActiveValue::set(Some(p.link.to_owned())),
            description: ActiveValue::set(Some(p.description.to_owned())),
            ..Default::default()
        }
    }
}
