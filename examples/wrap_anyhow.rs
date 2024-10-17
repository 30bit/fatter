use anyhow::anyhow;
use fatter::{Anyhow, NoManager, NoTags};

/// An error that works like anyhow
type FatterError = fatter::Error<Anyhow, NoTags, NoManager>;

fn anyhow_err_result() -> anyhow::Result<()> {
    Err(anyhow!("got anyhow error"))
}

#[derive(Debug, thiserror::Error)]
#[error("got other error")]
struct OtherError;

fn other_err_result() -> Result<(), OtherError> {
    Err(OtherError)
}

fn main() -> Result<(), FatterError> {
    // errors that are supported to be `?`-returned by `anyhow` can be `?`-returned
    other_err_result()?;
    // `anyhow::Error` itself doesn't implement `std::error::Error`, so it must be wrapped
    anyhow_err_result().map_err(Anyhow)?;
    Ok(())
}
