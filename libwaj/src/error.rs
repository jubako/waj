use thiserror::Error;

#[derive(Error, Debug)]
#[error("Path {0} not found in archive")]
pub struct PathNotFoud(pub String);

#[derive(Error, Debug)]
#[error("Jbk archive is not a valid Waj archive : {0}")]
pub struct WajFormatError(pub &'static str);

#[derive(Error, Debug)]
pub enum BaseError {
    #[error("{0}")]
    Jbk(#[from] jbk::Error),

    #[error("{0}")]
    WajFormatError(#[from] WajFormatError),
}

#[derive(Error, Debug)]
pub enum WajError {
    #[error("{0}")]
    BaseError(#[from] BaseError),

    #[error("{0}")]
    IoError(#[from] std::io::Error),

    #[error("Path {0} not found in archive")]
    PathNotFound(String),
}

impl From<jbk::Error> for WajError {
    fn from(value: jbk::Error) -> Self {
        Self::BaseError(value.into())
    }
}

impl From<WajFormatError> for WajError {
    fn from(value: WajFormatError) -> Self {
        Self::BaseError(value.into())
    }
}

#[derive(Error, Debug)]
pub enum CreatorError {
    #[error("{0}")]
    Jbk(#[from] jbk::creator::Error),

    #[error("{0}")]
    IoError(#[from] std::io::Error),
}
