#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub enum SmtpState {
    #[default]
    Init,
    Mail,
    Rcpt,
    Data,
    Done,
}

#[derive(Default, Debug, Clone)]
pub struct SmtpMessage {
    pub sender_domain: String,
    pub from: String,
    pub to: Vec<String>,
    pub data: Vec<u8>,
}

#[derive(Default, Debug, Clone)]
pub struct SmtpMessageReader {
    pub message: SmtpMessage,
    pub state: SmtpState,
}

impl SmtpMessageReader {
    pub fn read(&mut self, line: &[u8]) -> Result<Option<&'static str>, &'static str> {
        let command = crate::smtp::command::Command::from_bytes(line);

        match self.state {
            SmtpState::Init => match command {
                crate::smtp::command::Command::Helo(domain) => {
                    self.message.sender_domain = domain;
                    self.state = SmtpState::Mail;
                    Ok(Some("250 mail.humphreyway.com is my domain name"))
                }
                _ => Err("503 Bad sequence of commands"),
            },
            SmtpState::Mail => match command {
                crate::smtp::command::Command::MailFrom(from) => {
                    self.message.from = from;
                    self.state = SmtpState::Rcpt;
                    Ok(Some("250 OK"))
                }
                _ => Err("503 Bad sequence of commands"),
            },
            SmtpState::Rcpt => match command {
                crate::smtp::command::Command::RcptTo(to) => {
                    self.message.to.push(to);
                    Ok(Some("250 OK"))
                }
                crate::smtp::command::Command::Data => {
                    self.state = SmtpState::Data;
                    Ok(Some("354 Enter mail body. End new line with just a '.'"))
                }
                _ => Err("503 Bad sequence of commands"),
            },
            SmtpState::Data => {
                if line == b".\r\n" {
                    self.state = SmtpState::Done;
                    Ok(Some("250 Mail Delivered"))
                } else if line.starts_with(b"..") {
                    self.message.data.extend_from_slice(&line[1..]);
                    Ok(None)
                } else {
                    self.message.data.extend_from_slice(line);
                    Ok(None)
                }
            }
            _ => Err("503 Bad sequence of commands"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_state_transitions() {
        let mut reader = SmtpMessageReader::default();

        // Initial state should be Init
        assert_eq!(reader.state, SmtpState::Init);

        // Transition from Init to Mail
        assert_eq!(
            reader.read(b"HELO example.com"),
            Ok(Some("250 mail.humphreyway.com is my domain name"))
        );
        assert_eq!(reader.state, SmtpState::Mail);

        // Transition from Mail to Rcpt
        assert_eq!(
            reader.read(b"MAIL FROM: <user@example.com>"),
            Ok(Some("250 OK"))
        );
        assert_eq!(reader.state, SmtpState::Rcpt);

        // Add a recipient
        assert_eq!(
            reader.read(b"RCPT TO: <recipient@example.com>"),
            Ok(Some("250 OK"))
        );

        // Transition from Rcpt to Data
        assert_eq!(
            reader.read(b"DATA"),
            Ok(Some("354 Enter mail body. End new line with just a '.'"))
        );
        assert_eq!(reader.state, SmtpState::Data);

        // Handle data input
        assert_eq!(reader.read(b"Hello, World!\r\n"), Ok(None));
        assert_eq!(reader.read(b".\r\n"), Ok(Some("250 Mail Delivered")));
        assert_eq!(reader.state, SmtpState::Done);
    }
}
