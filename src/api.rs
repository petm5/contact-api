use std::{
    env,
    collections::HashMap,
};
use tokio::io::AsyncBufRead;
use tokio::sync::Mutex;
use std::sync::Arc;
use tracing::error;

use crate::http;

use crate::mail;

#[derive(Debug)]
struct Message {
    name: String,
    email: String,
    subject: String,
    message: String,
}

#[derive(Clone)]
pub struct Api {
    success_url: String,
    mailer: Arc<Mutex<mail::Mailer>>
}

impl TryFrom<HashMap<String, String>> for Message {
    type Error = anyhow::Error;

    fn try_from(payload: HashMap<String, String>) -> Result<Self, Self::Error> {
        Ok(Self {
            name: payload.get("name")
                .ok_or(anyhow::anyhow!("missing field: name"))?
                .to_owned(),
            email: payload.get("email")
                .ok_or(anyhow::anyhow!("missing field: email"))?
                .to_owned(),
            subject: payload.get("subject")
                .ok_or(anyhow::anyhow!("missing field: subject"))?
                .to_owned(),
            message: payload.get("message")
                .ok_or(anyhow::anyhow!("missing field: message"))?
                .to_owned(),
        })
    }
}

impl Api {
    pub fn init() -> Self {
        let relay = env::var("SMTP_RELAY").unwrap();
        let username = env::var("USER").unwrap();
        let password = env::var("PASSWORD").unwrap();
        let recipient = env::var("SENDTO").unwrap();
        let domain = env::var("DOMAIN").unwrap();

        let mailer = mail::Mailer::init(username, password, relay, recipient, domain);

        Self {
            success_url: env::var("SUCCESS_URL").unwrap(),
            mailer: Arc::new(Mutex::new(mailer))
        }
    }

    pub async fn route_http(&mut self, request: http::req::Request) -> http::resp::Response<impl AsyncBufRead + Unpin> {
        match request.method {
            http::req::Method::Post => {
                if let Some(payload) = request.payload {
                    match request.path.as_str() {
                        "/send" => {
                            return match TryInto::try_into(payload) {
                                Ok(message) => match self.send_mail(message).await {
                                    Ok(_) => http::resp::Response::redirect(&self.success_url),
                                    Err(e) => {
                                        error!(?e, "send mail error");
                                        http::resp::Response::from_html(
                                            http::resp::Status::ServerError,
                                            format!("{}", e)
                                        )
                                    }
                                }
                                Err(e) => http::resp::Response::from_html(
                                    http::resp::Status::BadRequest,
                                    format!("{}", e)
                                )
                            }
                        }
                        _ => ()
                    }
                } else {
                    return http::resp::Response::from_html(
                        http::resp::Status::BadRequest,
                        "missing payload"
                    )
                }
            }
            _ => ()
        }

        http::resp::Response::from_html(
            http::resp::Status::NotFound,
            "not found",
        )
    }

    async fn send_mail(&mut self, message: Message) -> anyhow::Result<()> {

        let body = format!("From: {} <{}>\r\n\r\n{}", message.name, message.email, message.message);

        self.mailer.lock().await.send(message.subject, body)

    }
}
