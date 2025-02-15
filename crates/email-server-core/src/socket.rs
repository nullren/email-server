use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Debug)]
pub enum SmtpServerError {
    ListenError(std::io::Error),
    SocketWriteError(std::io::Error),
    SocketReadError(std::io::Error),
    SocketClosed,
}

impl Display for SmtpServerError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            SmtpServerError::ListenError(e) => write!(f, "ListenError: {}", e),
            SmtpServerError::SocketWriteError(e) => write!(f, "SocketWriteError: {}", e),
            SmtpServerError::SocketReadError(e) => write!(f, "SocketReadError: {}", e),
            SmtpServerError::SocketClosed => write!(f, "SocketClosed"),
        }
    }
}

impl Error for SmtpServerError {}

pub async fn start_smtp_server(addr: &str) -> Result<(), SmtpServerError> {
    let listener = TcpListener::bind(addr)
        .await
        .map_err(SmtpServerError::ListenError)?;
    println!("SMTP Server listening on {}", addr);

    loop {
        let (socket, addr) = listener
            .accept()
            .await
            .map_err(SmtpServerError::ListenError)?;
        println!("New connection: {}", addr);

        tokio::spawn(async move {
            handle_smtp_connection(socket).await;
        });
    }
}

async fn handle_smtp_connection(mut socket: TcpStream) {
    if let Err(e) = send_greeting(&mut socket).await {
        eprintln!("Failed to send greeting; err = {:?}", e);
        return;
    }

    let mut buf = [0; 1024];

    loop {
        match read_request(&mut socket, &mut buf).await {
            Ok(request) => {
                if let Err(e) = process_request(&mut socket, &request).await {
                    eprintln!("Failed to process request; err = {:?}", e);
                    return;
                }
            }
            Err(e) => {
                eprintln!("Failed to read request; err = {:?}", e);
                return;
            }
        }
    }
}

async fn send_greeting(socket: &mut tokio::net::TcpStream) -> Result<(), SmtpServerError> {
    socket
        .write_all(b"220 Welcome to the SMTP server\r\n")
        .await
        .map_err(SmtpServerError::SocketWriteError)?;
    Ok(())
}

async fn read_request(
    socket: &mut tokio::net::TcpStream,
    buf: &mut [u8],
) -> Result<String, SmtpServerError> {
    let n = socket
        .read(buf)
        .await
        .map_err(SmtpServerError::SocketReadError)?;
    if n == 0 {
        return Err(SmtpServerError::SocketClosed);
    }
    Ok(String::from_utf8_lossy(&buf[0..n]).to_string())
}

async fn process_request(
    socket: &mut tokio::net::TcpStream,
    request: &str,
) -> Result<(), SmtpServerError> {
    if request.starts_with("HELO") {
        socket.write_all(b"250 Hello\r\n").await
    } else {
        socket.write_all(b"500 Command not recognized\r\n").await
    }
    .map_err(SmtpServerError::SocketWriteError)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn test_handle_smtp_connection() -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:2526").await?;
        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            handle_smtp_connection(socket).await;
        });

        let mut client = TcpStream::connect("127.0.0.1:2526").await?;
        let mut buf = [0; 1024];

        // Read the greeting message
        let n = client.read(&mut buf).await?;
        assert_eq!(
            String::from_utf8_lossy(&buf[0..n]),
            "220 Welcome to the SMTP server\r\n"
        );

        // Send HELO command
        client.write_all(b"HELO example.com\r\n").await?;
        let n = client.read(&mut buf).await?;
        assert_eq!(String::from_utf8_lossy(&buf[0..n]), "250 Hello\r\n");

        // Send unknown command
        client.write_all(b"UNKNOWN\r\n").await?;
        let n = client.read(&mut buf).await?;
        assert_eq!(
            String::from_utf8_lossy(&buf[0..n]),
            "500 Command not recognized\r\n"
        );

        Ok(())
    }
}
