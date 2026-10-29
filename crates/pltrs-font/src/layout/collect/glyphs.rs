//! What one lookup reads and writes: the output glyphs of `GSUB` lookups
//! (with nested contextual lookups), the input glyphs of `GPOS` lookups,
//! and `GSUB`'s single-glyph "would apply" test.

use std::collections::BTreeSet;

use super::sets::{ClassTable, CoverageTable, Digest, GlyphSink};
use crate::layout::common::{LayoutTable, Lookup};
use crate::layout::context::{Rule, parse_rule, read_sequence};
use crate::reader::{Array, Reader, peek, tail};
use crate::types::{GlyphId, Tag};

/// Lookup types of the two tables.
const GSUB_CONTEXT: u16 = 5;
const GSUB_CHAIN: u16 = 6;
const GSUB_REVERSE_CHAIN: u16 = 8;
const GPOS_CONTEXT: u16 = 7;
const GPOS_CHAIN: u16 = 8;

/// The values of a `u16` array (after its count) at `at` in `data`.
fn array_at(data: &[u8], at: usize, tag: Tag) -> Array<'_, u16> {
    Reader::at(data, at, tag)
        .and_then(|mut r| read_sequence(&mut r))
        .unwrap_or_default()
}

/// The sub-table at `offset` in `data`, if readable and non-null.
fn sub(data: &[u8], offset: u16, tag: Tag) -> Option<&[u8]> {
    (offset != 0)
        .then(|| tail(data, usize::from(offset), tag).ok())
        .flatten()
}

/// The coverage table whose offset is at byte `at` of `data`.
fn coverage_at(data: &[u8], at: usize, tag: Tag) -> CoverageTable<'_> {
    peek::<u16>(data, at).map_or(CoverageTable::Empty, |offset| {
        CoverageTable::at(data, offset, tag)
    })
}

/// The class definition whose offset is at byte `at` of `data`.
fn classes_at(data: &[u8], at: usize, tag: Tag) -> ClassTable<'_> {
    ClassTable::at(data, peek::<u16>(data, at).unwrap_or(0), tag)
}

