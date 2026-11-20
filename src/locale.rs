use core::fmt;

/// A structurally checked RFC 5646 language tag that borrows its original spelling.
///
/// Validation checks syntax and rejects duplicate variants and extension singletons.
/// IANA registration, subtag prefixes, and extension semantics are outside its scope.
#[derive(Clone, Copy, Debug)]
pub struct Locale<'tag>(&'tag str);

impl<'tag> Locale<'tag> {
    /// Checks a language tag without allocating or changing its spelling.
    pub fn parse(tag: &'tag str) -> Result<Self, LocaleError> {
        if GRANDFATHERED
            .iter()
            .any(|old| tag.eq_ignore_ascii_case(old))
        {
            return Ok(Self(tag));
        }

        let subtags = tag.split('-');
        if !subtags.clone().all(|part| {
            (1..=8).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_alphanumeric())
        }) {
            return Err(LocaleError::InvalidSyntax);
        }

        let mut parts = subtags.peekable();
        let language = parts.next().ok_or(LocaleError::InvalidSyntax)?;
        if language.eq_ignore_ascii_case("x") {
            return if parts.next().is_some() {
                Ok(Self(tag))
            } else {
                Err(LocaleError::InvalidSyntax)
            };
        }
        if language.len() < 2 || !is_alpha(language) {
            return Err(LocaleError::InvalidSyntax);
        }
        if language.len() <= 3 {
            for _ in 0..3 {
                if parts
                    .next_if(|part| part.len() == 3 && is_alpha(part))
                    .is_none()
                {
                    break;
                }
            }
        }
        parts.next_if(|part| part.len() == 4 && is_alpha(part));
        parts.next_if(|part| {
            (part.len() == 2 && is_alpha(part))
                || (part.len() == 3 && part.bytes().all(|byte| byte.is_ascii_digit()))
        });

        let variants = parts.clone();
        let mut variant_count = 0;
        while let Some(variant) = parts.next_if(|part| {
            part.len() >= 5 || (part.len() == 4 && part.as_bytes()[0].is_ascii_digit())
        }) {
            if variants
                .clone()
                .take(variant_count)
                .any(|prior| variant.eq_ignore_ascii_case(prior))
            {
                return Err(LocaleError::DuplicateVariant);
            }
            variant_count += 1;
        }

        let mut seen_extensions = 0_u64;
        while let Some(singleton) = parts.next_if(|part| part.len() == 1) {
            let byte = singleton.as_bytes()[0].to_ascii_lowercase();
            if byte == b'x' {
                return if parts.next().is_some() {
                    Ok(Self(tag))
                } else {
                    Err(LocaleError::InvalidSyntax)
                };
            }
            let bit = 1
                << if byte.is_ascii_digit() {
                    byte - b'0'
                } else {
                    byte - b'a' + 10
                };
            if seen_extensions & bit != 0 {
                return Err(LocaleError::DuplicateExtension);
            }
            seen_extensions |= bit;
            if parts.next_if(|part| part.len() >= 2).is_none() {
                return Err(LocaleError::InvalidSyntax);
            }
            while parts.next_if(|part| part.len() >= 2).is_some() {}
        }

        if parts.next().is_some() {
            return Err(LocaleError::InvalidSyntax);
        }
        Ok(Self(tag))
    }

    /// Returns the original spelling, including its original ASCII casing.
    pub const fn as_str(self) -> &'tag str {
        self.0
    }

    /// Returns the next shorter lookup candidate, if one exists.
    ///
    /// Drops the last subtag and any single-character subtags now at the end.
    /// A private-use-only tag has no parent once its leading `x` is reached.
    pub fn parent(self) -> Option<Self> {
        let (mut prefix, _) = self.0.rsplit_once('-')?;
        while prefix.rsplit('-').next()?.len() == 1 {
            (prefix, _) = prefix.rsplit_once('-')?;
        }
        Some(Self(prefix))
    }
}

impl PartialEq for Locale<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(other.0)
    }
}

impl Eq for Locale<'_> {}

impl fmt::Display for Locale<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// The reason a language tag failed validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LocaleError {
    /// The tag is outside the RFC 5646 grammar.
    InvalidSyntax,
    /// A variant subtag occurs more than once.
    DuplicateVariant,
    /// An extension's single-character identifier occurs more than once.
    DuplicateExtension,
}

impl fmt::Display for LocaleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSyntax => "invalid language-tag syntax",
            Self::DuplicateVariant => "duplicate language-tag variant",
            Self::DuplicateExtension => "duplicate language-tag extension",
        })
    }
}

impl core::error::Error for LocaleError {}

fn is_alpha(subtag: &str) -> bool {
    subtag.bytes().all(|byte| byte.is_ascii_alphabetic())
}

// The fixed grandfathered productions in RFC 5646 §2.1.
const GRANDFATHERED: &[&str] = &[
    "en-GB-oed",
    "i-ami",
    "i-bnn",
    "i-default",
    "i-enochian",
    "i-hak",
    "i-klingon",
    "i-lux",
    "i-mingo",
    "i-navajo",
    "i-pwn",
    "i-tao",
    "i-tay",
    "i-tsu",
    "sgn-BE-FR",
    "sgn-BE-NL",
    "sgn-CH-DE",
    "art-lojban",
    "cel-gaulish",
    "no-bok",
    "no-nyn",
    "zh-guoyu",
    "zh-hakka",
    "zh-min",
    "zh-min-nan",
    "zh-xiang",
];
