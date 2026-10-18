//! The glyphs `GSUB` substitutions can produce from a glyph set
//! (fontTools' `GSUB.closure_glyphs` over the features its subsetter keeps).

use std::collections::{BTreeSet, HashMap};

use crate::error::Result;
use crate::layout::common::{ClassDef, Coverage, LayoutTable};
use crate::reader::{Array, Reader, tail};
use crate::types::{GlyphId, Tag};

const GSUB: Tag = Tag::new(b"GSUB");

/// The feature tags fontTools' subsetter keeps by default
/// (`Options._layout_features_default`).
const LAYOUT_FEATURES: [&[u8; 4]; 66] = [
    b"BUZZ", b"Buzz", b"HARF", b"Harf", b"abvf", b"abvm", b"abvs", b"akhn", b"blwf", b"blwm",
    b"blws", b"calt", b"ccmp", b"cfar", b"chws", b"cjct", b"clig", b"cswh", b"curs", b"dist",
    b"dnom", b"fin2", b"fin3", b"fina", b"frac", b"half", b"halt", b"haln", b"init", b"isol",
    b"jalt", b"kern", b"liga", b"ljmo", b"locl", b"ltra", b"ltrm", b"mark", b"med2", b"medi",
    b"mkmk", b"mset", b"nukt", b"numr", b"palt", b"pref", b"pres", b"pstf", b"psts", b"rand",
    b"rclt", b"rkrf", b"rlig", b"rphf", b"rtla", b"rtlm", b"rvrn", b"stch", b"tjmo", b"valt",
    b"vatu", b"vchw", b"vert", b"vhal", b"vjmo", b"vkrn",
];

/// Deepest nesting of contextual lookups followed (HarfBuzz's limit).
/// Each nested application may add glyphs, which resets the memo, so a
/// self-applying lookup would otherwise recurse once per glyph. Deeper
/// applications are left to the next round of the fixpoint loop, with the
/// memo cleared since it no longer tells which glyphs were fully processed.
const MAX_NESTING: u8 = 64;

/// Lookup types that may replace one glyph by several or several by one
/// (fontTools' `may_have_non_1to1`).
const fn may_have_non_1to1(kind: u16) -> bool {
    matches!(kind, 2 | 4..=6)
}

/// A nested lookup application of a contextual rule.
#[derive(Clone, Copy, Debug)]
struct LookupRecord {
    sequence_index: u16,
    lookup_index: u16,
}

impl crate::reader::FromBytes for LookupRecord {
    const SIZE: usize = 4;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            sequence_index: u16::from_be_bytes([bytes[0], bytes[1]]),
            lookup_index: u16::from_be_bytes([bytes[2], bytes[3]]),
        }
    }
}

/// Which glyphs a lookup is applied to.
enum Targets {
    /// Every glyph of the set (fontTools' `cur_glyphs=None`).
    All,
    /// These glyphs only.
    Some(BTreeSet<GlyphId>),
}

/// The closure computation: the growing glyph set and fontTools'
/// per-lookup memo of glyphs already processed.
struct Closure<'a> {
    table: LayoutTable<'a>,
    glyphs: BTreeSet<GlyphId>,
    /// Per lookup: the glyph count the memo is valid for, and the glyphs
    /// processed since.
    done: HashMap<u16, (usize, BTreeSet<GlyphId>)>,
    /// Lookups being applied inside each other.
    nesting: u8,
}

/// The coverage-indexed offsets of a subtable after `skip` bytes: the
/// sub-tables of the covered glyphs in `glyphs`, with those glyphs.
fn covered<'d>(
    coverage: &Coverage<'_>,
    data: &'d [u8],
    offsets_at: usize,
    glyphs: &BTreeSet<GlyphId>,
) -> Result<Vec<(GlyphId, &'d [u8])>> {
    let mut r = Reader::at(data, offsets_at, GSUB)?;
    let count = r.u16()?;
    let offsets: Array<'_, u16> = r.array(usize::from(count))?;
    let mut out = Vec::new();
    for (index, glyph) in coverage.entries() {
        if !glyphs.contains(&glyph) {
            continue;
        }
        if let Some(offset) = offsets.get(usize::from(index)).filter(|&o| o != 0) {
            out.push((glyph, tail(data, usize::from(offset), GSUB)?));
        }
    }
    Ok(out)
}

