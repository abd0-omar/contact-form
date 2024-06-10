use contact_form::Application;

#[tokio::main]
async fn main() {
    let application = Application::run().await.unwrap();
    application.run_until_stopped().await.unwrap()
}
