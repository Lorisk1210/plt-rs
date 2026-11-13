//! The extension's allocator on Linux: the system's, asking the kernel
//! for transparent huge pages on every block of 4 MiB or more, as numpy
//! does for its arrays.
//!
//! A figure's large buffers (a copy of the data, the RGBA image of an
//! `imshow`, the vertices of a long line) are made and freed by every
//! draw. The C library returns freed memory to the kernel, so the next
//! draw faults it in again, a page at a time: 4 KiB pages cost one fault
//! each, huge pages one per 2 MiB. The memory a process keeps is the same
//! either way. Where the kernel has transparent huge pages off (or set to
//! `never`), the advice changes nothing.

use std::alloc::{GlobalAlloc, Layout, System};

use crate::ffi;

/// Blocks this large or larger are advised (numpy's threshold).
const LARGE: usize = 4 << 20;

/// The size of an x86-64 or arm64 (4 KiB pages) transparent huge page.
/// The advised range is cut to whole huge pages: only those can be backed
/// by one, and their bounds are page bounds whatever the base page size.
const HUGE_PAGE: usize = 2 << 20;

/// The system allocator, with huge pages for large blocks.
pub(crate) struct HugePages;

/// The whole huge pages of the block of `size` bytes at `address`, as
/// their start and length, if the block is large and holds one.
fn advised_range(address: usize, size: usize) -> Option<(usize, usize)> {
    if address == 0 || size < LARGE {
        return None;
    }
    let start = address.checked_next_multiple_of(HUGE_PAGE)?;
    let end = address.checked_add(size)? / HUGE_PAGE * HUGE_PAGE;
    (start < end).then(|| (start, end - start))
}

/// Asks for huge pages on the whole huge pages of the block of `size`
/// bytes at `ptr`, if it is large. The advice is a hint: if the kernel
/// declines it, nothing changes, so its result is not looked at.
fn advise(ptr: *mut u8, size: usize) {
    let Some((start, length)) = advised_range(ptr.addr(), size) else {
        return;
    };
    // SAFETY: `start..start + length` lies within the block just
    // allocated, which this process owns, and is page-aligned; the
    // advice changes how its pages are backed, never their contents.
    unsafe {
        ffi::madvise(ptr.with_addr(start).cast(), length, ffi::MADV_HUGEPAGE);
    }
}

// SAFETY: every block comes from the system allocator, with the layout it
// was asked for, and goes back to it; the advice keeps the contents.
unsafe impl GlobalAlloc for HugePages {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract for `layout` is the system's.
        let ptr = unsafe { System.alloc(layout) };
        advise(ptr, layout.size());
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as for `alloc`.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        advise(ptr, layout.size());
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System` with `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: `ptr` came from `System` with `layout`, and the caller
        // keeps `realloc`'s contract for `new_size`.
        let ptr = unsafe { System.realloc(ptr, layout, new_size) };
        advise(ptr, new_size);
        ptr
    }
}

/// The allocator of the extension's Rust code.
#[global_allocator]
static ALLOCATOR: HugePages = HugePages;

#[cfg(test)]
mod tests {
    use super::{HUGE_PAGE, LARGE, advised_range};

    #[test]
    fn small_and_null_blocks_are_not_advised() {
        assert_eq!(advised_range(HUGE_PAGE, LARGE - 1), None);
        assert_eq!(advised_range(0, 16 * LARGE), None);
        assert_eq!(advised_range(HUGE_PAGE, 0), None);
    }

    #[test]
    fn the_advice_covers_the_whole_huge_pages_inside_the_block() {
        // Aligned: all of it.
        assert_eq!(advised_range(HUGE_PAGE, LARGE), Some((HUGE_PAGE, LARGE)));
        // A malloc header's 16 bytes in: the pages after the first bound,
        // up to the last bound inside.
        let address = 3 * HUGE_PAGE + 16;
        assert_eq!(
            advised_range(address, LARGE),
            Some((4 * HUGE_PAGE, HUGE_PAGE))
        );
        // A block of 4 MiB less one byte past a bound holds one page.
        assert_eq!(
            advised_range(HUGE_PAGE + 1, LARGE + HUGE_PAGE - 2),
            Some((2 * HUGE_PAGE, HUGE_PAGE))
        );
    }

    #[test]
    fn blocks_at_the_end_of_the_address_space_are_not_advised() {
        assert_eq!(advised_range(usize::MAX - LARGE, LARGE + 1), None);
        assert_eq!(advised_range(usize::MAX - HUGE_PAGE / 2, LARGE), None);
    }

    /// The start and end of the mapping a header line of
    /// `/proc/self/smaps` describes (`start-end perms ...`).
    fn mapping_bounds(line: &str) -> Option<(usize, usize)> {
        let (range, _) = line.split_once(' ')?;
        let (start, end) = range.split_once('-')?;
        Some((
            usize::from_str_radix(start, 16).ok()?,
            usize::from_str_radix(end, 16).ok()?,
        ))
    }

    /// The flags of the mapping of `/proc/self/smaps` holding `address`.
    fn mapping_flags(address: usize) -> Option<String> {
        let smaps = std::fs::read_to_string("/proc/self/smaps").ok()?;
        let mut inside = false;
        for line in smaps.lines() {
            if let Some((start, end)) = mapping_bounds(line) {
                inside = (start..end).contains(&address);
            } else if inside && let Some(flags) = line.strip_prefix("VmFlags:") {
                return Some(flags.to_owned());
            }
        }
        None
    }

    #[test]
    fn large_blocks_ask_for_huge_pages_and_keep_their_contents() {
        let thp = std::path::Path::new("/sys/kernel/mm/transparent_hugepage/enabled");
        let mut block = vec![0_u8; 3 * LARGE];
        block.iter_mut().enumerate().for_each(|(i, byte)| {
            *byte = u8::try_from(i % 251).expect("below 251");
        });
        let (start, _) = advised_range(block.as_ptr().addr(), block.len()).expect("a large block");
        if thp.exists() {
            let flags = mapping_flags(start).expect("the block is mapped");
            assert!(flags.split_whitespace().any(|flag| flag == "hg"), "{flags}");
        }
        block.resize(5 * LARGE, 7);
        assert!(
            block[..3 * LARGE]
                .iter()
                .enumerate()
                .all(|(i, &byte)| usize::from(byte) == i % 251)
        );
        assert!(block[3 * LARGE..].iter().all(|&byte| byte == 7));
    }
}
