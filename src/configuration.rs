use config::{Config, ConfigError};
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

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
    // U&P HPN
    // postgres://name:password@host:port/db_name
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        // U&P HPN
        // format!(
        //     "postgres://{}:{}@{}:{}",
        //     self.username,
        //     self.password.expose_secret(),
        //     self.host,
        //     self.port
        // )
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            // try encrypted connection, fallback to unencrypted if it fails
            // which what we will use in local development
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .username(&self.username)
            .password(&self.password.expose_secret())
            .host(&self.host)
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        // add on the (without_db() PgConnection) the database name
        self.without_db().database(&self.database_name)
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
        .expect("Failed to determine the current directory aka \"folder:)\" for windows users");

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

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{}, is not supported enviroment. Use either `local` or `production`.",
                other
            )),
        }
    }
}
