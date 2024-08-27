use std::{
    env,
    io::{prelude::*, BufReader, Lines},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, mpsc},
};

mod http;
mod formdata;
mod lib;

use lib::ThreadPool;

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

fn main() {

    let relay = env::var("SMTP_RELAY").unwrap();
    let username = env::var("USER").unwrap();
    let password = env::var("PASSWORD").unwrap();
    let recipient = env::var("SENDTO").unwrap();
    let site = env::var("DOMAIN").unwrap();
    let final_url = env::var("SUCCESS_URL").unwrap();
    let error_url = env::var("ERROR_URL").unwrap();

    let creds = Credentials::new(username, password);

    let transport = SmtpTransport::relay(relay.as_str())
        .unwrap()
        .credentials(creds)
        .build();

    let mailer = Arc::new(Mailer { transport, recipient, site });

    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("Listening...");

    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let final_url = final_url.clone();
        let error_url = error_url.clone();
        let mailer = mailer.clone();

        pool.execute(|id| {
            handle_connection(id, stream, final_url, error_url, mailer);
        });
    }

}

struct Mailer {
    transport: SmtpTransport,
    recipient: String,
    site: String
}

impl Mailer {
    fn send(&self, subject: String, body: String) -> Result<(), String> {

        let email = Message::builder()
            .from(format!("{} <noreply@{}>", self.site, self.site).parse().unwrap())
            .to(format!("<{}>", self.recipient).parse().unwrap())
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .unwrap();
        
        self.transport.send(&email).map(|_| Ok(())).or_else(|e| Err(format!("{e:?}")))?
        
    }
}

enum StatusCode {
    Empty,
    Ok,
    BadRequest,
    NotFound,
    Error,
}

fn handle_http_submission<T: std::io::Read>(mut reader: &mut BufReader<T>, mailer: Arc<Mailer>) -> Result<(), StatusCode> {

    let headers = http::read_headers(reader).ok_or(StatusCode::BadRequest)?;

    let formdata = formdata::read_multipart(&mut reader, headers.get("content-type").ok_or(StatusCode::BadRequest)?).ok_or(StatusCode::BadRequest)?;

    let name = formdata.get("name").ok_or(StatusCode::BadRequest)?.clone();
    let email = formdata.get("email").ok_or(StatusCode::BadRequest)?.clone();
    let subject = formdata.get("subject").ok_or(StatusCode::BadRequest)?.clone();
    let message = formdata.get("message").ok_or(StatusCode::BadRequest)?.clone();

    let body = format!("From: {} <{}>\r\n\r\n{}", name, email, message);

    mailer.send(subject, body).or_else(|e| {
        println!("Could not send email: {e}");
        Err(StatusCode::Error)
    })

}

fn handle_connection(worker_id: usize, mut stream: TcpStream, final_url: String, error_url: String, mailer: Arc<Mailer>) {

    println!("Worker {worker_id} accepted connection: {}", stream.peer_addr().unwrap());

    let mut reader = BufReader::new(&mut stream);

    http::read_request(&mut reader).and_then(|(method, path)| {
        println!("Worker {worker_id} got request: {method} {path}");
        match (method.as_str(), path.as_str()) {
            ("POST", "/") => {
                handle_http_submission(&mut reader, mailer)
                    .err()
                    .or(Some(StatusCode::Ok))
            },
            ("GET", "/healthcheck") => Some(StatusCode::Empty),
            _ => Some(StatusCode::NotFound)
        }
    }).or_else(|| {
        println!("Worker {worker_id} got malformed request (not HTTP?)");
        None
    }).and_then(|status: StatusCode| {
        let (response_code, headers, body) = match status {
            StatusCode::Ok => ("302 Found", vec![format!("Location: {final_url}")], ""),
            StatusCode::BadRequest => ("400 Bad Request", vec![], ""),
            StatusCode::NotFound => ("404 Not Found", vec![], ""),
            StatusCode::Error => ("302 Found", vec![format!("Location: {error_url}")], ""),
            StatusCode::Empty => ("204 No Content", vec![], "")
        };
        println!("Worker {worker_id} responded: {response_code}");
        let headers = headers.iter().fold(String::new(), |a, b| a + b + "\r\n");
        let response = format!("HTTP/1.1 {}\r\n{}\r\n{}", response_code, headers, body);
        stream.write_all(response.as_bytes()).unwrap_or_else(|err| println!("{err}"));
        Some(())
    });
}
