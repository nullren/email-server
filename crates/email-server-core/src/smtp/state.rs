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
    pub fn read(&mut self, line: &[u8]) -> Result<Option<&str>, &str> {
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
