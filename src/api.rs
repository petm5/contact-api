use std::sync::Arc;
use std::env;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::http::primitives::{HttpRequest, HttpResponse, HttpStatus, HttpMethod, HttpPayload};
use crate::mail::Mailer;

pub struct Config {
    pub final_url: String,
    pub error_url: String,
    pub mailer: Mutex<Mailer>,
}

impl Config {
    pub fn from_env() -> Self {
        let relay = env::var("SMTP_RELAY").unwrap();
        let username = env::var("USER").unwrap();
        let password = env::var("PASSWORD").unwrap();
        let recipient = env::var("SENDTO").unwrap();
        let site = env::var("DOMAIN").unwrap();
        let final_url = env::var("SUCCESS_URL").unwrap();
        let error_url = env::var("ERROR_URL").unwrap();

        let mailer = Mutex::new(Mailer::new(username, password, relay, recipient, site));

        Self {
            final_url,
            error_url,
            mailer
        }
    }
}

fn api_handler(config: Arc<Config>, formdata: HashMap<String, String>) -> Option<()> {

    let name = formdata.get("name")?.clone();
    let email = formdata.get("email")?.clone();
    let subject = formdata.get("subject")?.clone();
    let message = formdata.get("message")?.clone();

    let body = format!("From: {} <{}>\r\n\r\n{}", name, email, message);

    let mail_result = config.mailer.lock().unwrap().send(subject, body);

    if let Err(error) = mail_result {
        println!("Mail error: {error}");
    }

    Some(())

}

pub fn http_handler(config: Arc<Config>, request: HttpRequest) -> HttpResponse {
    match (request.method, request.path.as_str(), request.payload) {
        (HttpMethod::Post, "/submit", Some(HttpPayload::KeyValue(formdata))) => {
            let result = api_handler(config.clone(), formdata);
            if result.is_some() {
                HttpResponse {
                    status: HttpStatus::Found,
                    headers: vec![format!("Location: {}", config.final_url)]
                }
            } else {
                HttpResponse {
                    status: HttpStatus::Found,
                    headers: vec![format!("Location: {}", config.error_url)]
                }
            }
        },
        _ => HttpResponse {
            status: HttpStatus::NotFound,
            headers: vec![]
        }
    }
}
