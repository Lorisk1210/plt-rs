//! Randomised robustness tests: deterministic pseudo-random inputs reach
//! every public entry point that takes user input — string and byte
//! parsers, plotting methods with their options and data, rendering to
//! every output format and interaction — and nothing may panic or hang.
//! Errors are fine; they are how invalid input is meant to be reported.
//!
//! [`harness`] runs each target for a small budget by default, so that
//! `cargo test` stays quick; `PLTRS_FUZZ_SCALE=100` makes a long run, and a
//! failure names the seed that reproduces it.

#[path = "../support/scratch.rs"]
mod scratch;

mod config;
mod dates;
mod fonts;
mod harness;
mod images;
mod interaction;
mod mathtext;
mod memory;
mod plotting;
mod rc;
mod rendering;
mod support;
mod text_parsers;
mod units;
