//! Headers widely used in email messages

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter, Write},
    ops::Deref,
};

use email_encoding::headers::EmailWriter;

pub use self::{
    content::*,
    content_disposition::ContentDisposition,
    content_type::{ContentType, ContentTypeErr},
    date::Date,
    mailbox::*,
    special::*,
    textual::*,
};
use crate::BoxError;

mod content;
mod content_disposition;
mod content_type;
mod date;
mod mailbox;
mod special;
mod textual;

const WHITESPACE_CHARS: &[u8] = b" \t";

/// Represents an email header
///
/// Email header as defined in [RFC5322](https://datatracker.ietf.org/doc/html/rfc5322) and extensions.
pub trait Header: Clone {
    fn name() -> HeaderName;

    fn parse(s: &str) -> Result<Self, BoxError>;

    fn display(&self) -> HeaderValue;
}

/// A set of email headers
#[derive(Debug, Clone, Default)]
pub struct Headers {
    headers: Vec<HeaderValue>,
}

impl Headers {
    /// Create an empty `Headers`
    ///
    /// This function does not allocate.
    #[inline]
    pub const fn new() -> Self {
        Self {
            headers: Vec::new(),
        }
    }

    /// Create an empty `Headers` with a pre-allocated capacity
    ///
    /// Pre-allocates a capacity of at least `capacity`.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            headers: Vec::with_capacity(capacity),
        }
    }

    /// Returns a copy of an `Header` present in `Headers`
    ///
    /// Returns `None` if `Header` isn't present in `Headers`.
    pub fn get<H: Header>(&self) -> Option<H> {
        self.get_raw(&H::name())
            .and_then(|raw_value| H::parse(raw_value).ok())
    }

    /// Sets `Header` into `Headers`, overriding `Header` if it
    /// was already present in `Headers`
    pub fn set<H: Header>(&mut self, header: H) {
        self.insert_raw(header.display());
    }

    /// Remove `Header` from `Headers`, returning it
    ///
    /// Returns `None` if `Header` isn't in `Headers`.
    pub fn remove<H: Header>(&mut self) -> Option<H> {
        self.remove_raw(&H::name())
            .and_then(|value| H::parse(&value.raw_value).ok())
    }

    /// Clears `Headers`, removing all headers from it
    ///
    /// Any pre-allocated capacity is left untouched.
    #[inline]
    pub fn clear(&mut self) {
        self.headers.clear();
    }

    /// Returns a reference to the raw value of header `name`
    ///
    /// Returns `None` if `name` isn't present in `Headers`.
    pub fn get_raw(&self, name: &str) -> Option<&str> {
        self.find_header(name).map(|value| value.raw_value.as_str())
    }

    /// Inserts a raw header into `Headers`, overriding `value` if it
    /// was already present in `Headers`.
    pub fn insert_raw(&mut self, value: HeaderValue) {
        match self.find_header_mut(&value.name) {
            Some(current_value) => {
                *current_value = value;
            }
            None => {
                self.headers.push(value);
            }
        }
    }

    /// Remove a raw header from `Headers`, returning it
    ///
    /// Returns `None` if `name` isn't present in `Headers`.
    pub fn remove_raw(&mut self, name: &str) -> Option<HeaderValue> {
        self.find_header_index(name).map(|i| self.headers.remove(i))
    }

    pub(crate) fn find_header(&self, name: &str) -> Option<&HeaderValue> {
        self.headers
            .iter()
            .find(|value| name.eq_ignore_ascii_case(&value.name))
    }

    fn find_header_mut(&mut self, name: &str) -> Option<&mut HeaderValue> {
        self.headers
            .iter_mut()
            .find(|value| name.eq_ignore_ascii_case(&value.name))
    }

    fn find_header_index(&self, name: &str) -> Option<usize> {
        self.headers
            .iter()
            .enumerate()
            .find(|(_i, value)| name.eq_ignore_ascii_case(&value.name))
            .map(|(i, _)| i)
    }
}

impl Display for Headers {
    /// Formats `Headers`, ready to put them into an email
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for value in &self.headers {
            f.write_str(&value.name)?;
            f.write_str(": ")?;
            f.write_str(&value.encoded_value)?;
            f.write_str("\r\n")?;
        }

        Ok(())
    }
}

