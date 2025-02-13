#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct Size {
    pub bytes: usize,
}

impl Size {
    pub fn from_bytes(bytes: usize) -> Self {
        Self { bytes }
    }
}
