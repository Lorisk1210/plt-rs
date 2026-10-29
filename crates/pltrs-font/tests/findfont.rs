//! Font catalogue entries and `findfont` results compared with matplotlib's
//! font manager restricted to its bundled fonts
//! (`tests/fixtures/findfont.json`); Inter, which pltrs bundles besides
//! them, is left out of the catalogue comparison.

mod common;

use common::fixture;
use common::json::{Fixture, Json};
use pltrs_font::{FontDatabase, FontQuery, FontSource, FontStyle, FontVariant, Stretch, Weight};

fn file_of(db: &FontDatabase, id: pltrs_font::FontId) -> String {
    db.source(id)
        .and_then(FontSource::file_name)
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn catalogue_matches_matplotlib_ttflist() {
    let expected = fixture("findfont.json");
    let db = FontDatabase::with_bundled_fonts();
    let mut actual: Vec<String> = db
        .entries()
        .iter()
        .filter(|e| !file_of(&db, e.font).starts_with("Inter-"))
        .map(|e| {
            format!(
                "{}|{}|{:?}|{:?}|{}|{}",
                file_of(&db, e.font),
                e.name,
                e.style,
                e.variant,
                e.weight,
                e.stretch.0
            )
        })
        .collect();
    let mut wanted: Vec<String> = expected
        .field("entries")
        .array()
        .iter()
        .map(|e| {
            let style = e.field("style").str().parse::<FontStyle>().unwrap();
            let variant = if e.field("variant").str() == "small-caps" {
                FontVariant::SmallCaps
            } else {
                FontVariant::Normal
            };
            let stretch = e.field("stretch").str().parse::<Stretch>().unwrap();
            format!(
                "{}|{}|{style:?}|{variant:?}|{}|{}",
                e.field("file").str(),
                e.field("name").str(),
                e.field("weight").i64(),
                stretch.0
            )
        })
        .collect();
    actual.sort();
    wanted.sort();
    assert_eq!(actual, wanted);
}

#[test]
fn findfont_matches_matplotlib() {
    let expected = fixture("findfont.json");
    let db = FontDatabase::with_bundled_fonts();
    let mut failures = Vec::new();
    let queries = expected.field("queries").array();
    for query in queries {
        let families: Vec<String> = query
            .field("families")
            .array()
            .iter()
            .map(|f| f.str().to_owned())
            .collect();
        let request = FontQuery {
            families,
            style: query.field("style").str().parse::<FontStyle>().unwrap(),
            variant: if query.field("variant").str() == "small-caps" {
                FontVariant::SmallCaps
            } else {
                FontVariant::Normal
            },
            weight: query.field("weight").str().parse::<Weight>().unwrap(),
            stretch: query.field("stretch").str().parse::<Stretch>().unwrap(),
            size: 10.0,
        };
        let found = file_of(&db, db.find_font(&request).unwrap());
        let ties: Vec<&str> = query.field("ties").array().iter().map(Json::str).collect();
        let ok = if ties.len() > 1 {
            ties.contains(&found.as_str())
        } else {
            found == query.field("file").str()
        };
        if !ok {
            failures.push(format!(
                "{request}: {found} != {}",
                query.field("file").str()
            ));
        }
    }
    assert!(queries.len() > 500);
    assert!(
        failures.is_empty(),
        "{} of {} queries differ:\n{}",
        failures.len(),
        queries.len(),
        failures.join("\n")
    );
}
