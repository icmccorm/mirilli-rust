use std::alloc::{Layout, alloc, dealloc};
use std::marker::PhantomData;
use std::ops::{Add, BitAnd, Deref, DerefMut};

/// Different targets have a different number
/// of significant bits in their pointer representation.
/// On 32-bit platforms, all 32-bits are addressable. Most
/// 64-bit platforms only use 48-bits. Following the LLVM Project,
/// we hard-code these values based on the underlying architecture. 
/// Most, if not all 64 bit architectures use 48-bits. However, a the
/// Armv8-A spec allows addressing 52 or 56 bits, as well. No processors
/// implement this yet, though, so we can use target_pointer_width.

#[cfg(target_pointer_width = 64)]
static SIG_BITS: usize = 48;

#[cfg(target_pointer_width = 32)]
static SIG_BITS: usize = 32;

#[cfg(target_pointer_width = 16)]
static SIG_BITS: usize = 16;

static PTR_BYTES: usize = std::mem::size_of::<usize>();

// We use a two-level shadow page table. The second level is an array
// covering 64kb of memory. This size is chosen arbitrarily to match the 
// implementation in Valgrind—we should experiment with larger or smaller
// sizes as necessary (e.g. 16, 32, 64, 128)
static L2_CHUNK_SIZE_BYTES: usize = 64 * 1024;

// We assume that pointers are stored at word-aligned addresses. 
// This means that we need to map every 8 bytes of memory to a
// provenance value stored in shadow memory. The width of the L1 page 
// table is the number of 8-byte sub-chunks within L1_CHUNK_SIZE_BYTES
static L2_WIDTH: usize = L1_CHUNK_SIZE_BYTES.strict_div(PTR_BYTES);

// The first level is an array of pointers to second-level entries. 
// We have (2^SIG_BITS) addressable bytes of addressable memory, which
// we need to cover with L2_CHUNK_SIZE_BYTES-sized chunks. 
static L1_WIDTH: usize = SIG_BITS.pow(2).strict_div(L1_CHUNK_SIZE_BYTES);

// Provenance values must be sized so that we can allocate an array of them
// for the L1 page table. We can make provenance values Copy since they should
// fit within 128 bits and they are not "owned" by any particular object.
pub trait Provenance: Copy + Sized {}

#[repr(C)]
pub struct L2<T: Provenance> {
    bytes: *mut T,
}

impl<T: Provenance> L2<T> {
    fn new() -> Self {
        unsafe {
            let bytes = alloc(
                Layout::from_size_align(L2_WIDTH * std::mem::size_of::<Provenance>, std::mem::align_of::<Provenance>()).unwrap());
            Self { bytes }
        }
    }
    #[inline]
    fn lookup(&mut self, index: usize) -> &mut T {
        debug_assert!(index < W);
        unsafe {
            let bytes = self.bytes.add(index);
            let bytes = bytes.cast::<T>();
            &mut *bytes
        }
    }
}

impl<T: Provenance> Drop for L2<T> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align(L2_WIDTH, 1).unwrap();
            dealloc(self.bytes, layout);
        }
    }
}

#[repr(C)]
pub struct L1<T: Provenance> {
    entries: *mut [*mut L2<T>; L1_WIDTH],
}

impl<T: Provenance> L1<T> {
    fn new() -> Self {
        let align = std::mem::align_of::<[*mut L2<T>; L1_WIDTH]>();
        let size = std::mem::size_of::<[*mut L2<T>; L1_WIDTH]>();
        unsafe {
            let entries = alloc(Layout::from_size_align(size, align).unwrap());
            let entries = entries.cast::<[*mut L2<T>; L1_WIDTH]>();
            Self { entries }
        }
    }
    #[inline]
    #[cfg(target_endian = "little")]
    fn lookup(&mut self, index: usize) -> &mut T {
        use std::ops::Shr;
        let as_l1 = index.shr(L2_WIDTH).bitand(L1_WIDTH - 1);
        let as_l2 = index.bitand(L2_WIDTH - 1);
        debug_assert!(as_l1 < L1_WIDTH);
        debug_assert!(as_l2 < L2_WIDTH);
        unsafe {
            let l2_entry = &mut *(*self.entries)[as_l1];
            l2_entry.lookup(as_l2)
        }
    }
    #[inline(always)]
    #[cfg(target_endian = "big")]
    fn lookup(&mut index: usize) -> &mut T {
        use std::ops::Shl;
        let as_l1 = index.shr(L2_WIDTH).bitand(L1_WIDTH - 1);
        let as_l2 = index.bitand(L2_WIDTH - 1);
        debug_assert!(as_l1 < L1_WIDTH);
        debug_assert!(as_l2 < L2_WIDTH);
        unsafe {
            let l2_entry = &mut *(*self.bytes)[as_l1];
            l2_entry.lookup(as_l2)
        }
    }
}

impl<T: Provenance> Drop for L1<T> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align(W1, 1).unwrap();
            let entries = self.entries.cast::<u8>();
            dealloc(entries, layout);
        }
    }
}

pub struct ShadowHeap<T: Provenance> {
    l1: L1<T>,
}

impl<T: Provenance> Default for ShadowHeap<T> {
    fn default() -> Self {
        let l1 = L1::<T>::new();
        Self { l1 }
    }
}

impl Deref for ShadowHeap<T> {
    type Target = L1<T>;
    fn deref(&self) -> &Self::Target {
        &self.l1
    }
}

impl DerefMut for ShadowHeap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.l1
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    type TestProv = u8;
    #[test]
    fn create_and_drop() {
        let _ = ShadowHeap<TestProv>::default();
    }
}