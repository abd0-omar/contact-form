// use std::{fs, sync::LazyLock};

// use newzletter::{
//     configuration::{configure_database, get_configuration},
//     startup::Application,
//     telemetry::{get_subscriber, init_subscriber},
// };
// use reqwest::{Client, StatusCode};
// use serde::Serialize;
// use sqlx::sqlite::SqlitePool;
// use tokio::fs::remove_file;
// use uuid::Uuid;

// // Ensure that the `tracing` stack is only initialised once using `once_cell`
// static TRACING: LazyLock<()> = LazyLock::new(|| {
//     let default_filter_level = "info".to_string();
//     let subscriber_name = "test".to_string();
//     if std::env::var("TEST_LOG").is_ok() {
//         let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
//         init_subscriber(subscriber);
//     } else {
//         let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
//         init_subscriber(subscriber);
//     };
// });

// pub struct TestApp {
//     address: String,
//     pool: SqlitePool,
//     // to later delete it
//     db_path: String,
// }

// async fn spawn_app() -> anyhow::Result<TestApp> {
//     // The first time `initialize` is invoked the code in `TRACING` is executed.
//     // All other invocations will instead skip execution.
//     LazyLock::force(&TRACING);

//     fs::create_dir_all("scripts/a_place_for_test_dbs_to_spawn_in_it,supposed_to_be_empty_cuz_tests_terminate_after_success_execution/")?;

//     let configuration = {
//         let mut configuration = get_configuration().expect("Failed to read configuration");
//         configuration.application.port = 0;
//         configuration.database.database_path = format!("scripts/a_place_for_test_dbs_to_spawn_in_it,supposed_to_be_empty_cuz_tests_terminate_after_success_execution/{}", Uuid::new_v4().to_string());
//         configuration.database.create_if_missing = true;
//         configuration.database.journal_mode = "MEMORY".to_string();
//         configuration.database.synchronous = "OFF".to_string();
//         configuration.database.busy_timeout = 1;
//         configuration.database.foreign_keys = true;
//         configuration.database.auto_vacuum = "NONE".to_string();
//         configuration.database.page_size = 4096;
//         configuration.database.cache_size = "-10000".to_string();
//         configuration.database.mmap_size = "0".to_string();
//         configuration.database.temp_store = "MEMORY".to_string();
//         configuration
//     };

//     let pool = configure_database(&configuration.database).await?;

//     sqlx::migrate!("./migrations").run(&pool).await?;
//     let db_path = configuration.database.database_path.clone();
//     let application_host = configuration.application.host.clone();

//     let application = Application::build(configuration).await?;

//     let address = format!("http://{}:{}", application_host, application.port());

//     tokio::spawn(async move { application.run_until_stopped().await.unwrap() });
//     Ok(TestApp {
//         address,
//         pool,
//         db_path,
//     })
// }

// pub async fn cleanup_test_db(db_path: &String) -> Result<(), sqlx::Error> {
//     remove_file(&format!("{}.db", db_path)).await?;
//     Ok(())
// }

// #[tokio::test]
// pub async fn health_check_works() {
//     // Arrange
//     let TestApp {
//         address,
//         pool: _,
//         db_path,
//     } = spawn_app().await.unwrap();
//     let client = Client::new();

//     // Act
//     let response = client
//         .get(&format!("{}/health_check", address))
//         .send()
//         .await
//         .unwrap();

//     // Assert
//     assert!(response.status().is_success());
//     assert_eq!(response.content_length(), Some(0));
//     dbg!(&db_path);

//     cleanup_test_db(&db_path).await.unwrap();
// }

// #[derive(Serialize)]
// struct FormData {
//     name: Option<String>,
//     email: Option<String>,
// }

// #[tokio::test]
// pub async fn subscribe_returns_a_200_for_valid_form_data() {
//     // Arrange
//     let TestApp {
//         address,
//         pool,
//         db_path,
//     } = spawn_app().await.unwrap();
//     let client = Client::new();
//     let fake_user_form_data = FormData {
//         name: Some("abood".to_string()),
//         email: Some("3la_el_7doood@yahoo.com".to_string()),
//     };

//     // Act
//     let response = client
//         .post(format!("{}/subscriptions", address))
//         .form(&fake_user_form_data)
//         .send()
//         .await
//         .unwrap();
//     // Assert
//     let saved = sqlx::query!(
//         r#"
//     SELECT name, email
//     FROM subscriptions
//     "#
//     )
//     .fetch_one(&pool)
//     .await
//     .unwrap();

//     assert_eq!(response.status(), StatusCode::OK);
//     assert_eq!(saved.name, fake_user_form_data.name.unwrap());
//     assert_eq!(saved.email, fake_user_form_data.email.unwrap());

//     cleanup_test_db(&db_path).await.unwrap();
// }

// #[tokio::test]
// pub async fn subscribe_returns_a_422_when_data_is_missing() {
//     // Arrange
//     let TestApp {
//         address,
//         pool: _,
//         db_path,
//     } = spawn_app().await.unwrap();
//     let client = Client::new();

//     let test_cases = vec![
//         (
//             FormData {
//                 name: Some("abood".to_string()),
//                 email: None,
//             },
//             "missing the email",
//         ),
//         (
//             FormData {
//                 name: None,
//                 email: Some("email@email_proivderdotcom".to_string()),
//             },
//             "missing the name",
//         ),
//         (
//             FormData {
//                 name: None,
//                 email: None,
//             },
//             "missing both",
//         ),
//     ];
//     // Act
//     for (invalid_form, error_message) in test_cases {
//         let response = client
//             .post(format!("{}/subscriptions", address))
//             .form(&invalid_form)
//             .send()
//             .await
//             .unwrap();
//         // Assert
//         assert_eq!(
//             response.status(),
//             StatusCode::UNPROCESSABLE_ENTITY,
//             "the API did not fail with 422 Bad Request when the payload was {}",
//             error_message
//         );
//     }

//     cleanup_test_db(&db_path).await.unwrap();
// }

// #[tokio::test]
// async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
//     // Arrange
//     let app = spawn_app().await.unwrap();
//     let client = reqwest::Client::new();
//     let test_cases = [
//         (
//             FormData {
//                 name: Some("".to_string()),
//                 email: Some("hamada123@yahoo.com".to_string()),
//             },
//             "name present (gift) but empty",
//         ),
//         (
//             FormData {
//                 name: Some("hamada".to_string()),
//                 email: Some("".to_string()),
//             },
//             "empty email",
//         ),
//         (
//             FormData {
//                 name: Some("hamada".to_string()),
//                 email: Some("definitely-not-(blitzcrank)-an-email".to_string()),
//             },
//             "invalid email",
//         ),
//     ];

//     for (body, description) in test_cases {
//         // Act
//         let response = client
//             .post(&format!("{}/subscriptions", &app.address))
//             .form(&body)
//             .send()
//             .await
//             .expect("Failed to execute request.");

//         // Assert
//         assert_eq!(
//             StatusCode::BAD_REQUEST,
//             response.status(),
//             "The API did not return a 200 OK when the payload was {}.",
//             description
//         );
//     }

//     cleanup_test_db(&app.db_path).await.unwrap();
// }
