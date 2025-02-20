pub(super) mod tree_borrows_wrapper {
    use std::ffi::c_void;
    pub type BorrowTag = u64;

    pub struct Tree {}
    impl Tree {
        pub fn new(object_address: *const c_void, alloc_size: usize) -> Self {
            Self {}
        }

        pub fn get_root_borrow_tag(&self) -> BorrowTag {
            0
        }
    }
}
