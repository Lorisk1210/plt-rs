//! The fuzzing harness of pltrs' robustness tests (`crates/pltrs/tests/fuzz`
//! and the `fuzz_tests` unit tests of `pltrs`, `pltrs-text`,
//! `pltrs-platform` and `pltrs-ui`): deterministic inputs from
//! [`SplitMix64`], panics caught and recorded with the seed that reproduces
//! them, and a watchdog for inputs that hang.
//!
//! Only the feature `fuzzing` builds it, which those crates turn on for
//! their tests alone: it lives here, in the crate below them all, so that
//! each crate's package carries what its tests need. It is a test tool of
//! the workspace, not part of pltrs' stable interface.
//!
//! Every target runs [`fuzz`] with a generator and a body. Iteration `i`
//! of target `name` draws its input from a generator seeded with a mix of
//! the base seed, the name and `i`, so a failure's seed alone rebuilds its
//! input. The environment tunes long runs:
//!
//! * `PLTRS_FUZZ_SCALE` multiplies every target's iteration budget (default
//!   1);
//! * `PLTRS_FUZZ_SEED` changes the base seed, to explore other inputs;
//! * `PLTRS_FUZZ_TIMEOUT` is the number of seconds one input may run before
//!   the watchdog reports it as a hang and ends the process (default 120);
//! * `PLTRS_FUZZ_TRACE` set to anything prints every seed and input before
//!   it runs, so that the last line names the input when the process dies
//!   (out of memory, say);
//! * `PLTRS_FUZZ_REPLAY` set to a seed (decimal or `0x` hexadecimal) runs
//!   only the input of that seed, printing it first;
//! * `PLTRS_FUZZ_STATS` set to anything prints each target's run time and
//!   the seed of its slowest input.

use std::cell::RefCell;
use std::fmt::{Debug, Write as _};
use std::panic::{self, AssertUnwindSafe, PanicHookInfo};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

use crate::rng::SplitMix64;

/// The base seed when `PLTRS_FUZZ_SEED` is not set.
const DEFAULT_SEED: u64 = 0x504C_5446_555A_5A31;

/// How many distinct failures a report lists in full.
const REPORTED: usize = 12;

/// A source of pseudo-random inputs: numbers biased towards the values
/// that break numeric code, strings built from a grammar's pieces and
/// mutations of valid examples.
#[derive(Debug)]
pub struct Gen(SplitMix64);

impl Gen {
    /// A generator seeded with `seed`.
    pub fn new(seed: u64) -> Self {
        Self(SplitMix64::new(seed))
    }