/// A possible error when converting a `HeaderName` from another type.
// comes from `http` crate
#[allow(missing_copy_implementations)]
#[derive(Clone)]
pub struct InvalidHeaderName {
    _priv: (),
}

impl fmt::Debug for InvalidHeaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InvalidHeaderName")
            // skip _priv noise
            .finish()
    }
}

impl fmt::Display for InvalidHeaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid header name")
    }
}

impl Error for InvalidHeaderName {}

/// A valid header name
#[derive(Debug, Clone)]
pub struct HeaderName(Cow<'static, str>);

impl HeaderName {
    /// Creates a new header name
    pub fn new_from_ascii(ascii: String) -> Result<Self, InvalidHeaderName> {
        if !ascii.is_empty()
            && ascii.len() <= 76
            && ascii.is_ascii()
            && !ascii.contains(|c| c == ':' || c == ' ')
        {
            Ok(Self(Cow::Owned(ascii)))
        } else {
            Err(InvalidHeaderName { _priv: () })
        }
    }

    /// Creates a new header name, panics on invalid name
    pub const fn new_from_ascii_str(ascii: &'static str) -> Self {
        macro_rules! static_assert {
            ($condition:expr) => {
                let _ = [()][(!($condition)) as usize];
            };
        }

        static_assert!(!ascii.is_empty());
        static_assert!(ascii.len() <= 76);

        let bytes = ascii.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            static_assert!(bytes[i].is_ascii());
            static_assert!(bytes[i] != b' ');
            static_assert!(bytes[i] != b':');

            i += 1;
        }

        Self(Cow::Borrowed(ascii))
    }
}

impl Display for HeaderName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl Deref for HeaderName {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for HeaderName {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        let s: &str = self.as_ref();
        s.as_bytes()
    }
}

impl AsRef<str> for HeaderName {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<HeaderName> for HeaderName {
    fn eq(&self, other: &HeaderName) -> bool {
        let s1: &str = self.as_ref();
        let s2: &str = other.as_ref();
        s1 == s2
    }
}

impl PartialEq<&str> for HeaderName {
    fn eq(&self, other: &&str) -> bool {
        let s: &str = self.as_ref();
        s == *other
    }
}

impl PartialEq<HeaderName> for &str {
    fn eq(&self, other: &HeaderName) -> bool {
        let s: &str = other.as_ref();
        *self == s
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderValue {
    name: HeaderName,
    raw_value: String,
    encoded_value: String,
}

impl HeaderValue {
    pub fn new(name: HeaderName, raw_value: String) -> Self {
        let mut encoded_value = String::with_capacity(raw_value.len());
        HeaderValueEncoder::encode(&name, &raw_value, &mut encoded_value).unwrap();

        Self {
            name,
            raw_value,
            encoded_value,
        }
    }

    pub fn dangerous_new_pre_encoded(
        name: HeaderName,
        raw_value: String,
        encoded_value: String,
    ) -> Self {
        Self {
            name,
            raw_value,
            encoded_value,
        }
    }

    pub(crate) fn get_raw(&self) -> &str {
        &self.raw_value
    }

    pub(crate) fn get_encoded(&self) -> &str {
        &self.encoded_value
    }
}

/// [RFC 1522](https://tools.ietf.org/html/rfc1522) header value encoder
struct HeaderValueEncoder<'a> {
    writer: EmailWriter<'a>,
    encode_buf: String,
}

impl<'a> HeaderValueEncoder<'a> {
    fn encode(name: &str, value: &'a str, f: &'a mut impl fmt::Write) -> fmt::Result {
        let (words_iter, encoder) = Self::new(name, value, f);
        encoder.format(words_iter)
    }

    fn new(
        name: &str,
        value: &'a str,
        writer: &'a mut dyn Write,
    ) -> (WordsPlusFillIterator<'a>, Self) {
        let line_len = name.len() + ": ".len();
        let writer = EmailWriter::new(writer, line_len, false);

        (
            WordsPlusFillIterator { s: value },
            Self {
                writer,
                encode_buf: String::new(),
            },
        )
    }

    fn format(mut self, words_iter: WordsPlusFillIterator<'_>) -> fmt::Result {
        for next_word in words_iter {
            let allowed = allowed_str(next_word);

            if allowed {
                // This word only contains allowed characters

                // the next word is allowed, but we may have accumulated some words to encode
                self.flush_encode_buf()?;

                self.writer.folding().write_str(next_word)?;
            } else {
                // This word contains unallowed characters
                self.encode_buf.push_str(next_word);
            }
        }

        self.flush_encode_buf()?;

        Ok(())
    }

