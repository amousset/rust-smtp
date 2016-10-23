//! SMTP response, containing a mandatory return code and an optional text
//! message

use self::Category::*;
use self::Severity::*;
use std::fmt::{Display, Formatter, Result};
use std::result;
use std::str::FromStr;
use transport::smtp::error::{Error, SmtpResult};

/// First digit indicates severity
#[derive(PartialEq,Eq,Copy,Clone,Debug)]
pub enum Severity {
    /// 2yx
    PositiveCompletion,
    /// 3yz
    PositiveIntermediate,
    /// 4yz
    TransientNegativeCompletion,
    /// 5yz
    PermanentNegativeCompletion,
}

impl FromStr for Severity {
    type Err = Error;
    fn from_str(s: &str) -> result::Result<Severity, Error> {
        match s {
            "2" => Ok(PositiveCompletion),
            "3" => Ok(PositiveIntermediate),
            "4" => Ok(TransientNegativeCompletion),
            "5" => Ok(PermanentNegativeCompletion),
            _ => Err(Error::ResponseParsing("First digit must be between 2 and 5")),
        }
    }
}

impl Display for Severity {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f,
               "{}",
               match *self {
                   PositiveCompletion => 2,
                   PositiveIntermediate => 3,
                   TransientNegativeCompletion => 4,
                   PermanentNegativeCompletion => 5,
               })
    }
}

/// Second digit
#[derive(PartialEq,Eq,Copy,Clone,Debug)]
pub enum Category {
    /// x0z
    Syntax,
    /// x1z
    Information,
    /// x2z
    Connections,
    /// x3z
    Unspecified3,
    /// x4z
    Unspecified4,
    /// x5z
    MailSystem,
}

impl FromStr for Category {
    type Err = Error;
    fn from_str(s: &str) -> result::Result<Category, Error> {
        match s {
            "0" => Ok(Syntax),
            "1" => Ok(Information),
            "2" => Ok(Connections),
            "3" => Ok(Unspecified3),
            "4" => Ok(Unspecified4),
            "5" => Ok(MailSystem),
            _ => Err(Error::ResponseParsing("Second digit must be between 0 and 5")),
        }
    }
}

impl Display for Category {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f,
               "{}",
               match *self {
                   Syntax => 0,
                   Information => 1,
                   Connections => 2,
                   Unspecified3 => 3,
                   Unspecified4 => 4,
                   MailSystem => 5,
               })
    }
}

/// Represents a 3 digit SMTP response code
#[derive(PartialEq,Eq,Clone,Debug)]
pub struct Code {
    /// First digit of the response code
    severity: Severity,
    /// Second digit of the response code
    category: Category,
    /// Third digit
    detail: u8,
}

impl FromStr for Code {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> result::Result<Code, Error> {
        if s.len() == 3 {
            match (s[0..1].parse::<Severity>(),
                   s[1..2].parse::<Category>(),
                   s[2..3].parse::<u8>()) {
                (Ok(severity), Ok(category), Ok(detail)) => {
                    Ok(Code {
                        severity: severity,
                        category: category,
                        detail: detail,
                    })
                }
                _ => Err(Error::ResponseParsing("Could not parse response code")),
            }
        } else {
            Err(Error::ResponseParsing("Wrong code length (should be 3 digit)"))
        }
    }
}

impl Code {
    /// Creates a new `Code` structure
    pub fn new(severity: Severity, category: Category, detail: u8) -> Code {
        Code {
            severity: severity,
            category: category,
            detail: detail,
        }
    }

    /// Returns the reply code
    pub fn code(&self) -> String {
        format!("{}{}{}", self.severity, self.category, self.detail)
    }
}

/// Parses an SMTP response
#[derive(PartialEq,Eq,Clone,Debug,Default)]
pub struct ResponseParser {
    /// Response code
    code: Option<Code>,
    /// Server response string (optional)
    /// Handle multiline responses
    message: Vec<String>,
}

impl ResponseParser {
    /// Parses a line and return a `bool` indicating if there are more lines to come
    pub fn read_line(&mut self, line: &str) -> result::Result<bool, Error> {

        if line.len() < 3 {
            return Err(Error::ResponseParsing("Wrong code length (should be 3 digit)"));
        }

        match self.code {
            Some(ref code) => {
                if code.code() != line[0..3] {
                    return Err(Error::ResponseParsing("Response code has changed during a \
                                                            reponse"));
                }
            }
            None => self.code = Some(try!(line[0..3].parse::<Code>())),
        }

        if line.len() > 4 {
            self.message.push(line[4..].to_string());
            Ok(line.as_bytes()[3] == b'-')
        } else {
            Ok(false)
        }
    }

