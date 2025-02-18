use crate::smtp::{status, Message};

pub trait SmtpState: Send {
    fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<String>, Option<Box<dyn SmtpState>>);
    fn is_data_collect(&self) -> bool {
        false
    }
    fn is_done(&self) -> bool {
        false
    }
}

pub fn new_state() -> Box<dyn SmtpState + Send> {
    Box::new(InitState)
}

#[derive(Default)]
pub struct InitState;
impl SmtpState for InitState {
    fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<String>, Option<Box<dyn SmtpState>>) {
        if line.starts_with(b"HELO") {
            message.sender_domain = String::from_utf8_lossy(&line[5..]).trim().to_string();
            (
                Some(status::Code::Helo.to_string()),
                Some(Box::new(MailState)),
            )
        } else {
            (Some(status::Code::BadSequence.to_string()), None)
        }
    }
}

pub struct MailState;
impl SmtpState for MailState {
    fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<String>, Option<Box<dyn SmtpState>>) {
        if line.starts_with(b"MAIL FROM:") {
            message.from = String::from_utf8_lossy(&line[10..]).trim().to_string();
            (
                Some(status::Code::Ok.to_string()),
                Some(Box::new(RcptState)),
            )
        } else {
            (Some(status::Code::BadSequence.to_string()), None)
        }
    }
}

pub struct RcptState;
impl SmtpState for RcptState {
    fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<String>, Option<Box<dyn SmtpState>>) {
        if line.starts_with(b"RCPT TO:") {
            message
                .to
                .push(String::from_utf8_lossy(&line[8..]).trim().to_string());
            (Some("250 OK".to_string()), Some(Box::new(RcptState)))
        } else if line == b"DATA\r\n" {
            (
                Some(status::Code::EnterMessage.to_string()),
                Some(Box::new(DataCollectState)),
            )
        } else {
            (Some(status::Code::BadSequence.to_string()), None)
        }
    }
}

pub struct DataCollectState;
impl SmtpState for DataCollectState {
    fn process_line(
        &mut self,
        line: &[u8],
        message: &mut Message,
    ) -> (Option<String>, Option<Box<dyn SmtpState>>) {
        if line == b".\r\n" {
            (
                Some(status::Code::MessageSent.to_string()),
                Some(Box::new(DoneState)),
            )
        } else {
            message.data.extend_from_slice(line);
            (None, None)
        }
    }
    fn is_data_collect(&self) -> bool {
        true
    }
}

pub struct DoneState;
impl SmtpState for DoneState {
    fn process_line(
        &mut self,
        _line: &[u8],
        _message: &mut Message,
    ) -> (Option<String>, Option<Box<dyn SmtpState>>) {
        (Some(status::Code::BadSequence.to_string()), None)
    }
    fn is_done(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::smtp::message::Message;
    use super::*;

    #[test]
    fn test_init_state_helo() {
        let mut msg = Message::default();
        let mut state = InitState {};
        let (resp, next) = state.process_line(b"HELO example.com\r\n", &mut msg);
        assert_eq!(resp, Some(status::Code::Helo.to_string()));
        assert_eq!(msg.sender_domain, "example.com");
        assert!(next.is_some());
    }

    #[test]
    fn test_mail_state_from() {
        let mut msg = Message::default();
        let mut state = MailState {};
        let (resp, next) = state.process_line(b"MAIL FROM: <sender@example>\r\n", &mut msg);
        assert_eq!(resp, Some(status::Code::Ok.to_string()));
        assert_eq!(msg.from, "<sender@example>");
        assert!(next.is_some());
    }

    #[test]
    fn test_rcpt_state_to() {
        let mut msg = Message::default();
        let mut state = RcptState {};
        let (resp, next) = state.process_line(b"RCPT TO: <recipient@example>\r\n", &mut msg);
        assert_eq!(resp, Some(status::Code::Ok.to_string()));
        assert_eq!(msg.to, vec!["<recipient@example>".to_string()]);
        assert!(next.is_some());
    }

    #[test]
    fn test_data_state() {
        let mut msg = Message::default();
        let mut state = RcptState {};
        let (resp, next) = state.process_line(b"DATA\r\n", &mut msg);
        assert_eq!(resp, Some(status::Code::EnterMessage.to_string()));
        assert!(next.is_some());
    }

    #[test]
    fn test_data_collect_state() {
        let mut msg = Message::default();
        let mut state = DataCollectState {};
        let (resp, next) = state.process_line(b"Hello\r\n", &mut msg);
        assert_eq!(resp, None);
        assert!(next.is_none());
        let (resp, next) = state.process_line(b"World\r\n", &mut msg);
        assert_eq!(resp, None);
        assert!(next.is_none());
        let (resp, next) = state.process_line(b".\r\n", &mut msg);
        assert_eq!(resp, Some(status::Code::MessageSent.to_string()));
        assert!(next.is_some());
    }

    #[test]
    fn test_done_state() {
        let mut msg = Message::default();
        let mut state = DoneState {};
        let (resp, next) = state.process_line(b"QUIT\r\n", &mut msg);
        assert_eq!(resp, Some(status::Code::BadSequence.to_string()));
        assert!(next.is_none());
        assert!(state.is_done());
    }
}
