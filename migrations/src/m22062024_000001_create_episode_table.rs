use async_trait::async_trait;
use sea_orm_migration::prelude::*;

use crate::m22062024_000001_create_podcast_table::Podcast;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Episode::Table)
                        .if_not_exists()
                            .col(ColumnDef::new(Episode::Id).integer().not_null().auto_increment().primary_key())
                            .col(ColumnDef::new(Episode::PodcastId).integer().not_null())
                            .col(ColumnDef::new(Episode::Title).string())
                            .col(ColumnDef::new(Episode::Link).string())
                            .col(ColumnDef::new(Episode::Description).string())
                            .col(ColumnDef::new(Episode::Guid).uuid())
                            .col(ColumnDef::new(Episode::PubDate).date_time())
                            .foreign_key(
                                ForeignKey::create()
                                    .name("fk-episode-podcast-id")
                                    .from(Episode::Table, Episode::PodcastId)
                                    .to(Podcast::Table, Podcast::Id)
                                    .on_delete(ForeignKeyAction::Cascade)
                            )
                            .to_owned()
            ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Episode::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Episode {
    Table,
    Id,
    PodcastId,
    Title,
    Link,
    Description,
    Guid,
    PubDate
}
