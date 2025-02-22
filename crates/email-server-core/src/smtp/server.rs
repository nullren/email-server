use crate::message::{self, Message};
use crate::smtp::{state, status};
use crate::socket::{SocketError, SocketHandler};
use async_trait::async_trait;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct Server {
    pub(crate) handler: Arc<dyn message::Handler + Sync + Send>,
}

#[async_trait]
impl SocketHandler for Server {
    async fn handle_connection(&mut self, stream: TcpStream) -> Result<(), SocketError> {
        self.handle_stream(stream).await
    }
}

macro_rules! outln {
    ($stream:expr, $msg:expr) => {
        $stream.write_all(format!("{}\r\n", $msg).as_bytes()).await?;
    };
    ($stream:expr, $fmt:expr, $($arg:tt)*) => {
        $stream.write_all(format!(concat!($fmt, "\r\n"), $($arg)*).as_bytes()).await?;
    };
}

impl Server {
    async fn handle_stream(&mut self, stream: TcpStream) -> Result<(), SocketError> {
        let stream = self.require_tls(stream).await?;
        self.handle_message(stream).await
    }

    async fn require_tls(&mut self, stream: TcpStream) -> Result<TcpStream, SocketError> {
        // EHLO
        // 250-{{domain}}
        // 250-PIPELINING
        // 250-SIZE 71000000
        // 250-ENHANCEDSTATUSCODES
        // 250-8BITMIME
        // 250 STARTTLS

        // NOOP
        // 250 2.0.0 OK

        // QUIT
        // 221 2.0.0 Bye

        // STARTTLS
        // 220 2.0.0 Start TLS
        // # just ignore   501 Syntax error (no parameters allowed)
        // # don't cont    454 TLS not available due to temporary reason

        // 530 5.7.1 Authentication required
        // unless NOOP, EHLO, STARTTLS, or QUIT

        // HELO booger.net
        // 250 smtp.fastmail.com
        // MAIL FROM: Ren <ren@booger.net>
        // 530 5.7.1 Authentication required

        Ok(stream)
    }

    async fn handle_message(&mut self, mut stream: TcpStream) -> Result<(), SocketError> {
        outln!(stream, status::Code::ServiceReady);
        let mut message = Message::default();
        let mut st = state::new_state();
        let mut buffer = BytesMut::with_capacity(4096);

        'outer: loop {
            let mut temp = [0; 1024];
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                break 'outer;
            }
            buffer.extend_from_slice(&temp[..n]);

            while let Some(pos) = buffer.windows(2).position(|x| x == b"\r\n") {
                let buf = buffer.split_to(pos + 2);
                match st.process(&buf, &mut message) {
                    (Some(output), Some(next_state)) => {
                        outln!(stream, output);
                        st = next_state;
                    }
                    (Some(output), None) => {
                        outln!(stream, output);
                        break 'outer;
                    }
                    (None, _) => {}
                }

                // we don't ever stay in a "Done" state, we just reset
                if st.is_done() {
                    if let Err(e) = self.handler.handle_message(message).await {
                        // we don't want to break the loop, just log the error
                        eprintln!("Error sending message: {}", e);
                    }
                    message = Message::default();
                    st = state::new_state();
                }
            }
        }
        Ok(())
    }
}
