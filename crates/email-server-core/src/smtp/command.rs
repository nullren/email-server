#[derive(Debug, Clone, Eq, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        assert_eq!(
            Command::from_bytes(b"HELO example.com"),
            Command::Helo("example.com".to_string())
        );
        assert_eq!(
            Command::from_bytes(b"MAIL FROM: <user@example.com>"),
            Command::MailFrom("<user@example.com>".to_string())
        );
        assert_eq!(
            Command::from_bytes(b"RCPT TO: <recipient@example.com>"),
            Command::RcptTo("<recipient@example.com>".to_string())
        );
        assert_eq!(Command::from_bytes(b"DATA"), Command::Data);
        assert_eq!(Command::from_bytes(b"QUIT"), Command::Quit);
        assert_eq!(Command::from_bytes(b"UNKNOWN"), Command::Unknown);
    }
}
