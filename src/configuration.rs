use std::str::FromStr;

use config::{Config, ConfigError};
use serde::Deserialize;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

// I know no need for this type (over abstraction)
// but it make it more easy if we needed to swap to postgresql or something
#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!("sqlite://{}.db", self.database_name)
    }
}

pub fn get_configuration() -> Result<Settings, ConfigError> {
    let settings = Config::builder()
        .add_source(config::File::new(
            "configuration.yaml",
            config::FileFormat::Yaml,
        ))
        .build()?;

    settings.try_deserialize::<Settings>()
}

pub async fn configure_database(config: &DatabaseSettings) -> anyhow::Result<SqlitePool> {
    // options then pool then migrate
    let options =
        SqliteConnectOptions::from_str(&config.connection_string())?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
