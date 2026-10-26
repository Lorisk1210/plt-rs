//! Type 1 fonts opened by path, as matplotlib's `get_font` opens the TeX
//! fonts `dviread` and `textpath` draw with: read once and shared.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use super::FontDatabase;
use crate::error::Result;
use crate::type1::Type1Face;

/// The Type 1 faces read so far, by path.
#[derive(Debug, Default)]
pub(super) struct Type1Faces(RwLock<HashMap<PathBuf, Arc<Type1Face>>>);

impl FontDatabase {
    /// The Type 1 font (`.pfa` or `.pfb`) at `path` as FreeType loads it,
    /// read on first use and kept for later calls (matplotlib's
    /// `get_font` of a file). Type 1 faces are not catalogued for
    /// matching.
    pub fn type1_face(&self, path: impl AsRef<Path>) -> Result<Arc<Type1Face>> {
        let path = path.as_ref();
        let faces = &self.type1.0;
        if let Some(face) = faces
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(path)
        {
            return Ok(Arc::clone(face));
        }
        let face = Arc::new(Type1Face::open(path)?);
        Ok(Arc::clone(
            faces
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(path.to_path_buf())
                .or_insert(face),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FontError;

    #[test]
    fn faces_are_read_once_by_path() {
        let db = FontDatabase::new();
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/type1/cmr10.pfb");
        let face = db.type1_face(&path).unwrap();
        assert_eq!(face.postscript_name(), Some("CMR10"));
        assert!(Arc::ptr_eq(&face, &db.type1_face(&path).unwrap()));
        assert!(db.entries().is_empty(), "not catalogued");
        assert!(matches!(
            db.type1_face("/nonexistent/font.pfb"),
            Err(FontError::Io { .. })
        ));
        let not_type1 = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts/cmr10.ttf");
        assert!(matches!(db.type1_face(not_type1), Err(FontError::Type1(_))));
    }
}
