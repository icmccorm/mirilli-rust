#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(strict_overflow_ops)]
#![allow(unused)]

mod shadow;
mod util;

use libc::{malloc, free};
use core::ffi::c_void;
use core::iter::Once;
use core::num::NonZero;
use core::{fmt, mem};


use log::info;

#[derive(Debug, Clone)]
struct FrameState {}

/// A unique identifier for a location in the program.
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span(u64);

/// The type of retag being performed.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RetagKind {
    /// The initial retag of arguments when entering a function.
    FnEntry,
    /// Retag preparing for a two-phase borrow.
    TwoPhase,
    /// Retagging raw pointers.
    Raw,
    /// A "normal" retag.
    Default,
}


/// The underlying mutability of a place as determined
/// by its type.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlaceKind {
    Freeze,
    Unpin,
    Default,
}

/// The underlying mutability of an allocation
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mutability {
    Const,
    Mut,
}

/// The unique identifier for an allocation.
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AllocId(u64);

impl AllocId {
    pub fn new(u: u64) -> Self {
        Self(u)
    }

    pub fn get(&self) -> u64 {
        self.0
    }

    pub fn valid(self) -> bool {
        self != Self::null()
    }

    /// Null pointers receive an ID of 0.
    pub fn null() -> Self {
        Self(0)
    }
}

impl core::default::Default for AllocId {
    fn default() -> Self {
        Self::null()
    }
}

impl fmt::Debug for AllocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() { write!(f, "a{}", self.0) } else { write!(f, "alloc{}", self.0) }
    }
}

/// Links a pointer to its node within the tree.
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BorTag(NonZero<u64>);

impl BorTag {
    pub fn new(u: u64) -> Option<Self> {
        NonZero::new(u).map(BorTag)
    }

    pub fn get(&self) -> u64 {
        self.0.get()
    }

    pub fn inner(&self) -> NonZero<u64> {
        self.0
    }

    pub fn succ(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }

    /// The minimum representable tag
    pub fn one() -> Self {
        Self::new(1).unwrap()
    }
}

impl core::default::Default for BorTag {
    fn default() -> Self {
        Self::one()
    }
}

impl fmt::Debug for BorTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

/// Each pointer is associated with provenance,
/// which identifies its permission to access memory.
#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Provenance {
    id: AllocId,
    tag: BorTag,
    /// Pointer to the allocation's metadata.
    /// Provenance is the "key," and AllocInfo is the "lock".
    info: *mut AllocInfo,
}


/// The metadata associated with an allocation.
#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct AllocInfo {
    id: AllocId,
    size: u64,
    base_address: *mut c_void,
    /// The tree remains uninitialized (null) until the allocation is
    /// borrowed for the first time. Prior to that point, `mutability` is
    /// used to check each access.
    mutability: Mutability,
    tree: *mut c_void,
}

#[no_mangle]
unsafe extern "C" fn bsan_init() {
    let _ = env_logger::builder().try_init();
    info!("Initialized global state");
}

#[inline]
#[no_mangle]
extern "C" fn bsan_expose_tag(
    ptr: *mut c_void,
    provenance: *const Provenance,
) {
    info!("Exposed tag for pointer: {:?}", ptr);
}

#[inline]
#[no_mangle]
extern "C" fn bsan_retag(ptr: *mut c_void, retag_kind: u8, place_kind: u8) -> u64 {
    info!("Retagged pointer: {:?}", ptr);
    0
}

#[inline]
#[no_mangle]
extern "C" fn bsan_read(
    ptr: *const c_void,
    access_size: u64,
    provenance: *const Provenance,
) {
    info!("Reading {} bytes starting at address: {:?}", access_size, ptr);
}

#[inline]
#[no_mangle]
extern "C" fn bsan_write(
    ptr: *const c_void,
    access_size: u64,
    provenance: *const Provenance,
) {
    info!("Writing {} bytes starting at address: {:?}", access_size, ptr);
}

/// Removes all protectors for the given function.
#[inline]
#[no_mangle]
extern "C" fn bsan_func_entry() {
    info!("Entered function");
}

/// Removes all protectors for the given function.
#[inline]
#[no_mangle]
extern "C" fn bsan_func_exit() {
    info!("Exited function");
}

/// Performs a deallocation access using the given `ptr` and `provenance`.
/// If successful, it deallocates the tree within `alloc_info`.
#[inline]
#[no_mangle]
extern "C" fn bsan_dealloc(
    ptr: *mut c_void,
    provenance: *const Provenance,
) {
    info!("Deallocating metadata for pointer: {:?}", ptr);
}

/// Allocates metadata for an allocation spanning `size` bytes 
/// starting at the address `ptr`.
#[inline]
#[no_mangle]
extern "C" fn bsan_alloc(ptr: *mut c_void, size: u64) -> *mut c_void {
    info!("Allocating metadata for {:?} bytes", size);
    unsafe {
        let x:*mut u8 = core::mem::transmute(libc::malloc(1));
        *x = 1;
        info!("x: {:?}", *x);
        libc::free(core::mem::transmute(x));
    }
    core::ptr::null_mut()
}
