#[derive(Debug)]
pub(crate) enum Command {
    Helo(String),
    MailFrom(String),
    RcptTo(String),
    Data,
    Quit,
    Unknown,
}

impl Command {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.starts_with(b"HELO") {
            Command::Helo(String::from_utf8_lossy(&bytes[5..]).trim().to_string())
        } else if bytes.starts_with(b"MAIL FROM:") {
            Command::MailFrom(String::from_utf8_lossy(&bytes[10..]).trim().to_string())
        } else if bytes.starts_with(b"RCPT TO:") {
            Command::RcptTo(String::from_utf8_lossy(&bytes[8..]).trim().to_string())
        } else if bytes.starts_with(b"DATA") {
            Command::Data
        } else if bytes.starts_with(b"QUIT") {
            Command::Quit
        } else {
            Command::Unknown
        }
    }
}
