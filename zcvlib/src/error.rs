use thiserror::Error;
use anyhow::Error as AnyError;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Vote(#[from] orchard::vote::VoteError),
    #[error(transparent)]
    Tonic(#[from] tonic::Status),
    #[error(transparent)]
    TonicTransport(#[from] tonic::transport::Error),
    #[error(transparent)]
    SQLite(#[from] sqlx::Error),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

pub trait IntoAnyhow<T> {
    fn anyhow(self) -> Result<T, AnyError>;
}

impl<T, E> IntoAnyhow<T> for Result<T, E>
where
    E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
{
    fn anyhow(self) -> Result<T, AnyError> {
        self.map_err(AnyError::msg)
    }
}

pub type ZCVResult<T> = Result<T, Error>;
