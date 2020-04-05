use crate::{
    address::{Address, AddressError},
    message::utf8_b,
};
use std::{
    convert::TryFrom,
    fmt::{Display, Formatter, Result as FmtResult, Write},
    slice::Iter,
    str::FromStr,
};

/// Email address with optional addressee name
///
/// This type contains email address and the sender/recipient name (_Some Name \<user@domain.tld\>_ or _withoutname@domain.tld_).
///
/// **NOTE**: Enable feature "serde" to be able serialize/deserialize it using [serde](https://serde.rs/).
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Mailbox {
    /// User name part
    pub name: Option<String>,

    /// Email address part
    pub email: Address,
}

impl Mailbox {
    /// Create new mailbox using email address and addressee name
    #[inline]
    pub fn new(name: Option<String>, email: Address) -> Self {
        Mailbox { name, email }
    }

    /// Encode addressee name using function
    pub(crate) fn recode_name<F>(&self, f: F) -> Self
    where
        F: FnOnce(&str) -> String,
    {
        Mailbox::new(self.name.clone().map(|s| f(&s)), self.email.clone())
    }
}

impl Display for Mailbox {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        if let Some(ref name) = self.name {
            let name = name.trim();
            if !name.is_empty() {
                f.write_str(&name)?;
                f.write_str(" <")?;
                self.email.fmt(f)?;
                return f.write_char('>');
            }
        }
        self.email.fmt(f)
    }
}

impl<S: Into<String>, T: Into<String>> TryFrom<(S, T)> for Mailbox {
    type Error = AddressError;

    fn try_from(header: (S, T)) -> Result<Self, Self::Error> {
        let (name, address) = header;
        Ok(Mailbox::new(Some(name.into()), address.into().parse()?))
    }
}

/*
impl<S: AsRef<&str>, T: AsRef<&str>> TryFrom<(S, T)> for Mailbox {
    type Error = AddressError;

    fn try_from(header: (S, T)) -> Result<Self, Self::Error> {
        let (name, address) = header;
        Ok(Mailbox::new(Some(name.as_ref()), address.as_ref().parse()?))
    }
}*/

impl FromStr for Mailbox {
    type Err = AddressError;

    fn from_str(src: &str) -> Result<Mailbox, Self::Err> {
        match (src.find('<'), src.find('>')) {
            (Some(addr_open), Some(addr_close)) if addr_open < addr_close => {
                let name = src.split_at(addr_open).0;
                let addr_open = addr_open + 1;
                let addr = src.split_at(addr_open).1.split_at(addr_close - addr_open).0;
                let addr = addr.parse()?;
                let name = name.trim();
                let name = if name.is_empty() {
                    None
                } else {
                    Some(name.into())
                };
                Ok(Mailbox::new(name, addr))
            }
            (Some(_), _) => Err(AddressError::Unbalanced),
            _ => {
                let addr = src.parse()?;
                Ok(Mailbox::new(None, addr))
            }
        }
    }
}

/// List or email mailboxes
///
/// This type contains a sequence of mailboxes (_Some Name \<user@domain.tld\>, Another Name \<other@domain.tld\>, withoutname@domain.tld, ..._).
///
/// **NOTE**: Enable feature "serde" to be able serialize/deserialize it using [serde](https://serde.rs/).
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Mailboxes(Vec<Mailbox>);

impl Mailboxes {
    /// Create mailboxes list
    #[inline]
    pub fn new() -> Self {
        Mailboxes(Vec::new())
    }

    /// Add mailbox to a list
    #[inline]
    pub fn with(mut self, mbox: Mailbox) -> Self {
        self.0.push(mbox);
        self
    }

    /// Add mailbox to a list
    #[inline]
    pub fn push(&mut self, mbox: Mailbox) {
        self.0.push(mbox);
    }

    /// Extract first mailbox
    #[inline]
    pub fn into_single(self) -> Option<Mailbox> {
        self.into()
    }

    /// Iterate over mailboxes
    #[inline]
    pub fn iter(&self) -> Iter<Mailbox> {
        self.0.iter()
    }
}

