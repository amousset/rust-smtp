//! SMTP response, containing a mandatory return code and an optional text
//! message

use nom::{crlf, ErrorKind as NomErrorKind};
use std::fmt::{Display, Formatter, Result};
use std::result;
use std::str::{FromStr, from_utf8};

/// First digit indicates severity
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[cfg_attr(feature = "serde-impls", derive(Serialize, Deserialize))]
pub enum Severity {
    /// 2yx
    PositiveCompletion = 2,
    /// 3yz
    PositiveIntermediate = 3,
    /// 4yz
    TransientNegativeCompletion = 4,
    /// 5yz
    PermanentNegativeCompletion = 5,
}

impl Display for Severity {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", *self as u8)
    }
}

/// Second digit
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[cfg_attr(feature = "serde-impls", derive(Serialize, Deserialize))]
pub enum Category {
    /// x0z
    Syntax = 0,
    /// x1z
    Information = 1,
    /// x2z
    Connections = 2,
    /// x3z
    Unspecified3 = 3,
    /// x4z
    Unspecified4 = 4,
    /// x5z
    MailSystem = 5,
}

impl Display for Category {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", *self as u8)
    }
}

/// The detail digit of a response code (third digit)
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[cfg_attr(feature = "serde-impls", derive(Serialize, Deserialize))]
pub enum Detail {
    #[allow(missing_docs)]
    Zero = 0,
    #[allow(missing_docs)]
    One = 1,
    #[allow(missing_docs)]
    Two = 2,
    #[allow(missing_docs)]
    Three = 3,
    #[allow(missing_docs)]
    Four = 4,
    #[allow(missing_docs)]
    Five = 5,
    #[allow(missing_docs)]
    Six = 6,
    #[allow(missing_docs)]
    Seven = 7,
    #[allow(missing_docs)]
    Eight = 8,
    #[allow(missing_docs)]
    Nine = 9,
}

impl Display for Detail {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", *self as u8)
    }
}

/// Represents a 3 digit SMTP response code
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[cfg_attr(feature = "serde-impls", derive(Serialize, Deserialize))]
pub struct Code {
    /// First digit of the response code
    pub severity: Severity,
    /// Second digit of the response code
    pub category: Category,
    /// Third digit
    pub detail: Detail,
}

impl Display for Code {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}{}{}", self.severity, self.category, self.detail)
    }
}

impl Code {
    /// Creates a new `Code` structure
    pub fn new(severity: Severity, category: Category, detail: Detail) -> Code {
        Code {
            severity,
            category,
            detail,
        }
    }
}

/// Contains an SMTP reply, with separated code and message
///
/// The text message is optional, only the code is mandatory
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde-impls", derive(Serialize, Deserialize))]
pub struct Response {
    /// Response code
    pub code: Code,
    /// Server response string (optional)
    /// Handle multiline responses
    pub message: Vec<String>,
}

impl FromStr for Response {
    type Err = NomErrorKind;

    fn from_str(s: &str) -> result::Result<Response, NomErrorKind> {
        match parse_response(s.as_bytes()) {
            Ok((_, res)) => Ok(res),
            Err(e) => Err(e.into_error_kind()),
        }
    }
}

impl Response {
    /// Creates a new `Response`
    pub fn new(code: Code, message: Vec<String>) -> Response {
        Response { code, message }
    }

    /// Tells if the response is positive
    pub fn is_positive(&self) -> bool {
        match self.code.severity {
            Severity::PositiveCompletion | Severity::PositiveIntermediate => true,
            _ => false,
        }
    }

    /// Tests code equality
    pub fn has_code(&self, code: u16) -> bool {
        self.code.to_string() == code.to_string()
    }

    /// Returns only the first word of the message if possible
    pub fn first_word(&self) -> Option<&str> {
        self.message
            .get(0)
            .and_then(|line| line.split_whitespace().next())
    }

    /// Returns only the line of the message if possible
    pub fn first_line(&self) -> Option<&str> {
        self.message.first().map(String::as_str)
    }
}

// Parsers (originally from tokio-smtp)

named!(
    parse_code<Code>,
    map!(
        tuple!(parse_severity, parse_category, parse_detail),
        |(severity, category, detail)| Code {
            severity,
            category,
            detail,
        }
    )
);

named!(
    parse_severity<Severity>,
    alt!(
        tag!("2") => { |_| Severity::PositiveCompletion } |
        tag!("3") => { |_| Severity::PositiveIntermediate } |
        tag!("4") => { |_| Severity::TransientNegativeCompletion } |
        tag!("5") => { |_| Severity::PermanentNegativeCompletion }
    )
);

named!(
    parse_category<Category>,
    alt!(
        tag!("0") => { |_| Category::Syntax } |
        tag!("1") => { |_| Category::Information } |
        tag!("2") => { |_| Category::Connections } |
        tag!("3") => { |_| Category::Unspecified3 } |
        tag!("4") => { |_| Category::Unspecified4 } |
        tag!("5") => { |_| Category::MailSystem }
    )
);

named!(
    parse_detail<Detail>,
    alt!(
        tag!("0") => { |_| Detail::Zero } |
        tag!("1") => { |_| Detail::One } |
        tag!("2") => { |_| Detail::Two } |
        tag!("3") => { |_| Detail::Three } |
        tag!("4") => { |_| Detail::Four} |
        tag!("5") => { |_| Detail::Five } |
        tag!("6") => { |_| Detail::Six} |
        tag!("7") => { |_| Detail::Seven } |
        tag!("8") => { |_| Detail::Eight } |
        tag!("9") => { |_| Detail::Nine }
    )
);

