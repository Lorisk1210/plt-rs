//! Groups of axes sharing an axis (matplotlib's `_shared_axes` groupers).

use crate::artist::ArtistId;
use crate::ticker::AxisName;

/// Disjoint groups of axes that share their x or their y axis.
///
/// Sharing is permanent, as in matplotlib: groups only ever merge, and an
/// axes leaves its group only when it is removed from the figure.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ShareGroups {
    x: Vec<Vec<ArtistId>>,
    y: Vec<Vec<ArtistId>>,
}

impl ShareGroups {
    fn groups(&self, name: AxisName) -> &Vec<Vec<ArtistId>> {
        match name {
            AxisName::X => &self.x,
            AxisName::Y => &self.y,
        }
    }

    fn groups_mut(&mut self, name: AxisName) -> &mut Vec<Vec<ArtistId>> {
        match name {
            AxisName::X => &mut self.x,
            AxisName::Y => &mut self.y,
        }
    }

    /// Puts `a` and `b`, and everything already sharing with either, in
    /// one group.
    pub(crate) fn join(&mut self, name: AxisName, a: ArtistId, b: ArtistId) {
        let groups = self.groups_mut(name);
        let mut merged = Vec::new();
        groups.retain(|group| {
            let touches = group.contains(&a) || group.contains(&b);
            if touches {
                merged.extend_from_slice(group);
            }
            !touches
        });
        for id in [a, b] {
            if !merged.contains(&id) {
                merged.push(id);
            }
        }
        groups.push(merged);
    }

    /// The axes sharing axis `name` with `axes`, `axes` included.
    pub(crate) fn siblings(&self, name: AxisName, axes: ArtistId) -> Vec<ArtistId> {
        self.groups(name)
            .iter()
            .find(|group| group.contains(&axes))
            .cloned()
            .unwrap_or_else(|| vec![axes])
    }

    /// Forgets `axes`, dissolving groups left with a single member.
    pub(crate) fn remove(&mut self, axes: ArtistId) {
        for name in [AxisName::X, AxisName::Y] {
            let groups = self.groups_mut(name);
            for group in groups.iter_mut() {
                group.retain(|&id| id != axes);
            }
            groups.retain(|group| group.len() > 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    fn ids(n: usize) -> Vec<ArtistId> {
        let fig = Figure::new();
        (0..n)
            .map(|_| fig.add_axes([0.1, 0.1, 0.8, 0.8]).id())
            .collect()
    }

    #[test]
    fn joining_merges_groups() {
        let id = ids(4);
        let mut groups = ShareGroups::default();
        assert_eq!(groups.siblings(AxisName::X, id[0]), [id[0]]);
        groups.join(AxisName::X, id[0], id[1]);
        groups.join(AxisName::X, id[2], id[3]);
        assert_eq!(groups.siblings(AxisName::Y, id[0]), [id[0]]);
        groups.join(AxisName::X, id[1], id[3]);
        let mut all = groups.siblings(AxisName::X, id[2]);
        all.sort();
        let mut expected = id.clone();
        expected.sort();
        assert_eq!(all, expected);
    }

    #[test]
    fn removal_dissolves_pairs() {
        let id = ids(3);
        let mut groups = ShareGroups::default();
        groups.join(AxisName::Y, id[0], id[1]);
        groups.join(AxisName::Y, id[0], id[2]);
        groups.remove(id[1]);
        assert_eq!(groups.siblings(AxisName::Y, id[0]), [id[0], id[2]]);
        groups.remove(id[2]);
        assert_eq!(groups.siblings(AxisName::Y, id[0]), [id[0]]);
    }
}
