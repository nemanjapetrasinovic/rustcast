use async_trait::async_trait;
use sea_orm_migration::prelude::*;

use crate::m22062024_000001_create_podcast_table::Podcast;
use crate::m22062024_000001_create_episode_table::Episode;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EpisodeState::Table)
                        .if_not_exists()
                            .col(ColumnDef::new(EpisodeState::Id).integer().not_null().auto_increment().primary_key())
                            .col(ColumnDef::new(EpisodeState::Time).float().not_null().default(0.0))
                            .col(ColumnDef::new(EpisodeState::Finished).boolean().not_null().default(false))
                            .col(ColumnDef::new(EpisodeState::PodcastId).integer().not_null())
                            .col(ColumnDef::new(EpisodeState::EpLink).string().not_null().unique_key())
                            .foreign_key(
                                ForeignKey::create()
                                    .name("fk-episode-state-podcast-id")
                                    .from(EpisodeState::Table, EpisodeState::PodcastId)
                                    .to(Podcast::Table, Podcast::Id)
                                    .on_delete(ForeignKeyAction::Cascade)
                            )
                            .to_owned()
            ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EpisodeState::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum EpisodeState {
    Table,
    Id,
    PodcastId,
    EpLink,
    Time,
    Finished
}