    /// Builds a response from a `ResponseParser`
    pub fn response(self) -> SmtpResult {
        match self.code {
            Some(code) => Ok(Response::new(code, self.message)),
            None => {
                Err(Error::ResponseParsing("Incomplete response, could not read response \
                                                 code"))
            }
        }
    }
}

/// Contains an SMTP reply, with separated code and message
///
/// The text message is optional, only the code is mandatory
#[derive(PartialEq,Eq,Clone,Debug)]
pub struct Response {
    /// Response code
    code: Code,
    /// Server response string (optional)
    /// Handle multiline responses
    message: Vec<String>,
}

impl Response {
    /// Creates a new `Response`
    pub fn new(code: Code, message: Vec<String>) -> Response {
        Response {
            code: code,
            message: message,
        }
    }

    /// Tells if the response is positive
    pub fn is_positive(&self) -> bool {
        match self.code.severity {
            PositiveCompletion => true,
            PositiveIntermediate => true,
            _ => false,
        }
    }

    /// Returns the message
    pub fn message(&self) -> Vec<String> {
        self.message.clone()
    }

    /// Returns the severity (i.e. 1st digit)
    pub fn severity(&self) -> Severity {
        self.code.severity
    }

    /// Returns the category (i.e. 2nd digit)
    pub fn category(&self) -> Category {
        self.code.category
    }

    /// Returns the detail (i.e. 3rd digit)
    pub fn detail(&self) -> u8 {
        self.code.detail
    }

    /// Returns the reply code
    fn code(&self) -> String {
        self.code.code()
    }

    /// Tests code equality
    pub fn has_code(&self, code: u16) -> bool {
        self.code() == format!("{}", code)
    }

    /// Returns only the first word of the message if possible
    pub fn first_word(&self) -> Option<String> {
        if self.message.is_empty() {
            None
        } else {
            match self.message[0].split_whitespace().next() {
                Some(word) => Some(word.to_string()),
                None => None,
            }
        }

    }
}

#[cfg(test)]
mod test {
    use super::{Category, Code, Response, ResponseParser, Severity};

    #[test]
    fn test_severity_from_str() {
        assert_eq!("2".parse::<Severity>().unwrap(),
                   Severity::PositiveCompletion);
        assert_eq!("4".parse::<Severity>().unwrap(),
                   Severity::TransientNegativeCompletion);
        assert!("1".parse::<Severity>().is_err());
    }

    #[test]
    fn test_severity_fmt() {
        assert_eq!(format!("{}", Severity::PositiveCompletion), "2");
    }

    #[test]
    fn test_category_from_str() {
        assert_eq!("2".parse::<Category>().unwrap(), Category::Connections);
        assert_eq!("4".parse::<Category>().unwrap(), Category::Unspecified4);
        assert!("6".parse::<Category>().is_err());
    }

    #[test]
    fn test_category_fmt() {
        assert_eq!(format!("{}", Category::Unspecified4), "4");
    }

    #[test]
    fn test_code_new() {
        assert_eq!(Code::new(Severity::TransientNegativeCompletion,
                             Category::Connections,
                             0),
                   Code {
                       severity: Severity::TransientNegativeCompletion,
                       category: Category::Connections,
                       detail: 0,
                   });
    }

    #[test]
    fn test_code_from_str() {
        assert_eq!("421".parse::<Code>().unwrap(),
                   Code {
                       severity: Severity::TransientNegativeCompletion,
                       category: Category::Connections,
                       detail: 1,
                   });
    }

    #[test]
    fn test_code_code() {
        let code = Code {
            severity: Severity::TransientNegativeCompletion,
            category: Category::Connections,
            detail: 1,
        };

        assert_eq!(code.code(), "421");
    }

