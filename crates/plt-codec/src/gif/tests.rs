use super::*;

/// A frame read back from a GIF.
#[derive(Debug)]
struct ReadFrame {
    delay: u16,
    disposal: u8,
    transparent: Option<u8>,
    area: Area,
    indices: Vec<u8>,
}

/// A GIF read back: the screen, the global table, the loop count and the
/// frames.
#[derive(Debug)]
struct ReadGif {
    screen: Screen,
    colors: Vec<[u8; 3]>,
    repeats: Option<u16>,
    frames: Vec<ReadFrame>,
}

/// A cursor over the bytes of a GIF.
struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> &[u8] {
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        head
    }

    fn byte(&mut self) -> u8 {
        self.bytes(1)[0]
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes([self.byte(), self.byte()])
    }

    fn sub_blocks(&mut self) -> Vec<u8> {
        let mut data = Vec::new();
        loop {
            let len = usize::from(self.byte());
            if len == 0 {
                return data;
            }
            data.extend_from_slice(self.bytes(len));
        }
    }
}

fn read(gif: &[u8]) -> ReadGif {
    let mut r = Reader(gif);
    assert_eq!(r.bytes(6), b"GIF89a");
    let screen = Screen {
        width: r.u16(),
        height: r.u16(),
    };
    let packed = r.byte();
    assert_eq!(packed & 0x80, 0x80, "a global table");
    r.bytes(2);
    let colors = r.bytes(3 << ((packed & 7) + 1)).as_chunks::<3>().0.to_vec();
    let (mut repeats, mut frames, mut control) = (None, Vec::new(), None);
    loop {
        match r.byte() {
            EXTENSION => match r.byte() {
                0xF9 => {
                    let block = r.sub_blocks();
                    let delay = u16::from_le_bytes([block[1], block[2]]);
                    let transparent = (block[0] & 1 == 1).then_some(block[3]);
                    control = Some((delay, block[0] >> 2 & 7, transparent));
                }
                0xFF => {
                    assert_eq!(r.bytes(1 + 11), b"\x0bNETSCAPE2.0");
                    let block = r.sub_blocks();
                    repeats = Some(u16::from_le_bytes([block[1], block[2]]));
                }
                other => panic!("unexpected extension {other:#x}"),
            },
            IMAGE => {
                let area = Area {
                    left: r.u16(),
                    top: r.u16(),
                    width: r.u16(),
                    height: r.u16(),
                };
                assert_eq!(r.byte(), 0, "no local table, not interlaced");
                let min_code_size = r.byte();
                let indices = lzw::decompress(&r.sub_blocks(), min_code_size).unwrap();
                assert_eq!(
                    indices.len(),
                    usize::from(area.width) * usize::from(area.height)
                );
                let (delay, disposal, transparent) = control.take().unwrap();
                frames.push(ReadFrame {
                    delay,
                    disposal,
                    transparent,
                    area,
                    indices,
                });
            }
            TRAILER => break,
            other => panic!("unexpected block {other:#x}"),
        }
    }
    assert!(r.0.is_empty(), "nothing after the trailer");
    ReadGif {
        screen,
        colors,
        repeats,
        frames,
    }
}

/// The RGBA picture after each frame, as a viewer composes them (the
/// background is transparent black).
fn compose(gif: &ReadGif) -> Vec<Vec<[u8; 4]>> {
    let width = usize::from(gif.screen.width);
    let mut canvas = vec![[0; 4]; gif.screen.pixels()];
    let mut pictures = Vec::new();
    for frame in &gif.frames {
        let area = frame.area;
        let pixels = (0..usize::from(area.height)).flat_map(|y| {
            (0..usize::from(area.width))
                .map(move |x| (usize::from(area.top) + y) * width + usize::from(area.left) + x)
        });
        for (at, &index) in pixels.clone().zip(&frame.indices) {
            if Some(index) != frame.transparent {
                let [r, g, b] = gif.colors[usize::from(index)];
                canvas[at] = [r, g, b, 255];
            }
        }
        pictures.push(canvas.clone());
        if frame.disposal == 2 {
            for at in pixels {
                canvas[at] = [0; 4];
            }
        }
    }
    pictures
}

fn rgba(pixels: &[[u8; 4]]) -> Vec<u8> {
    pixels.iter().flatten().copied().collect()
}

/// A `width × height` picture painted by `paint(x, y)`.
fn picture(width: usize, height: usize, paint: impl Fn(usize, usize) -> [u8; 4]) -> Vec<[u8; 4]> {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| paint(x, y))
        .collect()
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const BLUE: [u8; 4] = [31, 119, 180, 255];

