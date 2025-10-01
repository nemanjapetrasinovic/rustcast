use sea_orm::*;
use crate::entity::episode;
use crate::entity::podcast;
use crate::entity::episode_state;
use crate::error::{RustcastError, RustcastResult};

pub struct DataProvider {
    db: DatabaseConnection
}

impl DataProvider {
    pub fn new(db: DatabaseConnection) -> Self {
        DataProvider {
            db
        }
    }

    pub async fn add_podcast(&self, title: String, link: String, description: String) -> Result<(), sea_orm::DbErr> {
        let podcast_to_add = podcast::ActiveModel {
            title: ActiveValue::set(Some(title)),
            link: ActiveValue::set(Some(link)),
            description: ActiveValue::set(Some(description)),
            ..Default::default()
        };

        podcast_to_add.insert(&self.db).await?;
        Ok(())
    }

    pub async fn get_podcasts(&self) -> Result<Vec<podcast::Model>, sea_orm::DbErr> {
        let res: Vec<podcast::Model> = podcast::Entity::find()
            .all(&self.db)
        .await?;
        Ok(res)
    }

    pub async  fn get_all_episodes(&self, podcast_id: i32) -> Result<Vec<episode::Model>, sea_orm::DbErr> {
        let episodes: Vec<episode::Model> = episode::Entity::find()
            .filter(episode::Column::PodcastId.eq(podcast_id))
            .all(&self.db)
            .await?;

        Ok(episodes)
    }

    pub async fn add_episodes(&self, episodes: Vec<rss::Item>, podcast_id: i32) -> RustcastResult<()> {
        let mut active_models = Vec::new();
        let mut failed_count = 0;

        for rss_item in episodes {
            match episode::ActiveModel::try_from_rss_item(rss_item) {
                Ok(mut episode_model) => {
                    episode_model.podcast_id = ActiveValue::Set(podcast_id);
                    active_models.push(episode_model);
                }
                Err(e) => {
                    log::warn!("Failed to parse episode: {}", e);
                    failed_count += 1;
                }
            }
        }

        if active_models.is_empty() {
            return Err(RustcastError::Rss(crate::error::RssError::ParseFailed(
                "No valid episodes found in feed".to_string()
            )));
        }

        if failed_count > 0 {
            log::warn!("Failed to parse {} episodes out of {}", failed_count, active_models.len() + failed_count);
        }

        episode::Entity::insert_many(active_models).exec(&self.db).await?;
        Ok(())
    }

    pub async fn delete_episodes_by_podcast_id(&self, podcast_id: i32) -> Result<(), sea_orm::DbErr> {
        episode::Entity::delete_many().filter(episode::Column::PodcastId.eq(podcast_id)).exec(&self.db).await?;
        Ok(())
    }

    pub async  fn upsert_episode_state(&self, progress: f64, podcast_id: i32, link: &str) -> Result<(), sea_orm::DbErr> {
        let episode_state_active_model = episode_state::ActiveModel {
            time: ActiveValue::Set(progress),
            finished: ActiveValue::Set(false),
            podcast_id: ActiveValue::Set(podcast_id),
            ep_link: ActiveValue::Set(link.to_string()),
            ..Default::default()
        };

        episode_state::Entity::insert(episode_state_active_model)
            .on_conflict(
                sea_query::OnConflict::column(episode_state::Column::EpLink)
                    .update_column(episode_state::Column::Time)
                    .to_owned()
            )
            .exec(&self.db)
            .await?;

        Ok(())
    }

    pub async fn get_episode_state(&self, link: &str) -> Result<Option<episode_state::Model>, sea_orm::DbErr> {
        let res: Option<episode_state::Model> = episode_state::Entity::find()
            .filter(episode_state::Column::EpLink.eq(link))
            .one(&self.db)
            .await?;

        Ok(res)
    }

    pub async fn get_all_episode_states(&self, podcast_id: i32) -> Result<std::collections::HashMap<String, f64>, sea_orm::DbErr> {
        let states: Vec<episode_state::Model> = episode_state::Entity::find()
            .filter(episode_state::Column::PodcastId.eq(podcast_id))
            .all(&self.db)
            .await?;

        let mut result = std::collections::HashMap::new();
        for state in states {
            result.insert(state.ep_link, state.time);
        }

        Ok(result)
    }
}
