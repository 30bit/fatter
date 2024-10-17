use crate::{Chain, GlobalManager, Manager, Tags};
use core::{any::Any, error::Error as StdError, fmt, iter};

#[derive(Copy, Clone)]
pub struct NoTags;

impl Tags for NoTags {
    #[inline]
    fn empty() -> Self {
        Self
    }

    #[inline]
    fn union(self, _: Self) -> Self {
        Self
    }

    #[inline]
    fn debug_fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct NoChain;

impl Chain for NoChain {
    #[inline]
    fn new<E: StdError + Send + Sync + 'static>(_: E) -> Self {
        Self
    }

    #[inline]
    fn append(self, _: Self) -> Self {
        Self
    }

    #[inline]
    fn iter(&self) -> impl Iterator<Item = &'_ (dyn StdError + 'static)> {
        iter::empty()
    }
}

#[derive(Copy, Clone)]
pub struct NoManager;

impl<C: Chain, X: Tags> Manager<C, X> for NoManager {
    #[inline]
    fn derive<E: Any + Send + Sync + ?Sized + 'static>(&self, _: &E) -> X {
        X::empty()
    }
}

impl<C: Chain, X: Tags> GlobalManager<C, X> for NoManager {
    #[inline]
    fn global() -> Self {
        Self
    }
}
