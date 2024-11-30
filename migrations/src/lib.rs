pub use sea_orm_migration::prelude::*;

mod m22062024_000001_create_podcast_table;
mod m22062024_000001_create_episode_table;
mod m26102024_000001_create_episode_state;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m22062024_000001_create_podcast_table::Migration),
            Box::new(m22062024_000001_create_episode_table::Migration),
            Box::new(m26102024_000001_create_episode_state::Migration)
        ]
    }
}