/// The glyph array after a count at the start of `data`.
fn counted_glyphs(data: &[u8]) -> Result<Vec<GlyphId>> {
    let mut r = Reader::new(data, GSUB);
    let count = r.u16()?;
    Ok(r.array::<GlyphId>(usize::from(count))?.iter().collect())
}

/// Coverage tables at the offsets following a count at `at`.
fn coverages<'d>(data: &'d [u8], r: &mut Reader<'d>) -> Result<Vec<Coverage<'d>>> {
    let count = r.u16()?;
    let offsets: Array<'_, u16> = r.array(usize::from(count))?;
    offsets
        .iter()
        .map(|offset| Coverage::at(data, offset, GSUB))
        .collect()
}

/// Glyphs of `glyphs` in class `class` of `classes` (fontTools'
/// `intersect_class`).
fn class_members(
    classes: &ClassDef<'_>,
    glyphs: &BTreeSet<GlyphId>,
    class: u16,
) -> BTreeSet<GlyphId> {
    glyphs
        .iter()
        .copied()
        .filter(|&g| classes.class(g) == class)
        .collect()
}

/// The classes of `glyphs` (fontTools' `ClassDef.intersect`).
fn classes_of(classes: &ClassDef<'_>, glyphs: &BTreeSet<GlyphId>) -> BTreeSet<u16> {
    glyphs.iter().map(|&g| classes.class(g)).collect()
}

