use crate::Tags;
use bitflags::{
    parser::{to_writer, WriteHex},
    Flags,
};
use core::fmt;

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Bitflags<F: Flags<Bits: WriteHex> + Send + Sync>(pub F);

impl<F: Flags<Bits: WriteHex> + Send + Sync> Tags for Bitflags<F> {
    #[inline]
    fn empty() -> Self {
        Self(Flags::empty())
    }

    #[inline]
    fn union(self, other: Self) -> Self {
        Self(self.0.union(other.0))
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        to_writer(&self.0, f)
    }
}
