use super::parse::Dialect;
use super::*;

#[test]
fn parses_nested_documents() {
    let doc = Json::parse(r#"{"a": [1, 2.5, -3e2], "b": {"c": null, "d": true}}"#).unwrap();
    assert_eq!(
        doc.get("a").unwrap().as_f64_vec().unwrap(),
        vec![1.0, 2.5, -300.0]
    );
    assert!(doc.get("b").unwrap().get("c").unwrap().is_null());
    assert_eq!(
        doc.get("b").unwrap().get("d").unwrap().as_bool(),
        Some(true)
    );
    assert_eq!(doc.get("missing"), None);
}

#[test]
fn parses_client_messages_strictly() {
    let message = Json::parse_strict(
        r#"{"type": "motion_notify", "x": 12.5, "y": -3e2, "button": 0,
            "modifiers": ["ctrl", "shift"], "guiEvent": {"isTrusted": true, "key": null}}"#,
    )
    .unwrap();
    assert_eq!(
        message.get("type").and_then(Json::as_str),
        Some("motion_notify")
    );
    assert_eq!(message.get("x").and_then(Json::as_f64), Some(12.5));
    assert_eq!(message.get("y").and_then(Json::as_f64), Some(-300.0));
    assert_eq!(message.get("button").and_then(Json::as_f64), Some(0.0));
    let modifiers = message.get("modifiers").and_then(Json::as_array).unwrap();
    assert_eq!(modifiers.len(), 2);
    let gui = message.get("guiEvent").unwrap();
    assert_eq!(gui.get("isTrusted").and_then(Json::as_bool), Some(true));
    assert_eq!(gui.get("key"), Some(&Json::Null));
}

#[test]
fn integers_and_floats_stay_apart() {
    let doc = Json::parse("[1, -7, 1.0, 1e2, 12345678901234567890]").unwrap();
    let items = doc.as_array().unwrap();
    assert_eq!(items[0], Json::Integer(1));
    assert_eq!(items[1], Json::Integer(-7));
    assert_eq!(items[2], Json::Number(1.0));
    assert_eq!(items[3], Json::Number(100.0));
    assert_eq!(items[4], Json::Number(12_345_678_901_234_567_890.0));
    assert!(items[0].is_integer() && !items[2].is_integer());
    assert_eq!(items[3].as_i64(), Some(100));
    // An integer has no negative zero.
    for parse in [Json::parse, Json::parse_strict] {
        let zero = parse("-0").unwrap().as_f64().unwrap();
        assert!(zero == 0.0 && zero.is_sign_negative());
    }
}

#[test]
fn only_the_python_dialect_reads_non_finite_numbers() {
    let doc = Json::parse("[NaN, Infinity, -Infinity, 1e999]").unwrap();
    let values = doc.as_f64_vec().unwrap();
    assert!(values[0].is_nan());
    assert_eq!(values[1], f64::INFINITY);
    assert_eq!(values[2], f64::NEG_INFINITY);
    assert_eq!(values[3], f64::INFINITY);
    for text in ["NaN", "Infinity", "-Infinity", "1e999", "[-1e999]"] {
        assert!(Json::parse_strict(text).is_err(), "{text:?} should fail");
    }
}

#[test]
fn strings_unescape() {
    let parsed = Json::parse_strict(r#""a\"b\\c\/\n\u00e9\ud83d\ude00\ud800x""#).unwrap();
    assert_eq!(parsed.as_str(), Some("a\"b\\c/\né\u{1f600}\u{fffd}x"));
    assert_eq!(
        Json::parse_strict(r#""\ud800\u0041""#).unwrap().as_str(),
        Some("\u{fffd}A")
    );
    assert_eq!(Json::parse("\"日本\"").unwrap().as_str(), Some("日本"));
    assert_eq!(
        Json::parse(r#""a\n−😀\"""#).unwrap().as_str(),
        Some("a\n\u{2212}😀\"")
    );
}

#[test]
fn malformed_documents_are_rejected() {
    for text in [
        "",
        "{",
        "[1,",
        "[1,]",
        "[1 2]",
        "[1] x",
        "{\"a\" 1}",
        "{\"a\":1,}",
        "01",
        "1.",
        "1e",
        "-",
        "\"abc",
        "\"\u{1}\"",
        "\"\\x\"",
        "\"\\u12\"",
        "\"\\u12g4\"",
        "\"\\u+123\"",
        "tru",
        "{} {}",
        "{1: 2}",
    ] {
        assert!(Json::parse(text).is_err(), "{text:?} should be rejected");
        assert!(
            Json::parse_strict(text).is_err(),
            "{text:?} should be rejected"
        );
    }
    assert!(Json::parse_strict("NaN").is_err());
}

#[test]
fn errors_report_their_offset() {
    let err = Json::parse("[1, x]").unwrap_err();
    assert_eq!(err.offset, 4);
    assert_eq!(
        err.to_string(),
        "invalid JSON at byte 4: unexpected character"
    );
}

#[test]
fn nesting_is_limited() {
    for (parse, depth) in [
        (
            Json::parse_strict as fn(&str) -> _,
            Dialect::STRICT.max_depth,
        ),
        (Json::parse, Dialect::PYTHON.max_depth),
    ] {
        let deep = "[".repeat(depth) + &"]".repeat(depth);
        assert!(parse(&deep).is_ok());
        let too_deep = "[".repeat(depth + 1) + &"]".repeat(depth + 1);
        assert!(parse(&too_deep).is_err());
        // Hostile input fails fast instead of overflowing the stack.
        assert!(parse(&"{\"a\":".repeat(100_000)).is_err());
    }
}

#[test]
fn display_round_trips() {
    let text = r#"{"k": [1.5, "x\"y", null, false, NaN, 3, -Infinity, 1.0]}"#;
    let doc = Json::parse(text).unwrap();
    assert_eq!(doc.to_string(), text);
    assert_eq!(Json::parse(&doc.to_string()).unwrap().to_string(), text);
}

#[test]
fn writes_literals() {
    assert_eq!(string("a\"b\\\n\u{1}é"), r#""a\"b\\\n\u0001é""#);
    assert_eq!(number(640.0), "640");
    assert_eq!(number(-0.5), "-0.5");
    assert_eq!(number(f64::NAN), "null");
    assert_eq!(Json::from(640.0).to_string(), "640.0");
    let round_trip = Json::parse_strict(&string("tab\there")).unwrap();
    assert_eq!(round_trip.as_str(), Some("tab\there"));
}

#[test]
fn converts_from_rust_values() {
    assert_eq!(
        Json::from(vec![Some(1.5), None]),
        Json::Array(vec![Json::Number(1.5), Json::Null])
    );
    assert_eq!(Json::from("s"), Json::String("s".to_owned()));
    assert_eq!(Json::from(true), Json::Bool(true));
}
