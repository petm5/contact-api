use tokio::{io::BufStream, net::TcpListener};
use tracing::info;

mod http;
mod mail;

mod api;

static DEFAULT_PORT: &str = "8080";

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    tracing_subscriber::fmt::init();

    let port: u16 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_PORT.to_string())
        .parse()?;

    let api = api::Api::init();

    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();

    info!("listening on: {}", listener.local_addr()?);

    loop {
        let (stream, addr) = listener.accept().await?;
        let mut stream = BufStream::new(stream);
        let mut api = api.clone();

        tokio::spawn(async move {
            match http::req::parse_request(&mut stream).await {
                Ok(req) => {
                    info!(?addr, ?req, "incoming request");

                    let resp = api.route_http(req).await;

                    resp.write(&mut stream).await.unwrap();
                },
                Err(e) => {
                    info!(?e, "failed to parse request");
                }
            }
        });
    }

} 
