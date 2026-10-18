//! Requests and events of the selection protocol (ICCCM section 2): owning
//! the clipboard and answering requests for its contents.

use super::{Request, request};
use crate::x11::wire::WireWrite;

const SET_SELECTION_OWNER: u8 = 22;
/// Event code of `SelectionNotify`.
const SELECTION_NOTIFY: u8 = 31;

/// `SetSelectionOwner`: makes `owner` the owner of `selection` from `time`.
pub(crate) fn set_selection_owner(owner: u32, selection: u32, time: u32) -> Request {
    request(SET_SELECTION_OWNER, 0, |b| {
        b.put_u32(owner);
        b.put_u32(selection);
        b.put_u32(time);
    })
}

/// A `SelectionNotify` event telling `requestor` that `property` holds the
/// selection converted to `target` (`property` 0: it could not be), for
/// [`super::send_event`].
pub(crate) fn selection_notify(
    time: u32,
    requestor: u32,
    selection: u32,
    target: u32,
    property: u32,
) -> [u8; 32] {
    let mut event = Vec::with_capacity(32);
    event.put_u8(SELECTION_NOTIFY);
    event.put_zeros(3);
    for value in [time, requestor, selection, target, property] {
        event.put_u32(value);
    }
    event.put_zeros(8);
    event.try_into().expect("a SelectionNotify is 32 bytes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifications_are_laid_out_like_the_protocol() {
        let event = selection_notify(7, 0x10, 0x20, 0x30, 0x40);
        assert_eq!(event[0], SELECTION_NOTIFY);
        assert_eq!(&event[4..8], &7u32.to_le_bytes());
        assert_eq!(&event[20..24], &0x40u32.to_le_bytes());
        assert!(event[24..].iter().all(|&b| b == 0));
        assert_eq!(set_selection_owner(1, 2, 3).encoded().body().len(), 12);
    }
}
