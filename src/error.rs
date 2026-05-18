use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    LibraryLoad {
        path: String,
        message: String,
    },
    LockPoisoned,
    NulByte {
        field: &'static str,
        value: String,
    },
    CoolProp {
        code: Option<i64>,
        message: String,
    },
    InvalidKey {
        kind: &'static str,
        name: String,
    },
    InvalidOutput {
        function: &'static str,
        message: String,
    },
    BufferTooSmall {
        function: &'static str,
        size: usize,
    },
    LengthOverflow {
        what: &'static str,
        len: usize,
    },
}

impl Error {
    pub(crate) fn coolprop_message(message: impl Into<String>) -> Self {
        Self::CoolProp {
            code: None,
            message: message.into(),
        }
    }

    pub(crate) fn coolprop_code(code: i64, message: impl Into<String>) -> Self {
        Self::CoolProp {
            code: Some(code),
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LibraryLoad { path, message } => {
                write!(f, "failed to load CoolProp library at {path}: {message}")
            }
            Self::LockPoisoned => write!(f, "CoolProp library lock is poisoned"),
            Self::NulByte { field, value } => {
                write!(f, "{field} contains an interior NUL byte: {value:?}")
            }
            Self::CoolProp {
                code: Some(code),
                message,
            } => {
                write!(f, "CoolProp error {code}: {message}")
            }
            Self::CoolProp {
                code: None,
                message,
            } => {
                write!(f, "CoolProp error: {message}")
            }
            Self::InvalidKey { kind, name } => {
                write!(f, "invalid CoolProp {kind} key: {name}")
            }
            Self::InvalidOutput { function, message } => {
                write!(f, "{function} failed: {message}")
            }
            Self::BufferTooSmall { function, size } => {
                write!(f, "{function} output did not fit in a {size}-byte buffer")
            }
            Self::LengthOverflow { what, len } => {
                write!(
                    f,
                    "{what} length {len} cannot be represented by CoolProp's C ABI"
                )
            }
        }
    }
}

impl std::error::Error for Error {}
