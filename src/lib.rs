#![feature(const_slice_ptr_len)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(slice_ptr_get)]
#![feature(const_fn)]
#![feature(allocator_api)]
#![feature(dropck_eyepatch)]
#![feature(slice_ptr_len)]
#![no_std]

extern crate alloc;

use {
    alloc::alloc::{handle_alloc_error, AllocError, Allocator, Global, Layout, LayoutError},
    core::{
        mem::{self, MaybeUninit},
        ptr::{self, NonNull},
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
    Uninitialized
}

impl<T, A: Allocator> RawVec<T, A> {
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
        }  else {
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
}

unsafe impl<#[may_dangle] T, A: Allocator> Drop for RawVec<T, A> {
    fn drop(&mut self) {
        if let Ok(layout) = Self::layout(self.capacity()) {
            let ptr = self.0.as_non_null_ptr().cast();
            unsafe { self.1.deallocate(ptr, layout) };
        }
    }
}
