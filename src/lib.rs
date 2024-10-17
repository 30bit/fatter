#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(doc, feature(doc_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "anyhow")]
mod anyhow;
#[cfg(feature = "bitflags")]
mod bitflags;
mod default;

#[cfg(feature = "anyhow")]
#[cfg_attr(doc, doc(cfg(feature = "anyhow")))]
pub use self::anyhow::{Anyhow, AnyhowVec};
#[cfg(feature = "bitflags")]
#[cfg_attr(doc, doc(cfg(feature = "bitflags")))]
pub use self::bitflags::Bitflags;
pub use self::default::{NoChain, NoManager, NoTags};

use core::{
    any::Any,
    error::Error as StdError,
    fmt::{self, Debug, Display},
    marker::PhantomData,
};

pub trait GlobalManager<C: Chain, X: Tags>: Manager<C, X> + Sized + Send + Sync + 'static {
    #[must_use]
    fn global() -> Self;
}

pub trait Manager<C: Chain, X: Tags> {
    fn derive<E: Any + Send + Sync + ?Sized + 'static>(&self, err: &E) -> X;
}

impl<C: Chain, X: Tags, D: Manager<C, X>> Manager<C, X> for &D
where
    D: Manager<C, X> + ?Sized,
{
    #[inline]
    fn derive<E: Any + Send + Sync + ?Sized + 'static>(&self, err: &E) -> X {
        D::derive(self, err)
    }
}

pub trait Tags: Sized + Send + Sync + 'static {
    #[must_use]
    fn empty() -> Self;

    #[must_use]
    fn union(self, other: Self) -> Self;

    #[expect(clippy::missing_errors_doc)]
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;

    #[expect(clippy::missing_errors_doc)]
    fn display_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.debug_fmt(f)
    }
}

pub trait Chain: Sized + Send + Sync + 'static {
    #[must_use]
    fn new<E: StdError + Send + Sync + 'static>(err: E) -> Self;

    #[must_use]
    fn push<E: StdError + Send + Sync + 'static>(self, err: E) -> Self {
        self.append(Self::new(err))
    }

    #[must_use]
    fn append(self, other: Self) -> Self;

    fn iter(&self) -> impl Iterator<Item = &'_ (dyn StdError + 'static)>;

    #[expect(clippy::missing_errors_doc)]
    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            f.debug_list().entries(self.iter()).finish()
        } else {
            for err in self.iter() {
                Debug::fmt(err, f)?;
                f.write_str("\n\n")?;
            }
            Ok(())
        }
    }

    #[expect(clippy::missing_errors_doc)]
    fn display_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(last) = self.iter().last() {
            Display::fmt(last, f)
        } else {
            Ok(())
        }
    }
}

pub trait ErrorExt<C: Chain, X: Tags, D: GlobalManager<C, X>>:
    Sized + Send + Sync + 'static
{
    fn derive_in<M: Manager<C, X>>(self, manager: M) -> Error<C, X, D>;

    fn chain_in<R, M>(self, dst: R, manager: M) -> Error<C, X, D>
    where
        R: ErrorExt<C, X, D>,
        M: Manager<C, X>;

    fn tag_in<M: Manager<C, X>>(self, tags: X, manager: M) -> Error<C, X, D>;

    fn derive(self) -> Error<C, X, D> {
        self.derive_in(D::global())
    }

    fn chain<R>(self, rhs: R) -> Error<C, X, D>
    where
        R: ErrorExt<C, X, D>,
    {
        self.chain_in(rhs, D::global())
    }

    fn tag(self, tags: X) -> Error<C, X, D> {
        self.tag_in(tags, D::global())
    }
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>> ErrorExt<C, X, D> for Error<C, X, D> {
    #[inline]
    fn derive_in<M>(self, _: M) -> Error<C, X, D>
    where
        M: Manager<C, X>,
    {
        self
    }

    fn chain_in<R, M>(mut self, rhs: R, manager: M) -> Error<C, X, D>
    where
        R: ErrorExt<C, X, D>,
        M: Manager<C, X>,
    {
        let rhs = R::derive_in(rhs, manager);
        self.0.tags = self.0.tags.union(rhs.0.tags);
        self.0.chain = self.0.chain.append(rhs.0.chain);
        self
    }

    fn tag_in<M: Manager<C, X>>(mut self, tags: X, _: M) -> Error<C, X, D> {
        self.0.tags = self.0.tags.union(tags);
        self
    }
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>, E> ErrorExt<C, X, D> for E
where
    E: StdError + Send + Sync + 'static,
{
    fn derive_in<M>(self, manager: M) -> Error<C, X, D>
    where
        M: Manager<C, X>,
    {
        let tags = manager.derive(&self);
        let chain = C::new(self);
        Error::with_tags(chain, tags)
    }

    fn chain_in<R, M>(self, rhs: R, manager: M) -> Error<C, X, D>
    where
        R: ErrorExt<C, X, D>,
        M: Manager<C, X>,
    {
        let lhs = self.derive_in(&manager);
        lhs.chain_in(rhs, manager)
    }

    fn tag_in<M>(self, tags: X, manager: M) -> Error<C, X, D>
    where
        M: Manager<C, X>,
    {
        let tags = manager.derive(&self).union(tags);
        let chain = C::new(self);
        Error::with_tags(chain, tags)
    }
}

struct ErrorImpl<C: Chain, X: Tags, D: GlobalManager<C, X>> {
    chain: C,
    tags: X,
    manager: PhantomData<D>,
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>> Debug for ErrorImpl<C, X, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct FormatterFn<T>(T, fn(T, &mut fmt::Formatter) -> fmt::Result);

        impl<T: Copy> Debug for FormatterFn<T> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                self.1(self.0, f)
            }
        }

        if f.alternate() {
            f.debug_struct("Error")
                .field("chain", &FormatterFn(&self.chain, C::debug_fmt))
                .field("tags", &FormatterFn(&self.tags, X::debug_fmt))
                .finish()
        } else {
            self.chain.debug_fmt(f)?;
            self.tags.debug_fmt(f)
        }
    }
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>> Display for ErrorImpl<C, X, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.chain.display_fmt(f)?;
        self.tags.display_fmt(f)
    }
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>> StdError for ErrorImpl<C, X, D> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        // TODO: maybe link together more sources
        self.chain.iter().last()
    }
}

