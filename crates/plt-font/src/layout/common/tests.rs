//! Tests of the shared layout structures, and builders of raw layout
//! data that the other layout modules' tests use too.

use super::{ClassDef, Coverage, LayoutTable};
use crate::types::{GlyphId, Tag};

const TAG: Tag = Tag::new(b"GSUB");

pub(crate) fn coverage_format1(glyphs: &[u16]) -> Vec<u8> {
    let mut out = vec![0, 1];
    out.extend_from_slice(&(glyphs.len() as u16).to_be_bytes());
    for g in glyphs {
        out.extend_from_slice(&g.to_be_bytes());
    }
    out
}

pub(crate) fn ranges(format: u16, ranges: &[(u16, u16, u16)]) -> Vec<u8> {
    let mut out = format.to_be_bytes().to_vec();
    out.extend_from_slice(&(ranges.len() as u16).to_be_bytes());
    for r in ranges {
        for v in [r.0, r.1, r.2] {
            out.extend_from_slice(&v.to_be_bytes());
        }
    }
    out
}

#[test]
fn coverage_formats() {
    let glyphs = coverage_format1(&[3, 7, 9]);
    let cov = Coverage::parse(&glyphs, TAG).unwrap();
    assert_eq!(cov.index(GlyphId(7)), Some(1));
    assert_eq!(cov.index(GlyphId(8)), None);
    let range_data = ranges(2, &[(10, 12, 0), (20, 20, 3)]);
    let cov = Coverage::parse(&range_data, TAG).unwrap();
    assert_eq!(cov.index(GlyphId(11)), Some(1));
    assert_eq!(cov.index(GlyphId(20)), Some(3));
    assert!(!cov.contains(GlyphId(13)));
    assert!(Coverage::parse(&[0, 3, 0, 0], TAG).is_err());
    assert!(Coverage::parse(&coverage_format1(&[3, 7, 9])[..6], TAG).is_err());
}

#[test]
fn class_definitions() {
    let classes = ClassDef::parse(&[0, 1, 0, 5, 0, 2, 0, 3, 0, 4], TAG).unwrap();
    assert_eq!(classes.class(GlyphId(5)), 3);
    assert_eq!(classes.class(GlyphId(6)), 4);
    assert_eq!(classes.class(GlyphId(7)), 0);
    assert_eq!(classes.class(GlyphId(1)), 0);
    let class_data = ranges(2, &[(1, 4, 2)]);
    let classes = ClassDef::parse(&class_data, TAG).unwrap();
    assert_eq!(classes.class(GlyphId(3)), 2);
    assert_eq!(classes.class(GlyphId(5)), 0);
    assert_eq!(ClassDef::at(&[], 0, TAG).unwrap().class(GlyphId(1)), 0);
    assert!(ClassDef::at(&[0, 1], 4, TAG).is_err());
}

/// Builds a layout table with one script (with default language system
/// enabling every feature), the given features and raw lookups.
pub(crate) fn layout_table(
    script: [u8; 4],
    features: &[(&[u8; 4], &[u16])],
    lookups: &[Vec<u8>],
) -> Vec<u8> {
    layout_table_with_languages(script, &[], features, lookups)
}