named!(
    parse_response<Response>,
    map_res!(
        tuple!(
            // Parse any number of continuation lines.
            many0!(tuple!(
                parse_code,
                preceded!(char!('-'), take_until_and_consume!(b"\r\n".as_ref()))
            )),
            // Parse the final line.
            tuple!(
                parse_code,
                terminated!(
                    opt!(preceded!(char!(' '), take_until!(b"\r\n".as_ref()))),
                    crlf
                )
            )
        ),
        |(lines, (last_code, last_line)): (Vec<_>, _)| {
            // Check that all codes are equal.
            if !lines.iter().all(|&(ref code, _)| *code == last_code) {
                return Err(());
            }

            // Extract text from lines, and append last line.
            let mut lines = lines.into_iter().map(|(_, text)| text).collect::<Vec<_>>();
            if let Some(text) = last_line {
                lines.push(text);
            }

            Ok(Response {
                code: last_code,
                message: lines
                    .into_iter()
                    .map(|line| from_utf8(line).map(|s| s.to_string()))
                    .collect::<result::Result<Vec<_>, _>>()
                    .map_err(|_| ())?,
            })
        }
    )
);

#[cfg(test)]
mod test {
    use super::{Category, Code, Detail, Response, Severity};

    #[test]
    fn test_severity_fmt() {
        assert_eq!(format!("{}", Severity::PositiveCompletion), "2");
    }

    #[test]
    fn test_category_fmt() {
        assert_eq!(format!("{}", Category::Unspecified4), "4");
    }

    #[test]
    fn test_code_new() {
        assert_eq!(
            Code::new(
                Severity::TransientNegativeCompletion,
                Category::Connections,
                Detail::Zero,
            ),
            Code {
                severity: Severity::TransientNegativeCompletion,
                category: Category::Connections,
                detail: Detail::Zero,
            }
        );
    }

    #[test]
    fn test_code_display() {
        let code = Code {
            severity: Severity::TransientNegativeCompletion,
            category: Category::Connections,
            detail: Detail::One,
        };

        assert_eq!(code.to_string(), "421");
    }

    #[test]
    fn test_response_from_str() {
        let raw_response = "250-me\r\n250-8BITMIME\r\n250-SIZE 42\r\n250 AUTH PLAIN CRAM-MD5\r\n";
        assert_eq!(
            raw_response.parse::<Response>().unwrap(),
            Response {
                code: Code {
                    severity: Severity::PositiveCompletion,
                    category: Category::MailSystem,
                    detail: Detail::Zero,
                },
                message: vec![
                    "me".to_string(),
                    "8BITMIME".to_string(),
                    "SIZE 42".to_string(),
                    "AUTH PLAIN CRAM-MD5".to_string(),
                ],
            }
        );

        let wrong_code = "2506-me\r\n250-8BITMIME\r\n250-SIZE 42\r\n250 AUTH PLAIN CRAM-MD5\r\n";
        assert!(wrong_code.parse::<Response>().is_err());

        let wrong_end = "250-me\r\n250-8BITMIME\r\n250-SIZE 42\r\n250-AUTH PLAIN CRAM-MD5\r\n";
        assert!(wrong_end.parse::<Response>().is_err());
    }

    #[test]
    fn test_response_is_positive() {
        assert!(
            Response::new(
                Code {
                    severity: Severity::PositiveCompletion,
                    category: Category::MailSystem,
                    detail: Detail::Zero,
                },
                vec![
                    "me".to_string(),
                    "8BITMIME".to_string(),
                    "SIZE 42".to_string(),
                ],
            ).is_positive()
        );
        assert!(!Response::new(
            Code {
                severity: Severity::TransientNegativeCompletion,
                category: Category::MailSystem,
                detail: Detail::Zero,
            },
            vec![
                "me".to_string(),
                "8BITMIME".to_string(),
                "SIZE 42".to_string(),
            ],
        ).is_positive());
    }

    #[test]
    fn test_response_has_code() {
        assert!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec![
                    "me".to_string(),
                    "8BITMIME".to_string(),
                    "SIZE 42".to_string(),
                ],
            ).has_code(451)
        );
        assert!(!Response::new(
            Code {
                severity: Severity::TransientNegativeCompletion,
                category: Category::MailSystem,
                detail: Detail::One,
            },
            vec![
                "me".to_string(),
                "8BITMIME".to_string(),
                "SIZE 42".to_string(),
            ],
        ).has_code(251));
    }

    #[test]
    fn test_response_first_word() {
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec![
                    "me".to_string(),
                    "8BITMIME".to_string(),
                    "SIZE 42".to_string(),
                ],
            ).first_word(),
            Some("me")
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec![
                    "me mo".to_string(),
                    "8BITMIME".to_string(),
                    "SIZE 42".to_string(),
                ],
            ).first_word(),
            Some("me")
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec![],
            ).first_word(),
            None
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec![" ".to_string()],
            ).first_word(),
            None
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec!["  ".to_string()],
            ).first_word(),
            None
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec!["".to_string()],
            ).first_word(),
            None
        );
    }

    #[test]
    fn test_response_first_line() {
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec![
                    "me".to_string(),
                    "8BITMIME".to_string(),
                    "SIZE 42".to_string(),
                ],
            ).first_line(),
            Some("me")
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec![
                    "me mo".to_string(),
                    "8BITMIME".to_string(),
                    "SIZE 42".to_string(),
                ],
            ).first_line(),
            Some("me mo")
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec![],
            ).first_line(),
            None
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec![" ".to_string()],
            ).first_line(),
            Some(" ")
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec!["  ".to_string()],
            ).first_line(),
            Some("  ")
        );
        assert_eq!(
            Response::new(
                Code {
                    severity: Severity::TransientNegativeCompletion,
                    category: Category::MailSystem,
                    detail: Detail::One,
                },
                vec!["".to_string()],
            ).first_line(),
            Some("")
        );
    }
}
