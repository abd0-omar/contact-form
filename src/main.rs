use anyhow;
use zero2prod_rewrite::{configuration::get_configuration, startup::Application};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Hello, world!");

    let configuration = get_configuration()?;

    let application = Application::build(&configuration).await?;

    Ok(application.run_until_stopped().await?)
}
