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

    pub async fn add_podcast(&self, podcast: Podcast) -> Result<(), sea_orm::DbErr> {
        let podcast_to_add = podcast::ActiveModel::from(podcast);
        podcast_to_add.insert(&self.db).await?;
        Ok(())
    }

    pub async fn get_podcasts(&self) -> Result<Vec<Podcast>, sea_orm::DbErr> {
        let podcasts: Vec<podcast::Model> = podcast::Entity::find()
            .all(&self.db)
        .await?;
        let p: Vec<Podcast> = podcasts.iter().map(|e| e.clone().into()).collect();
        println!("{:?}", p);
        Ok(p)
    }
}
