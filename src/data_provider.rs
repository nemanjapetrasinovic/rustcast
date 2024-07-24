use sea_orm::*;
use crate::entity::podcast;

const DATABASE_URL: &str = "sqlite:///Users/nemanja/.rustcast.db?mode=rwc";

pub async fn add_podcast(title: &str, link: &str, desc: &str) -> Result<(), sea_orm::DbErr> {
    let db = Database::connect(DATABASE_URL).await?;

    let podcast_to_add = podcast::ActiveModel {
        title: ActiveValue::set(Some(title.to_owned())),
        link: ActiveValue::set(Some(link.to_owned())),
        description: ActiveValue::set(Some(desc.to_owned())),
        ..Default::default()
    };

    podcast_to_add.insert(&db).await?;
    Ok(())
}
