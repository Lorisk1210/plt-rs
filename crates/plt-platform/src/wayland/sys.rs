//! The crate's only unsafe code: the C library calls the Wayland backend
//! needs and `std` lacks, behind safe wrappers.
//!
//! * [`send`] and [`receive`] pass file descriptors along with bytes over
//!   a Unix socket (`sendmsg`/`recvmsg` with `SCM_RIGHTS` ancillary data;
//!   `std`'s `SocketAncillary` is unstable). Wayland hands shared-memory
//!   pools, keymaps and clipboard pipes over this way.
//! * [`memfd`] creates anonymous shared memory (`memfd_create`) for pixel
//!   buffers. The buffers are written with `pwrite` through [`File`], not
//!   mapped: no memory is shared with the compositor, so no Rust reference
//!   can observe another process's writes and a compositor shrinking the
//!   file cannot fault this process.
//!
//! The ancillary data is assembled and parsed as plain bytes; only the
//! system calls themselves and the adoption of received descriptors are
//! `unsafe`. Libc structures use the kernel's layout for Linux, which
//! glibc and musl share on every architecture Rust supports.

use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;

/// `SOL_SOCKET`: the level of socket-wide ancillary data.
const SOL_SOCKET: c_int = 1;
/// `SCM_RIGHTS`: ancillary data carrying file descriptors.
const SCM_RIGHTS: c_int = 1;
/// `MSG_DONTWAIT`: fail with `EAGAIN` instead of blocking.
const MSG_DONTWAIT: c_int = 0x40;
/// `MSG_NOSIGNAL`: report a closed peer as `EPIPE`, without `SIGPIPE`.
const MSG_NOSIGNAL: c_int = 0x4000;
/// `MSG_CMSG_CLOEXEC`: make received descriptors close-on-exec.
const MSG_CMSG_CLOEXEC: c_int = 0x4000_0000;
/// `MSG_CTRUNC`: the ancillary data did not fit (descriptors were lost).
const MSG_CTRUNC: c_int = 0x8;
/// `MFD_CLOEXEC`: the memfd is closed on exec.
const MFD_CLOEXEC: c_uint = 1;

/// `SCM_MAX_FD`: the most descriptors Linux passes in one message.
const MAX_FDS: usize = 253;
/// Bytes of one descriptor in `SCM_RIGHTS` data.
const FD_LEN: usize = size_of::<c_int>();
/// Bytes of a `cmsghdr`, padded as `CMSG_ALIGN` pads it.
const HEADER_LEN: usize = align(size_of::<ControlHeader>());
/// Room for the ancillary data of [`MAX_FDS`] descriptors
/// (`CMSG_SPACE(MAX_FDS * 4)`).
const CONTROL_LEN: usize = HEADER_LEN + align(MAX_FDS * FD_LEN);

/// `struct iovec`.
#[repr(C)]
struct IoVec {
    base: *mut c_void,
    len: usize,
}

/// `struct msghdr`.
#[repr(C)]
struct MsgHdr {
    name: *mut c_void,
    name_len: u32,
    iov: *mut IoVec,
    iov_len: usize,
    control: *mut c_void,
    control_len: usize,
    flags: c_int,
}

/// `struct cmsghdr`, followed by its data.
#[repr(C)]
struct ControlHeader {
    len: usize,
    level: c_int,
    kind: c_int,
}

/// A buffer for ancillary data, aligned like a `cmsghdr`.
#[repr(C, align(8))]
struct Control([u8; CONTROL_LEN]);

unsafe extern "C" {
    fn sendmsg(socket: c_int, message: *const MsgHdr, flags: c_int) -> isize;
    fn recvmsg(socket: c_int, message: *mut MsgHdr, flags: c_int) -> isize;
    fn memfd_create(name: *const c_char, flags: c_uint) -> c_int;
}

/// `CMSG_ALIGN`: rounds up to the alignment of ancillary data.
const fn align(len: usize) -> usize {
    len.next_multiple_of(size_of::<usize>())
}

