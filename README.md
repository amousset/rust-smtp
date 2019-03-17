# lettre

**Lettre is a mailer library for Rust.**

[![Build Status](https://travis-ci.org/lettre/lettre.svg?branch=master)](https://travis-ci.org/lettre/lettre)
[![Build status](https://ci.appveyor.com/api/projects/status/mpwglemugjtkps2d/branch/master?svg=true)](https://ci.appveyor.com/project/amousset/lettre/branch/master)
[![codecov](https://codecov.io/gh/lettre/lettre/branch/master/graph/badge.svg)](https://codecov.io/gh/lettre/lettre)

[![Crate](https://img.shields.io/crates/v/lettre.svg)](https://crates.io/crates/lettre)
[![Docs](https://docs.rs/lettre/badge.svg)](https://docs.rs/lettre/)
[![Required Rust version](https://img.shields.io/badge/rustc-1.20-green.svg)]()
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

[![Gitter](https://badges.gitter.im/lettre/lettre.svg)](https://gitter.im/lettre/lettre?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)
[![Average time to resolve an issue](http://isitmaintained.com/badge/resolution/lettre/lettre.svg)](http://isitmaintained.com/project/lettre/lettre "Average time to resolve an issue")
[![Percentage of issues still open](http://isitmaintained.com/badge/open/lettre/lettre.svg)](http://isitmaintained.com/project/lettre/lettre "Percentage of issues still open")

Useful links:

* [User documentation](http://lettre.at/)
* [API documentation](https://docs.rs/lettre/)
* [Changelog](https://github.com/lettre/lettre/blob/master/CHANGELOG.md)

---

## Features

Lettre provides the following features:

* Multiple transport methods
* Unicode support (for email content and addresses)
* Secure delivery with SMTP using encryption and authentication
* Easy email builders

## Example

This library requires Rust 1.20 or newer.
To use this library, add the following to your `Cargo.toml`:

```toml
[dependencies]
lettre = "0.8"
lettre_email = "0.8"
```

```rust,no_run
extern crate lettre;
extern crate lettre_email;

use lettre::{EmailTransport, SmtpTransport};
use lettre_email::EmailBuilder;
use std::path::Path;

fn main() {
    let email = EmailBuilder::new()
        // Addresses can be specified by the tuple (email, alias)
        .to(("user@example.org", "Firstname Lastname"))
        // ... or by an address only
        .from("user@example.com")
        .subject("Hi, Hello world")
        .text("Hello world.")
        .build()
        .unwrap();

    // Open a local connection on port 25
    let mut mailer = SmtpTransport::builder_unencrypted_localhost().unwrap()
                                                                   .build();
    // Send the email
    let result = mailer.send(&email);

    if result.is_ok() {
        println!("Email sent");
    } else {
        println!("Could not send email: {:?}", result);
    }

    assert!(result.is_ok());
}
```

## Testing

The `lettre` tests require an open mail server listening locally on port 2525 and the `sendmail` command.

## Code of conduct

Anyone who interacts with Lettre in any space, including but not limited to
this GitHub repository, must follow our [code of conduct](https://github.com/lettre/lettre/blob/master/CODE_OF_CONDUCT.md).

## License

This program is distributed under the terms of the MIT license.

See [LICENSE](./LICENSE) for details.
