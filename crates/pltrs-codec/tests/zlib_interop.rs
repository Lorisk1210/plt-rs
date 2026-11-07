//! Interoperability with the reference zlib: streams written by Python's
//! zlib (every level, strategy, window size and flush mode) must inflate,
//! and our output (zlib-ng's, pinned by `zlib_ng.rs`) must be about as
//! small as classic zlib's at the default and best levels.

use std::fs;
use std::path::{Path, PathBuf};

use pltrs_codec::{CompressionLevel, deflate, zlib};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zlib")
}

fn read(name: &str) -> Vec<u8> {
    fs::read(fixture_dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn inflates_every_reference_stream() {
    let corpus = read("corpus.bin");
    let mut streams = 0;
    for entry in fs::read_dir(fixture_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "zlib") {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let decoded = zlib::decompress(&fs::read(&path).unwrap())
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(decoded == corpus, "{name} decodes to different data");
            streams += 1;
        }
    }
    assert_eq!(streams, 16);
    assert_eq!(
        deflate::decompress(&read("level9_raw.deflate")).unwrap(),
        corpus
    );
}

#[test]
fn compression_ratio_tracks_zlib() {
    let corpus = read("corpus.bin");
    for level in [6, 9] {
        let ours = zlib::compress(&corpus, CompressionLevel::new(level).unwrap()).unwrap();
        let theirs = read(&format!("level{level}.zlib"));
        assert!(
            ours.len() * 100 <= theirs.len() * 101,
            "level {level}: {} bytes vs zlib's {}",
            ours.len(),
            theirs.len()
        );
        assert_eq!(zlib::decompress(&ours).unwrap(), corpus);
    }
    let stored = zlib::compress(&corpus, CompressionLevel::NONE).unwrap();
    assert_eq!(stored, read("level0.zlib"), "stored blocks are zlib's");
}
