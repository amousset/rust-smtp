use std::{
    fmt::Display,
    io::{self, BufRead, BufReader, Write},
    net::{IpAddr, ToSocketAddrs},
    sync::Arc,
    time::Duration,
};

use rsasl::{
    mechname::Mechname,
    prelude::{SASLClient, SASLConfig},
};

#[cfg(feature = "tracing")]
use super::escape_crlf;
use super::{ClientCodec, NetworkStream, TlsParameters};
use crate::transport::smtp::error::Kind;
use crate::{
    address::Envelope,
    transport::smtp::{
        commands::{Auth, Data, Ehlo, Mail, Noop, Quit, Rcpt, Starttls},
        error,
        error::Error,
        extension::{ClientId, Extension, MailBodyParameter, MailParameter, ServerInfo},
        response::{parse_response, Response},
    },
};

macro_rules! try_smtp (
    ($err: expr, $client: ident) => ({
        match $err {
            Ok(val) => val,
            Err(err) => {
                $client.abort();
                return Err(From::from(err))
            },
        }
    })
);

/// Structure that implements the SMTP client
pub struct SmtpConnection {
    /// TCP stream between client and server
    /// Value is None before connection
    stream: BufReader<NetworkStream>,
    /// Panic state
    panic: bool,
    /// Information about the server
    server_info: ServerInfo,
}

impl SmtpConnection {
    /// Get information about the server
    pub fn server_info(&self) -> &ServerInfo {
        &self.server_info
    }

    // FIXME add simple connect and rename this one

    /// Connects to the configured server
    ///
    /// Sends EHLO and parses server information
    pub fn connect<A: ToSocketAddrs>(
        server: A,
        timeout: Option<Duration>,
        hello_name: &ClientId,
        tls_parameters: Option<&TlsParameters>,
        local_address: Option<IpAddr>,
    ) -> Result<SmtpConnection, Error> {
        let stream = NetworkStream::connect(server, timeout, tls_parameters, local_address)?;
        let stream = BufReader::new(stream);
        let mut conn = SmtpConnection {
            stream,
            panic: false,
            server_info: ServerInfo::default(),
        };
        conn.set_timeout(timeout).map_err(error::network)?;
        // TODO log
        let _response = conn.read_response()?;

        conn.ehlo(hello_name)?;

        // Print server information
        #[cfg(feature = "tracing")]
        tracing::debug!("server {}", conn.server_info);
        Ok(conn)
    }

    pub fn send(&mut self, envelope: &Envelope, email: &[u8]) -> Result<Response, Error> {
        // Mail
        let mut mail_options = vec![];

        // Internationalization handling
        //
        // * 8BITMIME: https://tools.ietf.org/html/rfc6152
        // * SMTPUTF8: https://tools.ietf.org/html/rfc653

        // Check for non-ascii addresses and use the SMTPUTF8 option if any.
        if envelope.has_non_ascii_addresses() {
            if !self.server_info().supports_feature(Extension::SmtpUtfEight) {
                // don't try to send non-ascii addresses (per RFC)
                return Err(error::client(
                    "Envelope contains non-ascii chars but server does not support SMTPUTF8",
                ));
            }
            mail_options.push(MailParameter::SmtpUtfEight);
        }

        // Check for non-ascii content in message
        if !email.is_ascii() {
            if !self.server_info().supports_feature(Extension::EightBitMime) {
                return Err(error::client(
                    "Message contains non-ascii chars but server does not support 8BITMIME",
                ));
            }
            mail_options.push(MailParameter::Body(MailBodyParameter::EightBitMime));
        }

        try_smtp!(
            self.command(Mail::new(envelope.from().cloned(), mail_options)),
            self
        );

        // Recipient
        for to_address in envelope.to() {
            try_smtp!(self.command(Rcpt::new(to_address.clone(), vec![])), self);
        }

        // Data
        try_smtp!(self.command(Data), self);

        // Message content
        let result = try_smtp!(self.message(email), self);
        Ok(result)
    }

    pub fn has_broken(&self) -> bool {
        self.panic
    }

    pub fn can_starttls(&self) -> bool {
        !self.is_encrypted() && self.server_info.supports_feature(Extension::StartTls)
    }

