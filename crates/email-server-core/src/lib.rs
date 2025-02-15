pub mod smtp;
pub mod socket;

pub async fn smtp_server(addr: &str) -> Result<(), socket::SocketError> {
    socket::run(addr, smtp::SmtpServer {}).await
}