    /// A number in `0..n` (`0` when `n` is zero).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { self.0.below(n) }
    }

    /// A uniform number in `[low, high)`.
    pub fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.0.next_f64()
    }

    /// `true` with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.0.next_f64() < p
    }

    /// One of `items`, which must not be empty.
    pub fn pick<T: Clone>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())].clone()
    }

    /// A length up to `max`, small lengths being the most likely.
    pub fn len(&mut self, max: usize) -> usize {
        let cap = match self.below(4) {
            0 => 2,
            1 => 8,
            _ => max,
        };
        self.below(cap.min(max) + 1)
    }

    /// A number: ordinary most of the time, otherwise one of the values
    /// that break numeric code (NaN, infinities, signed zeros,
    /// subnormals, the extremes of `f64`).
    pub fn number(&mut self) -> f64 {
        match self.below(10) {
            0 => self.pick(&SPECIAL_NUMBERS),
            1 => self.between(-1.0, 1.0) * 10f64.powi(self.below(600) as i32 - 300),
            2 => self.below(20) as f64,
            _ => self.between(-10.0, 10.0),
        }
    }

    /// A number in `[low, high]` most of the time, otherwise any
    /// [`number`](Self::number).
    pub fn number_in(&mut self, low: f64, high: f64) -> f64 {
        if self.chance(0.8) {
            self.between(low, high)
        } else {
            self.number()
        }
    }

    /// `len` numbers: half the time an evenly spaced series with the odd
    /// [`number`](Self::number) in it, otherwise any numbers.
    pub fn numbers(&mut self, len: usize) -> Vec<f64> {
        let shape = self.below(4);
        let start = self.number_in(-5.0, 5.0);
        (0..len)
            .map(|i| match shape {
                // A smooth series with the occasional odd value.
                0 | 1 if !self.chance(0.1) => start + i as f64,
                _ => self.number(),
            })
            .collect()
    }

    /// The text of a number as a user might type it, valid or not.
    pub fn number_text(&mut self) -> String {
        match self.below(4) {
            0 => self.pick(&NUMBER_TEXTS).to_owned(),
            1 => format!("{}", self.number()),
            2 => format!("{:e}", self.number()),
            _ => format!("{}", self.below(100)),
        }
    }

    /// A string of up to `max` pieces drawn from `pieces`, with random
    /// characters (control characters, combining marks, astral
    /// codepoints) mixed in.
    pub fn soup(&mut self, pieces: &[&str], max: usize) -> String {
        let len = self.len(max);
        let mut out = String::new();
        for _ in 0..len {
            match self.below(10) {
                0 => out.push(self.character()),
                1 => out.push_str(&self.number_text()),
                _ => out.push_str(self.pick(pieces)),
            }
        }
        out
    }

    /// A random character: ASCII mostly, sometimes anything.
    pub fn character(&mut self) -> char {
        match self.below(6) {
            0 => self.pick(&ODD_CHARS),
            1 => char::from_u32(self.below(0x11_0000) as u32).unwrap_or('\u{FFFD}'),
            _ => char::from(b' ' + self.below(95) as u8),
        }
    }

    /// `text` after a few random edits: pieces inserted, characters
    /// deleted or replaced, a span repeated, numbers made extreme.
    pub fn mutate(&mut self, text: &str, pieces: &[&str]) -> String {
        let mut chars: Vec<char> = text.chars().collect();
        for _ in 0..=self.below(4) {
            let at = self.below(chars.len() + 1);
            match self.below(6) {
                0 if at < chars.len() => {
                    chars.remove(at);
                }
                1 if at < chars.len() => chars[at] = self.character(),
                2 => {
                    let end = (at + 1 + self.below(8)).min(chars.len());
                    let span: Vec<char> = chars[at.min(end)..end].to_vec();
                    for _ in 0..self.below(4) {
                        chars.splice(at..at, span.iter().copied());
                    }
                }
                3 => {
                    let number = self.number_text();
                    chars.splice(at..at, number.chars());
                }
                _ => {
                    let piece = self.pick(pieces);
                    chars.splice(at..at, piece.chars());
                }
            }
        }
        chars.into_iter().collect()
    }

    /// A string for a parser: one of the valid `examples`, a mutation of
    /// one, or a soup of `pieces`.
    pub fn text(&mut self, examples: &[&str], pieces: &[&str]) -> String {
        match self.below(5) {
            0 => self.pick(examples).to_owned(),
            1 => self.soup(pieces, 12),
            _ => {
                let example = self.pick(examples);
                self.mutate(example, pieces)
            }
        }
    }

    /// `bytes` after random corruption: bits flipped, bytes overwritten
    /// with extreme values, spans repeated or deleted, the end cut off.
    pub fn corrupt(&mut self, bytes: &[u8]) -> Corruption {
        let mut edits = Vec::new();
        let len = bytes.len().max(1);
        for _ in 0..=self.len(5) {
            let at = self.biased_offset(len);
            edits.push(match self.below(6) {
                0 => Edit::Flip(at, 1 << self.below(8)),
                1 => Edit::Fill(
                    at,
                    1 + self.below(4),
                    self.pick(&[0, 0xFF, 0x7F, 0x80, 0x01]),
                ),
                2 => Edit::Fill(at, 1, self.below(256) as u8),
                3 => Edit::Repeat(at, 1 + self.below(64), 1 + self.below(4)),
                4 => Edit::Delete(at, 1 + self.below(64)),
                _ => Edit::Flip(at, self.below(256) as u8),
            });
        }
        let truncate = self.chance(0.2).then(|| self.biased_offset(len));
        Corruption { edits, truncate }
    }

    /// An offset into a buffer of `len` bytes, the headers near the start
    /// being likelier than the rest.
    fn biased_offset(&mut self, len: usize) -> usize {
        if self.chance(0.5) {
            self.below(len.min(512))
        } else {
            self.below(len)
        }
    }
}

