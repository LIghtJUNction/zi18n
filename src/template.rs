use core::fmt;

/// One named replacement value. Its display output is written directly to the sink.
#[derive(Clone, Copy)]
pub struct Argument<'name, 'value> {
    name: &'name str,
    value: &'value dyn fmt::Display,
}

impl<'name, 'value> Argument<'name, 'value> {
    /// Pairs a case-sensitive name and a displayable value.
    ///
    /// The formatter validates the name, which must match
    /// `[A-Za-z_][A-Za-z0-9_]*`.
    pub const fn new(name: &'name str, value: &'value dyn fmt::Display) -> Self {
        Self { name, value }
    }
}

/// A template syntax error with a zero-based UTF-8 byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TemplateError {
    /// An opening brace has no closing brace.
    UnclosedPlaceholder {
        /// Byte offset of the opening brace.
        position: usize,
    },
    /// A placeholder name does not match `[A-Za-z_][A-Za-z0-9_]*`.
    InvalidPlaceholder {
        /// Byte offset of the opening brace.
        position: usize,
    },
    /// A closing brace occurs outside a placeholder or an escape.
    UnexpectedClosingBrace {
        /// Byte offset of the closing brace.
        position: usize,
    },
}

impl fmt::Display for TemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (message, position) = match *self {
            Self::UnclosedPlaceholder { position } => ("unclosed placeholder", position),
            Self::InvalidPlaceholder { position } => ("invalid placeholder name", position),
            Self::UnexpectedClosingBrace { position } => ("unexpected closing brace", position),
        };
        write!(formatter, "{message} at byte {position}")
    }
}

impl core::error::Error for TemplateError {}

/// A syntax, argument, or output error while formatting a template.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatError<'input> {
    /// The template has invalid syntax.
    InvalidTemplate(TemplateError),
    /// An argument has an invalid name.
    InvalidArgumentName(&'input str),
    /// The caller passed the same argument name more than once.
    DuplicateArgument(&'input str),
    /// No argument was supplied for this placeholder.
    MissingArgument(&'input str),
    /// The sink or a replacement's [`fmt::Display`] returned an error.
    Write,
}

impl fmt::Display for FormatError<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTemplate(error) => error.fmt(formatter),
            Self::InvalidArgumentName(name) => write!(formatter, "invalid argument name {name:?}"),
            Self::DuplicateArgument(name) => write!(formatter, "duplicate argument {name:?}"),
            Self::MissingArgument(name) => write!(formatter, "missing argument {name:?}"),
            Self::Write => formatter.write_str("template output failed"),
        }
    }
}

impl core::error::Error for FormatError<'_> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::InvalidTemplate(error) => Some(error),
            _ => None,
        }
    }
}

/// Writes a named template to a [`fmt::Write`] sink without allocating.
///
/// `{name}` inserts the corresponding value. `{{` and `}}` emit literal braces.
/// Names are case-sensitive ASCII identifiers. Valid, uniquely named extra
/// arguments are allowed and ignored. Repeated placeholders reuse the argument.
/// Replacement output is opaque, including any braces it contains.
///
/// Syntax and argument errors produce zero sink writes and no replacement
/// formatting. A sink or replacement [`fmt::Display`] failure may leave a
/// partial prefix and returns [`FormatError::Write`].
pub fn format_to<'input>(
    sink: &mut (impl fmt::Write + ?Sized),
    template: &'input str,
    arguments: &[Argument<'input, '_>],
) -> Result<(), FormatError<'input>> {
    for (index, argument) in arguments.iter().enumerate() {
        if !valid_name(argument.name) {
            return Err(FormatError::InvalidArgumentName(argument.name));
        }
        if arguments[..index]
            .iter()
            .any(|prior| prior.name == argument.name)
        {
            return Err(FormatError::DuplicateArgument(argument.name));
        }
    }

    for token in Tokens::new(template) {
        if let Token::Placeholder(name) = token.map_err(FormatError::InvalidTemplate)? {
            find_argument(arguments, name)?;
        }
    }

    for token in Tokens::new(template) {
        match token.map_err(FormatError::InvalidTemplate)? {
            Token::Literal(text) => sink.write_str(text),
            Token::Placeholder(name) => write!(sink, "{}", find_argument(arguments, name)?),
        }
        .map_err(|_| FormatError::Write)?;
    }
    Ok(())
}

/// Formats a template into a new string using the same contract as [`format_to`].
#[cfg(feature = "alloc")]
pub fn format<'input>(
    template: &'input str,
    arguments: &[Argument<'input, '_>],
) -> Result<alloc::string::String, FormatError<'input>> {
    let mut output = alloc::string::String::new();
    format_to(&mut output, template, arguments)?;
    Ok(output)
}

pub(crate) fn validate_template(template: &str) -> Result<(), TemplateError> {
    for token in Tokens::new(template) {
        token?;
    }
    Ok(())
}

fn valid_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn find_argument<'input, 'value>(
    arguments: &[Argument<'input, 'value>],
    name: &'input str,
) -> Result<&'value dyn fmt::Display, FormatError<'input>> {
    arguments
        .iter()
        .find(|argument| argument.name == name)
        .map(|argument| argument.value)
        .ok_or(FormatError::MissingArgument(name))
}

enum Token<'text> {
    Literal(&'text str),
    Placeholder(&'text str),
}

struct Tokens<'text> {
    text: &'text str,
    position: usize,
}

impl<'text> Tokens<'text> {
    fn new(text: &'text str) -> Self {
        Self { text, position: 0 }
    }

    fn next_token(&mut self) -> Option<Result<Token<'text>, TemplateError>> {
        let rest = &self.text[self.position..];
        let bytes = rest.as_bytes();
        let token = match (*bytes.first()?, bytes.get(1).copied()) {
            (b'{' | b'}', Some(second)) if bytes[0] == second => {
                self.position += 2;
                Token::Literal(&rest[..1])
            }
            (b'{', _) => {
                let Some(close) = rest.find('}') else {
                    return Some(Err(TemplateError::UnclosedPlaceholder {
                        position: self.position,
                    }));
                };
                let name = &rest[1..close];
                if !valid_name(name) {
                    return Some(Err(TemplateError::InvalidPlaceholder {
                        position: self.position,
                    }));
                }
                self.position += close + 1;
                Token::Placeholder(name)
            }
            (b'}', _) => {
                return Some(Err(TemplateError::UnexpectedClosingBrace {
                    position: self.position,
                }));
            }
            _ => {
                // ASCII brace bytes are always at UTF-8 character boundaries.
                let end = bytes
                    .iter()
                    .position(|&byte| matches!(byte, b'{' | b'}'))
                    .unwrap_or(rest.len());
                self.position += end;
                Token::Literal(&rest[..end])
            }
        };
        Some(Ok(token))
    }
}

impl<'text> Iterator for Tokens<'text> {
    type Item = Result<Token<'text>, TemplateError>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token()?;
        if token.is_err() {
            self.position = self.text.len();
        }
        Some(token)
    }
}
