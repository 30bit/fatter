use anyhow::anyhow;
use fatter::{Anyhow, ErrorExt as _, NoManager, NoTags, ResultExt as _};

/// An error that works like anyhow
type FatterError = fatter::Error<Anyhow, NoTags, NoManager>;

#[derive(Debug, thiserror::Error)]
enum OtherError {
    #[error("got my `X` error")]
    X,
    #[error("got my `Y` error")]
    Y,
}

fn anyhow_err(index: usize) -> Anyhow {
    anyhow!("got anyhow error #{index}").into()
}

fn main() -> Result<(), FatterError> {
    // Chaining errors links an error as the source of the argument
    Err(anyhow_err(0))
        .chain_err(anyhow_err(1))
        .chain_err(OtherError::X.derive().chain(OtherError::Y))
        .chain_err(anyhow_err(3))
}