/// The edits of a [`Corruption`].
#[derive(Clone, Debug)]
pub enum Edit {
    /// XOR the byte at an offset with a mask.
    Flip(usize, u8),
    /// Overwrite the bytes at an offset: `(offset, length, value)`; runs
    /// of `0xFF` make sizes and counts huge.
    Fill(usize, usize, u8),
    /// Insert copies of the span at an offset: `(offset, length, times)`.
    Repeat(usize, usize, usize),
    /// Remove the span at an offset: `(offset, length)`.
    Delete(usize, usize),
}

/// A recorded corruption of a byte buffer, small enough to print and
/// replay.
#[derive(Clone, Debug)]
pub struct Corruption {
    /// The edits, applied in order.
    pub edits: Vec<Edit>,
    /// Where the corrupted buffer is cut off, if anywhere.
    pub truncate: Option<usize>,
}

impl Corruption {
    /// Moves every edit `offset` bytes further, so that a corruption of a
    /// part of a buffer applies to the whole (the cut is dropped).
    pub fn shift(&mut self, offset: usize) {
        for edit in &mut self.edits {
            match edit {
                Edit::Flip(at, _)
                | Edit::Fill(at, ..)
                | Edit::Repeat(at, ..)
                | Edit::Delete(at, _) => {
                    *at += offset;
                }
            }
        }
        self.truncate = None;
    }

    /// `bytes` with the corruption applied.
    pub fn apply(&self, bytes: &[u8]) -> Vec<u8> {
        let mut out = bytes.to_vec();
        for edit in &self.edits {
            match *edit {
                Edit::Flip(at, mask) => {
                    if let Some(byte) = out.get_mut(at) {
                        *byte ^= mask;
                    }
                }
                Edit::Fill(at, len, value) => {
                    let at = at.min(out.len());
                    let end = (at + len).min(out.len());
                    out[at..end].fill(value);
                }
                Edit::Repeat(at, len, times) => {
                    let at = at.min(out.len());
                    let span = out[at..(at + len).min(out.len())].to_vec();
                    for _ in 0..times {
                        out.splice(at..at, span.iter().copied());
                    }
                }
                Edit::Delete(at, len) => {
                    let at = at.min(out.len());
                    out.drain(at..(at + len).min(out.len()));
                }
            }
        }
        if let Some(len) = self.truncate {
            out.truncate(len);
        }
        out
    }
}

/// The values numeric code trips over.
const SPECIAL_NUMBERS: [f64; 16] = [
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
    0.0,
    -0.0,
    5e-324,
    -5e-324,
    2.2e-308,
    f64::MAX,
    f64::MIN,
    1e300,
    -1e300,
    f64::EPSILON,
    1e-12,
    1e12,
    -1.0,
];

/// Number texts at the edges of what parsers accept.
const NUMBER_TEXTS: [&str; 24] = [
    "nan",
    "-nan",
    "inf",
    "-inf",
    "Infinity",
    "1e308",
    "1e309",
    "-1e309",
    "1e-320",
    "-0",
    "0",
    "99999999999999999999999",
    "-99999999999999999999999",
    "4294967296",
    "18446744073709551616",
    "-1",
    "0x10",
    "1_000",
    ".",
    "-",
    "+",
    "1e",
    "1.5.5",
    "",
];

