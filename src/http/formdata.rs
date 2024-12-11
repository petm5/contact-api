use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use std::collections::HashMap;

pub fn parse_content_type_boundary(content_type: &str) -> Option<String> {

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

fn parse_content_disposition(value: &str) -> Option<String> {
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

pub async fn parse_multipart(mut stream: impl AsyncBufRead + Unpin, boundary: &String) -> anyhow::Result<HashMap<String, String>> {
    let mut line_buffer = String::new();
    stream.read_line(&mut line_buffer).await?;

    if !line_buffer.trim().ends_with(boundary) {
        return Err(anyhow::anyhow!("cannot find multipart boundary"))
    }

    let mut parts = HashMap::new();

    'outer: loop {
        let content_disposition = loop {
            line_buffer.clear();
            stream.read_line(&mut line_buffer).await?;

            if line_buffer.is_empty() || line_buffer == "\n" || line_buffer == "\r\n" {
                break None;
            }
            
            let mut comps = line_buffer.split(":");
            let key = comps.next().ok_or(anyhow::anyhow!("missing multipart header name"))?;
            let value = comps
                .next()
                .ok_or(anyhow::anyhow!("missing multipart header value"))?
                .trim();

            if key.to_lowercase().as_str() == "content-disposition" {
                break Some(value);
            }
        };

        let content_disposition = content_disposition
            .ok_or(anyhow::anyhow!("missing multipart content disposition"))?;

        let part_name = parse_content_disposition(content_disposition)
            .ok_or(anyhow::anyhow!("missing multipart part name"))?;

        let mut data = String::new();

        loop {
            line_buffer.clear();
            stream.read_line(&mut line_buffer).await?;

            if line_buffer.starts_with(format!("--{boundary}").as_str()) {
                parts.insert(part_name, data.trim().to_owned());

                if line_buffer.trim().ends_with("--") {
                    break 'outer
                }

                break
            }

            data.push_str(&line_buffer.as_str());
        }
    }

    Ok(parts)
}