/// How the input values of a contextual rule name glyphs.
#[derive(Clone, Copy)]
enum InputValues<'a> {
    Glyphs,
    Classes(ClassTable<'a>),
    Coverages(&'a [u8]),
}

/// A contextual subtable decoded enough to walk its rules.
struct Contextual<'a> {
    /// The coverage of the first glyph (HarfBuzz's `get_coverage`).
    coverage: CoverageTable<'a>,
    /// Every rule, with its raw input count (first glyph included).
    rules: Vec<(Rule<'a>, u16)>,
    input: InputValues<'a>,
}

impl<'a> Contextual<'a> {
    /// Decodes a contextual (`chained` false) or chaining contextual
    /// subtable; malformed parts read as empty.
    fn parse(data: &'a [u8], chained: bool, tag: Tag) -> Option<Self> {
        match peek::<u16>(data, 0)? {
            1 => Some(Self {
                coverage: coverage_at(data, 2, tag),
                rules: Self::all_rules(data, 4, chained, tag),
                input: InputValues::Glyphs,
            }),
            2 => {
                let input = classes_at(data, if chained { 6 } else { 4 }, tag);
                let sets_at = if chained { 10 } else { 6 };
                Some(Self {
                    coverage: coverage_at(data, 2, tag),
                    rules: Self::all_rules(data, sets_at, chained, tag),
                    input: InputValues::Classes(input),
                })
            }
            3 => Self::coverage_based(data, chained, tag),
            _ => None,
        }
    }

    /// The rules of every rule set listed at `sets_at`.
    fn all_rules(data: &'a [u8], sets_at: usize, chained: bool, tag: Tag) -> Vec<(Rule<'a>, u16)> {
        array_at(data, sets_at, tag)
            .iter()
            .flat_map(|set| Self::rule_set(data, set, chained, tag))
            .collect()
    }

    /// The rules of the rule set at `offset`.
    fn rule_set(data: &'a [u8], offset: u16, chained: bool, tag: Tag) -> Vec<(Rule<'a>, u16)> {
        let Some(set) = sub(data, offset, tag) else {
            return Vec::new();
        };
        array_at(set, 0, tag)
            .iter()
            .filter_map(|rule| {
                let rule = sub(set, rule, tag)?;
                let input_count = if chained {
                    let backtrack = peek::<u16>(rule, 0)?;
                    peek::<u16>(rule, 2 + 2 * usize::from(backtrack))?
                } else {
                    peek::<u16>(rule, 0)?
                };
                Some((parse_rule(rule, chained, tag).ok()?, input_count))
            })
            .collect()
    }

    /// Format 3: a single rule of coverage tables.
    fn coverage_based(data: &'a [u8], chained: bool, tag: Tag) -> Option<Self> {
        let mut r = Reader::new(data, tag);
        r.skip(2).ok()?;
        let (backtrack, input, lookahead, lookups) = if chained {
            let backtrack = read_sequence(&mut r).ok()?;
            let input = read_sequence(&mut r).ok()?;
            let lookahead = read_sequence(&mut r).ok()?;
            let lookups = r.u16().and_then(|count| r.array(usize::from(count))).ok()?;
            (backtrack, input, lookahead, lookups)
        } else {
            let input_count = r.u16().ok()?;
            let lookup_count = r.u16().ok()?;
            let input = r.array(usize::from(input_count)).ok()?;
            let lookups = r.array(usize::from(lookup_count)).ok()?;
            (Array::default(), input, Array::default(), lookups)
        };
        let coverage = input.get(0).map_or(CoverageTable::Empty, |offset| {
            CoverageTable::at(data, offset, tag)
        });
        let rule = Rule {
            backtrack,
            input: input.skip(1),
            lookahead,
            lookups,
        };
        Some(Self {
            coverage,
            rules: vec![(rule, input.len() as u16)],
            input: InputValues::Coverages(data),
        })
    }

    /// Adds the glyphs the input `values` of a rule name to `sink`.
    fn collect_input(&self, values: Array<'_, u16>, tag: Tag, sink: &mut GlyphSink<'_>) {
        for value in values.iter() {
            match self.input {
                InputValues::Glyphs => sink.add(u32::from(value)),
                InputValues::Classes(classes) => classes.collect_class(sink, value),
                InputValues::Coverages(data) => {
                    CoverageTable::at(data, value, tag).collect(sink);
                }
            }
        }
    }
}

/// Calls `f` with the lookup type and bytes of every subtable of `lookup`
/// (extensions resolved).
fn for_each_subtable<'a>(lookup: &Lookup<'a>, mut f: impl FnMut(u16, &'a [u8])) {
    for subtable in lookup.subtables() {
        f(lookup.kind, subtable);
    }
}

/// Adds to `output` the glyphs `GSUB` lookup `index` can produce,
/// following nested lookups of contextual subtables
/// (`hb_ot_layout_lookup_collect_glyphs` with only the output set).
pub(crate) fn gsub_output_glyphs(table: &LayoutTable<'_>, index: u16, output: &mut BTreeSet<u32>) {
    let mut pending = vec![index];
    let mut visited = BTreeSet::new();
    while let Some(index) = pending.pop() {
        if !visited.insert(index) {
            continue;
        }
        let Some(lookup) = table.lookup(index) else {
            continue;
        };
        for_each_subtable(&lookup, |kind, data| {
            subtable_output(kind, data, table.tag, output, &mut pending);
        });
    }
}

/// The output of one `GSUB` subtable; nested lookups go to `nested`.
fn subtable_output(
    kind: u16,
    data: &[u8],
    tag: Tag,
    output: &mut BTreeSet<u32>,
    nested: &mut Vec<u16>,
) {
    let coverage = coverage_at(data, 2, tag);
    // The sub-tables of the covered glyphs, paired in iteration order.
    let paired = |at: usize| -> Vec<&[u8]> {
        coverage
            .glyphs()
            .iter()
            .zip(array_at(data, at, tag).iter())
            .filter_map(|(_, offset)| tail(data, usize::from(offset), tag).ok())
            .collect()
    };
    match (kind, peek::<u16>(data, 0).unwrap_or(0)) {
        (1, 1) => {
            let delta = peek::<u16>(data, 4).unwrap_or(0);
            output.extend(
                coverage
                    .glyphs()
                    .iter()
                    .map(|&g| u32::from(g.wrapping_add(delta))),
            );
        }
        (1, 2) => {
            let substitutes = array_at(data, 4, tag);
            let pairs = coverage.glyphs().into_iter().zip(substitutes.iter());
            output.extend(pairs.map(|(_, glyph)| u32::from(glyph)));
        }
        (2 | 3, 1) => {
            for set in paired(4) {
                output.extend(array_at(set, 0, tag).iter().map(u32::from));
            }
        }
        (4, 1) => {
            for set in paired(4) {
                for ligature in array_at(set, 0, tag).iter() {
                    if let Some(glyph) = sub(set, ligature, tag).and_then(|l| peek::<u16>(l, 0)) {
                        output.insert(u32::from(glyph));
                    }
                }
            }
        }
        (GSUB_CONTEXT | GSUB_CHAIN, _) => {
            if let Some(contextual) = Contextual::parse(data, kind == GSUB_CHAIN, tag) {
                for (rule, _) in &contextual.rules {
                    nested.extend(rule.lookups.iter().map(|record| record.lookup_index));
                }
            }
        }
        (GSUB_REVERSE_CHAIN, 1) => {
            let mut r = Reader::new(data, tag);
            let substitutes = r.skip(4).and_then(|()| {
                read_sequence(&mut r)?;
                read_sequence(&mut r)?;
                read_sequence(&mut r)
            });
            output.extend(substitutes.unwrap_or_default().iter().map(u32::from));
        }
        _ => {}
    }
}

/// Adds to `input` the glyphs `GPOS` lookup `index` reads
/// (`hb_ot_layout_lookup_collect_glyphs` with only the input set).
pub(crate) fn gpos_input_glyphs(table: &LayoutTable<'_>, index: u16, input: &mut BTreeSet<u32>) {
    let Some(lookup) = table.lookup(index) else {
        return;
    };
    let tag = table.tag;
    for_each_subtable(&lookup, |kind, data| {
        let mut sink = GlyphSink(input);
        let coverage = |at: usize| coverage_at(data, at, tag);
        match (kind, peek::<u16>(data, 0).unwrap_or(0)) {
            (1, 1 | 2) | (3, 1) => {
                coverage(2).collect(&mut sink);
            }
            (2, 1) => {
                if coverage(2).collect(&mut sink) {
                    pair_set_glyphs(data, tag, &mut sink);
                }
            }
            (2, 2) => {
                if coverage(2).collect(&mut sink) {
                    classes_at(data, 10, tag).collect_coverage(&mut sink);
                }
            }
            (4..=6, 1) => {
                if coverage(2).collect(&mut sink) {
                    coverage(4).collect(&mut sink);
                }
            }
            (GPOS_CONTEXT | GPOS_CHAIN, _) => {
                if let Some(contextual) = Contextual::parse(data, kind == GPOS_CHAIN, tag) {
                    contextual.coverage.collect(&mut sink);
                    for (rule, _) in &contextual.rules {
                        contextual.collect_input(rule.input, tag, &mut sink);
                    }
                }
            }
            _ => {}
        }
    });
}

/// The second glyphs of a format 1 pair positioning subtable.
fn pair_set_glyphs(data: &[u8], tag: Tag, sink: &mut GlyphSink<'_>) {
    let value_size = |format: u16| 2 * format.count_ones() as usize;
    let (Some(first), Some(second)) = (peek::<u16>(data, 4), peek::<u16>(data, 6)) else {
        return;
    };
    let record = 2 + value_size(first) + value_size(second);
    for set in array_at(data, 8, tag).iter() {
        let Some(set) = sub(data, set, tag) else {
            continue;
        };
        let count = usize::from(peek::<u16>(set, 0).unwrap_or(0));
        for index in 0..count {
            if let Some(glyph) = peek::<u16>(set, 2 + index * record) {
                sink.add(u32::from(glyph));
            }
        }
    }
}

/// Whether `GSUB` lookup `index` would substitute `glyph` alone, with no
/// context (`hb_ot_layout_lookup_would_substitute` with
/// `zero_context`). Like HarfBuzz, contextual subtables only check their
/// rules' lengths once the lookup's digest admits the glyph.
pub(crate) fn would_substitute(table: &LayoutTable<'_>, index: u16, glyph: GlyphId) -> bool {
    if usize::from(index) >= table.lookup_count() {
        return false;
    }
    let Some(lookup) = table.lookup(index) else {
        return false;
    };
    let tag = table.tag;
    let mut subtables = Vec::new();
    for_each_subtable(&lookup, |kind, data| subtables.push((kind, data)));
    let mut digest = Digest::default();
    for &(kind, data) in &subtables {
        first_coverage(kind, data, tag).digest(&mut digest);
    }
    digest.may_have(u32::from(glyph.0))
        && subtables
            .iter()
            .any(|&(kind, data)| subtable_would_apply(kind, data, tag, glyph))
}

/// The coverage HarfBuzz's lookup accelerator digests for a subtable.
fn first_coverage(kind: u16, data: &[u8], tag: Tag) -> CoverageTable<'_> {
    match (kind, peek::<u16>(data, 0).unwrap_or(0)) {
        (GSUB_CONTEXT | GSUB_CHAIN, 3) => Contextual::parse(data, kind == GSUB_CHAIN, tag)
            .map_or(CoverageTable::Empty, |contextual| contextual.coverage),
        (1 | GSUB_CONTEXT | GSUB_CHAIN, 1 | 2) | (2..=4 | GSUB_REVERSE_CHAIN, 1) => {
            coverage_at(data, 2, tag)
        }
        _ => CoverageTable::Empty,
    }
}

