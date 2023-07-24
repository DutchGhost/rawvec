#![feature(const_try)]
#![feature(const_trait_impl)]
#![feature(const_slice_ptr_len)]
#![feature(slice_ptr_get)]
#![feature(allocator_api)]
#![feature(dropck_eyepatch)]
#![feature(slice_ptr_len)]
//#![no_std]

extern crate alloc;

use {
    alloc::alloc::{AllocError, Allocator, Global, Layout, LayoutError},
    core::{
        cmp,
        mem::{self, MaybeUninit},
        ptr::NonNull,
    },
};

pub struct RawVec<T, A: Allocator = Global>(NonNull<[MaybeUninit<T>]>, A);

#[derive(Debug)]
pub enum Failure {
    Layout,
    Allocation,
}

impl From<LayoutError> for Failure {
    fn from(_: LayoutError) -> Self {
        Self::Layout
    }
}

impl From<AllocError> for Failure {
    fn from(_: AllocError) -> Self {
        Self::Allocation
    }
}

impl<T, A: Allocator> RawVec<T, A> {
    fn layout(capacity: usize) -> Result<Layout, LayoutError> {
        Layout::array::<MaybeUninit<T>>(capacity)
    }
}

impl<T> RawVec<T> {
    pub const fn new() -> Self {
        RawVec::new_in(Global)
    }

    pub fn with_capacity(cap: usize) -> Result<Self, Failure> {
        RawVec::with_capacity_in(cap, Global)
    }
}

#[derive(Eq, PartialEq)]
pub enum AllocInit {
    Zeroed,
    Uninitialized,
}

impl<T, A: Allocator> RawVec<T, A> {
    pub(crate) const MIN_NON_ZERO_CAP: usize = if mem::size_of::<T>() == 1 {
        8
    } else if mem::size_of::<T>() <= 1024 {
        4
    } else {
        1
    };

    #[inline(always)]
    pub const fn new_in(alloc: A) -> Self {
        let slice = NonNull::<[MaybeUninit<T>; 0]>::dangling();
        Self(slice, alloc)
    }

    pub const fn capacity(&self) -> usize {
        self.0.len()
    }

    pub fn with_capacity_in(cap: usize, alloc: A) -> Result<Self, Failure> {
        Self::allocate_in(cap, AllocInit::Uninitialized, alloc)
    }

    pub fn allocate_in(capacity: usize, init: AllocInit, alloc: A) -> Result<Self, Failure> {
        if mem::size_of::<T>() == 0 {
            Ok(Self::new_in(alloc))
        } else {
            let layout = Self::layout(capacity)?;

            let slice: NonNull<[u8]> = match init {
                AllocInit::Zeroed => alloc.allocate_zeroed(layout),
                AllocInit::Uninitialized => alloc.allocate(layout),
            }?;

            let nonnull = slice.as_non_null_ptr().cast::<MaybeUninit<T>>();

            let slice = NonNull::slice_from_raw_parts(nonnull, capacity);
            Ok(Self(slice, alloc))
        }
    }

    fn current_memory(&self) -> Option<(NonNull<u8>, Layout)> {
        Self::layout(self.capacity())
            .map(|layout| (self.0.as_non_null_ptr().cast(), layout))
            .ok()
    }

    unsafe fn grow(&mut self, len: usize, additional: usize) -> Result<(), Failure> {
        let required_cap = len.checked_add(additional).ok_or(Failure::Allocation)?;
        let capacity = self.capacity();

        let cap = cmp::max(capacity * 2, required_cap);
        let cap = cmp::max(Self::MIN_NON_ZERO_CAP, cap);

        if let Some((old_ptr, old_layout)) = self.current_memory() {
            let new_layout = Self::layout(cap)?;

            let slice: NonNull<[u8]> = self.1.grow(old_ptr, old_layout, new_layout)?;
            let nonnull = slice.as_non_null_ptr().cast::<MaybeUninit<T>>();

            let slice = NonNull::slice_from_raw_parts(nonnull, cap);
            self.0 = slice;

            Ok(())
        } else {
            Err(Failure::Layout)
        }
    }
}

unsafe impl<#[may_dangle] T, A: Allocator> Drop for RawVec<T, A> {
    fn drop(&mut self) {
        if let Some((ptr, layout)) = self.current_memory() {
            unsafe { self.1.deallocate(ptr, layout) };
        }
    }
}

#[test]
fn test_vec() {
    let mut x = RawVec::<()>::allocate_in(10_000, AllocInit::Uninitialized, Global).unwrap();

    unsafe {
        x.grow(10000, 10).unwrap();
    }
}
