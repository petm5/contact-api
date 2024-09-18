use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

pub struct Mailer {
    transport: SmtpTransport,
    recipient: String,
    domain: String
}

impl Mailer {
    pub fn new(username: String, password: String, relay: String, recipient: String, domain: String) -> Self {

        let creds = Credentials::new(username, password);

        let transport = SmtpTransport::relay(relay.as_str())
            .unwrap()
            .credentials(creds)
            .build();

        Self { transport, recipient, domain }

    }
    pub fn send(&self, subject: String, body: String) -> Result<(), String> {

        let email = Message::builder()
            .from(format!("{} <noreply@{}>", self.domain, self.domain).parse().unwrap())
            .to(format!("<{}>", self.recipient).parse().unwrap())
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .unwrap();
        
        self.transport.send(&email).map(|_| Ok(())).or_else(|e| Err(format!("{e:?}")))?
        
    }
}
