use std::fmt::Display;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Code {
    ServiceReady,
    StartTLS,
    Ok,
    EncRequired,
    AuthRequired,
    Goodbye,
    BadSequence,
    Helo,
    EnterMessage,
    MessageSent,
}

impl Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Code::ServiceReady => write!(f, "220 Service ready"),
            Code::StartTLS => write!(f, "220 Start TLS"),
            Code::Goodbye => write!(f, "221 Goodbye"),
            Code::Helo => write!(f, "250 mail.example.com"),
            Code::Ok => write!(f, "250 OK"),
            Code::MessageSent => write!(f, "250 Message sent"),
            Code::EnterMessage => write!(f, "354 enter mail, end with line containing only \".\""),
            Code::BadSequence => write!(f, "503 Bad sequence of commands"),
            Code::EncRequired => write!(f, "530 Encryption required"),
            Code::AuthRequired => write!(f, "530 Authentication required"),
        }
    }
}