/// `would_apply` of one `GSUB` subtable for a lone glyph.
fn subtable_would_apply(kind: u16, data: &[u8], tag: Tag, glyph: GlyphId) -> bool {
    let format = peek::<u16>(data, 0).unwrap_or(0);
    match (kind, format) {
        (1, 1 | 2) | (2 | 3 | GSUB_REVERSE_CHAIN, 1) => coverage_at(data, 2, tag).contains(glyph),
        (4, 1) => {
            let set = coverage_at(data, 2, tag)
                .index(glyph)
                .and_then(|index| array_at(data, 4, tag).get(index))
                .and_then(|offset| sub(data, offset, tag));
            set.is_some_and(|set| {
                array_at(set, 0, tag).iter().any(|ligature| {
                    sub(set, ligature, tag).and_then(|l| peek::<u16>(l, 2)) == Some(1)
                })
            })
        }
        (GSUB_CONTEXT | GSUB_CHAIN, 1 | 2) => {
            let chained = kind == GSUB_CHAIN;
            let set_index = if format == 1 {
                coverage_at(data, 2, tag).index(glyph)
            } else {
                let at = if chained { 6 } else { 4 };
                Some(usize::from(classes_at(data, at, tag).class(glyph)))
            };
            let sets_at = match (format, chained) {
                (1, _) => 4,
                (_, false) => 6,
                (_, true) => 10,
            };
            set_index
                .and_then(|index| array_at(data, sets_at, tag).get(index))
                .is_some_and(|set| {
                    Contextual::rule_set(data, set, chained, tag)
                        .iter()
                        .any(single_glyph_rule)
                })
        }
        (GSUB_CONTEXT | GSUB_CHAIN, 3) => Contextual::parse(data, kind == GSUB_CHAIN, tag)
            .is_some_and(|contextual| contextual.rules.iter().any(single_glyph_rule)),
        _ => false,
    }
}