    fn flush_encode_buf(&mut self) -> fmt::Result {
        if self.encode_buf.is_empty() {
            // nothing to encode
            return Ok(());
        }

        // It is important that we don't encode leading whitespace otherwise it breaks wrapping.
        let first_not_allowed = self
            .encode_buf
            .bytes()
            .enumerate()
            .find(|(_i, c)| !allowed_char(*c))
            .map(|(i, _)| i);
        // May as well also write the tail in plain text.
        let last_not_allowed = self
            .encode_buf
            .bytes()
            .enumerate()
            .rev()
            .find(|(_i, c)| !allowed_char(*c))
            .map(|(i, _)| i + 1);

        let (prefix, to_encode, suffix) = match first_not_allowed {
            Some(first_not_allowed) => {
                let last_not_allowed = last_not_allowed.unwrap();

                let (remaining, suffix) = self.encode_buf.split_at(last_not_allowed);
                let (prefix, to_encode) = remaining.split_at(first_not_allowed);

                (prefix, to_encode, suffix)
            }
            None => ("", self.encode_buf.as_str(), ""),
        };

        self.writer.folding().write_str(prefix)?;
        email_encoding::headers::rfc2047::encode(to_encode, &mut self.writer)?;
        self.writer.folding().write_str(suffix)?;

        self.encode_buf.clear();
        Ok(())
    }
}

/// Iterator yielding a string split by space, but spaces are included before the next word.
struct WordsPlusFillIterator<'a> {
    s: &'a str,
}

impl<'a> Iterator for WordsPlusFillIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.s.is_empty() {
            return None;
        }

        let next_word = self
            .s
            .bytes()
            .enumerate()
            .skip(1)
            .find(|&(_i, c)| WHITESPACE_CHARS.contains(&c))
            .map(|(i, _)| i)
            .unwrap_or(self.s.len());

        let word = &self.s[..next_word];
        self.s = &self.s[word.len()..];
        Some(word)
    }
}

fn allowed_str(s: &str) -> bool {
    s.bytes().all(allowed_char)
}

