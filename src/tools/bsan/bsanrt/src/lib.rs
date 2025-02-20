#![feature(strict_overflow_ops)]
#![allow(unused)]

mod shadow;
mod tree_borrows;

use core::ffi::c_void;
use core::num::NonZero;

use log::info;
use tree_borrows::tree_borrows_wrapper as TreeBorrows;

type AllocId = u64;

// Atomic counter to assign unique IDs to each allocation
static ALLOC_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// This is the metadata stored with each allocation
struct AllocMetadata {
    alloc_id: AllocId,
    tree_address: Box<TreeBorrows::Tree>,
}

/// This is the metadata stored with each pointer in the shadow memory
/// to track the provenance of the pointer
#[repr(C)]
struct Provenance {
    alloc_id: AllocId,
    borrow_tag: TreeBorrows::BorrowTag,
    lock_address: *const AllocMetadata,
}

#[no_mangle]
extern "C" fn bsan_init() {
    let _ = env_logger::builder().try_init();
    info!("Initialized global state");
}

/// This function will be called by the malloc interceptor, everytime the
/// application calls malloc.
/// `object_address` is the address of the allocated object
/// `alloc_size` is the size of the allocated object
/// `result_provenance` is a pointer for returning the provenance (pointer metadata) for this allocation.
#[no_mangle]
extern "C" fn bsan_malloc(object_address: *const c_void, alloc_size: usize) -> Provenance {
    let tree = Box::new(TreeBorrows::Tree::new(object_address, alloc_size));
    let root_borrow_tag = tree.get_root_borrow_tag();
    // TODO(obraunsdorf): not sure about the ordering
    let alloc_id = ALLOC_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let alloc_metadata = Box::new(AllocMetadata { alloc_id, tree_address: tree });

    info!(
        "Creating Metadata for heap object at address: {:?}, with size: {}",
        object_address, alloc_size
    );

    Provenance {
        alloc_id,
        borrow_tag: root_borrow_tag,
        lock_address: Box::into_raw(alloc_metadata),
    }
}

#[no_mangle]
extern "C" fn bsan_expose_tag(ptr: *mut c_void) {
    info!("Exposed tag for pointer: {:?}", ptr);
}

#[no_mangle]
extern "C" fn bsan_retag(ptr: *mut c_void, retag_kind: u8, place_kind: u8) -> u64 {
    info!("Retagged pointer: {:?}", ptr);
    0
}

#[no_mangle]
extern "C" fn bsan_read(ptr: *mut c_void, access_size: u64) {
    info!("Reading {} bytes starting at address: {:?}", access_size, ptr);
}

#[no_mangle]
extern "C" fn bsan_write(ptr: *mut c_void, access_size: u64) {
    info!("Writing {} bytes starting at address: {:?}", access_size, ptr);
}

#[no_mangle]
extern "C" fn bsan_func_entry() {
    info!("Entered function");
}

#[no_mangle]
extern "C" fn bsan_func_exit() {
    info!("Exited function");
}
