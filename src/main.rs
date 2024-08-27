use std::env;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::{
    io::{prelude::*, BufReader, Lines},
    net::{TcpListener, TcpStream},
};

mod http;
mod formdata;

#[derive(Debug)]
pub struct MessageInfo {
    name: String,
    email: String,
    subject: String,
    message: String
}

fn main() {

    let username = env::var("USER").unwrap();
    let password = env::var("PASSWORD").unwrap();
    let recipient = env::var("SENDTO").unwrap();
    let site = env::var("SITE").unwrap();
    let final_url = env::var("FINALURL").unwrap();

    let creds = Credentials::new(username.to_owned(), password.to_owned());

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();

    let mut mail_stack: Vec<MessageInfo> = vec![];

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream, &mut mail_stack, &final_url);

        while mail_stack.len() > 0 {
            let message = mail_stack.pop().unwrap();
            send_email(&site, &recipient, &mailer, &message);
        }
    }

}

fn send_email(site: &str, recipient: &str, mailer: &SmtpTransport, message: &MessageInfo) -> Option<()> {

    let email = Message::builder()
        .from(format!("{site} <noreply@{site}>").parse().unwrap())
        .to(format!("<{recipient}>").parse().unwrap())
        .subject(message.subject.clone())
        .header(ContentType::TEXT_PLAIN)
        .body(format!("From: {} <{}>\r\n\r\n{}", message.name, message.email, message.message))
        .unwrap();

    match mailer.send(&email) {
        Ok(_) => (),
        Err(e) => println!("Could not send email: {e:?}"),
    }

    Some(())
    
}

fn handle_http_submission(stream: &TcpStream, mail_stack: &mut Vec<MessageInfo>) -> Result<(), &'static str> {

    let mut reader = BufReader::new(stream);

    let (method, path) = http::read_request(&mut reader).ok_or("400 Bad Request")?;

    match (method.as_str(), path.as_str()) {
        ("POST", "/") => {
            let headers = http::read_headers(&mut reader).ok_or("400 Bad Request\r\nStage: headers")?;
            let formdata = formdata::read_multipart(&mut reader, headers.get("content-type").ok_or("400 Bad Request\r\nStage: content-type")?).ok_or("400 Bad Request\r\nStage: multipart")?;
            let message = MessageInfo {
                name: formdata.get("name").ok_or("400 Bad Request\r\nStage: name")?.clone(),
                email: formdata.get("email").ok_or("400 Bad Request\r\nStage: email")?.clone(),
                subject: formdata.get("subject").ok_or("400 Bad Request\r\nStage: subject")?.clone(),
                message: formdata.get("message").ok_or("400 Bad Request\r\nStage: message")?.clone()
            };
            mail_stack.push(message);
            Ok(())
        },
        _ => Err("404 Not Found")
    }

}

fn handle_connection(mut stream: TcpStream, mut mail_stack: &mut Vec<MessageInfo>, final_url: &String) {

    let status_code = if let Err(status_code) = handle_http_submission(&mut stream, &mut mail_stack) {
        status_code.to_string()
    } else {
        format!("302 Found\r\nLocation: {final_url}").to_string()
    };

    let response = format!("HTTP/1.1 {status_code}\r\n\r\n");

    stream.write_all(response.as_bytes()).unwrap();
}
