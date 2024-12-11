use std::{
    fmt::{Debug, Display, Formatter},
    collections::HashMap
};

use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use crate::http::formdata;

pub struct Request {
    pub method: Method,
    pub path: String,
    pub payload: Option<HashMap<String, String>>
}

#[derive(PartialEq, Debug)]
pub enum Method {
    Get,
    Post,
}

impl Debug for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{} {}\"", self.method, self.path)
    }
}

impl TryFrom<&str> for Method {
    type Error = anyhow::Error;

    fn try_from(method: &str) -> Result<Self, anyhow::Error> {
        match method.to_ascii_lowercase().as_str() {
            "get" => Ok(Self::Get),
            "post" => Ok(Self::Post),
            m => Err(anyhow::anyhow!("unsupported method: {m}"))
        }
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
        }
    }
}

pub async fn parse_request(mut stream: impl AsyncBufRead + Unpin) -> anyhow::Result<Request> {
    let mut line_buffer = String::new();
    stream.read_line(&mut line_buffer).await?;

    let mut parts = line_buffer.split_whitespace();

    let method = parts.next()
        .ok_or(anyhow::anyhow!("missing method"))
        .and_then(TryInto::try_into)?;
    let path = parts.next()
        .ok_or(anyhow::anyhow!("missing path"))
        .map(Into::into)?;

    let mut headers = HashMap::new();

    loop {
        line_buffer.clear();
        stream.read_line(&mut line_buffer).await?;

        if line_buffer.is_empty() || line_buffer == "\n" || line_buffer == "\r\n" {
            break;
        }

        let mut comps = line_buffer.split(":");
        let key = comps.next().ok_or(anyhow::anyhow!("missing header name"))?.to_ascii_lowercase();
        let value = comps
            .next()
            .ok_or(anyhow::anyhow!("missing header value"))?
            .trim();

        headers.insert(key.to_string(), value.to_string());
    }

    let payload = if method == Method::Post {
        let content_type = headers.get("content-type")
            .ok_or(anyhow::anyhow!("missing content type"))?;

        let boundary = formdata::parse_content_type_boundary(content_type)
            .ok_or(anyhow::anyhow!("missing multipart boundary"))?;

        Some(formdata::parse_multipart(&mut stream, &boundary).await?)
    } else {
        None
    };

    Ok(Request { method, path, payload })
}
