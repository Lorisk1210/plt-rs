//! LZ77 parsing: turning the input into literals and back-references.
//!
//! Both parsers mirror zlib: [`greedy`] (`deflate_fast`) takes the longest
//! match at each position, [`lazy`] (`deflate_slow`) first checks whether
//! the next position starts an even longer match.

use super::block::BlockSink;
use super::input::Input;
use super::level::LevelParams;
use super::matcher::{Match, Matcher};
use super::tables::MIN_MATCH;

/// Three-byte matches farther back than this usually cost more than three
/// literals (the distance's extra bits alone exceed 6 bits) and are dropped.
/// zlib uses 4096; on filtered image data 256 is clearly better and on text
/// it makes no difference.
const TOO_FAR: usize = 256;

/// Whether a match is worth emitting rather than literals.
fn worthwhile(found: &Match) -> bool {
    found.length > MIN_MATCH || found.distance <= TOO_FAR
}

/// Greedy parsing for the fast levels.
pub(crate) fn greedy<I: Input>(input: &mut I, params: &LevelParams, mut sink: BlockSink<'_, '_>) {
    let len = input.view().len();
    let mut matcher = Matcher::new(len);
    let mut position = 0;
    while position < len {
        input.reach(position);
        let data = input.view();
        let found = matcher
            .insert(data, position)
            .and_then(|chain| matcher.longest_match(data, position, chain, 0, params))
            .filter(worthwhile);
        let Some(Match { length, distance }) = found else {
            sink.literal(input);
            position += 1;
            continue;
        };
        // Short matches get all their positions hashed. Long ones only their
        // last, which is cheap and lets runs continue at distance 1.
        let skipped = if length <= params.lazy_length {
            position + 1
        } else {
            position + length - 1
        };
        matcher.insert_all(data, skipped..position + length);
        sink.matched(input, length, distance);
        position += length;
    }
    sink.finish(input);
}

/// Lazy parsing for the default and best levels.
pub(crate) fn lazy<I: Input>(input: &mut I, params: &LevelParams, mut sink: BlockSink<'_, '_>) {
    let len = input.view().len();
    let mut matcher = Matcher::new(len);
    // The match starting at `position - 1`, if one was found there.
    let mut previous: Option<Match> = None;
    // Whether the byte at `position - 1` still awaits a decision.
    let mut pending = false;
    let mut position = 0;
    while position < len {
        input.reach(position);
        let data = input.view();
        let chain = matcher.insert(data, position);
        let previous_length = previous.map_or(0, |m| m.length);
        let current = chain
            .filter(|_| previous_length < params.lazy_length)
            .and_then(|chain| matcher.longest_match(data, position, chain, previous_length, params))
            .filter(worthwhile);

        match previous {
            Some(Match { length, distance }) if current.is_none() => {
                // The match at `position - 1` is at least as good as anything
                // starting here: emit it and skip past it.
                let end = position - 1 + length;
                matcher.insert_all(data, position + 1..end);
                sink.matched(input, length, distance);
                position = end;
                previous = None;
                pending = false;
            }
            _ => {
                if pending {
                    sink.literal(input);
                }
                previous = current;
                pending = true;
                position += 1;
            }
        }
    }
    if pending {
        sink.literal(input);
    }
    sink.finish(input);
}
