use async_trait::async_trait;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};
use tokio::net::{TcpListener, TcpStream};
use tokio::task;
use tokio::time::Instant;

#[derive(Debug)]
pub enum SocketError {
    BindFailed(std::io::Error),
    ConnectionFailed(std::io::Error),
    IoError(std::io::Error),
    BoxError(Box<dyn Error>),
    Closed,
}

impl SocketError {
    pub fn boxed<E: Error + 'static>(err: E) -> Self {
        SocketError::BoxError(Box::new(err))
    }
}

impl Display for SocketError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            SocketError::BindFailed(e) => write!(f, "BindFailed: {}", e),
            SocketError::ConnectionFailed(e) => write!(f, "ConnectionFailed: {}", e),
            SocketError::IoError(e) => write!(f, "IoError: {}", e),
            SocketError::BoxError(e) => write!(f, "BoxError: {}", e),
            SocketError::Closed => write!(f, "Closed"),
        }
    }
}

impl Error for SocketError {}

impl From<std::io::Error> for SocketError {
    fn from(e: std::io::Error) -> Self {
        SocketError::IoError(e)
    }
}

#[async_trait]
pub trait ToTcpListener {
    async fn to_tcp_listener(self) -> Result<TcpListener, std::io::Error>;
}

#[async_trait]
impl ToTcpListener for TcpListener {
    async fn to_tcp_listener(self) -> Result<TcpListener, std::io::Error> {
        Ok(self)
    }
}

#[async_trait]
impl ToTcpListener for &str {
    async fn to_tcp_listener(self) -> Result<TcpListener, std::io::Error> {
        TcpListener::bind(self).await
    }
}

#[async_trait]
pub trait SocketHandler {
    async fn handle_connection(&mut self, stream: TcpStream) -> Result<(), SocketError>;
}

pub async fn run<L, H>(addr: L, handler: H) -> Result<(), SocketError>
where
    L: ToTcpListener,
    H: SocketHandler + Clone + Send + 'static,
{
    let listener = addr
        .to_tcp_listener()
        .await
        .map_err(SocketError::BindFailed)?;
    tracing::info!("SMTP Server listening on {}", listener.local_addr()?);

    loop {
        let (socket, peer_addr) = listener
            .accept()
            .await
            .map_err(SocketError::ConnectionFailed)?;
        tracing::info!("New connection: {}", peer_addr);

        let mut handler = handler.clone();
        let start = Instant::now();

        task::spawn(async move {
            let conn_id = uuid::Uuid::new_v4();
            let span = tracing::info_span!("socket", id = %conn_id, peer = %peer_addr);
            let _guard = span.enter();
            match handler.handle_connection(socket).await {
                Ok(_) => {}
                Err(e) => tracing::error!("failed to handle connection: {}", e),
            }
            tracing::info!("Closing after {} ms", start.elapsed().as_millis());
        });
    }
}
