//! The C library's memory advice (Linux): declarations only.

use std::ffi::{c_int, c_void};

/// `MADV_HUGEPAGE`: back the range with transparent huge pages where the
/// kernel can.
pub(crate) const MADV_HUGEPAGE: c_int = 14;

unsafe extern "C" {
    /// `madvise(2)`: advice on how the pages of `addr..addr + length`
    /// (page-aligned) are used; 0, or -1 with `errno` set.
    pub(crate) fn madvise(addr: *mut c_void, length: usize, advice: c_int) -> c_int;
}
