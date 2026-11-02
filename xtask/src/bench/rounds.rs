//! Warm runs in interleaved rounds: every running implementation of a
//! workload makes one timed run per round, in an order rotated from round
//! to round, so that a change in the machine's load or clock weighs on
//! all of them alike, and the runs of a round form a pair. Rounds go on
//! until every implementation has run as often as the sampling asks, and
//! the slowest for as long: quick workloads get more rounds, so that the
//! medians of the quickest implementation settle, without slow ones
//! running for long.

use std::error::Error;

use super::session::{Outcome, Session};
use super::stats::{Samples, Sampling};

/// What the warm runs of one implementation gave.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Warm {
    /// The times of its runs, round by round, and the size of the
    /// document of its last run.
    Measured {
        /// The times.
        seconds: Samples,
        /// The document's size, in bytes.
        bytes: u64,
    },
    /// It raised this exception (pltrs (Python) lacking something).
    Unsupported(String),
}

/// Runs `sessions` once each to warm up, then in rounds as `sampling`
/// asks. A session that raises stops taking part; the others go on.
pub(super) fn interleave(
    sessions: &mut [&mut dyn Session],
    sampling: Sampling,
) -> Result<Vec<Warm>, Box<dyn Error>> {
    let count = sessions.len();
    let mut times: Vec<Vec<f64>> = vec![Vec::new(); count];
    let mut bytes = vec![0; count];
    let mut failed: Vec<Option<String>> = vec![None; count];
    let mut rounds = 0;
    loop {
        let warming_up = rounds == 0;
        let shift = rounds % count;
        for i in (shift..count).chain(0..shift) {
            if failed[i].is_some() {
                continue;
            }
            match sessions[i].run()? {
                Outcome::Done(run) if !warming_up => {
                    times[i].push(run.seconds);
                    bytes[i] = run.bytes;
                }
                Outcome::Done(_) => {}
                Outcome::Unsupported(why) => failed[i] = Some(why),
            }
        }
        rounds += 1;
        let live = (0..count).filter(|&i| failed[i].is_none());
        let most_time = live
            .map(|i| times[i].iter().sum::<f64>())
            .max_by(f64::total_cmp);
        let Some(most_time) = most_time else {
            break;
        };
        if !sampling.wants_more(rounds - 1, most_time) {
            break;
        }
    }
    (0..count)
        .map(|i| match failed[i].take() {
            Some(why) => Ok(Warm::Unsupported(why)),
            None => Ok(Warm::Measured {
                seconds: Samples::new(std::mem::take(&mut times[i]))
                    .ok_or("the sampling asked for no runs")?,
                bytes: bytes[i],
            }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::bench::implementation::Implementation;
    use crate::bench::session::Run;
    use crate::bench::workloads::Output;

    /// A session taking `seconds` per run, failing from run `fails_at` on.
    struct Fake {
        seconds: f64,
        runs: usize,
        fails_at: Option<usize>,
        log: std::rc::Rc<std::cell::RefCell<Vec<f64>>>,
    }

    impl Session for Fake {
        fn implementation(&self) -> Implementation {
            Implementation::Rust
        }

        fn run(&mut self) -> Result<Outcome<Run>, Box<dyn Error>> {
            self.runs += 1;
            self.log.borrow_mut().push(self.seconds);
            if self.fails_at.is_some_and(|at| self.runs >= at) {
                return Ok(Outcome::Unsupported("KeyError".to_owned()));
            }
            Ok(Outcome::Done(Run {
                seconds: self.seconds,
                bytes: 7,
            }))
        }

        fn save(&mut self, _: Output, _: &Path) -> Result<Outcome<()>, Box<dyn Error>> {
            Ok(Outcome::Done(()))
        }
    }

    #[test]
    fn rounds_rotate_and_pair_the_runs() {
        let log = std::rc::Rc::default();
        let fake = |seconds, fails_at| Fake {
            seconds,
            runs: 0,
            fails_at,
            log: std::rc::Rc::clone(&log),
        };
        let (mut a, mut b, mut c) = (fake(1.0, None), fake(2.0, None), fake(3.0, Some(2)));
        let sampling = Sampling {
            min_runs: 3,
            min_seconds: 0.0,
            max_runs: 10,
        };
        let warm = interleave(&mut [&mut a, &mut b, &mut c], sampling).expect("runs");
        assert_eq!(
            warm[..2],
            [
                Warm::Measured {
                    seconds: Samples::new(vec![1.0; 3]).expect("times"),
                    bytes: 7
                },
                Warm::Measured {
                    seconds: Samples::new(vec![2.0; 3]).expect("times"),
                    bytes: 7
                },
            ]
        );
        assert_eq!(warm[2], Warm::Unsupported("KeyError".to_owned()));
        // The warm-up round, then rounds starting with b, c (which failed
        // in the round before) and a.
        assert_eq!(
            *log.borrow(),
            [1.0, 2.0, 3.0, 2.0, 3.0, 1.0, 1.0, 2.0, 1.0, 2.0]
        );
    }

    #[test]
    fn rounds_last_until_the_slowest_has_run_long_enough() {
        let log = std::rc::Rc::default();
        let fake = |seconds| Fake {
            seconds,
            runs: 0,
            fails_at: None,
            log: std::rc::Rc::clone(&log),
        };
        let (mut quick, mut slow) = (fake(1.0), fake(2.0));
        let sampling = Sampling {
            min_runs: 1,
            min_seconds: 5.0,
            max_runs: 10,
        };
        let warm = interleave(&mut [&mut quick, &mut slow], sampling).expect("runs");
        let rounds = |warm: &Warm| match warm {
            Warm::Measured { seconds, .. } => seconds.runs().len(),
            Warm::Unsupported(_) => 0,
        };
        assert_eq!((rounds(&warm[0]), rounds(&warm[1])), (3, 3));
    }
}
