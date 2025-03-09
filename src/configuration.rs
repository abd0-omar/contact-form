use std::{str::FromStr, time::Duration};

use config::{Config, ConfigError};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    sqlite::{SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    SqlitePool,
};

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(Deserialize)]
pub struct ApplicationSettings {
    // env vars are strings for the config crate, and it will fail to pick up
    // integers using standard deserialization routine from serde
    // small caveat
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub database_name: String,
    pub create_if_missing: bool,
    pub journal_mode: String,
    pub synchronous: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub busy_timeout: u64,
    pub foreign_keys: bool,
    pub auto_vacuum: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub page_size: u32,
    pub cache_size: String,
    pub mmap_size: String,
    pub temp_store: String,
}

pub async fn configure_database(config: &DatabaseSettings) -> anyhow::Result<SqlitePool> {
    // options -> pool -> migrate
    let options = config.connect_options()?;
    let pool = SqlitePool::connect_with(options).await?;
    // no need to migrate in prod, will migrate manually
    // sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

impl DatabaseSettings {
    pub fn connect_options(&self) -> anyhow::Result<SqliteConnectOptions> {
        let options =
            SqliteConnectOptions::from_str(&format!("sqlite://{}.db", self.database_name))?
                // maybe do create_if_missing false for prod
                // and for testing true
                // beacuse litestream will pull the db if it doesn't exist from s3
                // https://gist.github.com/snow-jallen/4875b641082690595cc49fe625cc57ac
                // the free tier render machine is low on RAM (512 MB), so won't go all out
                // on RAM settings
                .create_if_missing(self.create_if_missing)
                .journal_mode(SqliteJournalMode::from_str(&self.journal_mode)?)
                // .journal_mode(SqliteJournalMode::from_str("WAL")?)
                .synchronous(SqliteSynchronous::from_str(&self.synchronous)?)
                // cache_size = -20000
                // mmap size
                // temp store
                .busy_timeout(Duration::from_secs(self.busy_timeout))
                .foreign_keys(self.foreign_keys)
                .auto_vacuum(SqliteAutoVacuum::from_str(&self.auto_vacuum)?)
                .page_size(self.page_size)
                .pragma("cache_size", self.cache_size.to_owned())
                // 512MB
                .pragma("mmap_size", self.mmap_size.to_owned())
                .pragma("temp_store", self.temp_store.to_owned());

        Ok(options)
    }
}

pub fn get_configuration() -> Result<Settings, ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");

    // Detect the running environment.
    // Default to `local` if unspecified.
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");
    let environment_filename = format!("{}.yaml", environment.as_str());
    let settings = Config::builder()
        .add_source(config::File::from(
            configuration_directory.join("base.yaml"),
        ))
        .add_source(config::File::from(
            configuration_directory.join(environment_filename),
        ))
        // Add in settings from environment variables (with a prefix of APP and '__' as separator)
        // E.g. `APP_APPLICATION__PORT=5001 would set `Settings.application.port`
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}

/// The possible runtime environment for our application.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}