/// The parts of a contextual rule, as glyphs, classes or coverages.
enum RuleParts<'d> {
    Glyphs(Vec<Vec<GlyphId>>),
    Classes(Vec<(ClassDef<'d>, Vec<u16>)>),
}

impl Closure<'_> {
    /// Whether every part of a rule can occur in the glyph set.
    fn matches(&self, parts: &RuleParts<'_>) -> bool {
        match parts {
            RuleParts::Glyphs(lists) => lists
                .iter()
                .flatten()
                .all(|glyph| self.glyphs.contains(glyph)),
            RuleParts::Classes(lists) => lists.iter().all(|(classes, list)| {
                list.iter()
                    .all(|&class| !class_members(classes, &self.glyphs, class).is_empty())
            }),
        }
    }

    /// Applies lookup `index` to `targets` (fontTools'
    /// `Lookup.closure_glyphs`).
    fn lookup(&mut self, index: u16, targets: Targets) -> Result<()> {
        if self.nesting >= MAX_NESTING {
            self.done.clear();
            return Ok(());
        }
        let Some(lookup) = self.table.lookup(index) else {
            return Ok(());
        };
        let current = match targets {
            Targets::All => self.glyphs.clone(),
            Targets::Some(glyphs) => glyphs,
        };
        let count = self.glyphs.len();
        let memo = self
            .done
            .entry(index)
            .or_insert_with(|| (count, BTreeSet::new()));
        if memo.0 != count {
            *memo = (count, BTreeSet::new());
        }
        if current.is_subset(&memo.1) {
            return Ok(());
        }
        memo.1.extend(current.iter().copied());
        self.nesting += 1;
        let applied = lookup
            .subtables()
            .try_for_each(|subtable| self.subtable(lookup.kind, subtable, &current));
        self.nesting -= 1;
        applied
    }

    fn subtable(&mut self, kind: u16, data: &[u8], current: &BTreeSet<GlyphId>) -> Result<()> {
        match kind {
            1 => self.single(data, current),
            2 | 3 => self.sequences(data, current),
            4 => self.ligatures(data, current),
            5 | 6 => self.context(kind == 6, data, current),
            8 => self.reverse_chain(data, current),
            _ => Ok(()),
        }
    }

    fn single(&mut self, data: &[u8], current: &BTreeSet<GlyphId>) -> Result<()> {
        let mut r = Reader::new(data, GSUB);
        let format = r.u16()?;
        let coverage = Coverage::at(data, r.u16()?, GSUB)?;
        let mut added = Vec::new();
        if format == 1 {
            let delta = r.i16()?;
            for (_, glyph) in coverage.entries() {
                if current.contains(&glyph) {
                    added.push(GlyphId(glyph.0.wrapping_add(delta as u16)));
                }
            }
        } else {
            let count = r.u16()?;
            let substitutes: Array<'_, GlyphId> = r.array(usize::from(count))?;
            for (index, glyph) in coverage.entries() {
                if current.contains(&glyph) {
                    added.extend(substitutes.get(usize::from(index)));
                }
            }
        }
        self.glyphs.extend(added);
        Ok(())
    }

    /// Multiple and alternate substitutions: every glyph of the covered
    /// glyphs' sequences or alternate sets.
    fn sequences(&mut self, data: &[u8], current: &BTreeSet<GlyphId>) -> Result<()> {
        let coverage = Coverage::at(data, Reader::at(data, 2, GSUB)?.u16()?, GSUB)?;
        let mut added = Vec::new();
        for (_, sequence) in covered(&coverage, data, 4, current)? {
            added.extend(counted_glyphs(sequence)?);
        }
        self.glyphs.extend(added);
        Ok(())
    }

    /// Ligatures whose components are all in the set.
    fn ligatures(&mut self, data: &[u8], current: &BTreeSet<GlyphId>) -> Result<()> {
        let coverage = Coverage::at(data, Reader::at(data, 2, GSUB)?.u16()?, GSUB)?;
        let mut added = Vec::new();
        for (_, set) in covered(&coverage, data, 4, current)? {
            let mut r = Reader::new(set, GSUB);
            let count = r.u16()?;
            let offsets: Array<'_, u16> = r.array(usize::from(count))?;
            for offset in offsets.iter() {
                let ligature = tail(set, usize::from(offset), GSUB)?;
                let mut r = Reader::new(ligature, GSUB);
                let glyph = r.glyph()?;
                let components = r.u16()?.saturating_sub(1);
                let rest: Array<'_, GlyphId> = r.array(usize::from(components))?;
                if rest.iter().all(|c| self.glyphs.contains(&c)) {
                    added.push(glyph);
                }
            }
        }
        self.glyphs.extend(added);
        Ok(())
    }

    fn reverse_chain(&mut self, data: &[u8], current: &BTreeSet<GlyphId>) -> Result<()> {
        let mut r = Reader::new(data, GSUB);
        r.skip(2)?;
        let coverage = Coverage::at(data, r.u16()?, GSUB)?;
        let backtrack = coverages(data, &mut r)?;
        let lookahead = coverages(data, &mut r)?;
        let count = r.u16()?;
        let substitutes: Array<'_, GlyphId> = r.array(usize::from(count))?;
        let indices: Vec<u16> = coverage
            .entries()
            .into_iter()
            .filter(|(_, g)| current.contains(g))
            .map(|(i, _)| i)
            .collect();
        let context_present = lookahead
            .iter()
            .chain(&backtrack)
            .all(|c| self.glyphs.iter().any(|&g| c.contains(g)));
        if indices.is_empty() || !context_present {
            return Ok(());
        }
        let added: Vec<GlyphId> = indices
            .into_iter()
            .filter_map(|i| substitutes.get(usize::from(i)))
            .collect();
        self.glyphs.extend(added);
        Ok(())
    }

    /// Contextual and chaining contextual substitutions.
    fn context(&mut self, chain: bool, data: &[u8], current: &BTreeSet<GlyphId>) -> Result<()> {
        match Reader::new(data, GSUB).u16()? {
            1 => self.context_glyphs(chain, data, current),
            2 => self.context_classes(chain, data, current),
            3 => self.context_coverages(chain, data, current),
            _ => Ok(()),
        }
    }

    /// Applies the nested lookups of a matching rule with fontTools'
    /// "chaos" rule: once a lookup may change the sequence length, later
    /// positions are applied to every glyph. `position` gives the glyphs
    /// at a sequence index when that is known.
    fn apply_records(
        &mut self,
        records: &[LookupRecord],
        input_len: usize,
        position: impl Fn(&Self, u16) -> BTreeSet<GlyphId>,
    ) -> Result<()> {
        let mut chaos = BTreeSet::new();
        for record in records {
            let index = record.sequence_index;
            let targets = if chaos.contains(&index) {
                Targets::All
            } else {
                Targets::Some(position(self, index))
            };
            chaos.insert(index);
            let non_1to1 = self
                .table
                .lookup(record.lookup_index)
                .is_some_and(|lookup| may_have_non_1to1(lookup.kind));
            if non_1to1 {
                chaos.extend(index..=input_len as u16);
            }
            self.lookup(record.lookup_index, targets)?;
        }
        Ok(())
    }

    fn context_glyphs(
        &mut self,
        chain: bool,
        data: &[u8],
        current: &BTreeSet<GlyphId>,
    ) -> Result<()> {
        let coverage = Coverage::at(data, Reader::at(data, 2, GSUB)?.u16()?, GSUB)?;
        for (first, set) in covered(&coverage, data, 4, current)? {
            for rule in rules(set)? {
                let GlyphRule {
                    parts,
                    input,
                    records,
                } = GlyphRule::parse(chain, rule)?;
                if !self.matches(&RuleParts::Glyphs(parts)) {
                    continue;
                }
                let input_len = input.len() + 1;
                self.apply_records(&records, input_len, |_, index| {
                    let glyph = if index == 0 {
                        Some(first)
                    } else {
                        input.get(usize::from(index) - 1).copied()
                    };
                    glyph.into_iter().collect()
                })?;
            }
        }
        Ok(())
    }

    fn context_classes(
        &mut self,
        chain: bool,
        data: &[u8],
        current: &BTreeSet<GlyphId>,
    ) -> Result<()> {
        let mut r = Reader::new(data, GSUB);
        r.skip(2)?;
        let coverage = Coverage::at(data, r.u16()?, GSUB)?;
        let (backtrack, input_classes, lookahead) = if chain {
            let backtrack = ClassDef::at(data, r.u16()?, GSUB)?;
            let input = ClassDef::at(data, r.u16()?, GSUB)?;
            (backtrack, input, ClassDef::at(data, r.u16()?, GSUB)?)
        } else {
            let input = ClassDef::at(data, r.u16()?, GSUB)?;
            (ClassDef::Empty, input, ClassDef::Empty)
        };
        let set_count = r.u16()?;
        let sets: Array<'_, u16> = r.array(usize::from(set_count))?;
        let current: BTreeSet<GlyphId> = coverage
            .entries()
            .into_iter()
            .map(|(_, g)| g)
            .filter(|g| current.contains(g))
            .collect();
        if current.is_empty() {
            return Ok(());
        }
        for class in classes_of(&input_classes, &current) {
            let Some(offset) = sets.get(usize::from(class)).filter(|&o| o != 0) else {
                continue;
            };
            let set = tail(data, usize::from(offset), GSUB)?;
            for rule in rules(set)? {
                let GlyphRule {
                    parts,
                    input,
                    records,
                } = GlyphRule::parse(chain, rule)?;
                let lists: Vec<Vec<u16>> = parts
                    .into_iter()
                    .map(|list| list.into_iter().map(|g| g.0).collect())
                    .collect();
                let class_parts = if chain {
                    vec![
                        (backtrack, lists[0].clone()),
                        (input_classes, lists[1].clone()),
                        (lookahead, lists[2].clone()),
                    ]
                } else {
                    vec![(input_classes, lists[0].clone())]
                };
                if !self.matches(&RuleParts::Classes(class_parts)) {
                    continue;
                }
                let input_len = input.len() + 1;
                self.apply_records(&records, input_len, |closure, index| {
                    if index == 0 {
                        class_members(&input_classes, &current, class)
                    } else {
                        input
                            .get(usize::from(index) - 1)
                            .map_or_else(BTreeSet::new, |c| {
                                class_members(&input_classes, &closure.glyphs, c.0)
                            })
                    }
                })?;
            }
        }
        Ok(())
    }

    fn context_coverages(
        &mut self,
        chain: bool,
        data: &[u8],
        current: &BTreeSet<GlyphId>,
    ) -> Result<()> {
        let mut r = Reader::new(data, GSUB);
        r.skip(2)?;
        let (backtrack, input, lookahead, records) = if chain {
            let backtrack = coverages(data, &mut r)?;
            let input = coverages(data, &mut r)?;
            let lookahead = coverages(data, &mut r)?;
            (backtrack, input, lookahead, read_records(&mut r)?)
        } else {
            let glyph_count = r.u16()?;
            let record_count = r.u16()?;
            let offsets: Array<'_, u16> = r.array(usize::from(glyph_count))?;
            let input = offsets
                .iter()
                .map(|offset| Coverage::at(data, offset, GSUB))
                .collect::<Result<Vec<_>>>()?;
            let records = r.array(usize::from(record_count))?.iter().collect();
            (Vec::new(), input, Vec::new(), records)
        };
        let Some(first) = input.first() else {
            return Ok(());
        };
        let current: BTreeSet<GlyphId> = first
            .entries()
            .into_iter()
            .map(|(_, g)| g)
            .filter(|g| current.contains(g))
            .collect();
        if current.is_empty() {
            return Ok(());
        }
        let present = backtrack
            .iter()
            .chain(&input)
            .chain(&lookahead)
            .all(|c| self.glyphs.iter().any(|&g| c.contains(g)));
        if !present {
            return Ok(());
        }
        self.apply_records(&records, input.len(), |closure, index| {
            if index == 0 {
                current.clone()
            } else {
                input
                    .get(usize::from(index))
                    .map_or_else(BTreeSet::new, |c| {
                        closure
                            .glyphs
                            .iter()
                            .copied()
                            .filter(|&g| c.contains(g))
                            .collect()
                    })
            }
        })
    }
}