/// [`layout_table`] whose script also has the language systems
/// `languages`, each enabling the listed features.
pub(crate) fn layout_table_with_languages(
    script: [u8; 4],
    languages: &[(&[u8; 4], &[u16])],
    features: &[(&[u8; 4], &[u16])],
    lookups: &[Vec<u8>],
) -> Vec<u8> {
    let lang_sys = |indices: &[u16]| {
        let mut out = vec![0, 0, 0xFF, 0xFF];
        out.extend_from_slice(&(indices.len() as u16).to_be_bytes());
        for index in indices {
            out.extend_from_slice(&index.to_be_bytes());
        }
        out
    };
    let every: Vec<u16> = (0..features.len() as u16).collect();
    let tables: Vec<Vec<u8>> = std::iter::once(lang_sys(&every))
        .chain(languages.iter().map(|(_, indices)| lang_sys(indices)))
        .collect();
    let mut offsets = Vec::new();
    let mut at = 4 + 6 * languages.len();
    for table in &tables {
        offsets.push(at as u16);
        at += table.len();
    }
    let mut script_list = vec![0, 1];
    script_list.extend_from_slice(&script);
    script_list.extend_from_slice(&8u16.to_be_bytes());
    script_list.extend_from_slice(&offsets[0].to_be_bytes());
    script_list.extend_from_slice(&(languages.len() as u16).to_be_bytes());
    for ((tag, _), offset) in languages.iter().zip(&offsets[1..]) {
        script_list.extend_from_slice(*tag);
        script_list.extend_from_slice(&offset.to_be_bytes());
    }
    script_list.extend(tables.concat());
    let mut feature_list = (features.len() as u16).to_be_bytes().to_vec();
    let mut feature_body = Vec::new();
    let header = 2 + 6 * features.len();
    for (tag, indices) in features {
        feature_list.extend_from_slice(*tag);
        feature_list.extend_from_slice(&((header + feature_body.len()) as u16).to_be_bytes());
        feature_body.extend_from_slice(&[0, 0]);
        feature_body.extend_from_slice(&(indices.len() as u16).to_be_bytes());
        for index in *indices {
            feature_body.extend_from_slice(&index.to_be_bytes());
        }
    }
    feature_list.extend_from_slice(&feature_body);
    let mut lookup_list = (lookups.len() as u16).to_be_bytes().to_vec();
    let mut lookup_body = Vec::new();
    let header = 2 + 2 * lookups.len();
    for lookup in lookups {
        lookup_list.extend_from_slice(&((header + lookup_body.len()) as u16).to_be_bytes());
        lookup_body.extend_from_slice(lookup);
    }
    lookup_list.extend_from_slice(&lookup_body);
    let mut out = vec![0, 1, 0, 0];
    let scripts_at = 10;
    let features_at = scripts_at + script_list.len();
    let lookups_at = features_at + feature_list.len();
    for v in [scripts_at, features_at, lookups_at] {
        out.extend_from_slice(&(v as u16).to_be_bytes());
    }
    out.extend_from_slice(&script_list);
    out.extend_from_slice(&feature_list);
    out.extend_from_slice(&lookup_list);
    out
}

/// Wraps subtables into a lookup of `kind` with `flags`.
pub(crate) fn lookup(kind: u16, flags: u16, subtables: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for v in [kind, flags, subtables.len() as u16] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    let mut offset = 6 + 2 * subtables.len();
    for sub in subtables {
        out.extend_from_slice(&(offset as u16).to_be_bytes());
        offset += sub.len();
    }
    for sub in subtables {
        out.extend_from_slice(sub);
    }
    out
}

#[test]
fn lists_and_extensions() {
    let single = vec![0, 1, 0, 6, 0, 1, 0, 1, 0, 1, 0, 5];
    let mut extension = vec![0, 1, 0, 1, 0, 0, 0, 8];
    extension.extend_from_slice(&single);
    let data = layout_table(
        *b"latn",
        &[(b"liga", &[0]), (b"kern", &[1])],
        &[
            lookup(1, 0, std::slice::from_ref(&single)),
            lookup(7, 8, &[extension]),
        ],
    );
    let table = LayoutTable::parse(&data, TAG, 7).unwrap();
    assert_eq!(
        table.script_tags().collect::<Vec<_>>(),
        vec![Tag::new(b"latn")]
    );
    let lang = table
        .lang_sys(Tag::new(b"latn"), &[Tag::new(b"ENG ")])
        .unwrap();
    assert_eq!(lang.feature_indices.len(), 2);
    assert!(lang.required_feature.is_none());
    assert!(table.lang_sys(Tag::new(b"grek"), &[]).is_none());
    let (tag, lookups) = table.feature(1).unwrap();
    assert_eq!((tag, lookups.get(0)), (Tag::new(b"kern"), Some(1)));
    let ext = table.lookup(1).unwrap();
    assert_eq!((ext.kind, ext.flags, ext.len()), (1, 8, 1));
    assert_eq!(ext.subtable(0), Some(&single[..]));
    assert_eq!(table.lookup(0).unwrap().subtables().count(), 1);
    assert_eq!(table.lookup_count(), 2);
    assert!(table.lookup(2).is_none());
    assert!(LayoutTable::parse(&data[..8], TAG, 7).is_err());
}
