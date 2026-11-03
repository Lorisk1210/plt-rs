//! The AVIF container (ISO/IEC 23008-12 HEIF and the AV1 Image File
//! Format): `ftyp`, then `meta` describing an `av01` colour item and an
//! optional alpha item, then `mdat` with their AV1 data, the boxes laid
//! out as libavif 1.4 writes them.

use crate::av1::{Bitstream, ChromaSampling, ColorDescription};

/// The auxiliary type of an alpha item.
const ALPHA_URN: &[u8] = b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha\0";

/// An image item: an AV1 still picture and its properties.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Item<'a> {
    /// The coded picture.
    pub(crate) bitstream: &'a Bitstream,
    /// The colour signalling of a colour item; `None` for alpha.
    pub(crate) color: Option<(ColorDescription, bool)>,
}

/// A box of `kind` around `body`.
fn boxed(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let size = u32::try_from(8 + body.len()).expect("AVIF boxes stay below 4 GiB");
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(body);
    out
}

/// A full box: version 0 and no flags (except where `version` is given).
fn full_box(kind: [u8; 4], version: u8, body: &[u8]) -> Vec<u8> {
    let mut content = vec![version, 0, 0, 0];
    content.extend_from_slice(body);
    boxed(kind, &content)
}

/// The concatenation of byte strings.
fn concat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// The `av1C` box of a bitstream: the AV1 codec configuration record,
/// without configuration OBUs.
pub(crate) fn av1c(bitstream: &Bitstream) -> Vec<u8> {
    let (ss_x, ss_y) = bitstream.sampling.subsampling();
    let monochrome = bitstream.sampling == ChromaSampling::Monochrome;
    let flags = (u8::from(monochrome) << 4) | ((ss_x as u8) << 3) | ((ss_y as u8) << 2);
    boxed(
        *b"av1C",
        &[0x81, (bitstream.profile << 5) | bitstream.level, flags, 0],
    )
}

/// The `ftyp` box: the AVIF brands, and the baseline (4:2:0) or advanced
/// (4:4:4) profile brand.
fn ftyp(sampling: ChromaSampling) -> Vec<u8> {
    let mut body = concat(&[b"avif", &[0; 4], b"avif", b"mif1", b"miaf"]);
    match sampling {
        ChromaSampling::Yuv420 => body.extend_from_slice(b"MA1B"),
        ChromaSampling::Yuv444 => body.extend_from_slice(b"MA1A"),
        ChromaSampling::Monochrome => {}
    }
    boxed(*b"ftyp", &body)
}

/// The `iloc` box: each item's single extent in the file.
fn iloc(extents: &[(u32, u32)]) -> Vec<u8> {
    let count = extents.len() as u16;
    let mut body = vec![0x44, 0x00];
    body.extend_from_slice(&count.to_be_bytes());
    for (id, &(offset, length)) in (1u16..).zip(extents) {
        body.extend_from_slice(&id.to_be_bytes());
        body.extend_from_slice(&[0, 0, 0, 1]); // data_reference_index, extent_count
        body.extend_from_slice(&offset.to_be_bytes());
        body.extend_from_slice(&length.to_be_bytes());
    }
    full_box(*b"iloc", 0, &body)
}

/// The `iinf` box with an `av01` entry per item.
fn iinf(items: usize) -> Vec<u8> {
    let mut body = (items as u16).to_be_bytes().to_vec();
    for (id, name) in (1u16..).zip([&b"Color\0"[..], b"Alpha\0"].iter().take(items)) {
        let entry = concat(&[&id.to_be_bytes(), &[0, 0], b"av01", name]);
        body.extend(full_box(*b"infe", 2, &entry));
    }
    full_box(*b"iinf", 0, &body)
}

