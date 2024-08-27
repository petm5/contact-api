use std::io::{BufReader, BufRead};
use std::collections::HashMap;
type KeyValue = HashMap<String, String>;

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

pub fn read_multipart<T>(reader: &mut BufReader<T>, boundary: &String) -> Option<KeyValue> where T: std::io::Read {

    let boundary = &get_multipart_boundary(boundary)?;

    if !reader.lines().next()?.unwrap().ends_with(boundary) {
        return None
    }

    let mut parts = HashMap::new();

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
        loop {
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            if line.starts_with(format!("--{boundary}").as_str()) {
                let data = data.trim().to_owned();
                parts.insert(fname, data);
                if line.trim().ends_with("--") {
                    break 'outer
                }
                break
            }
            data.push_str(&line.as_str());
        }
    }

    Some(parts)
}
