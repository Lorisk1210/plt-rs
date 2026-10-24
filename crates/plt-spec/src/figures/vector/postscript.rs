//! What the PDF reader shares with the PostScript reader: PostScript's
//! token delimiters, which PDF syntax kept, and the `[a b c d e f]`
//! matrices of their common imaging model.

/// An affine matrix `[a b c d e f]`.
pub(super) type Matrix = [f64; 6];

/// The identity matrix.
pub(super) const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m` applied after `n` (the row-vector convention: `n × m`).
pub(super) fn concat(m: &Matrix, n: &Matrix) -> Matrix {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

/// The point `(x, y)` transformed by `m`.
pub(super) fn apply(m: &Matrix, x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// Whether `b` ends a token.
pub(super) fn is_delimiter(b: u8) -> bool {
    b.is_ascii_whitespace() || b"()<>[]{}/%".contains(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrices_compose_like_pdf() {
        let translate = [1.0, 0.0, 0.0, 1.0, 5.0, 0.0];
        let scale = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
        // `scale cm` after `translate cm`: points are scaled first.
        let ctm = concat(&scale, &translate);
        assert_eq!(apply(&ctm, 1.0, 1.0), (7.0, 2.0));
        assert_eq!(concat(&IDENTITY, &ctm), ctm);
    }

    #[test]
    fn delimiters() {
        assert!(is_delimiter(b'/') && is_delimiter(b' ') && is_delimiter(b'%'));
        assert!(!is_delimiter(b'a') && !is_delimiter(b'.'));
    }
}
