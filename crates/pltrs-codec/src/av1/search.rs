//! The rate-distortion search: for each square, the cost of coding it
//! whole against the cost of its quarters, and for each block, its luma
//! prediction or palette and transform type, then its chroma prediction
//! or palette. Costs are squared error plus λ times the bits the CDFs
//! predict; every trial is rolled back, and the choice made is left
//! reconstructed.

use super::block::{BlockChoice, Predictor};
use super::entropy::RateEstimate;
use super::palette::{Area, Palette, chroma_candidates, luma_candidates};
use super::predict::predict;
use super::syntax::{Mode, Prediction, Square};
use super::tile::{Partition, TileCoder};
use super::transform::TxType;

/// How many of the predictions that come nearest the source are tried
/// in full, in lossy and in lossless blocks.
const CANDIDATES: (usize, usize) = (3, 6);
/// How many of the directional modes nearest the source have their
/// angle deltas tried.
const DIRECTIONS_REFINED: usize = 2;

/// A luma prediction or palette worth trying for a block, and the error
/// of its prediction.
struct LumaCandidate {
    /// The prediction: DC under a palette.
    prediction: Prediction,
    /// The palette, if any.
    palette: Option<Palette>,
    /// The squared error of the prediction of the whole block.
    error: u64,
}

impl LumaCandidate {
    /// How the candidate predicts.
    fn predictor(&self) -> Predictor<'_> {
        match &self.palette {
            Some(palette) => Predictor::Palette(palette, 0),
            None => Predictor::Intra(self.prediction),
        }
    }
}