/// Whether a rule matches a lone glyph: one input glyph, no backtrack and
/// no lookahead.
fn single_glyph_rule((rule, input_count): &(Rule<'_>, u16)) -> bool {
    *input_count == 1 && rule.backtrack.is_empty() && rule.lookahead.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::tests::{coverage_format1, layout_table, lookup};

    /// Big-endian `u16` words.
    fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    /// `words` followed by a format 1 coverage of `glyphs`.
    fn covered(values: &[u16], glyphs: &[u16]) -> Vec<u8> {
        [words(values), coverage_format1(glyphs)].concat()
    }

    /// One `GSUB` lookup per subtable kind, plus nested contextual ones.
    fn gsub() -> Vec<u8> {
        layout_table(
            *b"latn",
            &[],
            &[
                // 0: single, 10 and 11 shifted by 5.
                lookup(1, 0, &[covered(&[1, 6, 5], &[10, 11])]),
                // 1: single, 12 and 13 to 30 and 31.
                lookup(1, 0, &[covered(&[2, 10, 2, 30, 31], &[12, 13])]),
                // 2: multiple, 20 to 40 41.
                lookup(2, 0, &[covered(&[1, 14, 1, 8, 2, 40, 41], &[20])]),
                // 3: ligatures from 22: 22 21 to 50, 22 alone to 51.
                lookup(
                    4,
                    0,
                    &[covered(&[1, 24, 1, 8, 2, 6, 12, 50, 2, 21, 51, 1], &[22])],
                ),
                // 4: chaining context (format 3), 60 alone through lookup 5.
                lookup(6, 0, &[covered(&[3, 0, 1, 16, 0, 1, 0, 5], &[60])]),
                // 5: single, 60 shifted by 100.
                lookup(1, 0, &[covered(&[1, 6, 100], &[60])]),
                // 6: context (format 3) through itself and lookup 0.
                lookup(5, 0, &[covered(&[3, 1, 2, 16, 0, 6, 0, 0], &[10])]),
                // 7: reverse chaining, 25 to 70.
                lookup(8, 0, &[covered(&[1, 12, 0, 0, 1, 70], &[25])]),
                // 8: chaining context (format 1), 80 81 through lookup 0.
                lookup(
                    6,
                    0,
                    &[covered(&[1, 26, 1, 8, 1, 4, 0, 2, 81, 0, 1, 0, 0], &[80])],
                ),
            ],
        )
    }

    #[test]
    fn gsub_outputs_follow_nested_lookups() {
        let data = gsub();
        let table = LayoutTable::parse(&data, Tag::new(b"GSUB"), 7).expect("valid table");
        let output = |index| {
            let mut glyphs = BTreeSet::new();
            gsub_output_glyphs(&table, index, &mut glyphs);
            glyphs.into_iter().collect::<Vec<_>>()
        };
        assert_eq!(output(0), [15, 16]);
        assert_eq!(output(1), [30, 31]);
        assert_eq!(output(2), [40, 41]);
        assert_eq!(output(3), [50, 51]);
        assert_eq!(output(4), [160]);
        assert_eq!(output(6), [15, 16], "a lookup calling itself");
        assert_eq!(output(7), [70]);
        assert_eq!(output(8), [15, 16]);
        assert_eq!(output(9), []);
    }

    #[test]
    fn lone_glyph_substitutions() {
        let data = gsub();
        let table = LayoutTable::parse(&data, Tag::new(b"GSUB"), 7).expect("valid table");
        let would = |index, glyph| would_substitute(&table, index, GlyphId(glyph));
        assert!(would(0, 10) && !would(0, 12));
        assert!(would(1, 13) && would(2, 20) && would(7, 25));
        assert!(would(3, 22), "a one-component ligature");
        assert!(would(4, 60) && !would(4, 61));
        assert!(!would(8, 80), "the rule needs two glyphs");
        assert!(!would(9, 10));
    }

    #[test]
    fn gpos_inputs_include_second_glyphs_and_classes() {
        let data = layout_table(
            *b"latn",
            &[],
            &[
                // 0: single adjustment of 5 and 6.
                lookup(1, 0, &[covered(&[1, 6, 0], &[5, 6])]),
                // 1: pairs 7 90 and 7 91.
                lookup(
                    2,
                    0,
                    &[covered(&[1, 22, 4, 0, 1, 12, 2, 90, 0xFFF6, 91, 5], &[7])],
                ),
                // 2: pair classes of 8 with glyphs 100..=102 (classes 1, 0, 2).
                lookup(
                    2,
                    0,
                    &[covered(
                        &[2, 32, 0, 0, 28, 16, 1, 2, 1, 100, 3, 1, 0, 2, 2, 0],
                        &[8],
                    )],
                ),
                // 3: context (format 3) of 110 then 111 or 112.
                lookup(
                    7,
                    0,
                    &[[
                        words(&[3, 2, 0, 10, 16]),
                        coverage_format1(&[110]),
                        coverage_format1(&[111, 112]),
                    ]
                    .concat()],
                ),
            ],
        );
        let table = LayoutTable::parse(&data, Tag::new(b"GPOS"), 9).expect("valid table");
        let input = |index| {
            let mut glyphs = BTreeSet::new();
            gpos_input_glyphs(&table, index, &mut glyphs);
            glyphs.into_iter().collect::<Vec<_>>()
        };
        assert_eq!(input(0), [5, 6]);
        assert_eq!(input(1), [7, 90, 91]);
        // HarfBuzz's class coverage takes the glyph ending each run.
        assert_eq!(input(2), [8, 100, 101, 102, 103]);
        assert_eq!(input(3), [110, 111, 112]);
        assert_eq!(input(4), []);
    }
}
