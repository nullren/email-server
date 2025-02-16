use crate::socket::{SocketError, SocketHandler};
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct SmtpServer {}

impl SocketHandler for SmtpServer {
    type Future = Pin<Box<dyn Future<Output = Result<(), SocketError>> + Send>>;

    fn handle_connection(&mut self, mut stream: TcpStream) -> Self::Future {
        Box::pin(async move {
            stream
                .write_all(b"220 Welcome to the SMTP server\r\n")
                .await?;
            let mut state = SmtpMessageReader::default();
            loop {
                let mut buf = [0; 1024];
                let n = stream.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                if state.state != SmtpState::Data && buf.starts_with(b"QUIT") {
                    stream.write_all(b"221 Goodbye\r\n").await?;
                    break;
                }
                match state.read(&buf[..n]) {
                    Ok(Some(output)) => {
                        stream
                            .write_all(format!("{}\r\n", output).as_bytes())
                            .await?;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        stream.write_all(format!("{}\r\n", e).as_bytes()).await?;
                    }
                }
                if state.state == SmtpState::Done {
                    // self.message_queue.push(state.message);
                    state = SmtpMessageReader::default();
                }
            }
            Ok(())
        })
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
enum SmtpState {
    #[default]
    Init,
    Mail,
    Rcpt,
    Data,
    Done,
}

#[derive(Default, Debug, Clone)]
struct SmtpMessage {
    sender_domain: String,
    from: String,
    to: Vec<String>,
    data: Vec<u8>,
}

#[derive(Default, Debug, Clone)]
struct SmtpMessageReader {
    message: SmtpMessage,
    state: SmtpState,
}

impl SmtpMessageReader {
    fn read(&mut self, line: &[u8]) -> Result<Option<&str>, &str> {
        match self.state {
            SmtpState::Init => {
                if line.starts_with(b"HELO") {
                    self.message.sender_domain =
                        String::from_utf8_lossy(&line[5..]).trim().to_string();
                    self.state = SmtpState::Mail;
                    Ok(Some("250 mail.humphreyway.com is my domain name"))
                } else {
                    Err("503 Bad sequence of commands")
                }
            }
            SmtpState::Mail => {
                if line.starts_with(b"MAIL FROM:") {
                    self.message.from = String::from_utf8_lossy(&line[10..]).trim().to_string();
                    self.state = SmtpState::Rcpt;
                    Ok(Some("250 OK"))
                } else {
                    Err("503 Bad sequence of commands")
                }
            }
            SmtpState::Rcpt => {
                if line.starts_with(b"RCPT TO:") {
                    self.message
                        .to
                        .push(String::from_utf8_lossy(&line[8..]).trim().to_string());
                    Ok(Some("250 OK"))
                } else if line.starts_with(b"DATA") {
                    self.state = SmtpState::Data;
                    Ok(Some("354 Enter mail body.  End new line with just a '.'"))
                } else {
                    Err("503 Bad sequence of commands")
                }
            }
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
