use std::{env, sync::LazyLock};

pub struct AppConfig {
    pub database_url: String,
    pub database_name: String,
    pub server_port: u16,
    pub bootstrap_server: String,
    pub command_topic: String,
    pub message_topic: String,
}

impl AppConfig {
    fn load() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let server_port = env::var("SERVER_PORT")?;
        let database_url = env::var("DATABASE_URL")?;
        let database_name = env::var("DATABASE_NAME")?;
        let bootstrap_server = env::var("KAFKA_BOOTSTRAP_SERVER")?;
        let command_topic = env::var("MATCHING_ENGINE_COMMAND_TOPIC")?;
        let message_topic = env::var("MATCHING_ENGINE_MESSAGE_TOPIC")?;
        Ok(Self {
            database_url,
            database_name,
            server_port: server_port.parse()?,
            bootstrap_server,
            command_topic,
            message_topic,
        })
    }
}

static GLOBAL_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| AppConfig::load().expect("Failed to load Configuration"));

pub fn get() -> &'static AppConfig {
    &GLOBAL_CONFIG
}
