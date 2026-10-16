//! Contextual and chaining contextual subtables (`GSUB` 5/6, `GPOS` 7/8),
//! formats 1 (glyphs), 2 (classes) and 3 (coverages).

use crate::error::Result;
use crate::layout::apply::{Applier, SequenceLookup};
use crate::layout::buffer::GlyphInfo;
use crate::layout::common::{ClassDef, Coverage};
use crate::reader::{Array, FromBytes, Reader, tail};
use crate::types::{GlyphId, Tag};

impl FromBytes for SequenceLookup {
    const SIZE: usize = 4;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            sequence_index: u16::from_bytes(&bytes[0..2]),
            lookup_index: u16::from_bytes(&bytes[2..4]),
        }
    }
}

/// How the values of a rule are compared with glyphs.
#[derive(Clone, Copy)]
enum Match<'a> {
    Glyphs,
    Classes(ClassDef<'a>),
    Coverages(&'a [u8]),
}

impl Match<'_> {
    fn matches(&self, value: u16, info: &GlyphInfo, tag: Tag) -> bool {
        match self {
            Self::Glyphs => info.glyph == GlyphId(value),
            Self::Classes(classes) => classes.class(info.glyph) == value,
            Self::Coverages(parent) => {
                Coverage::at(parent, value, tag).is_ok_and(|coverage| coverage.contains(info.glyph))
            }
        }
    }
}

/// One rule: backtrack (closest first), input after the first glyph, and
/// lookahead sequences plus the lookups to apply.
struct Rule<'a> {
    backtrack: Array<'a, u16>,
    input: Array<'a, u16>,
    lookahead: Array<'a, u16>,
    lookups: Array<'a, SequenceLookup>,
}

/// Tries `rule`; on success applies its lookups and returns true.
fn apply_rule(
    ctx: &mut Applier<'_, '_>,
    rule: &Rule<'_>,
    matchers: [Match<'_>; 3],
    tag: Tag,
) -> bool {
    let [backtrack_match, input_match, lookahead_match] = matchers;
    let input = |i: usize, info: &GlyphInfo| {
        rule.input
            .get(i - 1)
            .is_some_and(|v| input_match.matches(v, info, tag))
    };
    let Some(positions) = ctx.match_input(rule.input.len(), &input) else {
        return false;
    };
    let backtrack = |i: usize, info: &GlyphInfo| {
        rule.backtrack
            .get(i)
            .is_some_and(|v| backtrack_match.matches(v, info, tag))
    };
    let lookahead = |i: usize, info: &GlyphInfo| {
        rule.lookahead
            .get(i)
            .is_some_and(|v| lookahead_match.matches(v, info, tag))
    };
    let last = *positions.last().unwrap_or(&ctx.buffer.idx);
    if !ctx.match_backtrack(ctx.buffer.idx, rule.backtrack.len(), &backtrack)
        || !ctx.match_lookahead(last, rule.lookahead.len(), &lookahead)
    {
        return false;
    }
    let records: Vec<SequenceLookup> = rule.lookups.iter().collect();
    ctx.apply_sequence_lookups(positions, &records);
    true
}

fn read_sequence<'a>(r: &mut Reader<'a>) -> Result<Array<'a, u16>> {
    let count = r.u16()?;
    r.array(usize::from(count))
}

/// Parses a non-chained rule (`glyphCount, lookupCount, input, records`)
/// or a chained rule (`backtrack, input, lookahead, records`).
fn parse_rule(data: &[u8], chained: bool, tag: Tag) -> Result<Rule<'_>> {
    let mut r = Reader::new(data, tag);
    if chained {
        let backtrack = read_sequence(&mut r)?;
        let input_count = r.u16()?;
        let input = r.array(usize::from(input_count.saturating_sub(1)))?;
        let lookahead = read_sequence(&mut r)?;
        let lookup_count = r.u16()?;
        let lookups = r.array(usize::from(lookup_count))?;
        Ok(Rule {
            backtrack,
            input,
            lookahead,
            lookups,
        })
    } else {
        let input_count = r.u16()?;
        let lookup_count = r.u16()?;
        let input = r.array(usize::from(input_count.saturating_sub(1)))?;
        let lookups = r.array(usize::from(lookup_count))?;
        Ok(Rule {
            backtrack: Array::default(),
            input,
            lookahead: Array::default(),
            lookups,
        })
    }
}

