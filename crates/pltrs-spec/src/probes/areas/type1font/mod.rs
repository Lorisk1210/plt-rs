//! Probe operations for the `type1font` area: matplotlib's `_type1font`
//! (after its `tests/test_type1font.py`) and `FreeType`'s Type 1 driver as
//! `FT2Font` loads `.pfb` files (see
//! `spec/reference/probes/test_type1font.py`). The `FT2Font` operations
//! are in [`freetype`].

mod freetype;

use std::fmt::Write;
use std::path::PathBuf;

use pltrs_codec::sha1::sha1;
use pltrs_font::{
    PsNumber, PsValue, Type1Cipher, Type1Effects, Type1Encoding, Type1Font, Type1Parts, ps_tokenize,
};

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, made};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("_type1font.Type1Font", type1_font),
    Op::new("_type1font.Type1Font.transform", transform),
    Op::new("_type1font.Type1Font.with_encoding", with_encoding),
    Op::new("_type1font.Type1Font.subset", subset),
    Op::new("_type1font.Type1Font.pfa", pfa),
    Op::new("_type1font._tokenize", tokenize),
    Op::new("_type1font.Type1Font._encrypt", encrypt),
    Op::new("ft2font.FT2Font.attrs", freetype::attrs),
    Op::new("ft2font.FT2Font.load_glyph", freetype::load_glyph),
    Op::new("ft2font.FT2Font.load_glyph.sized", freetype::load_sized),
    Op::new("ft2font.FT2Font.get_name_index", freetype::name_index),
    Op::new(
        "ft2font.FT2Font._get_type1_encoding_vector",
        freetype::encoding_vector,
    ),
];

/// The path of the Type 1 fixture `file` of `pltrs-font`'s tests.
fn fixture_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pltrs-font/tests/fixtures/type1")
        .join(file)
}

/// The fixture named by the `file` argument.
fn font(args: Args<'_>) -> Result<Type1Font, ProbeError> {
    let file = args.str("file")?;
    let data = std::fs::read(fixture_path(file))
        .map_err(|err| ProbeError::BadArgs(format!("cannot read {file}: {err}")))?;
    made(Type1Font::from_bytes(&data))
}

/// A string argument encoded as Latin-1, as the reference encodes it.
fn latin1(args: Args<'_>, key: &str) -> Result<Vec<u8>, ProbeError> {
    args.str(key)?
        .chars()
        .map(|c| {
            u8::try_from(u32::from(c))
                .map_err(|_| ProbeError::BadArgs(format!("{key} is not Latin-1")))
        })
        .collect()
}

/// Bytes as Latin-1 text.
pub(crate) fn text(bytes: &[u8]) -> Json {
    Json::String(bytes.iter().map(|&b| char::from(b)).collect())
}

/// Lowercase hexadecimal digits of `bytes` (Python's `bytes.hex()`).
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut hex, b| {
        write!(hex, "{b:02x}").expect("writing to a String cannot fail");
        hex
    })
}

/// `[length, sha1 hex digest]`.
fn digest(bytes: &[u8]) -> Json {
    Json::Array(vec![
        Json::Integer(bytes.len() as i64),
        hex(&sha1(bytes)).into(),
    ])
}

fn number(value: PsNumber) -> Json {
    match value {
        PsNumber::Int(value) => Json::Integer(value),
        PsNumber::Real(value) => Json::Number(value),
    }
}

/// A `prop` value: `<...>` strings as `{"hex": ...}`.
fn value(value: &PsValue) -> Json {
    match value {
        PsValue::Number(n) => number(*n),
        PsValue::Bool(b) => Json::Bool(*b),
        PsValue::Name(s) | PsValue::Text(s) | PsValue::Expression(s) => s.as_str().into(),
        PsValue::Bytes(bytes) => Json::Object(vec![("hex".into(), hex(bytes).into())]),
    }
}