/// Characters that tend to confuse text handling.
const ODD_CHARS: [char; 16] = [
    '\0',
    '\t',
    '\n',
    '\r',
    '\u{7F}',
    '\u{A0}',
    '\u{301}',
    '\u{200B}',
    '\u{200D}',
    '\u{2028}',
    '\u{FEFF}',
    '\u{FFFD}',
    '\u{FFFF}',
    '\u{1F600}',
    '\u{10FFFF}',
    '\u{E000}',
];

/// A failure: the panic or the input that ran too long.
#[derive(Debug)]
struct Failure {
    /// The seed of the input's generator.
    seed: u64,
    /// The input, as `{:?}` prints it.
    input: String,
    /// The panic message.
    message: String,
    /// Where the panic was raised.
    location: String,
}

thread_local! {
    /// Whether this thread is running a fuzz input; panics raised then are
    /// recorded quietly instead of printed.
    static FUZZING: RefCell<bool> = const { RefCell::new(false) };
    /// The location of the last quiet panic on this thread.
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Installs a panic hook that records the location of panics raised by
/// fuzz inputs instead of printing them (all other panics print as usual).
fn install_hook() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let default = panic::take_hook();
        panic::set_hook(Box::new(move |info: &PanicHookInfo<'_>| {
            if FUZZING.with(|f| *f.borrow()) {
                let location = info
                    .location()
                    .map_or_else(|| "unknown".to_owned(), ToString::to_string);
                let location = if location.starts_with("/rustc/") {
                    caller_in_workspace().unwrap_or(location)
                } else {
                    location
                };
                LAST_PANIC.with(|last| *last.borrow_mut() = Some(location));
            } else {
                default(info);
            }
        }));
    });
}

/// For a panic raised in the standard library (an allocation, say), the
/// three innermost frames of the workspace that led to it, from a
/// backtrace.
fn caller_in_workspace() -> Option<String> {
    let backtrace = std::backtrace::Backtrace::force_capture().to_string();
    // Backtraces print workspace paths relative to the working directory
    // and the standard library's under `/rustc/`.
    let frames: Vec<&str> = backtrace
        .lines()
        .filter_map(|line| line.trim().strip_prefix("at "))
        .filter(|at| !at.starts_with("/rustc/") && !at.contains("tests/fuzz/"))
        .take(3)
        .collect();
    (!frames.is_empty()).then(|| frames.join(" < "))
}

/// A number (decimal or `0x` hexadecimal) from the environment variable
/// `name`.
fn env_number(name: &str) -> Option<u64> {
    let value = std::env::var(name).ok()?;
    let value = value.trim();
    match value.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16).ok(),
        None => value.parse().ok(),
    }
}

/// The seed of iteration `i` of target `name`.
fn iteration_seed(base: u64, name: &str, i: usize) -> u64 {
    let mut mix = SplitMix64::new(base);
    let name_hash = name.bytes().fold(mix.next_u64(), |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
    });
    SplitMix64::new(name_hash ^ i as u64).next_u64()
}

/// The message of a panic payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_owned())
}

/// What the watchdog watches: the seed and input running and since when.
type Running = Arc<Mutex<Option<(u64, String, Instant)>>>;

/// Runs a watchdog over `running` until `done` hangs up: an input running
/// longer than `limit` is reported as a hang and the process ends, since
/// a thread stuck in a loop cannot be stopped.
#[allow(
    clippy::print_stderr,
    reason = "the watchdog's report is the only output of a hanging test"
)]
fn watch(name: &str, running: &Running, done: &Receiver<()>, limit: Duration) {
    while let Err(RecvTimeoutError::Timeout) = done.recv_timeout(Duration::from_millis(200)) {
        let guard = running
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((seed, input, since)) = guard.as_ref()
            && since.elapsed() > limit
        {
            eprintln!(
                "fuzz target `{name}` hangs: seed {seed:#018x} has run for more than {limit:?}\n\
                 input: {input}"
            );
            std::process::exit(101);
        }
    }
}

