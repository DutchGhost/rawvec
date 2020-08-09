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

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Failure {
    CapacityOverflow,
    Layout,
    RawAlloc,
}

#[derive(Debug)]
pub struct AllocError {
    kind: Failure,
    size: usize,
}

impl AllocError {
    const fn new(size: usize, kind: Failure) -> Self {
        Self { size, kind }
    }
}

impl<T> RawVec<T> {
    fn layout(capacity: usize) -> Result<Layout, LayoutErr> {
        Layout::array::<MaybeUninit<T>>(capacity)
    }

    unsafe fn raw_alloc(layout: Layout) -> Result<*mut u8, AllocError> {
        let ptr = alloc(layout);

        if ptr.is_null() {
            Err(AllocError::new(layout.size(), Failure::RawAlloc))
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

    pub fn with_capacity(capacity: usize) -> Result<Self, AllocError> {
        if mem::size_of::<T>() == 0 {
            Ok(Self::new())
        } else {
            Self::layout(capacity)
                .map_err(|_| AllocError::new(capacity, Failure::Layout))
                .and_then(alloc_guard)
                .and_then(|layout| unsafe { Self::raw_alloc(layout) })
                .map(|ptr| unsafe {
                    let ptr: *mut [MaybeUninit<T>] =
                        ptr::slice_from_raw_parts_mut(ptr.cast(), capacity);
                    let ptr = NonNull::new_unchecked(ptr);

                    Self { ptr }
                })
        }
    }

    pub fn try_reserve(&mut self, len: usize, additional: usize) -> Result<(), AllocError> {
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

// We need to guarantee the following:
// * We don't ever allocate `> isize::MAX` byte-size objects.
// * We don't overflow `usize::MAX` and actually allocate too little.
//
// On 64-bit we just need to check for overflow since trying to allocate
// `> isize::MAX` bytes will surely fail. On 32-bit and 16-bit we need to add
// an extra guard for this in case we're running on a platform which can use
// all 4GB in user-space, e.g., PAE or x32.
#[inline]
fn alloc_guard(layout: Layout) -> Result<Layout, AllocError> {
    if mem::size_of::<usize>() < 8 && layout.size() > isize::MAX as usize {
        Err(AllocError::new(layout.size(), Failure::CapacityOverflow))
    } else {
        Ok(layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_capacity() {
        let v = RawVec::<u32>::with_capacity(1000);
    }
}
