use crate::Chain;
use core::{
    error::Error as StdError,
    fmt::{self, Debug, Display},
    ops::Deref,
};

#[repr(transparent)]
pub struct Anyhow(pub anyhow::Error);

impl Chain for Anyhow {
    fn new<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self(anyhow::Error::from(err))
    }

    fn push<E: StdError + Send + Sync + 'static>(self, err: E) -> Self {
        // TODO: should `err`` be converted to `anyhow::Error`?
        Self(self.0.context(err))
    }

    fn append(self, other: Self) -> Self {
        Self(self.0.context(other.0))
    }

    fn iter(&self) -> impl Iterator<Item = &'_ (dyn StdError + 'static)> {
        self.0.chain()
    }

    fn debug_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }

    fn display_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Debug for Anyhow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for Anyhow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl StdError for Anyhow {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        AsRef::<dyn StdError + Send + Sync + 'static>::as_ref(&self.0).source()
    }
}

impl From<anyhow::Error> for Anyhow {
    fn from(err: anyhow::Error) -> Self {
        Self(err)
    }
}

#[expect(clippy::module_name_repetitions)]
#[repr(transparent)]
pub struct AnyhowVec(raw::OwnedCell);

impl From<anyhow::Error> for AnyhowVec {
    fn from(err: anyhow::Error) -> Self {
        Self(raw::OwnedCell::new(err))
    }
}

impl From<Anyhow> for AnyhowVec {
    fn from(err: Anyhow) -> Self {
        Self::from(err.0)
    }
}

impl Deref for AnyhowVec {
    type Target = [anyhow::Error];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl Chain for AnyhowVec {
    fn new<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self(raw::OwnedCell::new(anyhow::Error::from(err)))
    }

    fn push<E: StdError + Send + Sync + 'static>(mut self, err: E) -> Self {
        self.0.push(anyhow::Error::from(err));
        self
    }

    fn append(mut self, other: Self) -> Self {
        self.0.append(other.0);
        self
    }

    fn iter(&self) -> impl Iterator<Item = &'_ (dyn StdError + 'static)> {
        self.0.as_slice().iter().map(AsRef::as_ref)
    }
}

mod raw {
    use core::{
        mem::{self, transmute, ManuallyDrop},
        ptr, slice,
    };

    /// Every [`Vec`] stores length, capacity, and at least one error
    const MIN_LEN: usize = 3;

    #[repr(transparent)]
    pub struct OwnedCell(*mut usize);

    unsafe impl Send for OwnedCell {}

    unsafe impl Sync for OwnedCell {}

    impl OwnedCell {
        pub fn new(first: anyhow::Error) -> Self {
            Self(View::new_dangling(first).into_ptr_mut())
        }

        pub fn as_slice(&self) -> &[anyhow::Error] {
            View::error_slice_of(self)
        }

        pub fn push(&mut self, err: anyhow::Error) {
            let mut view = View::of(self);
            view.push(err);
            self.0 = view.into_ptr_mut();
        }

        pub fn append(&mut self, other: Self) {
            let mut view = View::of(self);
            view.append(other);
            self.0 = view.into_ptr_mut();
        }
    }

    impl Drop for OwnedCell {
        fn drop(&mut self) {
            View::of(self).total_drop();
        }
    }

    #[repr(transparent)]
    pub struct View(ManuallyDrop<Vec<usize>>);

    /// Erasing is safe, but the [`Drop`] won't be ever called automatically.
    /// Casting back is also not safe, because [`anyhow::Error`] has bitwise requirements
    fn erase_anyhow_error(err: anyhow::Error) -> usize {
        // GUARANTEE: anyhow wraps around a pointer
        unsafe { transmute(err) }
    }

    impl View {
        fn of(cell: &OwnedCell) -> Self {
            let ptr = cell.0;
            // GUARANTEE: `OwnedCell` is properly allocated
            unsafe {
                let length = ptr.read();
                let capacity = ptr.add(1).read();
                Self(ManuallyDrop::new(Vec::from_raw_parts(
                    ptr, length, capacity,
                )))
            }
        }

        fn as_error_ptr(&self) -> *const anyhow::Error {
            // GUARANTEE: capacity and length take first two items
            unsafe { self.0.as_ptr().add(2).cast() }
        }

        fn as_error_mut_ptr(&mut self) -> *mut anyhow::Error {
            // GUARANTEE: capacity and length take first two items
            unsafe { self.0.as_mut_ptr().add(2).cast() }
        }

        fn as_error_mut_slice_ptr(&mut self) -> *mut [anyhow::Error] {
            let len = self.error_count();
            ptr::slice_from_raw_parts_mut(self.as_error_mut_ptr(), len)
        }

        fn error_slice_of(owned: &OwnedCell) -> &[anyhow::Error] {
            let self_ = Self::of(owned);
            // GUARANTEE: errors are always initialized by `OwnedCell`
            unsafe { slice::from_raw_parts(self_.as_error_ptr(), self_.error_count()) }
        }

        fn error_count(&self) -> usize {
            // GUARANTEE: capacity and length take first two items
            unsafe { self.0.len().unchecked_sub(2) }
        }

        fn new_dangling(first: anyhow::Error) -> Self {
            let mut vec = ManuallyDrop::new(Vec::<usize>::with_capacity(MIN_LEN));
            let capacity = vec.capacity();
            vec.push(MIN_LEN); // initial length
            vec.push(capacity); // pushes are within all capacity
            vec.push(erase_anyhow_error(first));
            Self(vec)
        }

        fn push(&mut self, err: anyhow::Error) {
            self.0.push(erase_anyhow_error(err));
        }

        fn append(&mut self, owned_other: OwnedCell) {
            let mut other = View::of(&owned_other);
            let other_error_ptr = other.as_error_mut_ptr();
            let other_error_count = other.error_count();
            let self_len = self.0.len();
            let self_error_ptr = self.as_error_mut_ptr();
            let self_error_count = self.error_count();

            self.0.reserve(other_error_count);
            unsafe {
                // GUARANTEE: memory is sufficiently reserved, errors are copied without drop
                ptr::copy_nonoverlapping(
                    other_error_ptr.cast_const(),
                    self_error_ptr.add(self_error_count),
                    other_error_count,
                );
                self.0.set_len(self_len + other_error_count);
                other.shallow_drop();
                // GUARANTEE: other is dropped shallowly
                mem::forget(owned_other);
            };
        }

        // GUARANTEE: errors are always initialized and expect drop
        fn total_drop(&mut self) {
            unsafe {
                // Drop anyhow errors
                ptr::drop_in_place(self.as_error_mut_slice_ptr());
                // Vec is deallocated
                ManuallyDrop::drop(&mut self.0);
            }
        }

        unsafe fn shallow_drop(&mut self) {
            // Vec is deallocated, anyhow errors aren't dropped
            ManuallyDrop::drop(&mut self.0);
        }

        fn into_ptr_mut(mut self) -> *mut usize {
            self.0.as_mut_ptr()
        }
    }
}