/// The `iprp` box: the properties and their association to the items.
fn iprp((width, height): (u32, u32), items: &[Item<'_>]) -> Vec<u8> {
    let ispe = full_box(
        *b"ispe",
        0,
        &concat(&[&width.to_be_bytes(), &height.to_be_bytes()]),
    );
    let mut properties = vec![ispe];
    let mut associations = Vec::new();
    for (id, item) in (1u16..).zip(items) {
        let channels = item.bitstream.sampling.planes() as u8;
        let mut pixi = vec![channels];
        pixi.resize(1 + usize::from(channels), 8);
        properties.push(full_box(*b"pixi", 0, &pixi));
        let pixi_index = properties.len() as u8;
        properties.push(av1c(item.bitstream));
        let av1c_index = properties.len() as u8;
        properties.push(match item.color {
            Some((color, full_range)) => {
                let code = |v: u8| u16::from(v).to_be_bytes();
                let nclx = concat(&[
                    b"nclx",
                    &code(color.primaries),
                    &code(color.transfer),
                    &code(color.matrix),
                    &[u8::from(full_range) << 7],
                ]);
                boxed(*b"colr", &nclx)
            }
            None => full_box(*b"auxC", 0, ALPHA_URN),
        });
        let last_index = properties.len() as u8;
        associations.extend_from_slice(&id.to_be_bytes());
        associations.extend_from_slice(&[4, 1, pixi_index, 0x80 | av1c_index, last_index]);
    }
    let ipco = boxed(*b"ipco", &properties.concat());
    let mut ipma_body = (items.len() as u32).to_be_bytes().to_vec();
    ipma_body.extend(associations);
    let ipma = full_box(*b"ipma", 0, &ipma_body);
    boxed(*b"iprp", &concat(&[&ipco, &ipma]))
}

/// The `meta` box, its items' data at `extents`.
fn meta(size: (u32, u32), items: &[Item<'_>], extents: &[(u32, u32)]) -> Vec<u8> {
    let hdlr = full_box(*b"hdlr", 0, &concat(&[&[0; 4], b"pict", &[0; 13]]));
    let pitm = full_box(*b"pitm", 0, &1u16.to_be_bytes());
    let mut body = concat(&[&hdlr, &pitm, &iloc(extents), &iinf(items.len())]);
    if items.len() > 1 {
        // The alpha item (2) is an auxiliary image of the colour item (1).
        let auxl = boxed(*b"auxl", &[0, 2, 0, 1, 0, 1]);
        body.extend(full_box(*b"iref", 0, &auxl));
    }
    body.extend(iprp(size, items));
    full_box(*b"meta", 0, &body)
}

/// An AVIF file of a `size` picture: the colour item, then the alpha item
/// if any. The alpha data comes first in `mdat`, as libavif writes it.
pub(crate) fn write(size: (u32, u32), items: &[Item<'_>]) -> Vec<u8> {
    let ftyp = ftyp(items[0].bitstream.sampling);
    let lengths: Vec<u32> = items
        .iter()
        .map(|item| u32::try_from(item.bitstream.data.len()).expect("AVIF items stay below 4 GiB"))
        .collect();
    let placeholder = vec![(0, 0); items.len()];
    let meta_size = meta(size, items, &placeholder).len();
    let mdat_start = u32::try_from(ftyp.len() + meta_size + 8).expect("AVIF headers are small");
    // mdat holds the items in reverse: alpha, then colour.
    let mut offset = mdat_start;
    let mut extents = vec![(0, 0); items.len()];
    for (i, &length) in lengths.iter().enumerate().rev() {
        extents[i] = (offset, length);
        offset += length;
    }
    let data: Vec<u8> = items
        .iter()
        .rev()
        .flat_map(|item| item.bitstream.data.iter().copied())
        .collect();
    concat(&[&ftyp, &meta(size, items, &extents), &boxed(*b"mdat", &data)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bitstream(sampling: ChromaSampling, data: &[u8]) -> Bitstream {
        Bitstream {
            data: data.to_vec(),
            profile: sampling.profile(),
            level: 0,
            sampling,
        }
    }

    /// The type and body of each box in `data`.
    fn boxes(data: &[u8]) -> Vec<(String, &[u8])> {
        let mut out = Vec::new();
        let mut rest = data;
        while rest.len() >= 8 {
            let size = u32::from_be_bytes(rest[..4].try_into().unwrap()) as usize;
            out.push((
                String::from_utf8_lossy(&rest[4..8]).into_owned(),
                &rest[8..size],
            ));
            rest = &rest[size..];
        }
        out
    }

    #[test]
    fn layout_matches_libavif() {
        // The boxes libavif 1.4 writes for a 13×10 RGBA image (Pillow 12).
        let color = bitstream(ChromaSampling::Yuv420, &[0xC0; 43]);
        let alpha = bitstream(ChromaSampling::Monochrome, &[0xA0; 18]);
        let items = [
            Item {
                bitstream: &color,
                color: Some((ColorDescription::SRGB_BT601, true)),
            },
            Item {
                bitstream: &alpha,
                color: None,
            },
        ];
        let file = write((13, 10), &items);
        let top = boxes(&file);
        let names: Vec<&str> = top.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["ftyp", "meta", "mdat"]);
        assert_eq!(top[0].1, b"avif\0\0\0\0avifmif1miafMA1B");
        assert_eq!(file.len(), 491);
        let meta = boxes(&top[1].1[4..]);
        let names: Vec<&str> = meta.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["hdlr", "pitm", "iloc", "iinf", "iref", "iprp"]);
        // iloc: the colour item after the alpha item in mdat.
        let iloc = meta[2].1;
        assert_eq!(&iloc[4..8], [0x44, 0, 0, 2]);
        assert_eq!(
            &iloc[8..22],
            [0, 1, 0, 0, 0, 1, 0, 0, 0x01, 0xc0, 0, 0, 0, 0x2b]
        );
        assert_eq!(
            &iloc[22..],
            [0, 2, 0, 0, 0, 1, 0, 0, 0x01, 0xae, 0, 0, 0, 0x12]
        );
        let iprp = boxes(meta[5].1);
        let ipco = boxes(iprp[0].1);
        let names: Vec<&str> = ipco.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            ["ispe", "pixi", "av1C", "colr", "pixi", "av1C", "auxC"]
        );
        assert_eq!(ipco[2].1, [0x81, 0x00, 0x0c, 0x00]);
        assert_eq!(ipco[3].1, b"nclx\0\x01\0\x0d\0\x06\x80");
        assert_eq!(ipco[5].1, [0x81, 0x00, 0x1c, 0x00]);
        assert_eq!(
            iprp[1].1,
            [
                0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 4, 1, 2, 0x83, 4, 0, 2, 4, 1, 5, 0x86, 7
            ]
        );
        assert_eq!(&file[430..448], [0xA0; 18]);
        assert_eq!(&file[448..], [0xC0; 43]);
    }

    #[test]
    fn opaque_pictures_have_one_item() {
        let color = bitstream(ChromaSampling::Yuv444, &[1, 2, 3]);
        let item = Item {
            bitstream: &color,
            color: Some((ColorDescription::SRGB_IDENTITY, true)),
        };
        let file = write((1, 1), &[item]);
        let top = boxes(&file);
        assert_eq!(top[0].1, b"avif\0\0\0\0avifmif1miafMA1A");
        let meta = boxes(&top[1].1[4..]);
        assert!(meta.iter().all(|(name, _)| name != "iref"));
        assert_eq!(top[2].1, [1, 2, 3]);
    }
}
