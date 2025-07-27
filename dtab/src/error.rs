#[derive(Debug)]
pub enum DtabError {
    ParseError(String),
    InvalidPath(String),
    LookupError(String),
    Other(String),
}

impl std::fmt::Display for DtabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DtabError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            DtabError::InvalidPath(msg) => write!(f, "Invalid path: {msg}"),
            DtabError::LookupError(msg) => write!(f, "Lookup error: {msg}"),
            DtabError::Other(msg) => write!(f, "Error: {msg}"),
        }
    }
}

impl std::error::Error for DtabError {}
