use contact_form::startup::build_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let application = build_router()?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    println!("listnening live on {}", listener.local_addr()?);
    axum::serve(listener, application).await?;
    Ok(())
}
