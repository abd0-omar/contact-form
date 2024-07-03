use config::{Config, ConfigError};

#[derive(serde::Deserialize)]
pub struct Settings {
    pub application_port: u16,
    pub database: DatabaseSettings,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    // U&P HPN
    // postgres://name:password@host:port/db_name
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string_with_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }

    pub fn connection_string_without_db(&self) -> String {
        // U&P HPN
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}

pub fn get_configuration() -> Result<Settings, ConfigError> {
    let settings: Settings = Config::builder()
        .add_source(config::File::with_name("configuration.yaml"))
        // .add_source(config::Environment::with_prefix("APP"))
        .build()?
        .try_deserialize()?;
    Ok(settings)
}