pub struct Error<C: Chain, X: Tags, D: GlobalManager<C, X>>(ErrorImpl<C, X, D>);

impl<C: Chain, X: Tags, D: GlobalManager<C, X>> Debug for Error<C, X, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>> Display for Error<C, X, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>> Error<C, X, D> {
    pub fn new(chain: C) -> Self {
        Self::with_tags(chain, X::empty())
    }

    pub fn with_tags(chain: C, tags: X) -> Self {
        Self(ErrorImpl {
            chain,
            tags,
            manager: PhantomData,
        })
    }

    pub fn get_chain(&self) -> &C {
        &self.0.chain
    }

    pub fn get_tags(&self) -> &X {
        &self.0.tags
    }

    pub fn get_chain_mut(&mut self) -> &mut C {
        &mut self.0.chain
    }

    pub fn get_tags_mut(&mut self) -> &mut X {
        &mut self.0.tags
    }

    pub fn into_parts(self) -> (C, X) {
        (self.0.chain, self.0.tags)
    }
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>> AsRef<dyn StdError + Send + Sync>
    for Error<C, X, D>
{
    fn as_ref(&self) -> &(dyn StdError + Send + Sync + 'static) {
        &self.0
    }
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>, E> From<E> for Error<C, X, D>
where
    E: StdError + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        ErrorExt::<C, X, D>::derive_in(err, D::global())
    }
}

#[expect(clippy::missing_errors_doc)]
pub trait ResultExt<C: Chain, X: Tags, D: GlobalManager<C, X>>: Sized {
    type Ok;
    type Err: ErrorExt<C, X, D>;

    fn derive_err_in<M>(self, manager: M) -> Result<Self::Ok, Error<C, X, D>>
    where
        M: Manager<C, X>;

    fn chain_err_in<R, M>(self, rhs: R, manager: M) -> Result<Self::Ok, Error<C, X, D>>
    where
        R: ErrorExt<C, X, D>,
        M: Manager<C, X>;

    fn tag_err_in<M>(self, tags: X, manager: M) -> Result<Self::Ok, Error<C, X, D>>
    where
        M: Manager<C, X>;

    fn derive_err(self) -> Result<Self::Ok, Error<C, X, D>> {
        self.derive_err_in(D::global())
    }

    fn chain_err<R>(self, rhs: R) -> Result<Self::Ok, Error<C, X, D>>
    where
        R: ErrorExt<C, X, D>,
    {
        self.chain_err_with(|| rhs)
    }

    fn chain_err_with<R, F>(self, rhs_f: F) -> Result<Self::Ok, Error<C, X, D>>
    where
        R: ErrorExt<C, X, D>,
        F: FnOnce() -> R,
    {
        self.chain_err_with_in(D::global(), rhs_f)
    }

    fn chain_err_with_in<R, M, F>(self, manager: M, rhs_f: F) -> Result<Self::Ok, Error<C, X, D>>
    where
        R: ErrorExt<C, X, D>,
        M: Manager<C, X>,
        F: FnOnce() -> R,
    {
        self.chain_err_in(rhs_f(), manager)
    }

    fn tag_err(self, tags: X) -> Result<Self::Ok, Error<C, X, D>> {
        self.tag_err_with(|| tags)
    }

    fn tag_err_with<F>(self, tags_f: F) -> Result<Self::Ok, Error<C, X, D>>
    where
        F: FnOnce() -> X,
    {
        self.tag_err_with_in(D::global(), tags_f)
    }

    fn tag_err_with_in<M, F>(self, manager: M, tags_f: F) -> Result<Self::Ok, Error<C, X, D>>
    where
        M: Manager<C, X>,
        F: FnOnce() -> X,
    {
        self.tag_err_in(tags_f(), manager)
    }
}

impl<C: Chain, X: Tags, D: GlobalManager<C, X>, T, E: ErrorExt<C, X, D>> ResultExt<C, X, D>
    for Result<T, E>
{
    type Ok = T;
    type Err = E;

    fn derive_err_in<M>(self, manager: M) -> Result<T, Error<C, X, D>>
    where
        M: Manager<C, X>,
    {
        self.map_err(move |err| err.derive_in(manager))
    }

    fn chain_err_in<R, M>(self, rhs: R, manager: M) -> Result<T, Error<C, X, D>>
    where
        R: ErrorExt<C, X, D>,
        M: Manager<C, X>,
    {
        self.map_err(move |lhs| lhs.chain_in(rhs, manager))
    }

    fn tag_err_in<M>(self, tags: X, manager: M) -> Result<T, Error<C, X, D>>
    where
        M: Manager<C, X>,
    {
        self.map_err(move |err| err.tag_in(tags, manager))
    }
}
