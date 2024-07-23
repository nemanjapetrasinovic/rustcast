use async_trait::async_trait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Podcast::Table)
                        .if_not_exists()
                            .col(ColumnDef::new(Podcast::Id).integer().not_null().auto_increment().primary_key())
                            .col(ColumnDef::new(Podcast::Title).string())
                            .col(ColumnDef::new(Podcast::Link).string())
                            .col(ColumnDef::new(Podcast::Description).string())
                            .to_owned()
            ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Podcast::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Podcast {
    Table,
    Id,
    Title,
    Link,
    Description,
}
