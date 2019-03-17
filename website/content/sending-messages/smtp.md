SMTP Transport

This transport uses the SMTP protocol to send emails over the network (locally or remotely).

It is designed to be:

* Secured: email are encrypted by default
* Modern: Unicode support for email content and sender/recipient addresses when compatible
* Fast: supports tcp connection reuse

This client is designed to send emails to a relay server, and should *not* be used to send
emails directly to the destination.

The relay server can be the local email server, a specific host or a third-party service.

#### Simple example

This is the most basic example of usage:

```rust,no_run
extern crate lettre;

use lettre::{SimpleSendableEmail, EmailTransport, SmtpTransport};

fn main() {
    let email = SimpleSendableEmail::new(
                    "user@localhost".to_string(),
                    &["root@localhost".to_string()],
                    "message_id".to_string(),
                    "Hello world".to_string(),
                ).unwrap();
    
    // Open a local connection on port 25
    let mut mailer =
    SmtpTransport::builder_unencrypted_localhost().unwrap().build();
    // Send the email
    let result = mailer.send(&email);
    
    assert!(result.is_ok());
}
```

#### Complete example

```rust,no_run
extern crate lettre;

use lettre::smtp::authentication::{Credentials, Mechanism};
use lettre::{SimpleSendableEmail, EmailTransport, SmtpTransport};
use lettre::smtp::extension::ClientId;
use lettre::smtp::ConnectionReuseParameters;

fn main() {
    let email = SimpleSendableEmail::new(
                    "user@localhost".to_string(),
                    &["root@localhost".to_string()],
                    "message_id".to_string(),
                    "Hello world".to_string(),
                ).unwrap();
    
    // Connect to a remote server on a custom port
    let mut mailer = SmtpTransport::simple_builder("server.tld").unwrap()
        // Set the name sent during EHLO/HELO, default is `localhost`
        .hello_name(ClientId::Domain("my.hostname.tld".to_string()))
        // Add credentials for authentication
        .credentials(Credentials::new("username".to_string(), "password".to_string()))
        // Enable SMTPUTF8 if the server supports it
        .smtp_utf8(true)
        // Configure expected authentication mechanism
        .authentication_mechanism(Mechanism::Plain)
        // Enable connection reuse
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited).build();
    
    let result_1 = mailer.send(&email);
    assert!(result_1.is_ok());
    
    // The second email will use the same connection
    let result_2 = mailer.send(&email);
    assert!(result_2.is_ok());
    
    // Explicitly close the SMTP transaction as we enabled connection reuse
    mailer.close();
}
```

You can specify custom TLS settings:

```rust,no_run
extern crate native_tls;
extern crate lettre;
extern crate lettre_email;

use native_tls::TlsConnector;
use native_tls::{Protocol};
use lettre::smtp::authentication::{Credentials, Mechanism};
use lettre::{EmailTransport, SimpleSendableEmail, ClientTlsParameters, ClientSecurity};
use lettre::smtp::ConnectionReuseParameters;
use lettre::smtp::{SmtpTransportBuilder};
use lettre_email::EmailBuilder;

fn main() {
    let email = SimpleSendableEmail::new(
                    "user@localhost".to_string(),
                    &["root@localhost".to_string()],
                    "message_id".to_string(),
                    "Hello world".to_string(),
                ).unwrap();

    let mut tls_builder = TlsConnector::builder().unwrap();
    tls_builder.supported_protocols(&[Protocol::Tlsv10]).unwrap();
    let tls_parameters =
        ClientTlsParameters::new(
            "smtp.example.com".to_string(),
            tls_builder.build().unwrap()
        );

    let mut mailer = SmtpTransportBuilder::new(
        ("smtp.example.com", 465), ClientSecurity::Wrapper(tls_parameters)
    )
        .expect("Failed to create transport")
        .authentication_mechanism(Mechanism::Login)
        .credentials(Credentials::new(
            "example_username".to_string(), "example_password".to_string()
        ))
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
        .build();

    let result = mailer.send(&email);
    
    assert!(result.is_ok());
    
    mailer.close();
}
```

#### Lower level

You can also send commands, here is a simple email transaction without
error handling:

```rust,no_run
extern crate lettre;

use lettre::EmailAddress;
use lettre::smtp::SMTP_PORT;
use lettre::smtp::client::Client;
use lettre::smtp::client::net::NetworkStream;
use lettre::smtp::extension::ClientId;
use lettre::smtp::commands::*;

fn main() {
    let mut email_client: Client<NetworkStream> = Client::new();
    let _ = email_client.connect(&("localhost", SMTP_PORT), None);
    let _ = email_client.command(EhloCommand::new(ClientId::new("my_hostname".to_string())));
    let _ = email_client.command(
                MailCommand::new(Some(EmailAddress::new("user@example.com".to_string()).unwrap()), vec![])
            );
    let _ = email_client.command(
                RcptCommand::new(EmailAddress::new("user@example.org".to_string()).unwrap(), vec![])
            );
    let _ = email_client.command(DataCommand);
    let _ = email_client.message(Box::new("Test email".as_bytes()));
    let _ = email_client.command(QuitCommand);
}
```

