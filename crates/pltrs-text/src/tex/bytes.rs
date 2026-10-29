//! Reading TeX's binary files (DVI, VF, TFM) as matplotlib's `dviread`
//! reads them: bytes and big-endian numbers, with short reads at the end
//! of the file.

/// A binary file and the position reached in it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bytes<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bytes<'a> {
    /// Reading `data` from its start.
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// How many bytes have been read.
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    /// The next byte (`file.read(1)[0]`), `None` at the end.
    pub(crate) fn byte(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    /// The next `n` bytes, or as many as are left (`file.read(n)`).
    pub(crate) fn read(&mut self, n: usize) -> &'a [u8] {
        let end = self.pos.saturating_add(n).min(self.data.len());
        let bytes = &self.data[self.pos..end];
        self.pos = end;
        bytes
    }

    /// `_read_arg`: a big-endian number of up to `n` bytes (fewer at the
    /// end of the file, none giving 0).
    pub(crate) fn arg(&mut self, n: usize, signed: bool) -> i128 {
        let bytes = self.read(n);
        let mut value: i128 = 0;
        for &byte in bytes {
            value = (value << 8) | i128::from(byte);
        }
        if signed && bytes.first().is_some_and(|&b| b & 0x80 != 0) {
            value -= 1 << (8 * bytes.len());
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_stop_at_the_end() {
        let mut file = Bytes::new(&[0xff, 0xfe, 0x01]);
        assert_eq!(file.arg(2, true), -2);
        assert_eq!(file.arg(4, false), 1, "a short read");
        assert_eq!(file.pos(), 3);
        assert_eq!(file.byte(), None);
        assert_eq!(file.read(usize::MAX), &[] as &[u8]);
        assert_eq!(file.arg(1, true), 0, "nothing left reads as 0");
    }
}
