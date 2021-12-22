use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub struct ConfigParseError {
    #[from]
    source: anyhow::Error,
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct ConvertBytesToBgpMessageError {
    #[from]
    source: anyhow::Error,
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct ConvertBgpMessageToBytesError {
    #[from]
    source: anyhow::Error,
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct CreateConnectionError {
    #[from]
    source: anyhow::Error,
}
