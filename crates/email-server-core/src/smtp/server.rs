use crate::message::{self, Message};
use crate::smtp::{state, status};
use crate::socket::{SocketError, SocketHandler};
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, LinesCodec};

macro_rules! outln {
    ($stream:expr, $msg:expr) => {
        $stream.write_all(format!("{}\r\n", $msg).as_bytes()).await?;
    };
    ($stream:expr, $fmt:expr, $($arg:tt)*) => {
        $stream.write_all(format!(concat!($fmt, "\r\n"), $($arg)*).as_bytes()).await?;
    };
}

#[derive(Clone)]
pub struct Server {
    pub(crate) handler: Arc<dyn message::Handler + Sync + Send>,
}

#[async_trait]
impl SocketHandler for Server {
    async fn handle_connection(&mut self, stream: TcpStream) -> Result<(), SocketError> {
        self.handle_tls_connection(stream).await
    }
}

// TODO: only allow EHLO, NOOP, QUIT, and STARTTLS
// EHLO
// 250-{{domain}}
// 250-PIPELINING
// 250-SIZE 71000000
// 250-ENHANCEDSTATUSCODES
// 250-8BITMIME
// 250 STARTTLS
//
// NOOP
// 250 2.0.0 OK
//
// QUIT
// 221 2.0.0 Bye
//
// STARTTLS
// 220 2.0.0 Start TLS
// # just ignore   501 Syntax error (no parameters allowed)
// # don't cont    454 TLS not available due to temporary reason
//
// 530 5.7.1 Authentication required
// unless NOOP, EHLO, STARTTLS, or QUIT
//
// HELO booger.net
// 250 smtp.fastmail.com
// MAIL FROM: Ren <ren@booger.net>
// 530 5.7.1 Authentication required

impl Server {
    async fn handle_tls_connection(&mut self, mut stream: TcpStream) -> Result<(), SocketError> {
        outln!(stream, status::Code::ServiceReady);

        let (reader, mut writer) = stream.into_split();
        let mut lines = FramedRead::new(reader, LinesCodec::new());

        let mut message = Message::default();
        let mut state = state::new_state();

        while let Some(line) = lines.next().await {
            let line = line.map_err(SocketError::boxed)?;
            tracing::debug!("state = {:?}; received: {:?}", state, line);

            if let (Some(output), next_state) = state.process(line.as_bytes(), &mut message).await {
                outln!(writer, output);
                if let Some(next_state) = next_state {
                    state = next_state;
                } else {
                    break;
                }
            }

            if state.is_message_completed() {
                if let Err(e) = self.handler.handle_message(message).await {
                    tracing::error!("Error handling message: {:?}", e);
                }
                message = Message::default();
            }
        }

        Ok(())
    }
}