/// The rules of a rule set (count, offsets).
fn rules(set: &[u8]) -> Result<Vec<&[u8]>> {
    let mut r = Reader::new(set, GSUB);
    let count = r.u16()?;
    let offsets: Array<'_, u16> = r.array(usize::from(count))?;
    offsets
        .iter()
        .filter(|&offset| offset != 0)
        .map(|offset| tail(set, usize::from(offset), GSUB))
        .collect()
}

/// The lookup records after their count.
fn read_records(r: &mut Reader<'_>) -> Result<Vec<LookupRecord>> {
    let count = r.u16()?;
    Ok(r.array(usize::from(count))?.iter().collect())
}

/// A glyph or class rule of a contextual subtable (classes are held in
/// glyph ids).
struct GlyphRule {
    /// Backtrack, input and lookahead for chains; the input otherwise.
    parts: Vec<Vec<GlyphId>>,
    /// The input after its first position.
    input: Vec<GlyphId>,
    /// The nested lookups.
    records: Vec<LookupRecord>,
}

impl GlyphRule {
    fn parse(chain: bool, rule: &[u8]) -> Result<Self> {
        let mut r = Reader::new(rule, GSUB);
        if chain {
            let backtrack_count = r.u16()?;
            let backtrack: Vec<GlyphId> = r.array(usize::from(backtrack_count))?.iter().collect();
            let input_count = r.u16()?.saturating_sub(1);
            let input: Vec<GlyphId> = r.array(usize::from(input_count))?.iter().collect();
            let lookahead_count = r.u16()?;
            let lookahead: Vec<GlyphId> = r.array(usize::from(lookahead_count))?.iter().collect();
            let records = read_records(&mut r)?;
            Ok(Self {
                parts: vec![backtrack, input.clone(), lookahead],
                input,
                records,
            })
        } else {
            let glyph_count = r.u16()?.saturating_sub(1);
            let record_count = r.u16()?;
            let input: Vec<GlyphId> = r.array(usize::from(glyph_count))?.iter().collect();
            let records = r.array(usize::from(record_count))?.iter().collect();
            Ok(Self {
                parts: vec![input.clone()],
                input,
                records,
            })
        }
    }
}

