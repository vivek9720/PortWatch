use std::fmt;

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    UnexpectedEof,
    InvalidMagic,
    UnsupportedVersion,
    InvalidLength,
    InvalidChecksum,
    InvalidUtf8,
    InvalidTag,
    InvalidValue,
    UnknownSection,
    MissingSection,
    CompressionError,
    TemplateError,
    LedgerError,
    LimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ErrorKind,
    pub offset: usize,
    pub context: &'static str,
}

impl ParseError {
    pub fn new(kind: ErrorKind, offset: usize, context: &'static str) -> Self {
        Self {
            kind,
            offset,
            context,
        }
    }

    pub fn eof(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::UnexpectedEof, offset, context)
    }

    pub fn invalid_magic(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::InvalidMagic, offset, context)
    }

    pub fn unsupported_version(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::UnsupportedVersion, offset, context)
    }

    pub fn invalid_length(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::InvalidLength, offset, context)
    }

    pub fn invalid_checksum(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::InvalidChecksum, offset, context)
    }

    pub fn invalid_utf8(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::InvalidUtf8, offset, context)
    }

    pub fn invalid_tag(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::InvalidTag, offset, context)
    }

    pub fn invalid_value(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::InvalidValue, offset, context)
    }

    pub fn unknown_section(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::UnknownSection, offset, context)
    }

    pub fn missing_section(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::MissingSection, offset, context)
    }

    pub fn compression(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::CompressionError, offset, context)
    }

    pub fn template(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::TemplateError, offset, context)
    }

    pub fn ledger(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::LedgerError, offset, context)
    }

    pub fn limit(offset: usize, context: &'static str) -> Self {
        Self::new(ErrorKind::LimitExceeded, offset, context)
    }

    pub fn with_context(self, context: &'static str) -> Self {
        Self { context, ..self }
    }

    pub fn at(self, offset: usize) -> Self {
        Self { offset, ..self }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            ErrorKind::UnexpectedEof => "unexpected end of input",
            ErrorKind::InvalidMagic => "invalid magic",
            ErrorKind::UnsupportedVersion => "unsupported version",
            ErrorKind::InvalidLength => "invalid length",
            ErrorKind::InvalidChecksum => "invalid checksum",
            ErrorKind::InvalidUtf8 => "invalid utf-8",
            ErrorKind::InvalidTag => "invalid tag",
            ErrorKind::InvalidValue => "invalid value",
            ErrorKind::UnknownSection => "unknown section",
            ErrorKind::MissingSection => "missing section",
            ErrorKind::CompressionError => "compression error",
            ErrorKind::TemplateError => "template error",
            ErrorKind::LedgerError => "ledger error",
            ErrorKind::LimitExceeded => "limit exceeded",
        };
        f.write_str(text)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at offset {} while parsing {}",
            self.kind, self.offset, self.context
        )
    }
}

impl std::error::Error for ParseError {}

pub fn require(condition: bool, offset: usize, context: &'static str) -> ParseResult<()> {
    if condition {
        Ok(())
    } else {
        Err(ParseError::invalid_value(offset, context))
    }
}

pub fn require_len(
    available: usize,
    needed: usize,
    offset: usize,
    context: &'static str,
) -> ParseResult<()> {
    if available >= needed {
        Ok(())
    } else {
        Err(ParseError::eof(offset, context))
    }
}
