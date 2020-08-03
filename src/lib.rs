#![feature(dropck_eyepatch)]

#![no_std]

extern crate alloc;

use core::{
    mem::{self, MaybeUninit},
    ptr::NonNull,
};

use alloc::alloc::{dealloc, Layout};

pub struct RawVec<T> {
    ptr: NonNull<[MaybeUninit<T>]>,
    cap: usize,
}

impl<T> RawVec<T> {
    const NON_NULL_PTR: NonNull<[MaybeUninit<T>]> = NonNull::<[MaybeUninit<T>; 0]>::dangling();

    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            ptr: Self::NON_NULL_PTR,
            cap: 0,
        }
    }
}

impl<T> RawVec<T> {
    const fn current_memory(&self) -> Option<(NonNull<u8>, Layout)> {
        if mem::size_of::<T>() == 0 || self.cap == 0 {
            None
        } else {
            // We have an allocated chunk of memory, so we can bypass runtime
            // checks to get our current layout.
            unsafe {
                let align = mem::align_of::<T>();
                let size = mem::size_of::<MaybeUninit<T>>() * self.cap;
                let layout = Layout::from_size_align_unchecked(size, align);
                Some((self.ptr.cast(), layout))
            }
        }
    }
}

unsafe impl <#[may_dangle] T> Drop for RawVec<T> {
    fn drop(&mut self) {
        if let Some((ptr, layout)) = self.current_memory() {
            unsafe { dealloc(ptr.as_ptr(), layout) }
        }
    }
}