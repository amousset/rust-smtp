//! The stub transport only logs message envelope and drops the content. It can be useful for
//! testing purposes.
//!

use crate::Envelope;
use crate::Transport;
use log::info;


/// This transport logs the message envelope and returns the given response
#[derive(Debug, Clone, Copy)]
pub struct StubTransport {
    response: StubResult,
}

impl StubTransport {
    /// Creates a new transport that always returns the given response
    pub fn new(response: StubResult) -> StubTransport {
        StubTransport { response }
    }

    /// Creates a new transport that always returns a success response
    pub fn new_positive() -> StubTransport {
        StubTransport { response: Ok(()) }
    }
}

/// SMTP result type
pub type StubResult = Result<(), ()>;

impl<'a> Transport<'a> for StubTransport {
    type Result = StubResult;

    fn send_raw(&mut self, envelope: &Envelope, _email: &[u8]) -> Self::Result {
        info!(
            "from=<{}> to=<{:?}>",
            match envelope.from() {
                Some(address) => address.to_string(),
                None => "".to_string(),
            },
            envelope.to()
        );
        self.response
    }
}
