use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

#[derive(Clone)]
pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email_postmark(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequestPostmark {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };
        self.http_client
            .post(&url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn send_email_mailgun(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/messages", self.base_url);

        // https://mailgun-docs.redoc.ly/docs/mailgun/api-reference/openapi-final/tag/Messages/#tag/Messages/operation/httpapi.(*apiHandler).handler-fm-18
        let form = reqwest::multipart::Form::new()
            .text::<&str, String>("from", self.sender.as_ref().to_string())
            .text::<&str, String>("to", recipient.into())
            .text("subject", subject.to_string())
            .text("html", html_content.to_string())
            .text("text", text_content.to_string());

        self.http_client
            .post(&url)
            .basic_auth("api", Some(self.authorization_token.expose_secret()))
            // .header("Content-Type", "multipart/form-data")
            .multipart(form)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequestPostmark<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcherPostmark;
    struct SendEmailBodyMatcherMailGun;

    impl wiremock::Match for SendEmailBodyMatcherPostmark {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                dbg!(&body);
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    impl wiremock::Match for SendEmailBodyMatcherMailGun {
        fn matches(&self, request: &Request) -> bool {
            // Content-Disposition: form-data; name="from"
            let body_string = std::str::from_utf8(&request.body)
                .expect("invalid Vec<u8> request body couldn't conver it to String");

            dbg!(&body_string);

            dbg!(&request.headers);

            let valid_bodies = vec![
                "Content-Disposition: form-data; name=\"from\"".as_bytes(),
                "Content-Disposition: form-data; name=\"to\"".as_bytes(),
                "Content-Disposition: form-data; name=\"subject\"".as_bytes(),
                "Content-Disposition: form-data; name=\"html\"".as_bytes(),
                "Content-Disposition: form-data; name=\"text\"".as_bytes(),
            ];
            valid_bodies.iter().all(|&valid_body| {
                request
                    .body
                    .windows(valid_body.len())
                    .any(|window| window == valid_body)
            })
        }
    }

    /// Generate a random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    /// Generate a random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    /// Generate a random subscriber email
    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    /// Get a test instance of `EmailClient`.
    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request_postmark() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcherPostmark)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let _ = email_client
            .send_email_postmark(email(), &subject(), &content(), &content())
            .await;

        // Assert
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request_mailgun() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("authorization"))
            // headers
            // {
            // "authorization": "Basic YXBpOk1mTlR0VFFXeUNaWXllN29EZA==",
            // "content-type": "multipart/form-data; boundary=7a1833ba80ea0801-d093cb4cf236f11b-5e3d1278be63541a-c8ff52205714f7ec",
            // "content-length": "1045",
            // "accept": "*/*",
            // "host": "127.0.0.1:44207",
            // }
            //
            // "content-type": "multipart/form-data; boundary=7a1833ba80ea0801-d093cb4cf236f11b-5e3d1278be63541a-c8ff52205714f7ec",
            // multipart/form-data; has boundary stuff next to so I can't check on that
            //
            // .and(header("Content-Type", "multipart/form-data"))
            .and(path("/messages"))
            .and(method("POST"))
            .and(SendEmailBodyMatcherMailGun)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let _ = email_client
            .send_email_mailgun(email(), &subject(), &content(), &content())
            .await;

        // Assert
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email_postmark(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email_postmark(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email_postmark(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }
}
