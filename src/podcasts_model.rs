use crate::{data_provider, entity::podcast};
use sea_orm::*;

#[derive(Default, PartialEq, Debug, Clone)]
pub struct Podcast {
    pub id: Option<i32>,
    pub title: String,
    pub link: String,
    pub description: String
}

pub struct PodcastsModel {
    pub podcasts: Option<Vec<podcast::Model>>,
    pub current_podcast: Podcast,
    pub podcast_dialog: PodcastDialog,
}

#[derive(Default, PartialEq, Debug, Clone)]
pub struct PodcastDialog {
    pub title: String,
    pub link: String,
    pub description: String
}

impl PodcastsModel {
    pub fn new() -> Self {
        PodcastsModel {
            podcasts: Default::default(),
            current_podcast: Default::default(),
            podcast_dialog: Default::default(),
        }
    }
}

impl From<podcast::Model> for PodcastDialog {
    fn from(p: podcast::Model) -> Self {
        PodcastDialog {
            link: p.link.unwrap(),
            title: p.title.unwrap(),
            description: p.description.unwrap()
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

impl From<podcast::Model> for Podcast {
    fn from(p: podcast::Model) -> Self {
        Podcast {
            id: Some(p.id),
            link: p.link.unwrap(),
            title: p.title.unwrap(),
            description: p.description.unwrap()
        }
    }
}
