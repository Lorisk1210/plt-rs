//! Encoding whole pictures: the checks of their planes and colours, and
//! what the bitstreams hold. Decoding them is checked against dav1d, through
//! Pillow, in `tests/avif_pillow.rs`.

use super::*;

/// A `width × height` plane of a diagonal ramp with a dark stroke.
fn ramp(width: usize, height: usize) -> Vec<u8> {
    (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            if x == y {
                10
            } else {
                ((x * 3 + y * 5) % 256) as u8
            }
        })
        .collect()
}

#[test]
fn pictures_check_their_planes() {
    let luma = [0u8; 12];
    assert_eq!(
        Picture::new(0, 3, ChromaSampling::Monochrome, &[&luma]).err(),
        Some(Av1Error::Size {
            width: 0,
            height: 3
        })
    );
    assert!(Picture::new(MAX_SIDE + 1, 1, ChromaSampling::Monochrome, &[&luma]).is_err());
    assert_eq!(
        Picture::new(4, 3, ChromaSampling::Yuv420, &[&luma]).err(),
        Some(Av1Error::Planes)
    );
    // 5×3 in 4:2:0 has chroma planes of 3×2.
    let (y, c) = ([0u8; 15], [0u8; 6]);
    assert!(Picture::new(5, 3, ChromaSampling::Yuv420, &[&y, &c, &c]).is_ok());
    assert_eq!(
        Picture::new(5, 3, ChromaSampling::Yuv420, &[&y, &c, &y]).err(),
        Some(Av1Error::Planes)
    );
    let picture = Picture::new(5, 3, ChromaSampling::Yuv420, &[&y, &c, &c]).unwrap();
    assert_eq!(
        picture
            .with_color(ColorDescription::SRGB_IDENTITY, true)
            .err(),
        Some(Av1Error::IdentityNeeds444)
    );
    assert!(Av1Error::IdentityNeeds444.to_string().contains("4:4:4"));
}

#[test]
fn bitstreams_hold_a_sequence_header_and_a_frame() {
    let luma = ramp(40, 24);
    let picture = Picture::new(40, 24, ChromaSampling::Monochrome, &[&luma]).unwrap();
    for qindex in [0, 1, 100, 255] {
        let bitstream = encode(&picture, qindex);
        let data = &bitstream.data;
        // OBU_SEQUENCE_HEADER with a size field, its size, then
        // seq_profile 0, still_picture and reduced_still_picture_header.
        assert_eq!(data[0], 0x0A);
        let header_end = 2 + usize::from(data[1]);
        assert_eq!(data[2] >> 3, 0b0_0011);
        // Then OBU_FRAME, whose size field covers the rest.
        assert_eq!(data[header_end], 0x32);
        let (mut size, mut at) = (0, header_end + 1);
        for shift in (0..).step_by(7) {
            size |= usize::from(data[at] & 0x7F) << shift;
            at += 1;
            if data[at - 1] & 0x80 == 0 {
                break;
            }
        }
        assert_eq!(at + size, data.len(), "qindex {qindex}");
        assert_eq!((bitstream.profile, bitstream.level), (0, 0));
    }
}

#[test]
fn coarser_quantizers_write_fewer_bytes() {
    let (width, height) = (70, 50);
    let luma = ramp(width, height);
    let chroma = ramp(width.div_ceil(2), height.div_ceil(2));
    let picture = Picture::new(70, 50, ChromaSampling::Yuv420, &[&luma, &chroma, &chroma])
        .unwrap()
        .with_color(ColorDescription::SRGB_BT601, true)
        .unwrap();
    let sizes: Vec<usize> = [0, 40, 120, 250]
        .into_iter()
        .map(|qindex| encode(&picture, qindex).data.len())
        .collect();
    assert!(sizes.windows(2).all(|pair| pair[0] > pair[1]), "{sizes:?}");
    // The same picture always codes the same way.
    assert_eq!(encode(&picture, 120), encode(&picture, 120));
}

#[test]
fn flat_and_tiny_pictures_code() {
    for (width, height) in [(1, 1), (1, 9), (9, 1), (8, 8), (65, 65)] {
        let n = width * height;
        let flat = vec![200u8; n];
        let picture = Picture::new(
            width as u32,
            height as u32,
            ChromaSampling::Yuv444,
            &[&flat, &flat, &flat],
        )
        .unwrap();
        let lossless = encode(&picture, 0);
        let lossy = encode(&picture, 200);
        assert_eq!(lossless.profile, 1);
        // A flat picture codes in a few bytes, whatever its size.
        assert!(
            lossy.data.len() < 60,
            "{width}×{height}: {}",
            lossy.data.len()
        );
        assert!(
            lossless.data.len() < 60,
            "{width}×{height}: {}",
            lossless.data.len()
        );
    }
}
