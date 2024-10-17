use bitflags::bitflags;
use fatter::{Anyhow, Bitflags, NoManager, ResultExt as _};

bitflags! {
    struct MyTag: usize {
        const A = 0x1;
        const B = 0x2;
        const C = 0x4;
    }
}

/// An error that works like anyhow,
/// but can also be tagged (by storing [`MyTag`] field)
type FatterError = fatter::Error<Anyhow, Bitflags<MyTag>, NoManager>;

#[derive(Debug, thiserror::Error)]
enum OtherError {
    #[error("got my `X` error")]
    X,
    #[error("got my `Y` error")]
    Y,
}

fn x_err_result() -> Result<(), OtherError> {
    Err(OtherError::X)
}

fn y_err_result() -> Result<(), OtherError> {
    Err(OtherError::Y)
}

fn main() -> Result<(), FatterError> {
    // `FatterError` with empty bitflags
    x_err_result()?;
    // `FatterError` with tags `A` and `B`
    x_err_result().tag_err(Bitflags(MyTag::A | MyTag::B))?;
    // More functions can be called using `.` on previous results
    x_err_result()
        .tag_err(Bitflags(MyTag::A))
        .and_then(|()| y_err_result().tag_err(Bitflags(MyTag::C)))
        .tag_err(Bitflags(MyTag::C))?;
    Ok(())
}
