use std::error::Error;
use std::fmt;

use chrono::prelude::*;

use crate::emoji::EMOJI;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum MessageType {
    Image,
    Text,
    Audio,
    Video,
    Contact,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub datetime: NaiveDateTime,
    pub author: String,
    pub text: String,
    pub kind: MessageType,
}

#[derive(Debug)]
pub enum MessageErrorKind {
    InvalidDate,
    EmptyMessage,
}

#[derive(Debug)]
pub struct MessageError(pub MessageErrorKind);

// TODO: use `error::Error`
impl Error for MessageError {
    fn description(&self) -> &str {
        match self.0 {
            MessageErrorKind::InvalidDate => "unable to parse date",
            MessageErrorKind::EmptyMessage => "empty message",
        }
    }
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.description().fmt(f)
    }
}

pub type Result<T> = ::std::result::Result<T, MessageError>;

impl Message {
    pub fn from_str(datetime: &str, author: &str, text: &str) -> Result<Message> {
        let datetime = match NaiveDateTime::parse_from_str(datetime, "%Y-%m-%d, %H:%M:%S") {
            Ok(v) => v,
            Err(_) => return Err(MessageError(MessageErrorKind::InvalidDate)),
        };

        let kind = if text.contains("omitted") {
            if text.contains("image") {
                MessageType::Image
            } else if text.contains("audio") {
                MessageType::Audio
            } else if text.contains("video") {
                MessageType::Video
            } else if text.contains("card") {
                MessageType::Contact
            } else {
                MessageType::Unknown
            }
        } else {
            MessageType::Text
        };

        let author = String::from(author);
        let text = String::from(text.trim());

        Ok(Message {
            datetime,
            author,
            text,
            kind,
        })
    }

    pub fn letters(&self) -> String {
        self.text
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect()
    }

    pub fn words(&self) -> Vec<String> {
        self.text
            .split_whitespace()
            .filter(|s| !EMOJI.contains(s))
            .map(|s| s.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_parses_date() {
        let m = Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "Baz Qux").unwrap();
        assert_eq!(format!("{}", m.datetime), "2019-09-11 01:57:17");
    }

    #[test]
    fn from_str_parses_author() {
        let m = Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "Baz Qux").unwrap();
        assert_eq!(m.author, "Foo Bar");
    }

    #[test]
    fn from_str_parses_text() {
        let m = Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "Baz Qux").unwrap();
        assert_eq!(m.text, "Baz Qux");
    }
    #[test]
    fn from_str_identifies_kind() {
        let m_text = Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "Baz Qux").unwrap();
        assert_eq!(m_text.kind, MessageType::Text);

        let m_image =
            Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "‎image omitted").unwrap();
        assert_eq!(m_image.kind, MessageType::Image);

        let m_audio =
            Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "audio omitted").unwrap();
        assert_eq!(m_audio.kind, MessageType::Audio);

        let m_video =
            Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "video omitted").unwrap();
        assert_eq!(m_video.kind, MessageType::Video);

        let m_contact =
            Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "Contact card omitted").unwrap();
        assert_eq!(m_contact.kind, MessageType::Contact);
    }

    #[test]
    fn from_str_throw_error_on_invalid_date() {
        assert!(
            Message::from_str("", "author", "text").is_err(),
            "unable to parse date"
        );
    }

    #[test]
    fn letters_works() {
        let m = Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "Baz Qux").unwrap();
        assert_eq!(m.letters(), "BazQux");
    }

    #[test]
    fn words_works() {
        let m = Message::from_str("2019-09-11, 01:57:17", "Foo Bar", "Baz Qux").unwrap();
        assert_eq!(m.words(), vec!["Baz", "Qux"]);
    }
}
