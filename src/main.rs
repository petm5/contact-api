use std::env;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::{
    io::{prelude::*, BufReader, Lines},
    net::{TcpListener, TcpStream},
};
use std::collections::HashMap;

type KeyValuePair = HashMap<String, String>;

fn main() {
    let username = env::var("USER");
    let password = env::var("PASSWORD");
    let site = env::var("SITE");
    let recipient = env::var("SENDTO");

    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }

    // let email = Message::builder()
    //     .from(format!("{} <{}>", site, username).parse().unwrap())
    //     .to(format!("<{}>", recipient).parse().unwrap())
    //     .subject(subject)
    //     .header(ContentType::TEXT_PLAIN)
    //     .body(body)
    //     .unwrap();

    // let creds = Credentials::new(username.to_owned(), password.to_owned());

    // let mailer = SmtpTransport::relay("smtp.gmail.com")
    //     .unwrap()
    //     .credentials(creds)
    //     .build();

    // match mailer.send(&email) {
    //     Ok(_) => println!("Email sent successfully!"),
    //     Err(e) => panic!("Could not send email: {e:?}"),
    // }
}

fn handle_http_request_line<T>(reader: &mut BufReader<T>) -> Option<(String, String)> where T: std::io::Read {
    let mut request_line = String::new();
    
    let _ = reader.read_line(&mut request_line).unwrap();

    let mut request_line_parts = request_line.split_whitespace();

    let method = request_line_parts.next()?;
    let path = request_line_parts.next()?;

    Some((method.to_string(), path.to_string()))
}

fn handle_http_headers<T>(reader: &mut BufReader<T>) -> Option<(String, Option<String>)> where T: std::io::Read {
    let mut content_type: Option<String> = None;
    let mut referer: Option<String> = None;

    let headers = reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty());

    for line in headers {
        let mut parts = line.split(':');
        let key = parts.next()?;
        let value = parts.next()?.trim().to_string();
        match key.to_lowercase().as_str() {
            "content-type" => content_type = Some(value),
            "referer" => referer = Some(value),
            &_ => ()
        }
    }

    Some((content_type?, referer))
}

fn get_multipart_boundary(content_type: &String) -> Option<String> {

    let mut params = content_type.split(';');

    if params.next()? != "multipart/form-data" {
        return None
    }

    let mut boundary: Option<String> = None;

    for param in params {
        let mut parts = param.split('=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim().to_string();
        match key.to_lowercase().as_str() {
            "boundary" => boundary = Some(value),
            &_ => ()
        }
    }

    return boundary
}

fn parse_content_disposition(value: &String) -> Option<String> {
    let mut cdisp = value.split(';');
    if cdisp.next()? != "form-data" {
        return None
    }

    let mut name: Option<String> = None;

    while let Some(param) = cdisp.next() {
        let mut parts = param.split("=");
        let key = parts.next()?.trim();
        let value = parts.next()?.trim_matches(|c| c == '\"').to_string();
        match key.to_lowercase().as_str() {
            "name" => name = Some(value),
            &_ => ()
        }
    }

    return name
}

fn handle_multipart<T>(reader: &mut BufReader<T>, boundary: &String) -> Option<(String, String, String, String)> where T: std::io::Read {

    let (mut name, mut email, mut subject, mut message): (Option<String>, Option<String>, Option<String>, Option<String>) = (None, None, None, None);

    if !reader.lines().next()?.unwrap().ends_with(boundary) {
        return None
    }

    'outer: loop {
        let mut headers = reader.lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty());
        let mut fname: Option<String> = None;
        for header in headers {
            let mut parts = header.split(':');
            let key = parts.next()?;
            let value = parts.next()?.trim();
            match key.to_lowercase().as_str() {
                "content-disposition" => {
                    fname = parse_content_disposition(&value.to_owned());
                },
                &_ => ()
            }
        }
        let fname = fname?;
        let mut data = String::new();
        for line in reader.lines().map(|result| result.unwrap()) {
            if line.starts_with(format!("--{boundary}").as_str()) {
                let data = data.trim().to_owned();
                match fname.as_str() {
                    "name" => name = Some(data),
                    "email" => email = Some(data),
                    "subject" => subject = Some(data),
                    "message" => message = Some(data),
                    &_ => ()
                }
                if line.ends_with("--") {
                    break 'outer
                }
                break
            }
            data.push_str(format!("{line}\n").as_str());
        }
    }

    Some((name?, email?, subject?, message?))
}

fn handle_http_submission(stream: &TcpStream) -> Option<String> {

    let mut reader = BufReader::new(stream);

    let (method, path) = handle_http_request_line(&mut reader)?;

    if path != "/" {
        return Some("404 Not Found".to_string())
    }

    if method != "POST" {
        return Some("405 Method Not Allowed".to_string())
    }

    let (content_type, referer) = handle_http_headers(&mut reader)?;

    let multipart_boundary = get_multipart_boundary(&content_type)?;

    let (name, email, subject, message) = handle_multipart(&mut reader, &multipart_boundary)?;

    Some(if let Some(referer) = referer {
        format!("302 Found\r\nLocation: {referer}").to_string()
    } else {
        "200 OK".to_string()
    })
}

fn handle_connection(mut stream: TcpStream) {

    let status_code = handle_http_submission(&mut stream).unwrap_or("400 Bad Request".to_string());

    let response = format!("HTTP/1.1 {status_code}\r\n\r\n");

    stream.write_all(response.as_bytes()).unwrap();
}
