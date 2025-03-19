use anyhow;
use newzletter::{
    configuration::get_configuration,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");

    let subscriber = get_subscriber("newzletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration()?;
    let application = Application::build(configuration).await?;
    Ok(application.run_until_stopped().await?)
}
