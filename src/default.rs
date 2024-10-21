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

impl<C: Chain> Chain for Option<C> {
    fn new<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Some(C::new(err))
    }

    fn push<E: StdError + Send + Sync + 'static>(self, err: E) -> Self {
        if let Some(inner) = self {
            Some(inner.push(err))
        } else {
            Self::new(err)
        }
    }

    fn append(self, err: Self) -> Self {
        match (self, err) {
            (None, None) => None,
            (None, err @ Some(_)) | (err @ Some(_), None) => err,
            (Some(lhs), Some(rhs)) => Some(lhs.append(rhs)),
        }
    }

    fn iter(&self) -> impl Iterator<Item = &'_ (dyn StdError + 'static)> {
        self.iter().flat_map(C::iter)
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(inner) = &self {
            inner.debug_fmt(f)
        } else {
            Ok(())
        }
    }

    fn display_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(inner) = &self {
            inner.display_fmt(f)
        } else {
            Ok(())
        }
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
