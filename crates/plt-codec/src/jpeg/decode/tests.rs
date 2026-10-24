use super::*;
use crate::jpeg::marker::{DHT, SOF0};
use crate::jpeg::{JpegOptions, encode_rgb8};
use crate::test_util::{PillowFile, Rng, assert_decodes_like_pillow, mutate, pillow_bytes};

/// Small files Pillow wrote: a label, the size, the CRC-32 of the array
/// matplotlib's `imread` returns for them, and the file in base64.
const PILLOW: [PillowFile; 5] = [
    (
        "progressive 4:2:0 restarts",
        19,
        13,
        0xB16E_6DDD,
        "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAYEBQYFBAYGBQYHBwYIChAKCgkJChQODwwQFxQYGBcUFhYaHSUfGhsjHBYWICwgIyYnKSopGR8tMC0oMCUoKSj/2wBDAQcHBwoIChMKChMoGhYaKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCj/wgARCAANABMDASIAAhEBAxEB/8QAGAAAAgMAAAAAAAAAAAAAAAAAAAQBAwb/xAAXAQADAQAAAAAAAAAAAAAAAAACAwQF/90ABAAC/9oADAMBAAIQAxAAAAFGLHKWPGkMcP/EABsQAAICAwEAAAAAAAAAAAAAAAECAxEEEiNB/9oACAEBAAEFAoV5iO2//9AQmvP/0cMW6INf/8QAGxEAAQQDAAAAAAAAAAAAAAAAAgABAwUREjH/2gAIAQMBAT8Bu5H2GMUItji//8QAGhEAAgMBAQAAAAAAAAAAAAAAAQIABCERE//aAAgBAgEBPwFLSWlPoNhonuGf/8QAFxABAAMAAAAAAAAAAAAAAAAAAAEhof/aAAgBAQAGPwJT/9Bj/9FD/8QAGxAAAwADAQEAAAAAAAAAAAAAAAERITFBcaH/2gAIAQEAAT8hkjeLj0drQ//QmRL4PELsH//RQniNKP/aAAwDAQACAAMAAAAQlD//xAAXEQEBAQEAAAAAAAAAAAAAAAARACFB/9oACAEDAQE/EHBkC8X/xAAYEQACAwAAAAAAAAAAAAAAAAAAAREhMf/aAAgBAgEBPxBLWCcwP//EAB4QAQABBAMBAQAAAAAAAAAAAAERACFBUTGB0XGx/9oACAEBAAE/EBQqEC2Wt8FfNlTOo6v3X//QU8JDT9pIxE+C3Djqv//RNLpMg6Y9pqFiTn2v/9k=",
    ),
    (
        "baseline 4:2:2",
        21,
        9,
        0xF62B_E2AF,
        "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAoHBwgHBgoICAgLCgoLDhgQDg0NDh0VFhEYIx8lJCIfIiEmKzcvJik0KSEiMEExNDk7Pj4+JS5ESUM8SDc9Pjv/2wBDAQoLCw4NDhwQEBw7KCIoOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozv/wAARCAAJABUDASEAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwDPsYAtoAy546/1oFm0lzlcMQ2emP8A9Ve3iIQxdNzS1OviWtywhTOz0nTmS1H3VyB1OOfyor8vrRrKpJJ9SaE4xpxT/X/M5GP/AFf/AAFqn0f/AI/Y/p/Sv0nLf92l8zn4p/jQ+R3Fr/qV+lFfIVv4jGtl8j//2Q==",
    ),
    (
        "grey progressive",
        11,
        10,
        0x5996_7CF5,
        "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/wgALCAAKAAsBAREA/8QAFQABAQAAAAAAAAAAAAAAAAAABQb/2gAIAQEAAAABlnz0v//EABkQAAIDAQAAAAAAAAAAAAAAAAMhAAECMv/aAAgBAQABBQIOHQ0Dmf/EABcQAAMBAAAAAAAAAAAAAAAAAAAQEUL/2gAIAQEABj8Chp//xAAcEAACAgIDAAAAAAAAAAAAAAAAASExEWFxkcH/2gAIAQEAAT8hZRtDsI6E159HZ//aAAgBAQAAABCf/8QAGxABAQACAwEAAAAAAAAAAAAAAREAITFBUWH/2gAIAQEAAT8QPQiKjdHvzFNC7m8vXOErBot71zZX2Z//2Q==",
    ),
    (
        "CMYK",
        7,
        5,
        0x4E76_3DB1,
        "/9j/7gAOQWRvYmUAZAAAAAAA/9sAQwAIBgYHBgUIBwcHCQkICgwUDQwLCwwZEhMPFB0aHx4dGhwcICQuJyAiLCMcHCg3KSwwMTQ0NB8nOT04MjwuMzQy/8AAFAgABQAHBEMRAE0RAFkRAEsRAP/EAB8AAAEFAQEBAQEBAAAAAAAAAAABAgMEBQYHCAkKC//EALUQAAIBAwMCBAMFBQQEAAABfQECAwAEEQUSITFBBhNRYQcicRQygZGhCCNCscEVUtHwJDNicoIJChYXGBkaJSYnKCkqNDU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6g4SFhoeIiYqSk5SVlpeYmZqio6Slpqeoqaqys7S1tre4ubrCw8TFxsfIycrS09TV1tfY2drh4uPk5ebn6Onq8fLz9PX29/j5+v/aAA4EQwBNAFkASwAAPwDzW7ZdQ8QXMjrj+PBwR1Ix0r1HxJdPaygLkkHg5xgHntXnM0JsbzbBK6Aqx+U4wM9B+X61m3uqnRvDNsdUhGtWTS3E81rcN5auYnSHA2jABeVpOVPRRyRvr//Z",
    ),
    (
        "progressive cut after 3 scans",
        24,
        16,
        0x69C1_48C9,
        "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAA0JCgsKCA0LCgsODg0PEyAVExISEyccHhcgLikxMC4pLSwzOko+MzZGNywtQFdBRkxOUlNSMj5aYVpQYEpRUk//2wBDAQ4ODhMREyYVFSZPNS01T09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT09PT0//wgARCAAQABgDASIAAhEBAxEB/8QAFwAAAwEAAAAAAAAAAAAAAAAAAAEDBP/EABcBAAMBAAAAAAAAAAAAAAAAAAECAwT/2gAMAwEAAhADEAAAAYN1o24oYz//xAAcEAADAAEFAAAAAAAAAAAAAAABAgMRABMhIjH/2gAIAQEAAQUCUdQudSXA8EuTVtqH/8QAGBEAAwEBAAAAAAAAAAAAAAAAAAECIQT/2gAIAQMBAT8B7r1SiZprD//EABsRAAEEAwAAAAAAAAAAAAAAAAIAAQMEEyEi/9k=",
    ),
];

