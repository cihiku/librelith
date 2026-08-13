use alloc::sync::Arc;

#[derive(Debug, Clone)]
pub struct Name {
    text: Arc<str>,
    colon: usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NameError {
    MissingColon,
    BadChar { ch: char, at: usize },
    EmptySegment { at: usize },
    EmptyNamespace,
}

impl NameError {
    pub fn span(&self) -> Option<(usize, usize)> {
        match self {
            NameError::BadChar { ch, at } => Some((*at, ch.len_utf8())),
            NameError::EmptySegment { at } => Some((*at, 1)),
            NameError::MissingColon | NameError::EmptyNamespace => None,
        }
    }
}

impl core::fmt::Display for NameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NameError::MissingColon => f.write_str("missing ':' separator"),
            NameError::BadChar { ch, at } => {
                write!(f, "invalid character {ch:?} at byte {at}")
            }
            NameError::EmptySegment { at } => {
                write!(f, "empty segment at byte {at}")
            }
            NameError::EmptyNamespace => f.write_str("empty namespace"),
        }
    }
}

impl core::error::Error for NameError {}

impl Name {
    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn namespace(&self) -> &str {
        &self.text[..self.colon]
    }

    pub fn path(&self) -> &str {
        &self.text[self.colon + 1..]
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.path().split('/')
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for Name {}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.text.cmp(&other.text)
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl core::hash::Hash for Name {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl core::borrow::Borrow<str> for Name {
    fn borrow(&self) -> &str {
        &self.text
    }
}

impl core::str::FromStr for Name {
    type Err = NameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let colon = s.find(':').ok_or(NameError::MissingColon)?;
        if colon == 0 {
            return Err(NameError::EmptyNamespace);
        }
        let ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.-".contains(c);
        let check = |seg: &str, start: usize| {
            if seg.is_empty() {
                return Err(NameError::EmptySegment { at: start });
            }
            match seg.char_indices().find(|&(_, c)| !ok(c)) {
                Some((off, ch)) => Err(NameError::BadChar {
                    ch,
                    at: start + off,
                }),
                None => Ok(()),
            }
        };
        check(&s[..colon], 0)?;
        let mut start = colon + 1;
        for seg in s[colon + 1..].split('/') {
            check(seg, start)?;
            start += seg.len() + 1;
        }
        Ok(Name {
            text: Arc::from(s),
            colon,
        })
    }
}

impl core::fmt::Display for Name {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for s in ["librelith:stone", "librelith:block/wood/oak_planks"] {
            assert_eq!(s.parse::<Name>().unwrap().as_str(), s)
        }
    }

    #[test]
    fn rejects() {
        assert_eq!("stone".parse::<Name>(), Err(NameError::MissingColon));
        assert_eq!(
            "Lib:stone".parse::<Name>(),
            Err(NameError::BadChar { ch: 'L', at: 0 })
        );
        assert_eq!(
            "a:B/c".parse::<Name>(),
            Err(NameError::BadChar { ch: 'B', at: 2 })
        );
        assert_eq!(
            "a:b//c".parse::<Name>(),
            Err(NameError::EmptySegment { at: 4 })
        );
        assert_eq!("a:".parse::<Name>(), Err(NameError::EmptySegment { at: 2 }));
        assert_eq!(":".parse::<Name>(), Err(NameError::EmptyNamespace));
        assert_eq!(":b".parse::<Name>(), Err(NameError::EmptyNamespace));
        assert_eq!(
            "a:b:c".parse::<Name>(),
            Err(NameError::BadChar { ch: ':', at: 3 })
        );
    }
}