impl TileCoder<'_> {
    /// The weight of a bit against squared error: lossless blocks have no
    /// error, so only their rate counts.
    fn lambda(&self) -> f64 {
        if self.quantizer.lossless() {
            1.0
        } else {
            self.quantizer.lambda()
        }
    }

    /// Searches how to code the square at `(r, c)`, leaving it
    /// reconstructed as chosen; returns the cost and the partition.
    pub(crate) fn search_partition(
        &mut self,
        (r, c): (usize, usize),
        size: Square,
    ) -> (f64, Partition) {
        if r >= self.frame.mi.1 || c >= self.frame.mi.0 {
            return (0.0, Partition::Outside);
        }
        let lambda = self.lambda();
        let (has_rows, has_cols) = self.has_rows_cols((r, c), size);
        let before = self.save((r, c), size);
        let whole = (has_rows && has_cols).then(|| {
            let mut estimate = RateEstimate::default();
            self.write_partition(&mut estimate, (r, c), size, false);
            let (cost, choice, distortion) = self.search_block((r, c), size);
            (cost + lambda * estimate.bits, choice, distortion)
        });
        // A lossy block coded without error is not split: quarters could
        // only save a few bits.
        let exact = |distortion: u64| distortion == 0 && !self.quantizer.lossless();
        match whole {
            Some((cost, choice, distortion)) if size == Square::S8 || exact(distortion) => {
                return (cost, Partition::Whole(choice));
            }
            _ => {}
        }
        let whole = whole.map(|whole| (whole, self.save((r, c), size)));
        self.restore(&before);
        let mut estimate = RateEstimate::default();
        self.write_partition(&mut estimate, (r, c), size, true);
        let mut cost = lambda * estimate.bits;
        let half = size.units() / 2;
        let quarters = [0, 1, 2, 3].map(|i| {
            let at = (r + half * (i / 2), c + half * (i % 2));
            let (quarter_cost, tree) = self.search_partition(at, size.half());
            cost += quarter_cost;
            tree
        });
        match whole {
            Some(((whole_cost, choice, _), snapshot)) if whole_cost <= cost => {
                self.restore(&snapshot);
                (whole_cost, Partition::Whole(choice))
            }
            _ => (cost, Partition::Split(Box::new(quarters))),
        }
    }

    /// Chooses the predictions, palettes and transform type of the block
    /// at `(r, c)` and codes it; returns its cost, the choice and its
    /// squared error.
    fn search_block(&mut self, at: (usize, usize), size: Square) -> (f64, BlockChoice, u64) {
        let lambda = self.lambda();
        let mut choice = BlockChoice {
            y: Prediction::plain(Mode::Dc),
            uv: Prediction::plain(Mode::Dc),
            tx_type: TxType::DctDct,
            y_palette: None,
            uv_palette: None,
        };
        // Every luma prediction and palette, with every transform type.
        let mut candidates: Vec<LumaCandidate> = self
            .ranked(0, at, size)
            .into_iter()
            .map(|(prediction, error)| LumaCandidate {
                prediction,
                palette: None,
                error,
            })
            .collect();
        candidates.extend(
            self.palettes(0, at, size)
                .into_iter()
                .map(|(palette, error)| LumaCandidate {
                    prediction: Prediction::plain(Mode::Dc),
                    palette: Some(palette),
                    error,
                }),
        );
        let types: &[TxType] = if self.quantizer.lossless() || size.log2() > 4 {
            &[TxType::DctDct]
        } else {
            &TxType::REDUCED_SET
        };
        let mut best = (f64::INFINITY, 0);
        for (i, candidate) in candidates.iter().enumerate() {
            // Without a residual, every transform codes nothing alike.
            let types = if candidate.error == 0 {
                &types[..1]
            } else {
                types
            };
            for &tx_type in types {
                let cost = self.luma_trial(at, size, candidate.predictor(), tx_type);
                if cost < best.0 {
                    best = (cost, i);
                    choice.tx_type = tx_type;
                }
            }
        }
        let chosen = best.1;
        let chosen = candidates.swap_remove(chosen);
        choice.y = chosen.prediction;
        choice.y_palette = chosen.palette;
        if self.frame.recon.len() > 1 {
            let mut best = f64::INFINITY;
            let luma = (choice.y.mode, choice.y_palette.is_some());
            for (prediction, _) in self.ranked(1, at, size) {
                let cost = self.chroma_trial(at, size, luma, Predictor::Intra(prediction));
                if cost < best {
                    best = cost;
                    choice.uv = prediction;
                }
            }
            for (palette, _) in self.palettes(1, at, size) {
                let cost = self.chroma_trial(at, size, luma, Predictor::Palette(&palette, 0));
                if cost < best {
                    best = cost;
                    choice.uv = Prediction::plain(Mode::Dc);
                    choice.uv_palette = Some(palette);
                }
            }
        }
        let mut estimate = RateEstimate::default();
        let distortion = self.code_block(&mut estimate, at, size, &choice);
        (
            distortion as f64 + lambda * estimate.bits,
            choice,
            distortion,
        )
    }

    /// The palettes worth trying for the luma (`plane` 0) or chroma of
    /// the block at `(r, c)`, with the squared error of their prediction.
    fn palettes(&self, plane: usize, (r, c): (usize, usize), size: Square) -> Vec<(Palette, u64)> {
        let (x, y, square) = self.plane_square(plane, (r, c), size);
        let area = Area {
            at: (x, y),
            side: square.samples(),
            onscreen: self.onscreen(plane, (r, c), size),
        };
        let source = &self.frame.source;
        let found = if plane == 0 {
            luma_candidates(&source[0], area, self.lambda())
        } else {
            chroma_candidates(&source[1], &source[2], area, self.lambda())
        };
        found
            .into_iter()
            .map(|palette| {
                let side = palette.side;
                let error = (0..palette.colors.len())
                    .map(|list| {
                        let predicted = palette.predict(list, (0, 0), side);
                        source[plane + list].error_against(x, y, side, &predicted)
                    })
                    .sum();
                (palette, error)
            })
            .collect()
    }

    /// The cost of the block's luma with `predictor` and `tx_type`.
    fn luma_trial(
        &mut self,
        at: (usize, usize),
        size: Square,
        predictor: Predictor<'_>,
        tx_type: TxType,
    ) -> f64 {
        let snapshot = self.save(at, size);
        let mut estimate = RateEstimate::default();
        let (prediction, palette) = match predictor {
            Predictor::Intra(prediction) => (prediction, None),
            Predictor::Palette(palette, _) => (Prediction::plain(Mode::Dc), Some(palette)),
        };
        self.write_y_mode(&mut estimate, at, prediction);
        if prediction.mode == Mode::Dc {
            self.write_luma_palette(&mut estimate, (at, size), palette);
        }
        if let Some(palette) = palette {
            self.write_palette_map(&mut estimate, true, palette);
        }
        let coding = self.compute_plane(0, at, size, (predictor, tx_type));
        let info = self.block_info(0, size, prediction.mode);
        self.write_plane(&mut estimate, &coding, &info);
        self.restore(&snapshot);
        coding.distortion as f64 + self.lambda() * estimate.bits
    }

    /// The cost of the block's chroma with `predictor`, after luma in
    /// mode `luma.0`, with a palette if `luma.1`.
    fn chroma_trial(
        &mut self,
        at: (usize, usize),
        size: Square,
        (y_mode, luma_palette): (Mode, bool),
        predictor: Predictor<'_>,
    ) -> f64 {
        let snapshot = self.save(at, size);
        let mut estimate = RateEstimate::default();
        let (prediction, palette) = match predictor {
            Predictor::Intra(prediction) => (prediction, None),
            Predictor::Palette(palette, _) => (Prediction::plain(Mode::Dc), Some(palette)),
        };
        self.write_uv_mode(&mut estimate, size, y_mode, prediction);
        if prediction.mode == Mode::Dc {
            self.write_chroma_palette(&mut estimate, (at, size), (luma_palette, palette));
        }
        if let Some(palette) = palette {
            self.write_palette_map(&mut estimate, false, palette);
        }
        let mut distortion = 0;
        for plane in 1..3 {
            let predictor = match predictor {
                Predictor::Palette(palette, _) => Predictor::Palette(palette, plane - 1),
                intra @ Predictor::Intra(_) => intra,
            };
            let coding = self.compute_plane(plane, at, size, (predictor, TxType::DctDct));
            let info = self.block_info(plane, size, y_mode);
            self.write_plane(&mut estimate, &coding, &info);
            distortion += coding.distortion;
        }
        self.restore(&snapshot);
        distortion as f64 + self.lambda() * estimate.bits
    }

    /// The squared error of `prediction` of the whole block at `(r, c)`
    /// from its neighbours, over the luma (`plane` 0) or chroma planes.
    fn prediction_error(
        &self,
        plane: usize,
        (r, c): (usize, usize),
        size: Square,
        prediction: Prediction,
    ) -> u64 {
        let planes = if plane == 0 {
            0..1
        } else {
            1..self.frame.recon.len()
        };
        planes
            .map(|p| {
                let (x, y, square) = self.plane_square(p, (r, c), size);
                let position = self.position(p, (r, c), (x, y), (x, y), square);
                let predicted = predict(&self.frame.recon[p], position, square, prediction);
                self.frame.source[p].error_against(x, y, square.samples(), &predicted)
            })
            .sum()
    }

    /// The predictions worth trying for luma (`plane` 0) or chroma, with
    /// the squared error of their prediction of the whole block from its
    /// neighbours: those that come nearest the source, among every mode
    /// and the angle deltas of the best directional ones.
    fn ranked(&self, plane: usize, at: (usize, usize), size: Square) -> Vec<(Prediction, u64)> {
        let score = |prediction| {
            (
                self.prediction_error(plane, at, size, prediction),
                prediction,
            )
        };
        let mut scored: Vec<(u64, Prediction)> = Mode::ALL
            .into_iter()
            .map(|mode| score(Prediction::plain(mode)))
            .collect();
        scored.sort_by_key(|&(error, _)| error);
        let directional: Vec<Mode> = scored
            .iter()
            .map(|&(_, prediction)| prediction.mode)
            .filter(|mode| mode.angle().is_some())
            .take(DIRECTIONS_REFINED)
            .collect();
        for mode in directional {
            scored.extend(
                Prediction::with_deltas(mode)
                    .filter(|prediction| prediction.delta != 0)
                    .map(score),
            );
        }
        scored.sort_by_key(|&(error, _)| error);
        let count = if self.quantizer.lossless() {
            CANDIDATES.1
        } else {
            CANDIDATES.0
        };
        scored
            .into_iter()
            .take(count)
            .map(|(error, prediction)| (prediction, error))
            .collect()
    }
}