fn pillow(index: usize) -> Vec<u8> {
    pillow_bytes(&PILLOW[index])
}

/// The offset of the first `marker` segment of a file.
fn find(data: &[u8], marker: u8) -> usize {
    data.windows(2).position(|w| w == [0xFF, marker]).unwrap()
}

#[test]
fn pillow_files_decode_exactly() {
    assert_decodes_like_pillow(&PILLOW, decode);
}

#[test]
fn baseline_files_from_the_encoder_round_trip() {
    let (width, height) = (19, 13);
    let rgb: Vec<u8> = (0..width * height)
        .flat_map(|i| {
            let (x, y) = (i % width, i / width);
            [(x * 13) as u8, (y * 17) as u8, ((x + y) * 5) as u8]
        })
        .collect();
    let options = JpegOptions {
        quality: 95,
        dpi: 72,
    };
    let file = encode_rgb8(width as u32, height as u32, &rgb, &options).unwrap();
    let image = decode(&file).unwrap();
    assert_eq!((image.width(), image.height()), (19, 13));
    let Pixels::Rgb(pixels) = image.pixels() else {
        panic!("three components decode to RGB");
    };
    let error = pixels
        .as_flattened()
        .iter()
        .zip(&rgb)
        .map(|(&a, &b)| a.abs_diff(b))
        .max()
        .unwrap();
    assert!(error < 40, "largest error {error}");
}

#[test]
fn unsupported_and_malformed_headers_are_rejected() {
    let file = pillow(1);
    let sof = find(&file, SOF0);
    let patched = |at: usize, value: u8| {
        let mut data = file.clone();
        data[at] = value;
        decode(&data)
    };
    assert_eq!(
        patched(sof + 1, 0xC9),
        Err(DecodeError::UnsupportedProcess { marker: 0xC9 })
    );
    assert_eq!(
        patched(sof + 4, 12),
        Err(DecodeError::UnsupportedPrecision(12))
    );
    assert_eq!(
        patched(sof + 9, 2),
        Err(DecodeError::UnsupportedComponents(2))
    );
    let mut no_height = file.clone();
    no_height[sof + 5..sof + 7].fill(0);
    assert_eq!(decode(&no_height), Err(DecodeError::EmptyImage));
    assert_eq!(patched(sof + 11, 0x51), Err(DecodeError::InvalidSampling));
    assert_eq!(
        patched(sof + 1, 0xDA),
        Err(DecodeError::MissingFrame),
        "a scan first"
    );
    assert_eq!(decode(b"\xff\xd8\xff\xd9"), Err(DecodeError::MissingFrame));
    assert_eq!(
        decode(b"\xff\xd8\xff\x02"),
        Err(DecodeError::UnknownMarker(2))
    );
    assert_eq!(decode(b"GIF89a"), Err(DecodeError::NotJpeg));
    let mut twice = file[..sof].to_vec();
    twice.extend_from_slice(&file[sof..find(&file, DHT)]);
    twice.extend_from_slice(&file[sof..]);
    assert_eq!(decode(&twice), Err(DecodeError::DuplicateMarker(SOF0)));
}

#[test]
fn huge_frames_are_refused_before_allocating() {
    let file = pillow(1);
    let sof = find(&file, SOF0);
    let mut huge = file.clone();
    huge[sof + 5..sof + 9].copy_from_slice(&[0xFF, 0xDC, 0xFF, 0xDC]);
    assert!(matches!(decode(&huge), Err(DecodeError::TooLarge { .. })));
    huge[sof + 5..sof + 9].copy_from_slice(&[0x10, 0x00, 0x10, 0x00]);
    assert!(
        matches!(decode(&huge), Err(DecodeError::TooLarge { .. })),
        "4096 × 4096 from a few hundred bytes"
    );
}

#[test]
fn restart_markers_must_be_in_sequence() {
    let mut file = pillow(0);
    let first = file.windows(2).position(|w| w == [0xFF, 0xD0]).unwrap();
    file[first + 1] = 0xD3;
    assert!(decode(&file).is_err());
}

#[test]
fn truncations_never_panic() {
    for (index, (label, ..)) in PILLOW.iter().enumerate() {
        let file = pillow(index);
        let scan = find(&file, crate::jpeg::marker::SOS);
        for len in 0..file.len() {
            let result = decode(&file[..len]);
            if len < scan {
                assert!(result.is_err(), "{label} cut to {len}");
            }
        }
    }
}

#[test]
fn corruptions_never_panic() {
    let mut rng = Rng::new(17);
    for round in 0..4000 {
        let file = pillow(round % PILLOW.len());
        let _ = decode(&mutate(&file, &mut rng));
    }
}
