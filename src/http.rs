use std::io::{BufReader, BufRead};
use std::collections::HashMap;
type KeyValue = HashMap<String, String>;

pub fn read_request<T>(reader: &mut BufReader<T>) -> Option<(String, String)> where T: std::io::Read {
    let mut request_line = String::new();
    
    let _ = reader.read_line(&mut request_line).unwrap();

    let mut request_line_parts = request_line.split_whitespace();

    let method = request_line_parts.next()?;
    let path = request_line_parts.next()?;
    let protocol = request_line_parts.next()?;

    if !protocol.starts_with("HTTP/") {
        return None
    }

    Some((method.to_string(), path.to_string()))
}

pub fn read_headers<T>(reader: &mut BufReader<T>) -> Option<KeyValue> where T: std::io::Read {
    let raw_headers = reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty());

    let mut headers = HashMap::new();

    for line in raw_headers {
        let mut parts = line.split(':');
        let key = parts.next()?;
        let value = parts.next()?.trim().to_string();
        headers.insert(key.to_lowercase().to_string(), value.to_string());
    }

    Some(headers)
}
