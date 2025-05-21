use anyhow;
use newzletter::{
    configuration::get_configuration,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");

    // TODO:
    // check out sqlx --offline wal mode and see if it would benefit you
    // TODO:
    // dyn error is a trait object, check out the relevant chapter in the rust
    // book for in-depth indtrouction to trait objects
    let subscriber = get_subscriber("newzletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration()?;
    let application = Application::build(configuration).await?;
    Ok(application.run_until_stopped().await?)
}
