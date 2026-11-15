//! The bounds of the calling thread's stack on Linux, with glibc or musl.
//!
//! A thread made with `pthread_create` (Python's threads, those of other
//! runtimes calling into Python) has a stack of fixed size, which both
//! libraries report through `pthread_getattr_np` (without its guard
//! page). The main thread's stack is the `[stack]` mapping, which the
//! kernel grows on demand down to its soft `RLIMIT_STACK` (`ulimit -s`),
//! and never closer than its stack guard gap (256 pages) to the mapping
//! below it: the bound for an unlimited or huge limit. musl reports only
//! the part mapped so far, glibc the limit without the gap, so the main
//! thread's bound is read from `/proc` here instead, once.

use std::ffi::c_void;

use super::Bounds;
use crate::ffi;

/// The kernel's `stack_guard_gap`, in pages (its default).
const GUARD_GAP_PAGES: usize = 256;

/// The page size where the auxiliary vector does not say (`AT_PAGESZ`).
const DEFAULT_PAGE: usize = 4096;

/// The bounds of the calling thread's stack: the lowest address it may
/// reach and its top.
pub(super) fn bounds(here: usize) -> Option<Bounds> {
    if is_main_thread() {
        main_thread_bounds(here)
    } else {
        thread_bounds()
            .filter(|&(low, high)| (low..high).contains(&here))
            .map(|(low, high)| Bounds {
                end: low,
                top: high,
            })
    }
}

/// Whether the calling thread is the process's main thread (its thread id
/// is the process id), from `/proc/thread-self`, which links to
/// `<pid>/task/<tid>`.
fn is_main_thread() -> bool {
    std::fs::read_link("/proc/thread-self")
        .ok()
        .and_then(|link| {
            let link = link.to_str()?.to_owned();
            let (pid, tid) = link.split_once("/task/")?;
            Some(pid == tid)
        })
        .unwrap_or(false)
}

/// The lowest and the highest address of the calling thread's stack, as
/// the C library made it.
fn thread_bounds() -> Option<(usize, usize)> {
    let mut attr = ffi::pthread_attr_t::uninit();
    let mut low: *mut c_void = std::ptr::null_mut();
    let mut size = 0_usize;
    // SAFETY: `pthread_self` has no preconditions; `pthread_getattr_np`
    // initialises the attributes of the calling (running) thread in
    // `attr`, storage as large as the C library's, which
    // `pthread_attr_getstack` then reads into two live locals and
    // `pthread_attr_destroy` releases, only once it was initialised.
    let read = unsafe {
        if ffi::pthread_getattr_np(ffi::pthread_self(), &raw mut attr) != 0 {
            return None;
        }
        let read = ffi::pthread_attr_getstack(&raw const attr, &raw mut low, &raw mut size);
        ffi::pthread_attr_destroy(&raw mut attr);
        read
    };
    (read == 0).then(|| (low.addr(), low.addr().saturating_add(size)))
}

/// The bounds of the main thread's stack, the mapping holding `here`: the
/// lowest address it may grow down to, and the mapping's top.
fn main_thread_bounds(here: usize) -> Option<Bounds> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    let (below, top) = stack_mapping(&maps, here)?;
    let limit = std::fs::read_to_string("/proc/self/limits")
        .ok()
        .and_then(|limits| soft_stack_limit(&limits))
        .unwrap_or(usize::MAX);
    let page = std::fs::read("/proc/self/auxv")
        .ok()
        .and_then(|auxv| page_size(&auxv))
        .unwrap_or(DEFAULT_PAGE);
    let by_neighbour = below.saturating_add(GUARD_GAP_PAGES * page);
    Some(Bounds {
        end: top.saturating_sub(limit).max(by_neighbour),
        top,
    })
}

/// The end of the mapping below the one holding `here`, and the end of
/// that one, from the lines of `/proc/self/maps` (`start-end perms ...`,
/// in address order).
fn stack_mapping(maps: &str, here: usize) -> Option<(usize, usize)> {
    let mut below = 0;
    for line in maps.lines() {
        let range = line.split_whitespace().next()?;
        let (start, end) = range.split_once('-')?;
        let start = usize::from_str_radix(start, 16).ok()?;
        let end = usize::from_str_radix(end, 16).ok()?;
        if (start..end).contains(&here) {
            return Some((below, end));
        }
        below = end;
    }
    None
}

