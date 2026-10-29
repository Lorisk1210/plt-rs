//! A charstring being read (`CF2_Buffer`): reading past its end yields
//! zeros, as in FreeType.

/// A position in a charstring.
#[derive(Clone, Copy, Debug)]
pub(super) struct Buffer<'a> {
    code: &'a [u8],
    at: usize,
}

impl<'a> Buffer<'a> {
    pub(super) const fn new(code: &'a [u8]) -> Self {
        Self { code, at: 0 }
    }

    pub(super) const fn is_end(&self) -> bool {
        self.at >= self.code.len()
    }

    /// The next byte, 0 at the end.
    pub(super) fn read(&mut self) -> u8 {
        match self.code.get(self.at) {
            Some(&byte) => {
                self.at += 1;
                byte
            }
            None => 0,
        }
    }

    /// Goes back to the start.
    pub(super) fn rewind(&mut self) {
        self.at = 0;
    }

    #[cfg(test)]
    pub(super) const fn position(&self) -> usize {
        self.at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_zeros_past_the_end() {
        let mut buffer = Buffer::new(&[7, 8]);
        assert_eq!((buffer.read(), buffer.read(), buffer.read()), (7, 8, 0));
        assert!(buffer.is_end());
        buffer.rewind();
        assert_eq!((buffer.position(), buffer.read()), (0, 7));
    }
}
