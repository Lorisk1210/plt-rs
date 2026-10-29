use super::*;
use crate::tex::files::MemoryFinder;

/// matplotlib's `tests/baseline_images/dviread/test.map`.
const MAP: &str = r#"% used by test_dviread.py
TeXfont1 PSfont1 <font1.pfb <font1.enc
TeXfont2 PSfont2 <font2.enc <font2.pfa
TeXfont3 PSfont3 "1.23 UnknownEffect" <[enc3.foo < font3.pfa
TeXfont4 PSfont4 "-0.1 SlantFont 1.2 ExtendFont" <font4.enc <font4.pfa
TeXfont5 PSfont5 <encoding1.enc <encoding2.enc <font5.pfb
TeXfont6 PSfont6
TeXfont7 PSfont7 < font7.enc
TeXfont8 PSfont8 <<font8.pfb
TeXfont9 </absolute/font9.pfb
% Only the first of a duplicate key is used.
TeXfontA PSfontA1
TeXfontA PSfontA2
%
% Check SlantFont/ExtendFont
%
% Options are only allowed on T1 fonts (3 TrueType, 1 bitmap, and 1 T1 below).
TeXfontB PSfontB1 "-0.1 SlantFont 1.2 ExtendFont" <fontB1.enc <fontB1.ttf
TeXfontB PSfontB2 "-0.1 SlantFont" <fontB2.enc <fontB2.ttc
TeXfontB PSfontB3 "1.2 ExtendFont" <fontB3.enc <fontB3.otf
TeXfontB "0.1 SlantFont 1.2 ExtendFont"
% Also, only within range ±1 / ±2.
TeXfontB PSfontB4 "1.1 SlantFont 1.2 ExtendFont"
TeXfontB PSfontB5 "0.1 SlantFont 2.2 ExtendFont"
% This is the only one that works:
TeXfontB PSfontB6
%
% Invalid TrueType entry.
%
% Must have encodingfile if subsetted.
TeXfontC PSfontC1 <fontC1.ttf
TeXfontC PSfontC2 <fontC2.ttc
TeXfontC PSfontC3 <fontC3.ttf <8r.enc
"#;

/// Finds every file by its own name, as `test_PsfontsMap` arranges.
#[derive(Debug)]
struct Identity;

impl FileFinder for Identity {
    fn find(&self, name: &str) -> Result<PathBuf, TexError> {
        Ok(PathBuf::from(name))
    }
}

/// matplotlib's `test_PsfontsMap`.
#[test]
fn maps_read_as_matplotlib_reads_them() {
    let path = |name: &str| Some(PathBuf::from(name));
    let map = PsfontsMap::parse(MAP.as_bytes(), "test.map");
    let get = |name: &[u8]| map.get(name, &Identity);
    for n in 1..=5 {
        let key = format!("TeXfont{n}");
        let entry = get(key.as_bytes()).unwrap();
        assert_eq!(entry.texname, key.as_bytes());
        assert_eq!(entry.psname, format!("PSfont{n}").as_bytes());
        match n {
            3 => assert_eq!(entry.encoding, path("enc3.foo")),
            5 => {}
            n => assert_eq!(entry.encoding, path(&format!("font{n}.enc"))),
        }
        let extension = if n == 1 || n == 5 { "pfb" } else { "pfa" };
        assert_eq!(entry.filename, path(&format!("font{n}.{extension}")));
        let effects = if n == 4 {
            Type1Effects {
                slant: Some(-0.1),
                extend: Some(1.2),
            }
        } else {
            Type1Effects::default()
        };
        assert_eq!(entry.effects, effects);
    }
    let six = get(b"TeXfont6").unwrap();
    assert_eq!((six.filename, six.encoding), (None, None));
    let seven = get(b"TeXfont7").unwrap();
    assert_eq!((seven.filename, seven.encoding), (None, path("font7.enc")));
    let eight = get(b"TeXfont8").unwrap();
    assert_eq!((eight.filename, eight.encoding), (path("font8.pfb"), None));
    let nine = get(b"TeXfont9").unwrap();
    assert_eq!(nine.psname, b"TeXfont9");
    assert_eq!(nine.filename, path("/absolute/font9.pfb"));
    assert_eq!(get(b"TeXfontA").unwrap().psname, b"PSfontA1");
    assert_eq!(get(b"TeXfontB").unwrap().psname, b"PSfontB6");
    assert_eq!(get(b"TeXfontC").unwrap().psname, b"PSfontC3");
    let missing = get(b"no-such-font").unwrap_err();
    assert!(matches!(missing, TexError::UnmappedFont { .. }));
    assert!(missing.to_string().contains("no-such-font"));
    assert!(get(b"%").unwrap_err().to_string().contains('%'));
}

#[test]
fn files_a_map_names_must_exist() {
    let map = PsfontsMap::parse(b"cmr10 CMR10 <cmr10.pfb\n", "pdftex.map");
    assert_eq!(
        map.get(b"cmr10", &MemoryFinder::default()),
        Err(TexError::NotFound {
            name: "cmr10.pfb".into()
        })
    );
    let bad = PsfontsMap::parse(b"x X \"SlantFont\" <x.pfb\n", "m");
    assert!(bad.get(b"x", &Identity).is_err());
    let dangling = PsfontsMap::parse(b"x X <\n", "m");
    assert!(dangling.get(b"x", &Identity).is_err());
}

#[test]
fn floats_parse_as_python_parses_them() {
    for (text, value) in [
        (&b".167"[..], Some(0.167)),
        (b" -1.5 ", Some(-1.5)),
        (b"1_0", Some(10.0)),
        (b"inf", Some(f64::INFINITY)),
        (b"1__0", None),
        (b"_1", None),
        (b"--1", None),
        (b"0x10", None),
        (b"", None),
    ] {
        assert_eq!(python_float(text), value, "{text:?}");
    }
    assert!(python_float(b"nan").is_some_and(f64::is_nan));
}

#[test]
fn encodings_are_arrays_of_names() {
    let enc = b"% comment [ /not ]\n/TeXBase1Encoding [ % in the array\n/a /b\n/c ] def\n";
    assert_eq!(parse_enc(enc).unwrap(), ["a", "b", "c"]);
    assert!(parse_enc(b"[ /a b ]").is_err());
    assert!(parse_enc(b"no array").is_err());
    assert!(parse_enc("[ /\u{e9} ]".as_bytes()).is_err());
    assert_eq!(parse_enc(b"[]").unwrap(), Vec::<String>::new());
}

/// Finds every file by its own name, slowly.
#[derive(Debug)]
struct Slow;

impl FileFinder for Slow {
    fn find(&self, name: &str) -> Result<PathBuf, TexError> {
        std::thread::sleep(std::time::Duration::from_millis(5));
        Ok(PathBuf::from(name))
    }
}

#[test]
fn fonts_being_parsed_are_found_by_other_threads() {
    let map = PsfontsMap::parse(MAP.as_bytes(), "test.map");
    std::thread::scope(|scope| {
        let lookups: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| map.get(b"TeXfont1", &Slow)))
            .collect();
        for lookup in lookups {
            let font = lookup.join().expect("no panic").expect("mapped");
            assert_eq!(font.psname, b"PSfont1");
        }
    });
}
