//! Lettre provides an email builder and several email transports.
//!

#![doc(html_root_url = "https://docs.rs/lettre/0.10.0")]
#![doc(html_favicon_url = "https://lettre.at/favicon.png")]
#![doc(html_logo_url = "https://avatars0.githubusercontent.com/u/15113230?v=4")]
#![deny(
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_import_braces,
    unsafe_code
)]

pub mod address;
pub mod error;
#[cfg(feature = "builder")]
pub mod message;
pub mod transport;

pub use crate::address::Address;
use crate::error::Error;
#[cfg(feature = "builder")]
pub use crate::message::{
    header::{self, Headers},
    EmailFormat, Mailboxes, Message,
};
#[cfg(feature = "file-transport")]
pub use crate::transport::file::FileTransport;
#[cfg(feature = "sendmail-transport")]
pub use crate::transport::sendmail::SendmailTransport;
#[cfg(feature = "smtp-transport")]
pub use crate::transport::smtp::client::net::ClientTlsParameters;
#[cfg(all(feature = "smtp-transport", feature = "connection-pool"))]
pub use crate::transport::smtp::r2d2::SmtpConnectionManager;
#[cfg(feature = "smtp-transport")]
pub use crate::transport::smtp::{ClientSecurity, SmtpClient, SmtpTransport};
#[cfg(feature = "builder")]
use std::convert::TryFrom;

/// Simple email envelope representation
///
/// We only accept mailboxes, and do not support source routes (as per RFC).
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Envelope {
    /// The envelope recipients' addresses
    ///
    /// This can not be empty.
    forward_path: Vec<Address>,
    /// The envelope sender address
    reverse_path: Option<Address>,
}

impl Envelope {
    /// Creates a new envelope, which may fail if `to` is empty.
    pub fn new(from: Option<Address>, to: Vec<Address>) -> Result<Envelope, Error> {
        if to.is_empty() {
            return Err(Error::MissingTo);
        }
        Ok(Envelope {
            forward_path: to,
            reverse_path: from,
        })
    }

    /// Destination addresses of the envelope
    pub fn to(&self) -> &[Address] {
        self.forward_path.as_slice()
    }

    /// Source address of the envelope
    pub fn from(&self) -> Option<&Address> {
        self.reverse_path.as_ref()
    }
}

impl TryFrom<&Headers> for Envelope {
    type Error = Error;

    fn try_from(headers: &Headers) -> Result<Self, Self::Error> {
        let from = match headers.get::<header::Sender>() {
            // If there is a Sender, use it
            Some(header::Sender(a)) => Some(a.email.clone()),
            // ... else use the first From address
            None => match headers.get::<header::From>() {
                Some(header::From(ref a)) => Some(a.iter().next().unwrap().email.clone()),
                None => None,
            },
        };

        fn add_addresses_from_mailboxes(
            addresses: &mut Vec<Address>,
            mailboxes: Option<&Mailboxes>,
        ) {
            if let Some(mailboxes) = mailboxes {
                for mailbox in mailboxes.iter() {
                    addresses.push(mailbox.email.clone());
                }
            }
        }
        let mut to = vec![];
        add_addresses_from_mailboxes(&mut to, headers.get::<header::To>().map(|h| &h.0));
        add_addresses_from_mailboxes(&mut to, headers.get::<header::Cc>().map(|h| &h.0));
        add_addresses_from_mailboxes(&mut to, headers.get::<header::Bcc>().map(|h| &h.0));

        Self::new(from, to)
    }
}

/// Transport method for emails
pub trait Transport<'a> {
    /// Result type for the transport
    type Result;

    /// Sends the email
    /// FIXME not mut

    fn send(&mut self, message: &Message) -> Self::Result {
        let raw = message.formatted();
        self.send_raw(message.envelope(), &raw)
    }

    fn send_raw(&mut self, envelope: &Envelope, email: &[u8]) -> Self::Result;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::message::{header, Mailbox, Mailboxes};
    use hyperx::header::Headers;

    #[test]
    fn envelope_from_headers() {
        let from = Mailboxes::new().with("kayo@example.com".parse().unwrap());
        let to = Mailboxes::new().with("amousset@example.com".parse().unwrap());

        let mut headers = Headers::new();
        headers.set(header::From(from));
        headers.set(header::To(to));

        assert_eq!(
            Envelope::try_from(&headers).unwrap(),
            Envelope::new(
                Some(Address::new("kayo", "example.com").unwrap()),
                vec![Address::new("amousset", "example.com").unwrap()]
            )
            .unwrap()
        );
    }

    #[test]
    fn envelope_from_headers_sender() {
        let from = Mailboxes::new().with("kayo@example.com".parse().unwrap());
        let sender = Mailbox::new(None, "kayo2@example.com".parse().unwrap());
        let to = Mailboxes::new().with("amousset@example.com".parse().unwrap());

        let mut headers = Headers::new();
        headers.set(header::From(from));
        headers.set(header::Sender(sender));
        headers.set(header::To(to));

        assert_eq!(
            Envelope::try_from(&headers).unwrap(),
            Envelope::new(
                Some(Address::new("kayo2", "example.com").unwrap()),
                vec![Address::new("amousset", "example.com").unwrap()]
            )
            .unwrap()
        );
    }

    #[test]
    fn envelope_from_headers_no_to() {
        let from = Mailboxes::new().with("kayo@example.com".parse().unwrap());
        let sender = Mailbox::new(None, "kayo2@example.com".parse().unwrap());

        let mut headers = Headers::new();
        headers.set(header::From(from));
        headers.set(header::Sender(sender));

        assert!(Envelope::try_from(&headers).is_err(),);
    }
}
