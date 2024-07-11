use contact_form::{
    configuration::get_configuration,
    email_client::EmailClient,
    startup::build_router,
    telemetry::{get_subscriber, init_subscriber},
};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = get_subscriber("zero2sixty".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("failed to read configuration");
    dbg!("don't forget to run postgres");
    // let pool = PgPoolOptions::new()
    //     .max_connections(5)
    //     .connect(&db_url)
    //     .await?;

    // no longer async, not sure exactly why
    let pool = PgPoolOptions::new().connect_lazy_with(configuration.database.with_db());

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    );

    let application = build_router(pool, email_client)?;
    let listener = tokio::net::TcpListener::bind(format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    ))
    .await?;
    println!("listnening live on {}", listener.local_addr()?);
    axum::serve(listener, application).await?;
    Ok(())
}