/// Sends `bytes` with `fds` attached (at most [`MAX_FDS`]); returns how
/// many bytes went out. The receiver gets its own copies of the
/// descriptors; the caller keeps its own.
pub(crate) fn send(socket: &UnixStream, bytes: &[u8], fds: &[BorrowedFd<'_>]) -> io::Result<usize> {
    if fds.len() > MAX_FDS {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut control = Control([0; CONTROL_LEN]);
    let control_len = if fds.is_empty() {
        0
    } else {
        let data_len = fds.len() * FD_LEN;
        let header = &mut control.0[..HEADER_LEN];
        header[..size_of::<usize>()].copy_from_slice(&(HEADER_LEN + data_len).to_ne_bytes());
        let level_at = size_of::<usize>();
        header[level_at..level_at + 4].copy_from_slice(&SOL_SOCKET.to_ne_bytes());
        header[level_at + 4..level_at + 8].copy_from_slice(&SCM_RIGHTS.to_ne_bytes());
        let data = control.0[HEADER_LEN..HEADER_LEN + data_len]
            .as_chunks_mut::<FD_LEN>()
            .0;
        for (slot, fd) in data.iter_mut().zip(fds) {
            slot.copy_from_slice(&fd.as_raw_fd().to_ne_bytes());
        }
        HEADER_LEN + align(data_len)
    };
    let mut iov = IoVec {
        base: bytes.as_ptr().cast_mut().cast(),
        len: bytes.len(),
    };
    let message = MsgHdr {
        name: std::ptr::null_mut(),
        name_len: 0,
        iov: &raw mut iov,
        iov_len: 1,
        control: if control_len == 0 {
            std::ptr::null_mut()
        } else {
            control.0.as_mut_ptr().cast()
        },
        control_len,
        flags: 0,
    };
    // SAFETY: `message` points to one valid `iovec` describing `bytes`,
    // which `sendmsg` only reads, and to `control_len` initialised bytes
    // of `control`, aligned for `cmsghdr`. Every pointer outlives the
    // call, and the descriptors in the ancillary data are borrowed, so
    // they stay open throughout.
    let sent = unsafe { sendmsg(socket.as_raw_fd(), &raw const message, MSG_NOSIGNAL) };
    usize::try_from(sent).map_err(|_| io::Error::last_os_error())
}

/// Receives into `buf`, appending the descriptors that came along to
/// `fds`; returns the number of bytes (0 at end of stream). Without
/// `block`, fails with [`io::ErrorKind::WouldBlock`] when nothing is
/// waiting.
pub(crate) fn receive(
    socket: &UnixStream,
    buf: &mut [u8],
    fds: &mut Vec<OwnedFd>,
    block: bool,
) -> io::Result<usize> {
    let mut control = Control([0; CONTROL_LEN]);
    let mut iov = IoVec {
        base: buf.as_mut_ptr().cast(),
        len: buf.len(),
    };
    let mut message = MsgHdr {
        name: std::ptr::null_mut(),
        name_len: 0,
        iov: &raw mut iov,
        iov_len: 1,
        control: control.0.as_mut_ptr().cast(),
        control_len: CONTROL_LEN,
        flags: 0,
    };
    let flags = MSG_CMSG_CLOEXEC | if block { 0 } else { MSG_DONTWAIT };
    // SAFETY: `message` points to one valid `iovec` describing `buf`,
    // which `recvmsg` may fill, and to `CONTROL_LEN` writable bytes of
    // `control`, aligned for `cmsghdr`. Every pointer outlives the call.
    let received = unsafe { recvmsg(socket.as_raw_fd(), &raw mut message, flags) };
    let received = usize::try_from(received).map_err(|_| io::Error::last_os_error())?;
    let control_len = message.control_len.min(CONTROL_LEN);
    for fd in received_fds(&control.0[..control_len]) {
        // SAFETY: the kernel just installed this descriptor in our table
        // for this message; nothing else refers to it, so it is ours to
        // own and close.
        fds.push(unsafe { OwnedFd::from_raw_fd(fd) });
    }
    if message.flags & MSG_CTRUNC != 0 {
        return Err(io::Error::other("file descriptors were dropped"));
    }
    Ok(received)
}

/// The descriptors in the `SCM_RIGHTS` messages of ancillary data.
fn received_fds(control: &[u8]) -> Vec<RawFd> {
    let mut fds = Vec::new();
    let mut at = 0;
    while let Some(header) = control.get(at..at + HEADER_LEN) {
        let (len, rest) = header.split_at(size_of::<usize>());
        let len = usize::from_ne_bytes(len.try_into().expect("a usize field"));
        let level = c_int::from_ne_bytes(rest[..4].try_into().expect("a c_int field"));
        let kind = c_int::from_ne_bytes(rest[4..8].try_into().expect("a c_int field"));
        let Some(data) = len
            .checked_sub(HEADER_LEN)
            .and_then(|data_len| control.get(at + HEADER_LEN..at + HEADER_LEN + data_len))
        else {
            break;
        };
        if level == SOL_SOCKET && kind == SCM_RIGHTS {
            let words = data.as_chunks::<FD_LEN>().0;
            fds.extend(words.iter().map(|&fd| RawFd::from_ne_bytes(fd)));
        }
        at += align(len);
    }
    fds
}

/// Creates an empty anonymous file, closed on exec, to back shared
/// memory.
pub(crate) fn memfd(name: &CStr) -> io::Result<File> {
    // SAFETY: `name` is a valid NUL-terminated string that outlives the
    // call; `memfd_create` reads nothing else.
    let fd = unsafe { memfd_create(name.as_ptr(), MFD_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `memfd_create` returned a new descriptor that nothing else
    // refers to, so it is ours to own and close.
    Ok(File::from(unsafe { OwnedFd::from_raw_fd(fd) }))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::os::fd::AsFd;
    use std::os::unix::fs::FileExt;

    use super::*;

    #[test]
    fn descriptors_travel_with_the_bytes() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let file = memfd(c"plt-test").unwrap();
        file.write_all_at(b"shared", 0).unwrap();
        let (reader, mut writer) = std::io::pipe().unwrap();
        assert_eq!(
            send(&ours, b"hello", &[file.as_fd(), writer.as_fd()]).unwrap(),
            5
        );
        let mut buf = [0; 16];
        let mut fds = Vec::new();
        assert_eq!(receive(&theirs, &mut buf, &mut fds, true).unwrap(), 5);
        assert_eq!(&buf[..5], b"hello");
        assert_eq!(fds.len(), 2);
        let received = File::from(fds.remove(0));
        let mut text = [0; 6];
        received.read_exact_at(&mut text, 0).unwrap();
        assert_eq!(&text, b"shared");
        let mut pipe_end = File::from(fds.remove(0));
        pipe_end.write_all(b"x").unwrap();
        drop(pipe_end);
        writer.write_all(b"y").unwrap();
        drop(writer);
        let mut piped = String::new();
        let mut reader = reader;
        reader.read_to_string(&mut piped).unwrap();
        assert_eq!(piped, "xy");
    }

    #[test]
    fn non_blocking_receive_and_end_of_stream() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let mut buf = [0; 4];
        let mut fds = Vec::new();
        let error = receive(&theirs, &mut buf, &mut fds, false).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(send(&ours, b"ab", &[]).unwrap(), 2);
        assert_eq!(receive(&theirs, &mut buf, &mut fds, false).unwrap(), 2);
        drop(ours);
        assert_eq!(receive(&theirs, &mut buf, &mut fds, true).unwrap(), 0);
        assert!(fds.is_empty());
    }

    #[test]
    fn sending_to_a_closed_peer_fails_without_a_signal() {
        let (ours, theirs) = UnixStream::pair().unwrap();
        drop(theirs);
        let error = send(&ours, b"lost", &[]).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn too_many_descriptors_are_refused() {
        let (ours, _theirs) = UnixStream::pair().unwrap();
        let fds = vec![ours.as_fd(); MAX_FDS + 1];
        assert_eq!(
            send(&ours, b"x", &fds).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn ancillary_data_is_parsed_defensively() {
        let mut control = vec![0; HEADER_LEN + 8];
        control[..size_of::<usize>()].copy_from_slice(&(HEADER_LEN + 8).to_ne_bytes());
        control[size_of::<usize>()..size_of::<usize>() + 4]
            .copy_from_slice(&SOL_SOCKET.to_ne_bytes());
        control[size_of::<usize>() + 4..HEADER_LEN].copy_from_slice(&SCM_RIGHTS.to_ne_bytes());
        control[HEADER_LEN..HEADER_LEN + 4].copy_from_slice(&7_i32.to_ne_bytes());
        control[HEADER_LEN + 4..].copy_from_slice(&9_i32.to_ne_bytes());
        assert_eq!(received_fds(&control), [7, 9]);
        let lying = {
            let mut lying = control.clone();
            lying[..size_of::<usize>()].copy_from_slice(&1000_usize.to_ne_bytes());
            lying
        };
        assert_eq!(received_fds(&lying), []);
        let short = {
            let mut short = control.clone();
            short[..size_of::<usize>()].copy_from_slice(&2_usize.to_ne_bytes());
            short
        };
        assert_eq!(received_fds(&short), []);
        assert_eq!(received_fds(&control[..3]), []);
    }
}
