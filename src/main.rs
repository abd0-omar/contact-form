use contact_form::{
    configuration::get_configuration,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = get_subscriber("newzletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("failed to read configuration");
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
