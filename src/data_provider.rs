use sea_orm::*;
use crate::entity::podcast;
use crate::podcasts_model::Podcast;

const DATABASE_URL: &str = "sqlite:///Users/nemanja/.rustcast.db?mode=rwc";

pub async fn add_podcast(podcast: Podcast) -> Result<(), sea_orm::DbErr> {
    let db = Database::connect(DATABASE_URL).await?;
    let podcast_to_add = podcast::ActiveModel::from(podcast);
    podcast_to_add.insert(&db).await?;
    Ok(())
}
