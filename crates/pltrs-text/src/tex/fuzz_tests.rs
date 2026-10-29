//! Randomised robustness tests of what TeX writes and reads: DVI files,
//! virtual fonts, font metrics, font maps and encodings, corrupted from
//! valid examples. Parsing must give a value or an error, never panic or
//! run away.

use crate::fuzz::{Gen, fuzz};

use super::dvi::read_dvi;
use super::psfonts::{PsfontsMap, parse_enc};
use super::test_support::{DviWriter, files, sample_dvi};
use super::tfm::Tfm;
use super::tfm::tests::tfm_bytes;
use super::vf::Vf;

/// A virtual font whose character 65 sets `B` of font `test` and a rule.
fn sample_vf() -> Vec<u8> {
    let mut vf = DviWriter::vf();
    vf.font_def(0, "test", 1 << 19);
    let mut packet = DviWriter::default();
    packet
        .op(141)
        .op(66)
        .right(1000)
        .rule(1 << 20, 1 << 20)
        .op(142);
    vf.op(packet.0.len() as u8).u1(65).op(0).op(0).op(0);
    vf.0.extend_from_slice(&packet.0);
    vf.op(248);
    vf.0
}

/// A DVI file setting a character of the virtual font `virt`.
fn virtual_dvi() -> Vec<u8> {
    let mut dvi = DviWriter::dvi();
    dvi.font_def(1, "virt", 10 << 16)
        .bop()
        .op(172)
        .op(65)
        .op(65)
        .end();
    dvi.0
}

/// A font map exercising every kind of field.
const MAP: &str = "% map\n\
cmr10 CMR10 <cmr10.pfb\n\
ptmr8r Times-Roman \"TeXBase1Encoding ReEncodeFont\" <8r.enc <utmr8a.pfb\n\
ptmro8r Times-Roman \".167 SlantFont TeXBase1Encoding ReEncodeFont\" <8r.enc <<utmr8a.pfb\n\
pcrr8rn Courier \"0.85 ExtendFont\" < pcrr8a.pfb <[8r.enc\n\
rm-lmr10 LMRoman10-Regular \"enclmrm ReEncodeFont\" <lm-rm.enc <lmr10.pfb\n";

/// An encoding file.
const ENC: &str = "% enc\n/Enc [ /grave /acute % comment\n/a /b ] def\n";

#[test]
fn corrupted_dvi_files() {
    let corpus = [sample_dvi(), virtual_dvi()];
    fuzz(
        "corrupted_dvi_files",
        2000,
        |g: &mut Gen| {
            let which = g.below(corpus.len());
            (which, g.corrupt(&corpus[which]), g.number())
        },
        |(which, corruption, dpi)| {
            let files = files(&[
                ("virt.tfm", tfm_bytes(65, 65, &[1 << 20], 0)),
                ("virt.vf", sample_vf()),
            ]);
            if let Ok(pages) = read_dvi(&corruption.apply(&corpus[*which]), &files) {
                for page in pages {
                    let _ = page.at(*dpi);
                }
            }
        },
    );
}

#[test]
fn corrupted_virtual_fonts() {
    let vf = sample_vf();
    fuzz(
        "corrupted_virtual_fonts",
        2000,
        |g: &mut Gen| g.corrupt(&vf),
        |corruption| {
            let corrupted = corruption.apply(&vf);
            let _ = Vf::parse(&corrupted, &files(&[]));
            let files = files(&[
                ("virt.tfm", tfm_bytes(65, 65, &[1 << 20], 0)),
                ("virt.vf", corrupted),
            ]);
            let _ = read_dvi(&virtual_dvi(), &files);
        },
    );
}

#[test]
fn corrupted_font_metrics() {
    let tfm = tfm_bytes(0, 127, &[1 << 19, 1 << 20, 3 << 18], 42);
    fuzz(
        "corrupted_font_metrics",
        2000,
        |g: &mut Gen| g.corrupt(&tfm),
        |corruption| {
            if let Ok(tfm) = Tfm::parse(&corruption.apply(&tfm)) {
                for code in tfm.codes() {
                    let _ = tfm.metrics(code);
                }
            }
        },
    );
}

#[test]
fn corrupted_font_maps_and_encodings() {
    #[derive(Debug)]
    struct Anything;
    impl super::files::FileFinder for Anything {
        fn find(&self, name: &str) -> Result<std::path::PathBuf, super::TexError> {
            Ok(std::path::PathBuf::from(name))
        }
    }
    fuzz(
        "corrupted_font_maps_and_encodings",
        2000,
        |g: &mut Gen| (g.corrupt(MAP.as_bytes()), g.corrupt(ENC.as_bytes())),
        |(map, enc)| {
            let map = PsfontsMap::parse(&map.apply(MAP.as_bytes()), "pdftex.map");
            for name in ["cmr10", "ptmr8r", "ptmro8r", "pcrr8rn", "rm-lmr10", "x"] {
                let _ = map.get(name.as_bytes(), &Anything);
            }
            let _ = parse_enc(&enc.apply(ENC.as_bytes()));
        },
    );
}
