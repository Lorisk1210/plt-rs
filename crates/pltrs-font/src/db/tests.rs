//! Font database tests: the bundled catalogue, matching and fallback,
//! entries added and removed, and files used without being catalogued.

use super::*;

#[test]
fn bundled_fonts_are_catalogued() {
    let db = FontDatabase::with_bundled_fonts();
    assert_eq!(db.faces.len(), BUNDLED_FONTS.len());
    assert_eq!(
        db.faces_from(&FontSource::Bundled(LAST_RESORT)),
        db.last_resort().into_iter().collect::<Vec<_>>()
    );
    assert!(db.entries().iter().any(|entry| entry.name == "DejaVu Sans"));
    let resort = db.last_resort().unwrap();
    assert_eq!(
        db.source(resort).and_then(FontSource::file_name),
        Some(LAST_RESORT)
    );
    assert_eq!(db.face_index(resort), Some(0));
    assert!(db.face(FontId(usize::MAX)).is_err());
    for index in 0..db.faces.len() {
        assert!(db.face(FontId(index)).is_ok());
    }
}

#[test]
fn find_font_basics() {
    let db = FontDatabase::with_bundled_fonts();
    let name = |id: FontId| {
        db.source(id)
            .and_then(FontSource::file_name)
            .unwrap_or_default()
            .to_owned()
    };
    assert_eq!(
        name(db.find_font(&FontQuery::default()).unwrap()),
        "DejaVuSans.ttf"
    );
    let bold = FontQuery::families(["serif"]).with_weight(Weight::BOLD);
    assert_eq!(name(db.find_font(&bold).unwrap()), "DejaVuSerif-Bold.ttf");
    let missing = FontQuery::families(["No Such Font"]);
    assert_eq!(name(db.find_font(&missing).unwrap()), "DejaVuSans.ttf");
    assert!(db.find_font_exact(&missing).is_err());
    let fonts = db
        .find_fonts(&FontQuery::families([
            "cmr10",
            "No Such Font",
            "DejaVu Sans",
        ]))
        .unwrap();
    assert_eq!(fonts.len(), 2);
    let chain = db.font_chain(&fonts, true).unwrap();
    assert_eq!(chain.faces().len(), 3);
    assert_eq!(
        FontDatabase::new().find_font(&FontQuery::default()),
        Err(FontError::NoMatchingFont(FontQuery::default().to_string()))
    );
}

#[test]
fn best_entries_carry_their_score() {
    let db = FontDatabase::with_bundled_fonts();
    let generics = db.generic_families().clone();
    let heavy = FontQuery::families(["DejaVu Sans"]).with_weight(Weight(750));
    let (entry, score) = db.best_entry(&heavy, &generics, |_| true).unwrap();
    assert_eq!((entry.name.as_str(), entry.weight), ("DejaVu Sans", 700));
    assert!(score < 10.0);
    let missing = FontQuery::families(["No Such Font"]);
    let (_, score) = db.best_entry(&missing, &generics, |_| true).unwrap();
    assert!(score >= 10.0);
    let serif = db.find_font(&FontQuery::families(["serif"])).unwrap();
    let (entry, _) = db
        .best_entry(&heavy, &generics, |font| font == serif)
        .unwrap();
    assert_eq!(entry.font, serif);
    assert!(db.best_entry(&heavy, &generics, |_| false).is_none());
}

#[test]
fn entries_are_added_and_removed() {
    let db = FontDatabase::with_bundled_fonts();
    let serif = db.find_font(&FontQuery::families(["serif"])).unwrap();
    let entry = FontEntry {
        font: serif,
        name: "Alias".to_owned(),
        ..db.entries()[0].clone()
    };
    let alias = FontQuery::families(["Alias"]);
    assert!(db.find_font_exact(&alias).is_err());
    db.add_entry(entry.clone());
    assert_eq!(db.find_font_exact(&alias), Ok(serif));
    assert!(db.remove_entry(&entry));
    assert!(!db.remove_entry(&entry));
    assert!(db.find_font_exact(&alias).is_err());
}

#[test]
fn bundled_files_are_available_by_name() {
    let inter = bundled_font_data("Inter-Regular.ttf").unwrap();
    assert_eq!(face_count(inter), Ok(1));
    assert!(bundled_font_data("Inter-Thin.ttf").is_none());
}

#[test]
fn inter_is_found_by_name_only() {
    let db = FontDatabase::with_bundled_fonts();
    let name = |query: &FontQuery| {
        let id = db.find_font(query).unwrap();
        db.source(id)
            .and_then(FontSource::file_name)
            .unwrap()
            .to_owned()
    };
    let semibold = FontQuery::families(["Inter", "DejaVu Sans"]).with_weight(Weight(600));
    assert_eq!(name(&semibold), "Inter-SemiBold.ttf");
    let italic = FontQuery::families(["Inter"]).with_style(FontStyle::Italic);
    assert_eq!(name(&italic), "Inter-Italic.ttf");
    // matplotlib's lists do not name Inter: their queries are unchanged.
    let sans = FontQuery::families(["sans-serif"]).with_weight(Weight(600));
    assert_eq!(name(&sans), "DejaVuSans-Bold.ttf");
    let unknown = FontQuery::families(["Helvetica"]).with_weight(Weight(600));
    assert_eq!(name(&unknown), "DejaVuSans-Bold.ttf");
}

#[test]
fn font_files_can_be_used_without_being_catalogued() {
    let db = FontDatabase::with_bundled_fonts();
    let bundled = db.file_faces("cmr10.ttf").unwrap();
    assert_eq!(
        bundled,
        db.faces_where(|source| source.file_name() == Some("cmr10.ttf"))
    );
    let count = db.face_count();
    let path = std::env::temp_dir().join(format!("pltrs-font-file-{}.ttf", std::process::id()));
    fs::write(&path, bundled_font_data("DejaVuSans-Bold.ttf").unwrap()).unwrap();
    let entries = db.entries().len();
    let faces = db.file_faces(&path).unwrap();
    assert_eq!((faces.len(), db.face_count()), (1, count + 1));
    assert_eq!(db.entries().len(), entries, "not catalogued");
    assert_eq!(db.file_faces(&path).unwrap(), faces, "read once");
    db.add_file(&path).unwrap();
    assert!(db.entries().len() > entries, "catalogued by add_file");
    fs::remove_file(path).unwrap();
}

#[test]
fn unreadable_files_are_reported() {
    let db = FontDatabase::new();
    assert!(matches!(
        db.add_file("/nonexistent/font.ttf"),
        Err(FontError::Io { .. })
    ));
    assert_eq!(db.add_directory("/nonexistent"), 0);
}
