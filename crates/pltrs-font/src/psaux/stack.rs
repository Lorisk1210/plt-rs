//! The operand stack of FreeType's Adobe charstring engine (`psstack.c`):
//! integers and 16.16 fixed-point numbers, 48 deep (a CFF2 font's
//! `maxstack` deep).

use super::error::CharstringError;

/// A number on the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Number {
    /// An integer (charstring numbers in Type 1 mode).
    Int(i32),
    /// A 16.16 fixed-point number (results of arithmetic).
    Fixed(i32),
}

/// `cf2_intToFixed`: shifts in 32 bits, wrapping.
pub(super) const fn int_to_fixed(value: i32) -> i32 {
    ((value as u32) << 16) as i32
}

impl Number {
    const fn to_fixed(self) -> i32 {
        match self {
            Self::Int(value) => int_to_fixed(value),
            Self::Fixed(value) => value,
        }
    }
}

const fn error(why: &'static str) -> CharstringError {
    CharstringError::Malformed(why)
}

/// The operand stack.
#[derive(Clone, Debug)]
pub(super) struct Stack {
    values: Vec<Number>,
    size: usize,
}

impl Stack {
    /// An empty stack of `size` numbers.
    pub(super) fn new(size: usize) -> Self {
        Self {
            values: Vec::with_capacity(size),
            size,
        }
    }

    pub(super) fn count(&self) -> usize {
        self.values.len()
    }

    pub(super) fn clear(&mut self) {
        self.values.clear();
    }

    fn push(&mut self, number: Number) -> Result<(), CharstringError> {
        if self.values.len() >= self.size {
            return Err(error("stack overflow"));
        }
        self.values.push(number);
        Ok(())
    }

    pub(super) fn push_int(&mut self, value: i32) -> Result<(), CharstringError> {
        self.push(Number::Int(value))
    }

    pub(super) fn push_fixed(&mut self, value: i32) -> Result<(), CharstringError> {
        self.push(Number::Fixed(value))
    }

    /// Pops an integer; a fixed-point number on top is an error.
    pub(super) fn pop_int(&mut self) -> Result<i32, CharstringError> {
        match self.values.last() {
            None => Err(error("stack underflow")),
            Some(Number::Fixed(_)) => Err(error("integer expected")),
            Some(&Number::Int(value)) => {
                self.values.pop();
                Ok(value)
            }
        }
    }

    /// Pops a number as fixed point.
    pub(super) fn pop_fixed(&mut self) -> Result<i32, CharstringError> {
        self.values
            .pop()
            .map(Number::to_fixed)
            .ok_or(error("stack underflow"))
    }

    /// The number at `index` from the bottom, as fixed point.
    pub(super) fn get(&self, index: usize) -> Result<i32, CharstringError> {
        self.values
            .get(index)
            .map(|&number| number.to_fixed())
            .ok_or(error("stack index out of range"))
    }

    /// Replaces the number at `index` from the bottom; one past the top is
    /// accepted and has no effect, as in FreeType.
    pub(super) fn set(&mut self, index: usize, value: i32) -> Result<(), CharstringError> {
        let len = self.values.len();
        match self.values.get_mut(index) {
            Some(slot) => *slot = Number::Fixed(value),
            None if index == len => {}
            None => return Err(error("stack index out of range")),
        }
        Ok(())
    }

    /// Drops the numbers above the first `count` (`cf2_stack_pop`).
    pub(super) fn truncate(&mut self, count: usize) {
        self.values.truncate(count);
    }

    /// `cf2_stack_roll`: rotates the top `count` numbers by `shift`.
    pub(super) fn roll(&mut self, count: i32, shift: i32) -> Result<(), CharstringError> {
        if count < 2 {
            return Ok(());
        }
        let count = count as usize;
        if count > self.values.len() {
            return Err(error("stack index out of range"));
        }
        let modulus = count as i32;
        let shift = if shift < 0 {
            -((-i64::from(shift)) % i64::from(modulus)) as i32
        } else {
            shift % modulus
        };
        if shift == 0 {
            return Ok(());
        }
        // FreeType rolls the bottom `count` entries of its buffer.
        let window = &mut self.values[..count];
        if shift > 0 {
            window.rotate_right(shift as usize);
        } else {
            window.rotate_left(shift.unsigned_abs() as usize);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psaux::STACK_SIZE;

    #[test]
    fn typed_numbers() {
        let mut stack = Stack::new(STACK_SIZE);
        stack.push_int(3).unwrap();
        stack.push_fixed(0x18000).unwrap();
        assert!(stack.pop_int().is_err());
        assert_eq!(stack.pop_fixed(), Ok(0x18000));
        assert_eq!(stack.get(0), Ok(3 << 16));
        assert_eq!(stack.pop_int(), Ok(3));
        assert!(stack.pop_int().is_err() && stack.pop_fixed().is_err());
        assert_eq!(int_to_fixed(0x8000), i32::MIN);
    }

    #[test]
    fn limits_set_and_roll() {
        let mut stack = Stack::new(STACK_SIZE);
        for i in 0..48 {
            stack.push_int(i).unwrap();
        }
        assert!(stack.push_int(0).is_err());
        assert!(stack.set(48, 1).is_ok() && stack.set(49, 1).is_err());
        stack.clear();
        for i in 0..4 {
            stack.push_int(i).unwrap();
        }
        stack.roll(4, 1).unwrap();
        let values: Vec<i32> = (0..4).map(|i| stack.get(i).unwrap() >> 16).collect();
        assert_eq!(values, [3, 0, 1, 2]);
        stack.roll(4, -5).unwrap();
        let values: Vec<i32> = (0..4).map(|i| stack.get(i).unwrap() >> 16).collect();
        assert_eq!(values, [0, 1, 2, 3]);
        assert!(stack.roll(5, 1).is_err());
        stack.clear();
        assert_eq!(stack.count(), 0);
    }
}