impl Default for Mailboxes {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Mailbox> for Mailboxes {
    fn from(single: Mailbox) -> Self {
        Mailboxes(vec![single])
    }
}

impl Into<Option<Mailbox>> for Mailboxes {
    fn into(self) -> Option<Mailbox> {
        self.into_iter().next()
    }
}

impl From<Vec<Mailbox>> for Mailboxes {
    fn from(list: Vec<Mailbox>) -> Self {
        Mailboxes(list)
    }
}

impl Into<Vec<Mailbox>> for Mailboxes {
    fn into(self) -> Vec<Mailbox> {
        self.0
    }
}

impl IntoIterator for Mailboxes {
    type Item = Mailbox;
    type IntoIter = ::std::vec::IntoIter<Mailbox>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Extend<Mailbox> for Mailboxes {
    fn extend<T: IntoIterator<Item = Mailbox>>(&mut self, iter: T) {
        for elem in iter {
            self.0.push(elem);
        }
    }
}

impl Display for Mailboxes {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let mut iter = self.iter();

        if let Some(mbox) = iter.next() {
            mbox.fmt(f)?;

            for mbox in iter {
                f.write_str(", ")?;
                mbox.fmt(f)?;
            }
        }

        Ok(())
    }
}

impl FromStr for Mailboxes {
    type Err = AddressError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        src.split(',')
            .map(|m| {
                m.trim().parse().and_then(|Mailbox { name, email }| {
                    if let Some(name) = name {
                        if let Some(name) = utf8_b::decode(&name) {
                            Ok(Mailbox::new(Some(name), email))
                        } else {
                            Err(AddressError::InvalidUtf8b)
                        }
                    } else {
                        Ok(Mailbox::new(None, email))
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Mailboxes)
    }
}

#[cfg(test)]
mod test {
    use super::Mailbox;
    use std::convert::TryInto;

    #[test]
    fn mailbox_format_address_only() {
        assert_eq!(
            format!(
                "{}",
                Mailbox::new(None, "kayo@example.com".parse().unwrap())
            ),
            "kayo@example.com"
        );
    }

    #[test]
    fn mailbox_format_address_with_name() {
        assert_eq!(
            format!(
                "{}",
                Mailbox::new(Some("K.".into()), "kayo@example.com".parse().unwrap())
            ),
            "K. <kayo@example.com>"
        );
    }

    #[test]
    fn format_address_with_empty_name() {
        assert_eq!(
            format!(
                "{}",
                Mailbox::new(Some("".into()), "kayo@example.com".parse().unwrap())
            ),
            "kayo@example.com"
        );
    }

    #[test]
    fn format_address_with_name_trim() {
        assert_eq!(
            format!(
                "{}",
                Mailbox::new(Some(" K. ".into()), "kayo@example.com".parse().unwrap())
            ),
            "K. <kayo@example.com>"
        );
    }

    #[test]
    fn parse_address_only() {
        assert_eq!(
            "kayo@example.com".parse(),
            Ok(Mailbox::new(None, "kayo@example.com".parse().unwrap()))
        );
    }

    #[test]
    fn parse_address_with_name() {
        assert_eq!(
            "K. <kayo@example.com>".parse(),
            Ok(Mailbox::new(
                Some("K.".into()),
                "kayo@example.com".parse().unwrap()
            ))
        );
    }

    #[test]
    fn parse_address_with_empty_name() {
        assert_eq!(
            "<kayo@example.com>".parse(),
            Ok(Mailbox::new(None, "kayo@example.com".parse().unwrap()))
        );
    }

    #[test]
    fn parse_address_with_empty_name_trim() {
        assert_eq!(
            " <kayo@example.com>".parse(),
            Ok(Mailbox::new(None, "kayo@example.com".parse().unwrap()))
        );
    }

    #[test]
    fn parse_address_from_tuple() {
        assert_eq!(
            ("K.".to_string(), "kayo@example.com".to_string()).try_into(),
            Ok(Mailbox::new(
                Some("K.".into()),
                "kayo@example.com".parse().unwrap()
            ))
        );
    }
}
