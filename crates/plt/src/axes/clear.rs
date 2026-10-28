//! Clearing an axes back to a fresh one at the same place (matplotlib's
//! `Axes.clear` and `cla`).

use std::rc::Rc;

use super::{Axes, AxesState, AxisName, ProjectionKind, TickStyle};
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup};
use crate::figure::{FigureState, TITLE_PLACES, axes_title};
use crate::rc::RcParams;
use crate::scale::LinearScale;
use crate::spines::{SpineArtist, SpinePosition};
use crate::text::TextArtist;

impl AxesState {
    /// The state a cleared axes restarts from (matplotlib's `__clear`): a
    /// fresh axes under `rc` that keeps the identity, placement, aspect,
    /// sharing, background colour, frame, navigation and status-bar
    /// formatters of `self`, the options of its projection, and the
    /// visibility of shared axes.
    fn cleared(&self, rc: Rc<RcParams>) -> Self {
        let mut fresh = Self::new(self.position, rc);
        *fresh.base_mut() = self.base().clone();
        fresh
            .projection_options
            .clone_from(&self.projection_options);
        fresh.aspect = self.aspect;
        fresh.axes_locator.clone_from(&self.axes_locator);
        fresh.subplot_spec.clone_from(&self.subplot_spec);
        fresh.shares = self.shares;
        fresh.secondary.clone_from(&self.secondary);
        fresh.spines.clone_from(&self.spines);
        fresh.titles.clone_from(&self.titles);
        fresh.children = self.spines.iter().chain(&self.titles).copied().collect();
        fresh.navigate = self.navigate;
        fresh.forward_navigation = self.forward_navigation;
        fresh.fmt_data.clone_from(&self.fmt_data);
        fresh.patch.facecolor = self.patch.facecolor.clone();
        fresh.decorations.frame_on = self.decorations.frame_on;
        fresh.axisbelow = self.axisbelow;
        for name in [AxisName::X, AxisName::Y] {
            // matplotlib's `Axis.clear` keeps the tick parameters, except
            // whether gridlines are on, which restarts from the rc.
            let (old, new) = (self.axis(name), fresh.axis_mut(name));
            for (kept, restarted) in [
                (&old.major_style, &mut new.major_style),
                (&old.minor_style, &mut new.minor_style),
            ] {
                *restarted = TickStyle {
                    grid_on: restarted.grid_on,
                    ..kept.clone()
                };
            }
            new.label_size = old.label_size;
            // The axis object stays, with its picker and pick radius.
            new.picking = old.picking.clone();
        }
        if self.shares.0.is_some() {
            fresh.xaxis.visible = self.xaxis.visible;
            fresh.patch.visible = self.patch.visible;
        }
        if self.shares.1.is_some() {
            fresh.yaxis.visible = self.yaxis.visible;
            fresh.patch.visible = self.patch.visible;
        }
        fresh
    }
}

impl FigureState {
    /// Removes the artists of axes `id` and resets it (matplotlib's
    /// `Axes.__clear`): the titles start empty, the spines at the edges,
    /// the shared axes' scales linear, and shared axes take their
    /// partners' limits again.
    pub(crate) fn clear_axes(&mut self, id: ArtistId) {
        let rc = crate::rc::params();
        let Some(old) = self.artist_as::<AxesState>(id) else {
            return;
        };
        let fresh = old.cleared(Rc::clone(&rc));
        let doomed: Vec<ArtistId> = old
            .children
            .iter()
            .copied()
            .filter(|child| !fresh.children.contains(child))
            .collect();
        let (shares, titles, spines) = (fresh.shares, fresh.titles.clone(), fresh.spines.clone());
        for child in doomed {
            self.remove_artist(child);
        }
        for name in [AxisName::X, AxisName::Y] {
            for sibling in self.shared.siblings(name, id) {
                if let Some(state) = self.artist_as_mut::<AxesState>(sibling)
                    && sibling != id
                {
                    state.axis_mut(name).set_scale(Rc::new(LinearScale));
                }
            }
        }
        if let Some(state) = self.artist_as_mut::<AxesState>(id) {
            *state = fresh;
        }
        for (title, (x, halign)) in titles.into_iter().zip(TITLE_PLACES) {
            if let Some(text) = self.artist_as_mut::<TextArtist>(title) {
                let parent = text.base().parent;
                *text = axes_title(id, x, halign, &rc);
                text.base_mut().parent = parent;
            }
        }
        for spine in spines {
            if let Some(spine) = self.artist_as_mut::<SpineArtist>(spine) {
                spine.position = SpinePosition::default();
            }
        }
        for (name, share) in [(AxisName::X, shares.0), (AxisName::Y, shares.1)] {
            match share {
                // Sharing again with the same partner cannot fail.
                Some(other) => {
                    let _ = self.share_axis(name, id, other);
                }
                // matplotlib's `axis._set_lim(0, 1, auto=True)`, which the
                // axes sharing this one follow.
                None => self.set_axis_limits(id, name, (0.0, 1.0), Some(true), true),
            }
        }
        self.stale = true;
        self.layout.pending = true;
    }
}

