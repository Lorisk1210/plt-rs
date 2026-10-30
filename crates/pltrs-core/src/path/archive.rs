//! [`Archive`] for paths, checked as [`Path::new`] checks them.

use super::{Path, PathCode};
use crate::archive::{Archive, ArchiveError, Decoder, Encoder};

crate::archive_enum!(PathCode {
    MoveTo,
    LineTo,
    Curve3,
    Curve4,
    ClosePoly
});

impl Archive for Path {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        let Self {
            vertices,
            codes,
            interpolation_steps,
        } = self;
        vertices.encode(encoder)?;
        codes.encode(encoder)?;
        interpolation_steps.encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let vertices = Vec::decode(decoder)?;
        let codes: Option<Vec<PathCode>> = Option::decode(decoder)?;
        let interpolation_steps = usize::decode(decoder)?;
        let path = match codes {
            Some(codes) => Self::new(vertices, codes)
                .map_err(|err| ArchiveError::invalid(format!("path: {err}")))?,
            None => Self::polyline(vertices),
        };
        Ok(path.with_interpolation_steps(interpolation_steps))
    }
}
