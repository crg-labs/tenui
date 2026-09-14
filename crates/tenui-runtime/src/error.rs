use std::{fmt, io};

#[derive(Debug)]
pub enum AppError {
    Terminal(tenui_core::TerminalError),
    Render(io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Terminal(e) => write!(f, "app terminal error: {e}"),
            Self::Render(e) => write!(f, "app render error: {e}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Terminal(e) => Some(e),
            Self::Render(e) => Some(e),
        }
    }
}

impl From<tenui_core::TerminalError> for AppError {
    fn from(e: tenui_core::TerminalError) -> Self {
        Self::Terminal(e)
    }
}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        Self::Render(e)
    }
}
