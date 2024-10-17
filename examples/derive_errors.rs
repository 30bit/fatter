use std::any::{Any, TypeId};

use bitflags::bitflags;
use fatter::{
    Anyhow, Bitflags, Chain, GlobalManager, Manager, NoManager, ResultExt as _, Tags as _,
};

bitflags! {
    struct MyTag: usize {
        const HAS_MY_ERROR = 0x1;
        const HAS_MY_OTHER_ERROR = 0x2;
        const EXTRA_TAG = 0x4;
    }
}

#[derive(Debug, thiserror::Error)]
#[error("got my error")]
struct MyError;

#[derive(Debug, thiserror::Error)]
#[error("got my other error")]
struct MyOtherError;

pub struct MyManager;

impl<C: Chain> Manager<C, Bitflags<MyTag>> for MyManager {
    fn derive<E: Any + Send + Sync + ?Sized + 'static>(&self, err: &E) -> Bitflags<MyTag> {
        if err.type_id() == TypeId::of::<MyError>() {
            Bitflags(MyTag::HAS_MY_ERROR)
        } else if err.type_id() == TypeId::of::<MyOtherError>() {
            Bitflags(MyTag::HAS_MY_OTHER_ERROR)
        } else {
            Bitflags::empty()
        }
    }
}

impl<C: Chain> GlobalManager<C, Bitflags<MyTag>> for MyManager {
    fn global() -> Self {
        Self
    }
}

type FatterError = fatter::Error<Anyhow, Bitflags<MyTag>, MyManager>;

fn my_err_result() -> Result<(), MyError> {
    Err(MyError)
}

fn main() -> Result<(), FatterError> {
    // `HAS_MY_ERROR` inserted
    my_err_result().derive_err()?;
    // `HAS_MY_ERROR` is additionally inserted as well
    my_err_result().tag_err(Bitflags(MyTag::EXTRA_TAG))?;
    // `HAS_MY_ERROR` is not inserted, because global manager is ignored in favor of `NoManager`.
    my_err_result().tag_err_in(Bitflags(MyTag::EXTRA_TAG), NoManager)?;
    // `HAS_MY_ERROR` and `HAS_MY_OTHER_ERROR` are both inserted
    my_err_result().chain_err(MyOtherError)?;
    Ok(())
}
