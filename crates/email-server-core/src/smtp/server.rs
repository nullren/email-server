use crate::smtp::{state, status, Message};
use crate::socket::{SocketError, SocketHandler};
use bytes::BytesMut;
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

macro_rules! outln {
    ($stream:expr, $msg:expr) => {
        $stream.write_all(format!("{}\r\n", $msg).as_bytes()).await?;
    };
    ($stream:expr, $fmt:expr, $($arg:tt)*) => {
        $stream.write_all(format!(concat!($fmt, "\r\n"), $($arg)*).as_bytes()).await?;
    };
}

#[derive(Clone)]
pub struct Server {}

impl SocketHandler for Server {
    type Future = Pin<Box<dyn Future<Output = Result<(), SocketError>> + Send>>;

    fn handle_connection(&mut self, stream: TcpStream) -> Self::Future {
        let mut this = self.clone();
        Box::pin(async move {
            this.handle_tls_connection(stream).await

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
        })
    }
}

impl Server {
    fn handle_tls_connection(
        &mut self,
        mut stream: TcpStream,
    ) -> Pin<Box<dyn Future<Output = Result<(), SocketError>> + Send>> {
        Box::pin(async move {
            outln!(stream, status::Code::ServiceReady);
            let mut message = Message::default();
            let mut state = state::new_state();
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
                    if !state.is_data_collect() && buf.starts_with(b"QUIT") {
                        outln!(stream, status::Code::Goodbye);
                        break 'outer;
                    }

                    match state.process_line(&buf, &mut message) {
                        (Some(output), Some(next_state)) => {
                            outln!(stream, output);
                            state = next_state;
                        }
                        (Some(output), None) => {
                            outln!(stream, output);
                            break 'outer;
                        }
                        (None, _) => {}
                    }

                    // we don't ever stay in a "Done" state, we just reset
                    if state.is_done() {
                        println!("Message received: {:?}", message);
                        // TODO: Handle the message
                        message = Message::default();
                        state = state::new_state();
                    }
                }
            }
            Ok(())
        })
    }
}
