use sea_orm::*;
use crate::entity::podcast;
use crate::podcasts_model::Podcast;

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
        let podcast_to_add = podcast::ActiveModel{
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
}