#[test]
fn opaque_frames_come_back_exactly() {
    let first = picture(20, 10, |x, _| if x < 5 { BLUE } else { WHITE });
    let second = picture(20, 10, |x, y| {
        if (8..12).contains(&x) && y == 4 {
            BLUE
        } else {
            WHITE
        }
    });
    let (a, b) = (rgba(&first), rgba(&second));
    let frames = [
        GifFrame {
            pixels: &a,
            delay: 5,
        },
        GifFrame {
            pixels: &b,
            delay: 7,
        },
    ];
    let gif = read(&GifEncoder::new().encode(20, 10, &frames).unwrap());
    assert_eq!(gif.repeats, Some(0), "loops forever");
    assert_eq!(compose(&gif), [first, second]);
    let delays: Vec<_> = gif.frames.iter().map(|f| f.delay).collect();
    assert_eq!(delays, [5, 7]);
    assert_eq!(gif.frames[0].area, Area::whole(gif.screen));
    let changed = Area {
        left: 0,
        top: 0,
        width: 12,
        height: 10,
    };
    assert_eq!(gif.frames[1].area, changed, "only the changed columns");
    assert!(gif.frames.iter().all(|f| f.disposal == 1));
}

#[test]
fn repeated_frames_are_merged() {
    let still = rgba(&picture(3, 3, |_, _| WHITE));
    let other = rgba(&picture(3, 3, |x, _| if x == 1 { BLUE } else { WHITE }));
    let frames = [
        GifFrame {
            pixels: &still,
            delay: 10,
        },
        GifFrame {
            pixels: &still,
            delay: 10,
        },
        GifFrame {
            pixels: &other,
            delay: 3,
        },
        GifFrame {
            pixels: &still,
            delay: 4,
        },
    ];
    let gif = read(&GifEncoder::new().encode(3, 3, &frames).unwrap());
    let delays: Vec<_> = gif.frames.iter().map(|f| f.delay).collect();
    assert_eq!(delays, [20, 3, 4]);
    let column = Area {
        left: 1,
        top: 0,
        width: 1,
        height: 3,
    };
    assert_eq!(gif.frames[2].area, column);
}

#[test]
fn transparent_frames_replace_each_other() {
    let clear = [0, 0, 0, 0];
    let first = picture(4, 4, |x, _| if x == 0 { BLUE } else { clear });
    let second = picture(4, 4, |x, _| if x == 3 { BLUE } else { clear });
    let (a, b) = (rgba(&first), rgba(&second));
    let frames = [
        GifFrame {
            pixels: &a,
            delay: 1,
        },
        GifFrame {
            pixels: &b,
            delay: 1,
        },
    ];
    let gif = read(&GifEncoder::new().encode(4, 4, &frames).unwrap());
    assert!(
        gif.frames
            .iter()
            .all(|f| f.disposal == 2 && f.transparent.is_some())
    );
    assert!(gif.frames.iter().all(|f| f.area == Area::whole(gif.screen)));
    assert_eq!(compose(&gif), [first, second]);
}

#[test]
fn play_counts_set_the_loop_extension() {
    let pixels = rgba(&[WHITE]);
    let frames = [GifFrame {
        pixels: &pixels,
        delay: 1,
    }];
    let repeats = |plays| {
        read(
            &GifEncoder::new()
                .plays(plays)
                .encode(1, 1, &frames)
                .unwrap(),
        )
        .repeats
    };
    assert_eq!(repeats(0), Some(0));
    assert_eq!(repeats(1), None, "a single play needs no extension");
    assert_eq!(repeats(3), Some(2));
}

#[test]
fn many_colours_stay_close() {
    let ramp = picture(64, 64, |x, y| {
        let [r, g] = [x, y].map(|v| u8::try_from(v * 4).unwrap());
        [r, g, 128, 255]
    });
    let pixels = rgba(&ramp);
    let frames = [GifFrame {
        pixels: &pixels,
        delay: 1,
    }];
    let gif = read(&GifEncoder::new().encode(64, 64, &frames).unwrap());
    assert_eq!(gif.colors.len(), 256);
    let shown = &compose(&gif)[0];
    let worst = ramp
        .iter()
        .zip(shown)
        .flat_map(|(a, b)| (0..3).map(move |c| a[c].abs_diff(b[c])))
        .max();
    assert!(worst.unwrap() <= 16, "worst channel error {worst:?}");
}

#[test]
fn bad_input_is_rejected() {
    let pixels = vec![0; 4];
    let one = [GifFrame {
        pixels: &pixels,
        delay: 1,
    }];
    let encoder = GifEncoder::new();
    for (width, height) in [(0, 1), (1, 0), (70_000, 1)] {
        assert_eq!(
            encoder.encode(width, height, &one),
            Err(GifError::InvalidDimensions { width, height })
        );
    }
    assert_eq!(
        encoder.encode(2, 1, &one),
        Err(GifError::DataLengthMismatch {
            index: 0,
            expected: 8,
            actual: 4
        })
    );
    assert_eq!(encoder.encode(1, 1, &[]), Err(GifError::NoFrames));
}