/// Adds to `glyphs` every glyph the `GSUB` lookups of the kept features
/// can substitute for them, until nothing more is added.
pub(super) fn close_over_gsub(gsub: &[u8], glyphs: &mut BTreeSet<GlyphId>) -> Result<()> {
    let table = LayoutTable::parse(gsub, GSUB, 7)?;
    let kept: Vec<Tag> = LAYOUT_FEATURES.iter().map(|tag| Tag::new(tag)).collect();
    let mut lookups: Vec<u16> = table
        .referenced_features()
        .into_iter()
        .filter_map(|index| table.feature(index))
        .filter(|(tag, _)| kept.contains(tag))
        .flat_map(|(_, lookups)| lookups.iter().collect::<Vec<_>>())
        .collect();
    lookups.sort_unstable();
    lookups.dedup();
    let mut closure = Closure {
        table,
        glyphs: std::mem::take(glyphs),
        done: HashMap::new(),
        nesting: 0,
    };
    loop {
        let before = closure.glyphs.len();
        for &index in &lookups {
            if usize::from(index) < closure.table.lookup_count() {
                closure.lookup(index, Targets::All)?;
            }
        }
        if closure.glyphs.len() == before {
            break;
        }
    }
    *glyphs = closure.glyphs;
    Ok(())
}