impl Axes {
    /// Removes everything drawn in the axes and resets it to a fresh axes
    /// at the same place under the current rcParams (matplotlib's
    /// `Axes.clear`): titles, labels, limits (autoscaled from `(0, 1)`),
    /// scales, tickers, grids and the property cycle start over; the
    /// position, aspect, sharing, background colour and frame are kept,
    /// and polar, geographic and custom axes stay so (a custom projection
    /// sets the axes up again). The limit callbacks of the axes are
    /// disconnected; those of the axes sharing its limits see them reset.
    pub fn clear(&self) -> &Self {
        let polar = self.projection() == ProjectionKind::Polar;
        let custom = self.take_custom_projection();
        let figure = self.raw().figure_state();
        let disconnected = figure.borrow_mut().limit_callbacks.take_axes(self.id());
        figure.borrow_mut().clear_axes(self.id());
        // Dropped once the figure is free: a callback may own handles whose
        // drop uses it.
        drop(disconnected);
        if polar {
            crate::polar::PolarAxes::convert(self.clone());
        } else if let Some(projection) = custom {
            crate::projections::reinstall(self, projection);
        }
        super::run_limit_callbacks(figure);
        self
    }

    /// Clears the axes (matplotlib's `cla`, an alias of
    /// [`Axes::clear`]).
    pub fn cla(&self) -> &Self {
        self.clear()
    }
}

#[cfg(test)]
mod tests {
    use crate::axes::{AxisName, ProjectionKind, TickParams, TitleLocation};
    use crate::colors::Color;
    use crate::figure::Figure;

    #[test]
    fn clearing_starts_over_in_place() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.plot([0.0, 5.0], [0.0, 10.0]).unwrap();
        ax.set_title("before");
        ax.set_xlabel("x").set_xscale("log").unwrap();
        ax.set_facecolor("0.9");
        let position = ax.position();
        ax.clear();
        assert_eq!(ax.lines(), Vec::<crate::lines::Line2D>::new());
        assert_eq!(ax.title(TitleLocation::Center), "");
        assert_eq!(ax.label(AxisName::X), "");
        assert_eq!(ax.xlim(), (0.0, 1.0));
        assert_eq!(ax.position(), position);
        assert_eq!(ax.facecolor(), crate::colors::Color::from("0.9"));
        // The colour cycle restarts.
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        assert_eq!(line.rgba(), crate::colors::Color::from("C0").resolve());
    }

    #[test]
    fn cleared_axes_autoscale_to_a_new_scale() {
        // Back at the unit box, the view counts as untouched (matplotlib's
        // `viewLim.mutatedy()`), so a log scale picks its default limits.
        let ax = Figure::new().subplot();
        ax.set_ylim(2.0, 3.0).expect("finite limits");
        ax.clear();
        ax.set_yscale("log").unwrap();
        let (low, high) = ax.ylim();
        assert!((low - 10f64.powf(-0.05)).abs() < 1e-12, "{low}");
        assert!((high - 10f64.powf(1.05)).abs() < 1e-12, "{high}");
    }

    #[test]
    fn tick_parameters_survive_but_the_grid_restarts() {
        // matplotlib's test_axes_clear_behavior.
        let ax = Figure::new().subplot();
        ax.tick_params(TickParams {
            length: Some(10.0),
            grid_color: Some(Color::from("r")),
            labelfontfamily: Some("serif".into()),
            ..TickParams::default()
        });
        ax.grid(true);
        ax.clear();
        let style = ax.xaxis().tick_style(false);
        assert_eq!(style.size, 10.0);
        assert_eq!(style.label_family.as_deref(), Some("serif"));
        assert_eq!(style.grid.color, Color::from("r"));
        assert!(!style.grid_on);
    }

    #[test]
    fn shared_axes_take_their_partners_limits() {
        let fig = Figure::new();
        let top = fig.add_subplot(2, 1, 1).unwrap();
        let bottom = fig.add_subplot(2, 1, 2).unwrap();
        bottom.share_x(&top).unwrap();
        top.set_xlim(2.0, 3.0).expect("finite limits");
        bottom.clear();
        assert_eq!(bottom.xlim(), (2.0, 3.0));
        bottom.set_xlim(4.0, 5.0).expect("finite limits");
        assert_eq!(top.xlim(), (4.0, 5.0), "still shared");
    }

    #[test]
    fn clearing_the_leader_resets_its_followers() {
        // matplotlib's test_inverted_cla: the leader's `_set_lim(0, 1)`
        // reaches the axes sharing its y axis.
        let fig = Figure::new();
        let leader = fig.add_subplot(2, 1, 1).unwrap();
        let follower = fig.add_subplot(2, 1, 2).unwrap();
        follower.share_y(&leader).unwrap();
        leader.yaxis().set_inverted(true);
        follower.plot([0.0, 1.0], [5.0, 6.0]).unwrap();
        assert!(follower.yaxis().inverted());
        leader.clear();
        assert_eq!(follower.ylim(), (0.0, 1.0));
        assert!(follower.autoscale_on(Some(AxisName::Y)));
    }

    #[test]
    fn polar_axes_stay_polar() {
        let fig = Figure::new();
        let ax = fig.polar_subplot();
        ax.set_theta_offset(1.0);
        ax.plot([0.0, 1.0], [1.0, 2.0]).unwrap();
        ax.clear();
        assert_eq!(ax.projection(), ProjectionKind::Polar);
        assert_eq!(ax.theta_offset(), 0.0);
        assert_eq!(ax.lines(), Vec::<crate::lines::Line2D>::new());
    }
}