/// An encoding as sorted `[code, name]` pairs.
fn encoding(encoding: &Type1Encoding) -> Json {
    Json::Array(
        encoding
            .iter()
            .map(|(code, name)| Json::Array(vec![Json::Integer(code), name.into()]))
            .collect(),
    )
}

fn digests<'a>(parts: impl IntoIterator<Item = &'a [u8]>) -> Json {
    Json::Array(parts.into_iter().map(digest).collect())
}

fn part_digests(parts: &Type1Parts) -> Json {
    digests([parts.cleartext.as_slice(), &parts.encrypted, &parts.finale])
}

/// The `prop` dictionary.
fn prop(font: &Type1Font) -> Json {
    let prop = font.prop();
    let mut members: Vec<(String, Json)> = prop
        .entries()
        .map(|(key, v)| (key.to_owned(), value(v)))
        .collect();
    let bbox = prop.font_bbox().map(number).to_vec();
    members.push(("FontBBox".into(), Json::Array(bbox)));
    if let Some(enc) = prop.encoding() {
        members.push(("Encoding".into(), encoding(enc)));
    }
    let char_strings = prop.char_strings();
    let glyphs = char_strings
        .names()
        .filter_map(|name| Some((name.to_owned(), digest(char_strings.get(name)?))))
        .collect();
    members.push(("CharStrings".into(), Json::Object(glyphs)));
    if let Some(subrs) = prop.subrs() {
        members.push((
            "Subrs".into(),
            digests(subrs.iter().map(pltrs_font::Program::decrypted)),
        ));
    }
    if let Some(other) = prop.other_subrs() {
        members.push(("OtherSubrs".into(), digest(other)));
    }
    Json::Object(members)
}

/// The summary of a font: digests, `prop`, `_pos` and `_abbr`.
fn summary(font: &Type1Font) -> Vec<(String, Json)> {
    let pos = font
        .span_keys()
        .map(|key| {
            let spans = font
                .spans(key)
                .iter()
                .map(|&(start, end)| {
                    Json::Array(vec![Json::Integer(start as i64), Json::Integer(end as i64)])
                })
                .collect();
            (key.to_owned(), Json::Array(spans))
        })
        .collect();
    let abbr = font.abbreviations();
    vec![
        ("parts".into(), part_digests(font.parts())),
        ("decrypted".into(), digest(font.decrypted())),
        ("prop".into(), prop(font)),
        ("pos".into(), Json::Object(pos)),
        (
            "abbr".into(),
            Json::Object(vec![
                ("RD".into(), abbr.rd.as_str().into()),
                ("ND".into(), abbr.nd.as_str().into()),
                ("NP".into(), abbr.np.as_str().into()),
            ]),
        ),
    ]
}

fn type1_font(args: Args<'_>) -> Outcome {
    let font = font(args)?;
    let decrypted = font.decrypted();
    let mut members = summary(&font);
    members.push(("head".into(), text(&decrypted[..decrypted.len().min(40)])));
    members.push((
        "tail".into(),
        text(&decrypted[decrypted.len().saturating_sub(40)..]),
    ));
    Ok(Json::Object(members))
}

/// The `effects` argument: `{"slant": ..., "extend": ...}`.
fn effects(json: &Json) -> Result<Type1Effects, ProbeError> {
    let get = |key| {
        json.get(key)
            .map(|v| {
                v.as_f64()
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad {key}")))
            })
            .transpose()
    };
    Ok(Type1Effects {
        slant: get("slant")?,
        extend: get("extend")?,
    })
}

fn transform(args: Args<'_>) -> Outcome {
    let font = made(font(args)?.transform(effects(args.json("effects")?)?))?;
    let prop = font.prop();
    let entry = |key: &str| prop.get(key).map_or(Json::Null, value);
    Ok(Json::Object(vec![
        ("cleartext".into(), text(&font.parts().cleartext)),
        ("parts".into(), part_digests(font.parts())),
        ("decrypted".into(), digest(font.decrypted())),
        (
            "prop".into(),
            Json::Object(
                ["FontName", "ItalicAngle", "FontMatrix"]
                    .into_iter()
                    .map(|key| (key.to_owned(), entry(key)))
                    .collect(),
            ),
        ),
        (
            "pos".into(),
            Json::Array(font.span_keys().map(Json::from).collect()),
        ),
    ]))
}

