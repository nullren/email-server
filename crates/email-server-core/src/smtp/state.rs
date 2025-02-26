use async_trait::async_trait;

use crate::message::Message;
use crate::smtp::status;
use std::fmt::Debug;
use std::sync::Arc;

use super::validator::{HeloValidator, NoopValidator};

#[async_trait]
pub trait SmtpState: Send + Debug {
    async fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<status::Code>, Option<Box<dyn SmtpState>>);

    async fn process(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<status::Code>, Option<Box<dyn SmtpState>>) {
        if !self.is_collecting_data() && line.starts_with(b"QUIT") {
            (Some(status::Code::Goodbye), None)
        } else if self.is_message_completed() {
            // reset the state
            let mut init = new_state();
            init.process_line(line, message).await
        } else {
            self.process_line(line, message).await
        }
    }

    fn is_collecting_data(&self) -> bool {
        false
    }
    fn is_message_completed(&self) -> bool {
        false
    }
}

pub fn new_state() -> Box<dyn SmtpState + Send> {
    Box::new(InitState::default())
}

#[derive(Debug)]
pub struct InitState {
    validator: Arc<NoopValidator>,
}
impl Default for InitState {
    fn default() -> Self {
        Self {
            validator: Arc::new(NoopValidator),
        }
    }
}
#[async_trait]
impl SmtpState for InitState {
    async fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<status::Code>, Option<Box<dyn SmtpState>>) {
        if line.starts_with(b"HELO") {
            let sender_domain = String::from_utf8_lossy(&line[5..]).trim().to_string();
            if self.validator.valid(&sender_domain).await {
                message.sender_domain = sender_domain;
                return (Some(status::Code::Helo), Some(Box::new(MailState)));
            }
            // TODO: need to auth or starttls
            (Some(status::Code::Helo), Some(Box::new(MailState)))
        } else {
            (
                Some(status::Code::BadSequence),
                Some(Box::new(InitState::default())),
            )
        }
    }
}

#[derive(Default, Debug)]
pub struct MailState;
#[async_trait]
impl SmtpState for MailState {
    async fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<status::Code>, Option<Box<dyn SmtpState>>) {
        if line.starts_with(b"MAIL FROM:") {
            message.from = String::from_utf8_lossy(&line[10..]).trim().to_string();
            (Some(status::Code::Ok), Some(Box::new(RcptState)))
        } else {
            (Some(status::Code::BadSequence), None)
        }
    }
}

#[derive(Default, Debug)]
pub struct RcptState;
#[async_trait]
impl SmtpState for RcptState {
    async fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<status::Code>, Option<Box<dyn SmtpState>>) {
        if line.starts_with(b"RCPT TO:") {
            message
                .to
                .push(String::from_utf8_lossy(&line[8..]).trim().to_string());
            (Some(status::Code::Ok), Some(Box::new(RcptState)))
        } else if line == b"DATA" {
            (
                Some(status::Code::EnterMessage),
                Some(Box::new(DataCollectState)),
            )
        } else {
            (Some(status::Code::BadSequence), None)
        }
    }
}

#[derive(Default, Debug)]
pub struct DataCollectState;
#[async_trait]
impl SmtpState for DataCollectState {
    async fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<status::Code>, Option<Box<dyn SmtpState>>) {
        if line == b"." {
            (
                Some(status::Code::MessageSent),
                Some(Box::new(MessageCompleted)),
            )
        } else {
            message.data.extend_from_slice(line);
            (None, Some(Box::new(DataCollectState)))
        }
    }
}

#[derive(Default, Debug)]
pub struct MessageCompleted;
#[async_trait]
impl SmtpState for MessageCompleted {
    async fn process_line(
        &mut self,
        _line: &[u8],
        _message: &mut Message,
    ) -> (Option<status::Code>, Option<Box<dyn SmtpState>>) {
        (Some(status::Code::BadSequence), None)
    }
    fn is_message_completed(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;

    #[tokio::test]
    async fn test_init_state_helo() {
        let mut msg = Message::default();
        let mut state = InitState::default();
        let (resp, next) = state.process_line(b"HELO example.com", &mut msg).await;
        assert_eq!(resp, Some(status::Code::Helo));
        assert_eq!(msg.sender_domain, "example.com");
        assert!(next.is_some());
    }

    #[tokio::test]
    async fn test_mail_state_from() {
        let mut msg = Message::default();
        let mut state = MailState {};
        let (resp, next) = state
            .process_line(b"MAIL FROM: <sender@example>", &mut msg)
            .await;
        assert_eq!(resp, Some(status::Code::Ok));
        assert_eq!(msg.from, "<sender@example>");
        assert!(next.is_some());
    }

    #[tokio::test]
    async fn test_rcpt_state_to() {
        let mut msg = Message::default();
        let mut state = RcptState {};
        let (resp, next) = state
            .process_line(b"RCPT TO: <recipient@example>", &mut msg)
            .await;
        assert_eq!(resp, Some(status::Code::Ok));
        assert_eq!(msg.to, vec!["<recipient@example>".to_string()]);
        assert!(next.is_some());
    }

    #[tokio::test]
    async fn test_data_state() {
        let mut msg = Message::default();
        let mut state = RcptState {};
        let (resp, next) = state.process_line(b"DATA", &mut msg).await;
        assert_eq!(resp, Some(status::Code::EnterMessage));
        assert!(next.is_some());
    }

    #[tokio::test]
    async fn test_data_collect_state() {
        let mut msg = Message::default();
        let mut state = DataCollectState {};
        let (resp, next) = state.process_line(b"Hello", &mut msg).await;
        assert!(resp.is_none());
        assert!(next.is_some());
        let (resp, next) = state.process_line(b"World", &mut msg).await;
        assert!(resp.is_none());
        assert!(next.is_some());
        let (resp, next) = state.process_line(b".", &mut msg).await;
        assert_eq!(resp, Some(status::Code::MessageSent));
        assert!(next.is_some());
    }

    #[tokio::test]
    async fn test_done_state() {
        let mut msg = Message::default();
        let mut state = MessageCompleted {};
        let (resp, next) = state.process_line(b"QUIT", &mut msg).await;
        assert_eq!(resp, Some(status::Code::BadSequence));
        assert!(next.is_none());
        assert!(state.is_message_completed());
    }
}
