use crate::entity::podcast;

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
    pub episodes: Option<Vec<rss::Item>>,
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
            episodes: Default::default(),
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
