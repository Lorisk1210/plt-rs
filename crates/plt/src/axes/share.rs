//! Groups of axes sharing an axis (matplotlib's `_shared_axes` groupers).

use crate::artist::ArtistId;
use crate::ticker::AxisName;

/// Disjoint groups of axes (matplotlib's `cbook.Grouper`): joining only
/// ever merges groups, and an axes leaves its group only when it is
/// removed from the figure.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Grouper {
    groups: Vec<Vec<ArtistId>>,
}

impl Grouper {
    /// Puts `a` and `b`, and everything already joined with either, in
    /// one group.
    pub(crate) fn join(&mut self, a: ArtistId, b: ArtistId) {
        let mut merged = Vec::new();
        self.groups.retain(|group| {
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
        self.groups.push(merged);
    }

    /// The members of the group of `axes`, `axes` included.
    pub(crate) fn siblings(&self, axes: ArtistId) -> Vec<ArtistId> {
        self.groups
            .iter()
            .find(|group| group.contains(&axes))
            .cloned()
            .unwrap_or_else(|| vec![axes])
    }

    /// Forgets `axes`, dissolving groups left with a single member.
    fn remove(&mut self, axes: ArtistId) {
        for group in &mut self.groups {
            group.retain(|&id| id != axes);
        }
        self.groups.retain(|group| group.len() > 1);
    }
}

/// The axes sharing their x or their y axis (matplotlib's `_shared_axes`)
/// and the axes twinned on top of each other (`_twinned_axes`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ShareGroups {
    x: Grouper,
    y: Grouper,
    twins: Grouper,
}

impl ShareGroups {
    fn axis(&mut self, name: AxisName) -> &mut Grouper {
        match name {
            AxisName::X => &mut self.x,
            AxisName::Y => &mut self.y,
        }
    }

    /// Makes `a` and `b`, and everything already sharing axis `name` with
    /// either, share it.
    pub(crate) fn join(&mut self, name: AxisName, a: ArtistId, b: ArtistId) {
        self.axis(name).join(a, b);
    }

    /// The axes sharing axis `name` with `axes`, `axes` included.
    pub(crate) fn siblings(&self, name: AxisName, axes: ArtistId) -> Vec<ArtistId> {
        match name {
            AxisName::X => self.x.siblings(axes),
            AxisName::Y => self.y.siblings(axes),
        }
    }

    /// Records that `twin` was made on top of `axes`.
    pub(crate) fn join_twins(&mut self, axes: ArtistId, twin: ArtistId) {
        self.twins.join(axes, twin);
    }

    /// The axes twinned with `axes`, `axes` included.
    pub(crate) fn twins(&self, axes: ArtistId) -> Vec<ArtistId> {
        self.twins.siblings(axes)
    }

    /// Forgets `axes`, dissolving groups left with a single member.
    pub(crate) fn remove(&mut self, axes: ArtistId) {
        for grouper in [&mut self.x, &mut self.y, &mut self.twins] {
            grouper.remove(axes);
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

    #[test]
    fn twins_are_grouped_apart_from_shares() {
        let id = ids(2);
        let mut groups = ShareGroups::default();
        groups.join_twins(id[0], id[1]);
        assert_eq!(groups.twins(id[1]), [id[0], id[1]]);
        assert_eq!(groups.siblings(AxisName::X, id[0]), [id[0]]);
    }
}
