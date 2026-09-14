use std::{fmt, io};

#[derive(Debug)]
pub enum TerminalError {
    Init(io::Error),
    Restore(io::Error),
    Draw(io::Error),
    Io(io::Error),
}

impl fmt::Display for TerminalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init(e) => write!(f, "terminal initialization failed: {e}"),
            Self::Restore(e) => write!(f, "terminal restore failed: {e}"),
            Self::Draw(e) => write!(f, "draw/present failed: {e}"),
            Self::Io(e) => write!(f, "terminal I/O error: {e}"),
        }
    }
}

impl std::error::Error for TerminalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Init(e) | Self::Restore(e) | Self::Draw(e) | Self::Io(e) => Some(e),
        }
    }
}

impl From<io::Error> for TerminalError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
