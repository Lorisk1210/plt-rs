//! Agg's forward-adjusted `dda2_line_interpolator`: integer steps from one
//! value to another, distributing the remainder evenly, as the image span
//! interpolators walk a span's source coordinates.

/// Walks from `y1` to `y2` in `count` integer steps (Agg's
/// `dda2_line_interpolator` with forward adjustment).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Dda2 {
    cnt: i32,
    lft: i32,
    rem: i32,
    modulo: i32,
    /// The current value.
    pub(crate) y: i32,
}

impl Dda2 {
    /// Starts at `y1`, reaching `y2` after `count` steps. As in Agg, a
    /// non-positive `count` divides by one but still adjusts the
    /// remainder by `count`.
    pub(crate) fn new(y1: i32, y2: i32, count: i32) -> Self {
        let cnt = count.max(1);
        let delta = y2.wrapping_sub(y1);
        let mut lft = delta.wrapping_div(cnt);
        let mut rem = delta.wrapping_rem(cnt);
        let mut modulo = rem;
        if modulo <= 0 {
            modulo += count;
            rem += count;
            lft = lft.wrapping_sub(1);
        }
        modulo -= count;
        Self {
            cnt,
            lft,
            rem,
            modulo,
            y: y1,
        }
    }

    /// Advances one step.
    pub(crate) fn step(&mut self) {
        self.modulo += self.rem;
        self.y = self.y.wrapping_add(self.lft);
        if self.modulo > 0 {
            self.modulo -= self.cnt;
            self.y = self.y.wrapping_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(y1: i32, y2: i32, count: i32) -> Vec<i32> {
        let mut d = Dda2::new(y1, y2, count);
        let mut ys = vec![d.y];
        for _ in 0..count {
            d.step();
            ys.push(d.y);
        }
        ys
    }

    #[test]
    fn hits_both_end_points() {
        let ys = walk(10, 1000, 7);
        assert_eq!(ys.first(), Some(&10));
        assert_eq!(ys.last(), Some(&1000));
        assert!(ys.windows(2).all(|w| w[1] >= w[0]));
    }

    #[test]
    fn distributes_evenly() {
        let ys = walk(0, 10, 4);
        assert_eq!(ys.first(), Some(&0));
        assert_eq!(ys.last(), Some(&10));
        assert!(ys.windows(2).all(|w| w[1] - w[0] >= 2 && w[1] - w[0] <= 3));
    }
}