/// Prints the seed and input about to run.
#[allow(
    clippy::print_stderr,
    reason = "tracing names the input that kills the process"
)]
fn trace(name: &str, seed: u64, input: &str) {
    eprintln!("fuzz {name} {seed:#018x}: {input}");
}

/// Prints how long target `name` took and its slowest input.
#[allow(
    clippy::print_stderr,
    reason = "the statistics are asked for to tune the budgets"
)]
fn print_stats(name: &str, total: Duration, slowest: &(Duration, u64)) {
    eprintln!(
        "fuzz {name}: {total:.2?} in all, slowest seed {:#018x} took {:.2?}",
        slowest.1, slowest.0
    );
}

/// Runs target `name` for `budget` iterations (scaled by
/// `PLTRS_FUZZ_SCALE`): each draws an input with `generate` and runs `run`
/// on it. Fails listing every distinct panic location with the seed and
/// the shortest input that reached it.
pub fn fuzz<I: Debug>(
    name: &str,
    budget: usize,
    generate: impl Fn(&mut Gen) -> I,
    run: impl Fn(&I),
) {
    install_hook();
    let base = env_number("PLTRS_FUZZ_SEED").unwrap_or(DEFAULT_SEED);
    let scale = env_number("PLTRS_FUZZ_SCALE").map_or(1, |scale| scale as usize);
    let limit = Duration::from_secs(env_number("PLTRS_FUZZ_TIMEOUT").unwrap_or(120));
    let tracing = std::env::var_os("PLTRS_FUZZ_TRACE").is_some();
    let replay = env_number("PLTRS_FUZZ_REPLAY");
    let seeds: Vec<u64> = match replay {
        Some(seed) => vec![seed],
        None => (0..budget.saturating_mul(scale))
            .map(|i| iteration_seed(base, name, i))
            .collect(),
    };
    let running: Running = Arc::new(Mutex::new(None));
    let (finished, done) = mpsc::channel::<()>();
    let mut failures: Vec<Failure> = Vec::new();
    let mut panics = 0;
    let started = Instant::now();
    let mut slowest = (Duration::ZERO, 0);
    std::thread::scope(|scope| {
        let watched = Arc::clone(&running);
        scope.spawn(move || watch(name, &watched, &done, limit));
        for seed in seeds {
            let input = generate(&mut Gen::new(seed));
            let shown = format!("{input:?}");
            if tracing || replay.is_some() {
                trace(name, seed, &shown);
            }
            *running
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                Some((seed, shown.clone(), Instant::now()));
            FUZZING.with(|f| *f.borrow_mut() = true);
            let begun = Instant::now();
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| run(&input)));
            slowest = slowest.max((begun.elapsed(), seed));
            FUZZING.with(|f| *f.borrow_mut() = false);
            *running
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
            let Err(payload) = outcome else { continue };
            panics += 1;
            let failure = Failure {
                seed,
                input: shown,
                message: panic_message(payload.as_ref()),
                location: LAST_PANIC
                    .with(|last| last.borrow_mut().take())
                    .unwrap_or_default(),
            };
            match failures.iter_mut().find(|f| f.location == failure.location) {
                Some(seen) if failure.input.len() < seen.input.len() => *seen = failure,
                Some(_) => {}
                None => failures.push(failure),
            }
        }
        drop(finished);
    });
    if std::env::var_os("PLTRS_FUZZ_STATS").is_some() {
        print_stats(name, started.elapsed(), &slowest);
    }
    assert!(failures.is_empty(), "{}", report(name, panics, &failures));
}

/// The failure report of target `name`.
fn report(name: &str, panics: usize, failures: &[Failure]) -> String {
    let mut out = format!(
        "fuzz target `{name}`: {panics} panicking inputs at {} locations\n",
        failures.len()
    );
    for failure in failures.iter().take(REPORTED) {
        let input: String = failure.input.chars().take(600).collect();
        let _ = write!(
            out,
            "\n{} — {}\n  seed {:#018x}\n  input {input}\n",
            failure.location, failure.message, failure.seed
        );
    }
    out
}