/// Tries every rule of rule set `set_index` (formats 1 and 2).
fn apply_rule_set(
    ctx: &mut Applier<'_, '_>,
    data: &[u8],
    sets_at: usize,
    set_index: u16,
    chained: bool,
    matchers: [Match<'_>; 3],
    tag: Tag,
) -> Result<bool> {
    let mut r = Reader::at(data, sets_at, tag)?;
    let count = r.u16()?;
    let sets: Array<'_, u16> = r.array(usize::from(count))?;
    let Some(set_offset) = sets
        .get(usize::from(set_index))
        .filter(|&offset| offset != 0)
    else {
        return Ok(false);
    };
    let set = tail(data, usize::from(set_offset), tag)?;
    let mut r = Reader::new(set, tag);
    let rule_count = r.u16()?;
    let rules: Array<'_, u16> = r.array(usize::from(rule_count))?;
    for offset in rules.iter() {
        let rule = parse_rule(tail(set, usize::from(offset), tag)?, chained, tag)?;
        if apply_rule(ctx, &rule, matchers, tag) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn current_glyph(ctx: &Applier<'_, '_>) -> Option<GlyphId> {
    ctx.cur().map(|info| info.glyph)
}

/// Applies a `GSUB` type 5 / `GPOS` type 7 subtable.
pub(crate) fn apply_context(ctx: &mut Applier<'_, '_>, data: &[u8], tag: Tag) -> Result<bool> {
    let Some(glyph) = current_glyph(ctx) else {
        return Ok(false);
    };
    let mut r = Reader::new(data, tag);
    match r.u16()? {
        1 => {
            let Some(index) = Coverage::at(data, r.u16()?, tag)?.index(glyph) else {
                return Ok(false);
            };
            apply_rule_set(ctx, data, 4, index, false, [Match::Glyphs; 3], tag)
        }
        2 => {
            if !Coverage::at(data, r.u16()?, tag)?.contains(glyph) {
                return Ok(false);
            }
            let classes = ClassDef::at(data, r.u16()?, tag)?;
            let matchers = [Match::Classes(classes); 3];
            apply_rule_set(ctx, data, 6, classes.class(glyph), false, matchers, tag)
        }
        3 => {
            let input_count = r.u16()?;
            let lookup_count = r.u16()?;
            let coverages: Array<'_, u16> = r.array(usize::from(input_count))?;
            let Some(first) = coverages.get(0) else {
                return Ok(false);
            };
            if !Coverage::at(data, first, tag)?.contains(glyph) {
                return Ok(false);
            }
            let rule = Rule {
                backtrack: Array::default(),
                input: coverages.skip(1),
                lookahead: Array::default(),
                lookups: r.array(usize::from(lookup_count))?,
            };
            Ok(apply_rule(ctx, &rule, [Match::Coverages(data); 3], tag))
        }
        _ => Ok(false),
    }
}

/// Applies a `GSUB` type 6 / `GPOS` type 8 subtable.
pub(crate) fn apply_chain_context(
    ctx: &mut Applier<'_, '_>,
    data: &[u8],
    tag: Tag,
) -> Result<bool> {
    let Some(glyph) = current_glyph(ctx) else {
        return Ok(false);
    };
    let mut r = Reader::new(data, tag);
    match r.u16()? {
        1 => {
            let Some(index) = Coverage::at(data, r.u16()?, tag)?.index(glyph) else {
                return Ok(false);
            };
            apply_rule_set(ctx, data, 4, index, true, [Match::Glyphs; 3], tag)
        }
        2 => {
            if !Coverage::at(data, r.u16()?, tag)?.contains(glyph) {
                return Ok(false);
            }
            let backtrack = ClassDef::at(data, r.u16()?, tag)?;
            let input = ClassDef::at(data, r.u16()?, tag)?;
            let lookahead = ClassDef::at(data, r.u16()?, tag)?;
            let matchers = [
                Match::Classes(backtrack),
                Match::Classes(input),
                Match::Classes(lookahead),
            ];
            apply_rule_set(ctx, data, 10, input.class(glyph), true, matchers, tag)
        }
        3 => {
            let backtrack = read_sequence(&mut r)?;
            let input = read_sequence(&mut r)?;
            let lookahead = read_sequence(&mut r)?;
            let lookup_count = r.u16()?;
            let lookups = r.array(usize::from(lookup_count))?;
            let Some(first) = input.get(0) else {
                return Ok(false);
            };
            if !Coverage::at(data, first, tag)?.contains(glyph) {
                return Ok(false);
            }
            let rule = Rule {
                backtrack,
                input: input.skip(1),
                lookahead,
                lookups,
            };
            Ok(apply_rule(ctx, &rule, [Match::Coverages(data); 3], tag))
        }
        _ => Ok(false),
    }
}
