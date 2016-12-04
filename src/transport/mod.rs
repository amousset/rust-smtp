//! Represents an Email transport

pub mod smtp;
pub mod sendmail;
pub mod stub;
pub mod file;
#[cfg(feature = "mailgun")] pub mod mailgun;

use email::SendableEmail;

/// Transport method for emails
pub trait EmailTransport<U> {
    /// Sends the email
    fn send<T: SendableEmail>(&mut self, email: T) -> U;
    /// Close the transport explicitly
    fn close(&mut self);
}
