use std::io::{BufReader, BufRead};
use std::collections::HashMap;
use std::fmt;

type KeyValue = HashMap<String, String>;

pub fn read_headers<T>(reader: &mut BufReader<T>) -> Option<KeyValue> where T: std::io::Read {
    let mut headers = HashMap::new();

    for line in reader.lines() {
        let line = line.ok()?;
        if line.is_empty() {
            break
        }
        let mut parts = line.split(':');
        let key = parts.next()?;
        let value = parts.next()?.trim().to_string();
        headers.insert(key.to_lowercase().to_string(), value.to_string());
    }

    Some(headers)
}

pub enum HttpStatus {
    Found,
    NotFound,
}

impl std::fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

impl HttpStatus {
    pub fn encode(&self) -> &'static str {
        match self {
            Self::Found => "302 Found",
            Self::NotFound => "404 Not Found"
        }
    }
}

pub enum HttpMethod {
    Get,
    Post,
}

impl HttpMethod {
    pub fn decode(method: &str) -> Option<Self> {
        match method.to_ascii_lowercase().as_str() {
            "get" => Some(Self::Get),
            "post" => Some(Self::Post),
            _ => None
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let result = match self {
            Self::Get => "GET",
            Self::Post => "POST",
        };
        write!(f, "{}", result)
    }
}

pub enum HttpPayload {
    KeyValue(KeyValue)
}

pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub payload: Option<HttpPayload>
}

pub struct HttpResponse {
    pub status: HttpStatus,
    pub headers: Vec<String>,
}

impl HttpResponse {
    pub fn encode_headers(&self) -> String {
        let headers = self.headers.iter()
            .fold(String::new(), |a, b| a + b + "\r\n");
        format!("{}\r\n", headers)
    }
}
