//! The Windows API the binding uses, beside CPython's: the bounds of the
//! calling thread's stack.

#[cfg_attr(target_env = "gnu", link(name = "kernel32"))]
#[cfg_attr(not(target_env = "gnu"), link(name = "kernel32", kind = "raw-dylib"))]
unsafe extern "system" {
    /// `GetCurrentThreadStackLimits` (Windows 8): the lowest and the
    /// highest address of the calling thread's stack.
    pub(crate) fn GetCurrentThreadStackLimits(low: *mut usize, high: *mut usize);
}
