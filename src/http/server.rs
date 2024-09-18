use std::io::{BufReader, BufRead, BufWriter, Write};

use crate::http::primitives::{HttpRequest, HttpPayload, HttpMethod, HttpResponse, read_headers};
use crate::http::formdata;

pub fn read_request<T>(stream: &mut T) -> Option<HttpRequest> where T: std::io::Read {
    let mut reader = BufReader::new(stream);

    let mut request_line = String::new();

    let _ = reader.read_line(&mut request_line).ok()?;

    let mut request_line_parts = request_line.split_whitespace();

    let method = request_line_parts.next()?;
    let path = request_line_parts.next()?.to_owned();
    let protocol = request_line_parts.next()?;

    if !protocol.starts_with("HTTP/") {
        return None
    }

    let method = HttpMethod::decode(method)?;

    let headers = read_headers(&mut reader)?;

    let should_read_payload = match method {
        HttpMethod::Post => true,
        _ => false
    };
    let payload = if should_read_payload {
        let content_type = headers.get("content-type")?;
        // TODO: Support formats other than multipart/form-data
        if let Some(boundary) = formdata::get_multipart_boundary(content_type) {
            Some(HttpPayload::KeyValue(
                formdata::read_multipart(&mut reader, &boundary)?
            ))
        } else {
            None
        }
    } else {
        None
    };

    Some(HttpRequest { method, path, payload })
}

pub fn respond<T>(stream: &mut T, response: HttpResponse) -> Result<(), std::io::Error> where T: std::io::Write {
    let mut writer = BufWriter::new(stream);

    writer.write(format!("HTTP/1.1 {}\r\n", response.status.encode()).as_bytes())?;

    let headers = response.encode_headers();
    writer.write(headers.as_bytes())?;

    Ok(())
}