/// The soft stack limit of `/proc/self/limits` (`Max stack size  8388608
/// unlimited  bytes`), `usize::MAX` for `unlimited`.
fn soft_stack_limit(limits: &str) -> Option<usize> {
    let line = limits
        .lines()
        .find_map(|line| line.strip_prefix("Max stack size"))?;
    match line.split_whitespace().next()? {
        "unlimited" => Some(usize::MAX),
        soft => soft.parse().ok(),
    }
}

/// The page size of the auxiliary vector `/proc/self/auxv`: pairs of
/// native words, `AT_PAGESZ` (6) holding it.
fn page_size(auxv: &[u8]) -> Option<usize> {
    /// `AT_PAGESZ`.
    const PAGE_SIZE: usize = 6;
    let (words, _) = auxv.as_chunks::<{ size_of::<usize>() }>();
    let (pairs, _) = words.as_chunks::<2>();
    pairs.iter().find_map(|[key, value]| {
        (usize::from_ne_bytes(*key) == PAGE_SIZE).then(|| usize::from_ne_bytes(*value))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAPS: &str = "\
55d0c0a00000-55d0c0a21000 r--p 00000000 08:01 1234    /usr/bin/python3
7f2a40000000-7f2a40021000 rw-p 00000000 00:00 0
7ffc8a5e0000-7ffc8a601000 rw-p 00000000 00:00 0       [stack]
7ffc8a7f1000-7ffc8a7f5000 r--p 00000000 00:00 0       [vvar]
";

    #[test]
    fn the_mapping_of_an_address_is_found_with_the_one_below() {
        assert_eq!(
            stack_mapping(MAPS, 0x7ffc_8a60_0f00),
            Some((0x7f2a_4002_1000, 0x7ffc_8a60_1000))
        );
        assert_eq!(stack_mapping(MAPS, 0x1000), None);
        assert_eq!(stack_mapping("", 0x1000), None);
        assert_eq!(stack_mapping("not a mapping", 0x1000), None);
    }

    #[test]
    fn the_soft_stack_limit_is_read_in_bytes_or_unlimited() {
        let limits = "Limit                     Soft Limit           Hard Limit           Units\n\
                      Max cpu time              unlimited            unlimited            seconds\n\
                      Max stack size            8388608              unlimited            bytes\n";
        assert_eq!(soft_stack_limit(limits), Some(8_388_608));
        let unlimited =
            "Max stack size            unlimited            unlimited            bytes\n";
        assert_eq!(soft_stack_limit(unlimited), Some(usize::MAX));
        assert_eq!(
            soft_stack_limit("Max cpu time  unlimited  unlimited  seconds"),
            None
        );
    }

    #[test]
    fn the_page_size_is_read_from_the_auxiliary_vector() {
        let auxv: Vec<u8> = [33_usize, 0x7fff_0000, 6, 16384, 0, 0]
            .iter()
            .flat_map(|word| word.to_ne_bytes())
            .collect();
        assert_eq!(page_size(&auxv), Some(16384));
        assert_eq!(page_size(&auxv[..2 * size_of::<usize>()]), None);
        assert_eq!(page_size(&[]), None);
    }

    #[test]
    fn this_process_reports_its_own_stacks() {
        let main_like = std::fs::read_to_string("/proc/self/maps").unwrap();
        assert!(main_like.contains("[stack]"));
        assert!(page_size(&std::fs::read("/proc/self/auxv").unwrap()).is_some());
        // The test runs on a thread of the harness, of a fixed stack.
        assert!(!is_main_thread());
        let (low, high) = thread_bounds().unwrap();
        let here = super::super::here();
        assert!((low..high).contains(&here));
        assert_eq!(
            bounds(here),
            Some(Bounds {
                end: low,
                top: high
            })
        );
    }

    #[test]
    fn the_main_thread_ends_at_its_limit_or_above_the_mapping_below() {
        let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
        let (below, top) = maps
            .lines()
            .find(|line| line.ends_with("[stack]"))
            .and_then(|line| {
                let (start, _) = line.split_once('-')?;
                stack_mapping(&maps, usize::from_str_radix(start, 16).ok()?)
            })
            .unwrap();
        let bounds = main_thread_bounds(top - 1).unwrap();
        assert_eq!(bounds.top, top);
        let end = bounds.end;
        assert!(end > below && end < top, "{below:x} < {end:x} < {top:x}");
    }
}