/// The `encoding` argument: `[code, name]` pairs.
fn encoding_arg(args: Args<'_>) -> Result<Type1Encoding, ProbeError> {
    let bad = || ProbeError::BadArgs("bad encoding".into());
    let pairs = args.json("encoding")?.as_array().ok_or_else(bad)?;
    let codes = pairs
        .iter()
        .map(|pair| match pair.as_array() {
            Some([code, name]) => Ok((
                code.as_f64().ok_or_else(bad)? as i64,
                name.as_str().ok_or_else(bad)?.to_owned(),
            )),
            _ => Err(bad()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Type1Encoding::from_codes(codes))
}

fn with_encoding(args: Args<'_>) -> Outcome {
    let font = made(font(args)?.with_encoding(&encoding_arg(args)?))?;
    Ok(Json::Object(vec![
        ("parts".into(), part_digests(font.parts())),
        ("decrypted".into(), digest(font.decrypted())),
        (
            "encoding".into(),
            font.prop().encoding().map_or(Json::Null, encoding),
        ),
    ]))
}

fn subset(args: Args<'_>) -> Outcome {
    let chars = args.str("chars")?.chars().map(u32::from);
    let font = made(font(args)?.subset(chars, args.str("prefix")?))?;
    let prop = font.prop();
    let mut glyphs: Vec<&str> = prop.char_strings().names().collect();
    glyphs.sort_unstable();
    Ok(Json::Object(vec![
        ("cleartext".into(), text(&font.parts().cleartext)),
        ("parts".into(), part_digests(font.parts())),
        ("decrypted".into(), digest(font.decrypted())),
        ("font_name".into(), value(prop.font_name())),
        (
            "encoding".into(),
            prop.encoding().map_or(Json::Null, encoding),
        ),
        (
            "glyphs".into(),
            Json::Array(glyphs.into_iter().map(Json::from).collect()),
        ),
        (
            "subrs".into(),
            digests(
                prop.subrs()
                    .unwrap_or_default()
                    .iter()
                    .map(pltrs_font::Program::decrypted),
            ),
        ),
    ]))
}

/// A PFA font as matplotlib's tests write one around `private`.
fn pfa(args: Args<'_>) -> Outcome {
    let mut plain = b"/FontName /X def\n/FontBBox [0 0 1 1] def\n".to_vec();
    plain.extend(latin1(args, "private")?);
    let hex = hex(&Type1Font::encrypt(&plain, Type1Cipher::Eexec));
    let data = format!(
        "%!PS-AdobeFont-1.0: X 001.000\neexec\n{hex}\n{}\ncleartomark\n",
        "0".repeat(512)
    );
    let font = made(Type1Font::from_bytes(data.as_bytes()))?;
    Ok(Json::Object(summary(&font)))
}

fn tokenize(args: Args<'_>) -> Outcome {
    let tokens = made(ps_tokenize(&latin1(args, "data")?, args.bool("skip_ws")?))?;
    Ok(Json::Array(
        tokens
            .iter()
            .map(|token| Json::Array(vec![token.kind.name().into(), value(&token.value)]))
            .collect(),
    ))
}

fn encrypt(args: Args<'_>) -> Outcome {
    let data = latin1(args, "data")?;
    let cipher = match args.str("key")? {
        "eexec" => Type1Cipher::Eexec,
        "charstring" => Type1Cipher::Charstring,
        other => return Err(ProbeError::BadArgs(format!("unknown key {other:?}"))),
    };
    let encrypted = Type1Font::encrypt(&data, cipher);
    Ok(Json::Object(vec![
        ("encrypted".into(), hex(&encrypted).into()),
        (
            "decrypted".into(),
            Json::Bool(Type1Font::decrypt(&encrypted, cipher) == data),
        ),
    ]))
}
