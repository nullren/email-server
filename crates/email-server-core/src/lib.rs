use crate::message::PrintHandler;
use crate::socket::{SocketError, ToTcpListener};
use std::path::Path;
use std::sync::Arc;

pub mod logging;
pub mod message;
pub mod smtp;
pub mod socket;
pub mod storage;

pub async fn smtp_server<L: ToTcpListener, P: AsRef<Path>>(
    addr: L,
    sqlite_db: P,
) -> Result<(), socket::SocketError> {
    let print_handler = Box::new(PrintHandler);
    let storage_handler = Box::new(
        storage::SqliteStore::new(sqlite_db)
            .await
            .map_err(SocketError::boxed)?,
    );
    let handler = Arc::new(message::multi_handler(vec![print_handler, storage_handler]));
    socket::run(addr, smtp::Server { handler }).await
}

#[cfg(test)]
mod tests {
    use crate::smtp_server;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    async fn start_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

        let local_addr = listener.local_addr().unwrap();
        let addr_str = local_addr.to_string();
        let temp_file = tempfile::NamedTempFile::new().unwrap();

        tokio::spawn(async move {
            if let Err(e) = smtp_server(listener, temp_file.path()).await {
                tracing::error!("SMTP Server Error: {}", e);
            }
        });

        // Ensure the server is actually accepting connections
        for _ in 0..10 {
            match TcpStream::connect(&addr_str).await {
                Ok(_) => return addr_str, // If we connect successfully, return address
                Err(_) => tokio::time::sleep(tokio::time::Duration::from_millis(50)).await,
            }
        }

        panic!("Server failed to start");
    }

    #[test]
    fn test_logging() {
        crate::logging::setup();
        tracing::info!("This should appear in test output!");
        tracing::debug!("Debug message from a test!");
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_smtp_server_initial_response() {
        let server_address = start_server().await;

        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("220"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_smtp_server_helo_command() {
        let server_address = start_server().await;

        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        stream.write_all(b"HELO example.com\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("250"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_smtp_server_quit_command() {
        let server_address = start_server().await;

        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        stream.write_all(b"QUIT\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        let response = String::from_utf8_lossy(&buffer[..n]);
        assert!(response.starts_with("221"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_full_message() {
        let server_address = start_server().await;

        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        stream.write_all(b"HELO example.com\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("250"));

        stream
            .write_all(b"MAIL FROM: Alice <Alice@example.com>\r\n")
            .await
            .unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("250"));

        stream
            .write_all(b"RCPT TO: Bob <bob@example.com>\r\n")
            .await
            .unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("250"));

        stream.write_all(b"DATA\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("354"));

        stream
            .write_all(b"Subject: Test\r\n\r\nHello, world!\r\n.\r\n")
            .await
            .unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("250"));

        stream.write_all(b"QUIT\r\n").await.unwrap();
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n]).starts_with("221"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pipeline_message() {
        crate::logging::setup();
        let server_address = start_server().await;

        let mut stream = TcpStream::connect(server_address).await.unwrap();
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer).await.unwrap();

        let input = b"HELO example.com\r\nMAIL FROM: Alice <Alice@example.com>\r\nRCPT TO: Bob <bob@example.com>\r\nDATA\r\nSubject: Test\r\n\r\nHello, world!\r\n.\r\nQUIT\r\n";
        let expected = "250 mail.example.com\r\n250 OK\r\n250 OK\r\n354 enter mail, end with line containing only \".\"\r\n250 Message sent\r\n221 Goodbye\r\n";

        tracing::debug!("Sending: {}", String::from_utf8_lossy(input));
        stream.write_all(input).await.unwrap();
        let mut buffer = Vec::new();
        let n = stream.read_to_end(&mut buffer).await.unwrap();
        let output = String::from_utf8_lossy(&buffer[..n]);
        tracing::debug!("Read: {:?}", output);
        assert_eq!(output, expected);
    }
}