const fn allowed_char(c: u8) -> bool {
    c >= 1 && c <= 9 || c == 11 || c == 12 || c >= 14 && c <= 127
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{HeaderName, HeaderValue, Headers};

    #[test]
    fn valid_headername() {
        assert!(HeaderName::new_from_ascii(String::from("From")).is_ok());
    }

    #[test]
    fn non_ascii_headername() {
        assert!(HeaderName::new_from_ascii(String::from("🌎")).is_err());
    }

    #[test]
    fn spaces_in_headername() {
        assert!(HeaderName::new_from_ascii(String::from("From ")).is_err());
    }

    #[test]
    fn colons_in_headername() {
        assert!(HeaderName::new_from_ascii(String::from("From:")).is_err());
    }

    #[test]
    fn empty_headername() {
        assert!(HeaderName::new_from_ascii(String::from("")).is_err());
    }

    #[test]
    fn const_valid_headername() {
        let _ = HeaderName::new_from_ascii_str("From");
    }

    #[test]
    #[should_panic]
    fn const_non_ascii_headername() {
        let _ = HeaderName::new_from_ascii_str("🌎");
    }

    #[test]
    #[should_panic]
    fn const_spaces_in_headername() {
        let _ = HeaderName::new_from_ascii_str("From ");
    }

    #[test]
    #[should_panic]
    fn const_colons_in_headername() {
        let _ = HeaderName::new_from_ascii_str("From:");
    }

    #[test]
    #[should_panic]
    fn const_empty_headername() {
        let _ = HeaderName::new_from_ascii_str("");
    }

    // names taken randomly from https://it.wikipedia.org/wiki/Pinco_Pallino

    #[test]
    fn format_ascii() {
        let mut headers = Headers::new();
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("To"),
            "John Doe <example@example.com>, Jean Dupont <jean@example.com>".to_string(),
        ));

        assert_eq!(
            headers.to_string(),
            "To: John Doe <example@example.com>, Jean Dupont <jean@example.com>\r\n"
        );
    }

    #[test]
    fn format_ascii_with_folding() {
        let mut headers = Headers::new();
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("To"),
            "Ascii <example@example.com>, John Doe <johndoe@example.com, John Smith <johnsmith@example.com>, Pinco Pallino <pincopallino@example.com>, Jemand <jemand@example.com>, Jean Dupont <jean@example.com>".to_string(),
        ));

        assert_eq!(
            headers.to_string(),
            concat!(
                "To: Ascii <example@example.com>, John Doe <johndoe@example.com, John Smith\r\n",
                " <johnsmith@example.com>, Pinco Pallino <pincopallino@example.com>, Jemand\r\n",
                " <jemand@example.com>, Jean Dupont <jean@example.com>\r\n"
            )
        );
    }

    #[test]
    fn format_ascii_with_folding_long_line() {
        let mut headers = Headers::new();
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("Subject"),
            "Hello! This is lettre, and this IsAVeryLongLineDoYouKnowWhatsGoingToHappenIGuessWeAreGoingToFindOut. Ok I guess that's it!".to_string()
        ));

        assert_eq!(
            headers.to_string(),
            concat!(
                "Subject: Hello! This is lettre, and this\r\n",
                " IsAVeryLongLineDoYouKnowWhatsGoingToHappenIGuessWeAreGoingToFindOut. Ok I\r\n",
                " guess that's it!\r\n"
            )
        );
    }

    #[test]
    fn format_ascii_with_folding_very_long_line() {
        let mut headers = Headers::new();
        headers.insert_raw(
            HeaderValue::new(
            HeaderName::new_from_ascii_str("Subject"),
            "Hello! IGuessTheLastLineWasntLongEnoughSoLetsTryAgainShallWeWhatDoYouThinkItsGoingToHappenIGuessWereAboutToFindOut! I don't know".to_string()
        ));

        assert_eq!(
            headers.to_string(),
            concat!(
                "Subject: Hello!\r\n",
                " IGuessTheLastLineWasntLongEnoughSoLetsTryAgainShallWeWhatDoYouThinkItsGoingToHappenIGuessWereAboutToFindOut!\r\n",
                " I don't know\r\n",
            )
        );
    }

    #[test]
    fn format_ascii_with_folding_giant_word() {
        let mut headers = Headers::new();
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("Subject"),
            "1abcdefghijklmnopqrstuvwxyz2abcdefghijklmnopqrstuvwxyz3abcdefghijklmnopqrstuvwxyz4abcdefghijklmnopqrstuvwxyz5abcdefghijklmnopqrstuvwxyz6abcdefghijklmnopqrstuvwxyz".to_string()
        ));

        assert_eq!(
            headers.to_string(),
            "Subject: 1abcdefghijklmnopqrstuvwxyz2abcdefghijklmnopqrstuvwxyz3abcdefghijklmnopqrstuvwxyz4abcdefghijklmnopqrstuvwxyz5abcdefghijklmnopqrstuvwxyz6abcdefghijklmnopqrstuvwxyz\r\n",
        );
    }

    #[test]
    fn format_special() {
        let mut headers = Headers::new();
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("To"),
            "Seán <sean@example.com>".to_string(),
        ));

        assert_eq!(
            headers.to_string(),
            "To: Se=?utf-8?b?w6E=?=n <sean@example.com>\r\n"
        );
    }

    #[test]
    fn format_special_emoji() {
        let mut headers = Headers::new();
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("To"),
            "🌎 <world@example.com>".to_string(),
        ));

        assert_eq!(
            headers.to_string(),
            "To: =?utf-8?b?8J+Mjg==?= <world@example.com>\r\n"
        );
    }

    #[test]
    fn format_special_with_folding() {
        let mut headers = Headers::new();
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("To"),
            "🌍 <world@example.com>, 🦆 Everywhere <ducks@example.com>, Иванов Иван Иванович <ivanov@example.com>, Jānis Bērziņš <janis@example.com>, Seán Ó Rudaí <sean@example.com>".to_string(),
         ) );

        assert_eq!(
            headers.to_string(),
            concat!(
                "To: =?utf-8?b?8J+MjQ==?= <world@example.com>, =?utf-8?b?8J+mhg==?= Everywhere\r\n",
                " <ducks@example.com>, =?utf-8?b?0JjQstCw0L3QvtCyINCY0LLQsNC9INCY0LLQsNC9?=\r\n",
                " =?utf-8?b?0L7QstC40Yc=?= <ivanov@example.com>, J=?utf-8?b?xIFuaXMgQsST?=\r\n",
                " =?utf-8?b?cnppxYbFoQ==?= <janis@example.com>, Se=?utf-8?b?w6FuIMOTIFJ1?=\r\n",
                " =?utf-8?b?ZGHDrQ==?= <sean@example.com>\r\n",
            )
        );
    }

    #[test]
    fn format_slice_on_char_boundary_bug() {
        let mut headers = Headers::new();
        headers.insert_raw(
            HeaderValue::new(
            HeaderName::new_from_ascii_str("Subject"),
            "🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳🥳".to_string(),)
        );

        assert_eq!(
            headers.to_string(),
            concat!(
                "Subject: =?utf-8?b?8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz?=\r\n",
                " =?utf-8?b?8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+ls/CfpbM=?=\r\n",
                " =?utf-8?b?8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+ls/CfpbM=?=\r\n",
                " =?utf-8?b?8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+ls/CfpbM=?=\r\n",
                " =?utf-8?b?8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+ls/CfpbM=?=\r\n",
                " =?utf-8?b?8J+ls/CfpbPwn6Wz8J+ls/CfpbPwn6Wz8J+lsw==?=\r\n"
            )
        );
    }

    #[test]
    fn format_bad_stuff() {
        let mut headers = Headers::new();
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("Subject"),
            "Hello! \r\n This is \" bad \0. 👋".to_string(),
        ));

        assert_eq!(
            headers.to_string(),
            "Subject: Hello! =?utf-8?b?DQo=?= This is \" bad =?utf-8?b?AC4g8J+Riw==?=\r\n"
        );
    }

    #[test]
    fn format_everything() {
        let mut headers = Headers::new();
        headers.insert_raw(
            HeaderValue::new(
            HeaderName::new_from_ascii_str("Subject"),
            "Hello! This is lettre, and this IsAVeryLongLineDoYouKnowWhatsGoingToHappenIGuessWeAreGoingToFindOut. Ok I guess that's it!".to_string()
            )
        );
        headers.insert_raw(
            HeaderValue::new(
            HeaderName::new_from_ascii_str("To"),
            "🌍 <world@example.com>, 🦆 Everywhere <ducks@example.com>, Иванов Иван Иванович <ivanov@example.com>, Jānis Bērziņš <janis@example.com>, Seán Ó Rudaí <sean@example.com>".to_string(),
            )
        );
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("From"),
            "Someone <somewhere@example.com>".to_string(),
        ));
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("Content-Transfer-Encoding"),
            "quoted-printable".to_string(),
        ));

        assert_eq!(
            headers.to_string(),
            concat!(
                "Subject: Hello! This is lettre, and this\r\n",
                " IsAVeryLongLineDoYouKnowWhatsGoingToHappenIGuessWeAreGoingToFindOut. Ok I\r\n",
                " guess that's it!\r\n",
                "To: =?utf-8?b?8J+MjQ==?= <world@example.com>, =?utf-8?b?8J+mhg==?= Everywhere\r\n",
                " <ducks@example.com>, =?utf-8?b?0JjQstCw0L3QvtCyINCY0LLQsNC9INCY0LLQsNC9?=\r\n",
                " =?utf-8?b?0L7QstC40Yc=?= <ivanov@example.com>, J=?utf-8?b?xIFuaXMgQsST?=\r\n",
                " =?utf-8?b?cnppxYbFoQ==?= <janis@example.com>, Se=?utf-8?b?w6FuIMOTIFJ1?=\r\n",
                " =?utf-8?b?ZGHDrQ==?= <sean@example.com>\r\n",
                "From: Someone <somewhere@example.com>\r\n",
                "Content-Transfer-Encoding: quoted-printable\r\n",
            )
        );
    }

    #[test]
    fn issue_653() {
        let mut headers = Headers::new();
        headers.insert_raw(HeaderValue::new(
            HeaderName::new_from_ascii_str("Subject"),
            "＋仮名 :a;go; ;;;;;s;;;;;;;;;;;;;;;;fffeinmjgggggggggｆっ".to_string(),
        ));

        assert_eq!(
            headers.to_string(),
            concat!(
                "Subject: =?utf-8?b?77yL5Luu5ZCN?= :a;go;\r\n",
                " ;;;;;s;;;;;;;;;;;;;;;;fffeinmjggggggggg=?utf-8?b?772G44Gj?=\r\n"
            )
        );
    }
}
