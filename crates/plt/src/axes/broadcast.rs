//! numpy-style broadcasting of per-item arguments.

use super::PlotError;

/// `values` stretched to `n` items: a single value is repeated, `n` values
/// are kept (matplotlib's `np.broadcast_to` on 1-D arguments).
pub(crate) fn broadcast<T: Clone>(
    argument: &'static str,
    values: &[T],
    n: usize,
) -> Result<Vec<T>, PlotError> {
    match values {
        [single] => Ok(vec![single.clone(); n]),
        _ if values.len() == n => Ok(values.to_vec()),
        _ => Err(PlotError::Broadcast {
            argument,
            len: values.len(),
            points: n,
        }),
    }
}

/// Flags stretched to `n` items; no flags mean all unset.
pub(crate) fn broadcast_flags(
    argument: &'static str,
    flags: &[bool],
    n: usize,
) -> Result<Vec<bool>, PlotError> {
    if flags.is_empty() {
        Ok(vec![false; n])
    } else {
        broadcast(argument, flags, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_single_values() {
        assert_eq!(broadcast("a", &[2.0], 3), Ok(vec![2.0; 3]));
        assert_eq!(broadcast("a", &[1.0, 2.0], 2), Ok(vec![1.0, 2.0]));
        assert!(broadcast("a", &[1.0, 2.0], 3).is_err());
        assert_eq!(broadcast_flags("f", &[], 2), Ok(vec![false, false]));
    }
}