    #[allow(unused_variables)]
    pub fn starttls(
        &mut self,
        tls_parameters: &TlsParameters,
        hello_name: &ClientId,
    ) -> Result<(), Error> {
        if self.server_info.supports_feature(Extension::StartTls) {
            #[cfg(any(feature = "native-tls", feature = "rustls-tls", feature = "boring-tls"))]
            {
                try_smtp!(self.command(Starttls), self);
                self.stream.get_mut().upgrade_tls(tls_parameters)?;
                #[cfg(feature = "tracing")]
                tracing::debug!("connection encrypted");
                // Send EHLO again
                try_smtp!(self.ehlo(hello_name), self);
                Ok(())
            }
            #[cfg(not(any(
                feature = "native-tls",
                feature = "rustls-tls",
                feature = "boring-tls"
            )))]
            // This should never happen as `Tls` can only be created
            // when a TLS library is enabled
            unreachable!("TLS support required but not supported");
        } else {
            Err(error::client("STARTTLS is not supported on this server"))
        }
    }

    /// Send EHLO and update server info
    fn ehlo(&mut self, hello_name: &ClientId) -> Result<(), Error> {
        let ehlo_response = try_smtp!(self.command(Ehlo::new(hello_name.clone())), self);
        self.server_info = try_smtp!(ServerInfo::from_response(&ehlo_response), self);
        Ok(())
    }

    pub fn quit(&mut self) -> Result<Response, Error> {
        Ok(try_smtp!(self.command(Quit), self))
    }

    pub fn abort(&mut self) {
        // Only try to quit if we are not already broken
        if !self.panic {
            self.panic = true;
            let _ = self.command(Quit);
        }
        let _ = self.stream.get_mut().shutdown(std::net::Shutdown::Both);
    }

    /// Sets the underlying stream
    pub fn set_stream(&mut self, stream: NetworkStream) {
        self.stream = BufReader::new(stream);
    }

    /// Tells if the underlying stream is currently encrypted
    pub fn is_encrypted(&self) -> bool {
        self.stream.get_ref().is_encrypted()
    }

    /// Set timeout
    pub fn set_timeout(&mut self, duration: Option<Duration>) -> io::Result<()> {
        self.stream.get_mut().set_read_timeout(duration)?;
        self.stream.get_mut().set_write_timeout(duration)
    }

    /// Checks if the server is connected using the NOOP SMTP command
    pub fn test_connected(&mut self) -> bool {
        self.command(Noop).is_ok()
    }

    /// Sends an AUTH command with the given mechanism, and handles challenge if needed
    pub fn auth(&mut self, config: Arc<SASLConfig>) -> Result<Response, Error> {
        let client = SASLClient::new(config);
        let offered = self
            .server_info
            .get_auth_mechanisms()
            .iter()
            .filter_map(|boxed| Mechname::parse(boxed.as_bytes()).ok());
        let mut session = client
            .start_suggested_iter(offered)
            .map_err(|_| error::client("No compatible authentication mechanism was found"))?;

        // Limit challenges to avoid blocking
        let mut challenges = 10;
        let (cmd, mut state) = Auth::initial(&mut session)?;
        let mut response = self.command(cmd)?;

        while response.has_code(334) {
            // If we think that we're finished but the server doesn't, or the exchange has gone on
            // for too long, return an error.
            if challenges == 0 || state.is_finished() {
                return Err(error::response("Unexpected number of challenges"));
            }

            challenges -= 1;
            response = try_smtp!(
                self.command(Auth::from_response(&mut session, &mut state, &response)?),
                self
            );
        }

        // If the server claims authentication finished but the SASL mechanism doesn't, we must
        // call the mechanism one last time to keep all security guarantees of mechanisms.
        // This *may* be okay! However, if it is not, the below `step64` will return an
        // appropriate error.
        if state.is_running() {
            let mut scratch = Vec::new();
            session
                .step64(None, &mut scratch)
                .map_err(|error| Error::new(Kind::Client, Some(Box::new(error))))?;
        }

        Ok(response)
    }

    /// Sends the message content
    pub fn message(&mut self, message: &[u8]) -> Result<Response, Error> {
        let mut codec = ClientCodec::new();
        let mut out_buf = Vec::with_capacity(message.len());
        codec.encode(message, &mut out_buf);
        self.write(out_buf.as_slice())?;
        self.write(b"\r\n.\r\n")?;

        self.read_response()
    }

    /// Sends an SMTP command
    pub fn command<C: Display>(&mut self, command: C) -> Result<Response, Error> {
        self.write(command.to_string().as_bytes())?;
        self.read_response()
    }

    /// Writes a string to the server
    fn write(&mut self, string: &[u8]) -> Result<(), Error> {
        self.stream
            .get_mut()
            .write_all(string)
            .map_err(error::network)?;
        self.stream.get_mut().flush().map_err(error::network)?;

        #[cfg(feature = "tracing")]
        tracing::debug!("Wrote: {}", escape_crlf(&String::from_utf8_lossy(string)));
        Ok(())
    }

    /// Gets the SMTP response
    pub fn read_response(&mut self) -> Result<Response, Error> {
        let mut buffer = String::with_capacity(100);

        while self.stream.read_line(&mut buffer).map_err(error::network)? > 0 {
            #[cfg(feature = "tracing")]
            tracing::debug!("<< {}", escape_crlf(&buffer));
            match parse_response(&buffer) {
                Ok((_remaining, response)) => {
                    return if response.is_positive() {
                        Ok(response)
                    } else {
                        Err(error::code(
                            response.code(),
                            Some(response.message().collect()),
                        ))
                    };
                }
                Err(nom::Err::Failure(e)) => {
                    return Err(error::response(e.to_string()));
                }
                Err(nom::Err::Incomplete(_)) => { /* read more */ }
                Err(nom::Err::Error(e)) => {
                    return Err(error::response(e.to_string()));
                }
            }
        }

        Err(error::response("incomplete response"))
    }

    /// The X509 certificate of the server (DER encoded)
    #[cfg(any(feature = "native-tls", feature = "rustls-tls", feature = "boring-tls"))]
    pub fn peer_certificate(&self) -> Result<Vec<u8>, Error> {
        self.stream.get_ref().peer_certificate()
    }
}
