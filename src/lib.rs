#![feature(dropck_eyepatch)]
#![feature(slice_ptr_len)]
#![no_std]

extern crate alloc;

use {
    alloc::alloc::{alloc, dealloc, handle_alloc_error, Layout, LayoutErr},
    core::{
        mem::{self, MaybeUninit},
        ptr::{self, NonNull},
    },
};

pub struct RawVec<T> {
    ptr: NonNull<[MaybeUninit<T>]>,
}

impl<T> RawVec<T> {
    fn layout(capacity: usize) -> Result<Layout, LayoutErr> {
        Layout::array::<MaybeUninit<T>>(capacity)
    }

    unsafe fn raw_alloc(layout: Layout) -> Result<*mut u8, ()> {
        let ptr = alloc(layout);

        if ptr.is_null() {
            Err(())
        } else {
            Ok(ptr)
        }
    }

    /// Returns if the buffer needs to grow to fulfill the needed extra capacity.
    /// Mainly used to make inlining reserve-calls possible without inlining `grow`.
    fn needs_grow(&self, len: usize, additional: usize) -> bool {
        additional > self.cap().wrapping_sub(len)
    }
}
impl<T> RawVec<T> {
    const NON_NULL_PTR: NonNull<[MaybeUninit<T>]> = NonNull::<[MaybeUninit<T>; 0]>::dangling();

    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            ptr: Self::NON_NULL_PTR,
        }
    }

    pub fn with_capacity(capacity: usize) -> Result<Self, ()> {
        if mem::size_of::<T>() == 0 {
            Ok(Self::new())
        } else {
            let layout = Self::layout(capacity).map_err(drop)?;

            match alloc_guard(layout.size()) {
                Ok(_) => {}
                Err(_) => capacity_overflow(),
            }

            unsafe {
                let ptr = Self::raw_alloc(layout)?;

                let raw_mut: *mut [MaybeUninit<T>] =
                    ptr::slice_from_raw_parts_mut(ptr.cast(), capacity);
                let nonnull = NonNull::new_unchecked(raw_mut);

                Ok(Self { ptr: nonnull })
            }
        }
    }
    
    pub fn try_reserve(&mut self, len: usize, additional: usize) -> Result<(), TryReserveError> {
        if self.needs_grow(len, additional) {
            // TODO
            Ok(())
        } else {
            Ok(())
        }
    }

    // FIXME: Make this const whenever possible
    #[inline(always)]
    pub fn cap(&self) -> usize {
        self.ptr.as_ptr().len()
    }
}

impl<T> RawVec<T> {
    // FIXME: make this const whenever possible
    //  - depends on Self::cap constness
    fn current_memory(&self) -> Option<(NonNull<u8>, Layout)> {
        let cap = self.cap();

        if mem::size_of::<T>() == 0 || cap == 0 {
            None
        } else {
            // We have an allocated chunk of memory, so we can bypass runtime
            // checks to get our current layout.
            unsafe {
                let align = mem::align_of::<T>();
                let size = mem::size_of::<MaybeUninit<T>>() * cap;
                let layout = Layout::from_size_align_unchecked(size, align);
                Some((self.ptr.cast(), layout))
            }
        }
    }
}

unsafe impl<#[may_dangle] T> Drop for RawVec<T> {
    fn drop(&mut self) {
        if let Some((ptr, layout)) = self.current_memory() {
            unsafe { dealloc(ptr.as_ptr(), layout) }
        }
    }
}

/// The error type for `try_reserve` methods.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TryReserveError {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,

    /// The memory allocator returned an error
    AllocError {
        /// The layout of allocation request that failed
        layout: Layout,

        #[doc(hidden)]
        non_exhaustive: (),
    },
}

// We need to guarantee the following:
// * We don't ever allocate `> isize::MAX` byte-size objects.
// * We don't overflow `usize::MAX` and actually allocate too little.
//
// On 64-bit we just need to check for overflow since trying to allocate
// `> isize::MAX` bytes will surely fail. On 32-bit and 16-bit we need to add
// an extra guard for this in case we're running on a platform which can use
// all 4GB in user-space, e.g., PAE or x32.
#[inline]
fn alloc_guard(alloc_size: usize) -> Result<(), TryReserveError> {
    if mem::size_of::<usize>() < 8 && alloc_size > isize::MAX as usize {
        Err(TryReserveError::CapacityOverflow)
    } else {
        Ok(())
    }
}
// One central function responsible for reporting capacity overflows. This'll
// ensure that the code generation related to these panics is minimal as there's
// only one location which panics rather than a bunch throughout the module.
fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_capacity() {
        let v = RawVec::<u32>::with_capacity(1000);
    }
}
