use std::str::FromStr;

use config::{Config, ConfigError};
use secrecy::Secret;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::sqlite::SqliteConnectOptions;

use crate::domain::SubscriberEmail;

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    // add host alongside port
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub email_client: EmailClientSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connect_options_with_db_file_or_create_if_missing(&self) -> SqliteConnectOptions {
        let db_path = format!("sqlite://{}.db", self.database_name);

        SqliteConnectOptions::from_str(&db_path)
            .expect("Failed to connect to sqlite")
            .create_if_missing(true)
    }
}

#[derive(serde::Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }
}

pub fn get_configuration() -> Result<Settings, ConfigError> {
    // configuration/base.yaml
    // configuration/local.yaml
    // configuration/production.yaml
    let base_path = std::env::current_dir()
        .expect("Failed to determine the current directory aka \"folder\" for windows users");

    let configuration_directory = base_path.join("configuration");

    // Detect the running enviroment
    // Default to `local` if unspecified
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    let environment_filename = format!("{}.yaml", environment.as_str());

    let settings: Settings = Config::builder()
        // read the "default" configuration file
        .add_source(config::File::from(
            configuration_directory.join("base.yaml"),
        ))
        // also read the file specific environment
        .add_source(config::File::from(
            configuration_directory.join(environment_filename),
        ))
        // Add in settings from environment variables (with a prefix of APP and '__' as separator
        // E.g.`APP_APPLICATION__PORT=5001 would set `Settings.application.port`
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?
        .try_deserialize()?;

    Ok(settings)
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    fn as_str(&self) -> &'static str {
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
                "{}, is not supported enviroment. Use either `local` or `production`.",
                other
            )),
        }
    }
}