    #[test]
    fn test_response_new() {
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["me".to_string(),
                                      "8BITMIME".to_string(),
                                      "SIZE 42".to_string()]),
                   Response {
                       code: Code {
                           severity: Severity::PositiveCompletion,
                           category: Category::Unspecified4,
                           detail: 1,
                       },
                       message: vec!["me".to_string(),
                                     "8BITMIME".to_string(),
                                     "SIZE 42".to_string()],
                   });
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec![]),
                   Response {
                       code: Code {
                           severity: Severity::PositiveCompletion,
                           category: Category::Unspecified4,
                           detail: 1,
                       },
                       message: vec![],
                   });
    }

    #[test]
    fn test_response_parser() {
        let mut parser = ResponseParser::default();

        assert!(parser.read_line("250-me").unwrap());
        assert!(parser.read_line("250-8BITMIME").unwrap());
        assert!(parser.read_line("250-SIZE 42").unwrap());
        assert!(!parser.read_line("250 AUTH PLAIN CRAM-MD5").unwrap());

        let response = parser.response().unwrap();

        assert_eq!(response,
                   Response {
                       code: Code {
                           severity: Severity::PositiveCompletion,
                           category: Category::MailSystem,
                           detail: 0,
                       },
                       message: vec!["me".to_string(),
                                     "8BITMIME".to_string(),
                                     "SIZE 42".to_string(),
                                     "AUTH PLAIN CRAM-MD5".to_string()],
                   });
    }

    #[test]
    fn test_response_is_positive() {
        assert!(Response::new(Code {
                                  severity: "2".parse::<Severity>().unwrap(),
                                  category: "4".parse::<Category>().unwrap(),
                                  detail: 1,
                              },
                              vec!["me".to_string(),
                                   "8BITMIME".to_string(),
                                   "SIZE 42".to_string()])
            .is_positive());
        assert!(!Response::new(Code {
                                   severity: "5".parse::<Severity>().unwrap(),
                                   category: "4".parse::<Category>().unwrap(),
                                   detail: 1,
                               },
                               vec!["me".to_string(),
                                    "8BITMIME".to_string(),
                                    "SIZE 42".to_string()])
            .is_positive());
    }

    #[test]
    fn test_response_message() {
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["me".to_string(),
                                      "8BITMIME".to_string(),
                                      "SIZE 42".to_string()])
                       .message(),
                   vec!["me".to_string(), "8BITMIME".to_string(), "SIZE 42".to_string()]);
        let empty_message: Vec<String> = vec![];
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec![])
                       .message(),
                   empty_message);
    }

    #[test]
    fn test_response_severity() {
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["me".to_string(),
                                      "8BITMIME".to_string(),
                                      "SIZE 42".to_string()])
                       .severity(),
                   Severity::PositiveCompletion);
        assert_eq!(Response::new(Code {
                                     severity: "5".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["me".to_string(),
                                      "8BITMIME".to_string(),
                                      "SIZE 42".to_string()])
                       .severity(),
                   Severity::PermanentNegativeCompletion);
    }

    #[test]
    fn test_response_category() {
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["me".to_string(),
                                      "8BITMIME".to_string(),
                                      "SIZE 42".to_string()])
                       .category(),
                   Category::Unspecified4);
    }

    #[test]
    fn test_response_detail() {
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["me".to_string(),
                                      "8BITMIME".to_string(),
                                      "SIZE 42".to_string()])
                       .detail(),
                   1);
    }

    #[test]
    fn test_response_code() {
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["me".to_string(),
                                      "8BITMIME".to_string(),
                                      "SIZE 42".to_string()])
                       .code(),
                   "241");
    }

    #[test]
    fn test_response_has_code() {
        assert!(Response::new(Code {
                                  severity: "2".parse::<Severity>().unwrap(),
                                  category: "4".parse::<Category>().unwrap(),
                                  detail: 1,
                              },
                              vec!["me".to_string(),
                                   "8BITMIME".to_string(),
                                   "SIZE 42".to_string()])
            .has_code(241));
        assert!(!Response::new(Code {
                                   severity: "2".parse::<Severity>().unwrap(),
                                   category: "4".parse::<Category>().unwrap(),
                                   detail: 1,
                               },
                               vec!["me".to_string(),
                                    "8BITMIME".to_string(),
                                    "SIZE 42".to_string()])
            .has_code(251));
    }

    #[test]
    fn test_response_first_word() {
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["me".to_string(),
                                      "8BITMIME".to_string(),
                                      "SIZE 42".to_string()])
                       .first_word(),
                   Some("me".to_string()));
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["me mo".to_string(),
                                      "8BITMIME".to_string(),
                                      "SIZE 42".to_string()])
                       .first_word(),
                   Some("me".to_string()));
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec![])
                       .first_word(),
                   None);
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec![" ".to_string()])
                       .first_word(),
                   None);
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["  ".to_string()])
                       .first_word(),
                   None);
        assert_eq!(Response::new(Code {
                                     severity: "2".parse::<Severity>().unwrap(),
                                     category: "4".parse::<Category>().unwrap(),
                                     detail: 1,
                                 },
                                 vec!["".to_string()])
                       .first_word(),
                   None);
    }
}
