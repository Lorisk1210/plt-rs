//! Clearing an axes back to a fresh one at the same place (matplotlib's
//! `Axes.clear` and `cla`).

use std::rc::Rc;

use super::axis::Tickers;
use super::{Axes, AxesState, AxisName, ProjectionKind, TitlePlacement};
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup, KeptArtist};
use crate::axes_grid1::parasites_of;
use crate::figure::{FigureState, TITLE_PLACES, axes_title};
use crate::rc::RcParams;
use crate::scale::LinearScale;
use crate::spines::{SpineArtist, SpinePosition};

impl AxesState {
    /// The state a cleared axes restarts from (matplotlib's `__clear`): a
    /// fresh axes under `rc` that keeps the identity, placement, aspect,
    /// sharing, background colour, frame, navigation and status-bar
    /// formatters of `self`, the options of its projection, whether each
    /// axis is shown (and, for an axes sharing an axis, its background),
    /// the axes the program draws, titles the program places, unless `rc`
    /// fixes their height, the `parasites` it hosts, and what makes it a
    /// parasite (matplotlib's host and parasite `clear` keep them). Its
    /// data limits stay too, as matplotlib's `dataLim` does, for the next
    /// data to replace (autoscaling and a data-limited aspect read them
    /// meanwhile).
    fn cleared(&self, rc: Rc<RcParams>, parasites: &[ArtistId]) -> Self {
        let mut fresh = Self::new(self.position, rc);
        *fresh.base_mut() = self.base().clone();
        fresh.data_lim = self.data_lim;
        fresh
            .projection_options
            .clone_from(&self.projection_options);
        fresh.aspect = self.aspect;
        fresh.axes_locator.clone_from(&self.axes_locator);
        fresh.subplot_spec.clone_from(&self.subplot_spec);
        fresh.home = self.home;
        fresh.shares = self.shares;
        fresh.secondary.clone_from(&self.secondary);
        fresh.parasite = self.parasite;
        fresh.aux_transform.clone_from(&self.aux_transform);
        fresh.spines.clone_from(&self.spines);
        // The titles are made anew (see `FigureState::renew_titles`).
        fresh.former_titles = [self.former_titles.as_slice(), &self.titles].concat();
        fresh.program_axes.clone_from(&self.program_axes);
        fresh.children = (self.spines.iter())
            .chain(&self.program_axes)
            .chain(parasites)
            .copied()
            .collect();
        fresh.navigate = self.navigate;
        fresh.forward_navigation = self.forward_navigation;
        fresh.fmt_data.clone_from(&self.fmt_data);
        fresh
            .patch
            .style
            .facecolor
            .clone_from(&self.patch.style.facecolor);
        fresh.decorations.frame_on = self.decorations.frame_on;
        fresh.axisbelow = self.axisbelow;
        if self.title_placement == TitlePlacement::Program
            && fresh.title_placement == TitlePlacement::Auto
        {
            // Lifted where the program lifts them, as before.
            fresh.title_placement = TitlePlacement::Program;
        }
        for name in [AxisName::X, AxisName::Y] {
            // matplotlib's `Axis.clear` keeps the tick parameters, except
            // whether gridlines are on, which restarts from the rc; the
            // axis object stays, with its picker and pick radius.
            fresh.axis_mut(name).keep_through_clear(self.axis(name));
            // matplotlib's `Axis.clear` disconnects the axes' handler of
            // the axis' changes of units.
            fresh.axis_mut(name).follows_units = false;
        }
        if self.shares.0.is_some() || self.shares.1.is_some() {
            fresh.patch.visible = self.patch.visible;
        }
        fresh
    }
}

impl FigureState {
    /// Removes the artists of axes `id` and resets it (matplotlib's
    /// `Axes.__clear`): the titles are new and empty, the spines at the edges,
    /// the shared axes' scales linear, and shared axes take their
    /// partners' limits again. The parasites of a host stay and are
    /// cleared in turn, and a parasite's decorations stay hidden
    /// (matplotlib's `HostAxesBase.clear` and `ParasiteAxesBase.clear`).
    pub(crate) fn clear_axes(&mut self, id: ArtistId) {
        let rc = crate::rc::params();
        let parasites = parasites_of(self, id);
        let Some(old) = self.artist_as::<AxesState>(id) else {
            return;
        };
        let fresh = old.cleared(Rc::clone(&rc), &parasites);
        let parasite = fresh.parasite.is_some();
        let doomed: Vec<ArtistId> = old
            .children
            .iter()
            .copied()
            .filter(|child| !fresh.children.contains(child) && !old.titles.contains(child))
            .collect();
        let (shares, titles, spines) = (fresh.shares, old.titles.len(), fresh.spines.clone());
        for child in doomed {
            self.remove_artist(child);
        }
        let reset: Vec<_> = [AxisName::X, AxisName::Y]
            .into_iter()
            .filter_map(|name| self.reset_shared_scales(id, name))
            .collect();
        if let Some(state) = self.artist_as_mut::<AxesState>(id) {
            *state = fresh;
        }
        for (name, last, tickers) in reset {
            for member in self.shared.siblings(name, id) {
                if let Some(state) = self.artist_as_mut::<AxesState>(member) {
                    let axis = state.axis_mut(name);
                    axis.use_tickers(tickers.clone());
                    axis.ticker_owner = Some(last);
                }
            }
        }
        self.renew_titles(id, titles, &rc);
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
                None if self.reshare_across(id, name) => {}
                // matplotlib's `axis._set_lim(0, 1, auto=True)`, which the
                // axes sharing this one follow.
                None => self.set_axis_limits(id, name, (0.0, 1.0), Some(true), true),
            }
        }
        if parasite {
            self.hide_parasite_decorations(id);
        }
        for parasite in parasites {
            self.clear_axes(parasite);
        }
        self.stale = true;
        self.layout.pending = true;
    }

    /// Takes out the former titles of every axes, removed or not, that
    /// `held` says no handle reaches, for the caller to drop: matplotlib's
    /// clear leaves its old title texts to whoever holds them, and an
    /// axes cleared for every frame of an animation would keep three more
    /// for every frame.
    pub(crate) fn release_former_titles(
        &mut self,
        held: impl Fn(ArtistId) -> bool,
    ) -> Vec<(ArtistId, KeptArtist)> {
        let axes: Vec<ArtistId> = (self.store.ids().chain(self.detached.ids()))
            .filter(|&id| {
                self.artist_as::<AxesState>(id)
                    .is_some_and(|state| !state.former_titles.is_empty())
            })
            .collect();
        let mut released = Vec::new();
        for id in axes {
            let Some(state) = self.artist_as_mut::<AxesState>(id) else {
                continue;
            };
            let (kept, gone): (Vec<ArtistId>, Vec<ArtistId>) =
                state.former_titles.iter().partition(|&&title| held(title));
            state.former_titles = kept;
            for title in gone {
                if let Some(artist) = self.store.take(title).or_else(|| self.detached.take(title)) {
                    released.push((title, artist));
                }
            }
        }
        released
    }

    /// Gives the cleared axes `id` the first `count` of its titles anew,
    /// empty, listed after its spines as the titles before them were.
    /// matplotlib's `clear` makes new title texts: the old ones stay as
    /// they were ([`AxesState::former_titles`]).
    fn renew_titles(&mut self, id: ArtistId, count: usize, rc: &Rc<RcParams>) {
        let titles: Vec<ArtistId> = (TITLE_PLACES.into_iter().take(count))
            .map(|(x, halign)| self.adopt(id, Box::new(axes_title(id, x, halign, rc))))
            .collect();
        if let Some(state) = self.artist_as_mut::<AxesState>(id) {
            let spines = state.spines.len();
            state.children.retain(|child| !titles.contains(child));
            let at = spines.min(state.children.len());
            state.children.splice(at..at, titles.iter().copied());
            state.titles = titles;
        }
    }

    /// Gives axis `name` of the other axes sharing it with `id` the linear
    /// scale and its default tick helpers in turn, as matplotlib's clear
    /// does on the `Ticker` they all share: the helpers of the last one
    /// stay, its axis the one the locator is bound to. Returns that last
    /// axes and its helpers, which the whole group takes, `None` when no
    /// other axes shares the axis.
    fn reset_shared_scales(
        &mut self,
        id: ArtistId,
        name: AxisName,
    ) -> Option<(AxisName, ArtistId, Tickers)> {
        let mut last = None;
        for sibling in self.shared.siblings(name, id) {
            if let Some(state) = self.artist_as_mut::<AxesState>(sibling)
                && sibling != id
            {
                let axis = state.axis_mut(name);
                axis.set_scale(Rc::new(LinearScale));
                last = Some((name, sibling, axis.tickers()));
            }
        }
        last
    }
}

impl Axes {
    /// Removes everything drawn in the axes and resets it to a fresh axes
    /// at the same place under the current rcParams (matplotlib's
    /// `Axes.clear`): titles, labels, limits (autoscaled from `(0, 1)`),
    /// scales, tickers, grids and the property cycle start over; the
    /// position, aspect, sharing, background colour and frame are kept,
    /// and polar, geographic and custom axes stay so (a custom projection
    /// sets the axes up again). The parasites of a host stay, cleared.
    /// The limit callbacks of the axes and its parasites are
    /// disconnected; those of the axes sharing its limits see them reset.
    pub fn clear(&self) -> &Self {
        let polar = self.projection() == ProjectionKind::Polar;
        // matplotlib's polar clear resets the origin and the θ offset and
        // direction, not where the radial labels are.
        let label_angle = self.as_polar().map(|polar| polar.rlabel_position());
        let custom = self.take_custom_projection();
        let figure = self.raw().figure_state();
        let disconnected: Vec<_> = {
            let mut state = figure.borrow_mut();
            let owners: Vec<ArtistId> = std::iter::once(self.id())
                .chain(parasites_of(&*state, self.id()))
                .collect();
            owners
                .into_iter()
                .flat_map(|owner| state.limit_callbacks.take_axes(owner))
                .collect()
        };
        figure.borrow_mut().clear_axes(self.id());
        // Dropped once the figure is free: a callback may own handles whose
        // drop uses it.
        drop(disconnected);
        if polar {
            let polar = crate::polar::PolarAxes::convert(self.clone());
            if let Some(angle) = label_angle {
                polar.set_rlabel_position(angle);
            }
        }
        if let Some(projection) = custom {
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
    use crate::axes::{Axes, AxisName, ProjectionKind, TickParams, TitleLocation, TitlePlacement};
    use crate::colors::Color;
    use crate::figure::{Figure, SaveError};

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
    fn a_hidden_axis_stays_hidden_through_a_clear() {
        // matplotlib's `Axis.clear` leaves the axis artist its visibility
        // (a projection class hiding the x and y axis in `_init_axis`
        // keeps them hidden through the clear of its `__init__`).
        let ax = Figure::new().subplot();
        ax.xaxis().set_visible(false);
        ax.yaxis().set_in_layout(false);
        ax.clear();
        assert!(!ax.xaxis().visible() && ax.yaxis().visible());
        assert!(ax.xaxis().in_layout() && !ax.yaxis().in_layout());
    }

    #[test]
    fn clearing_binds_the_shared_locator_to_the_last_other_sibling() {
        // matplotlib's clear gives every other axes sharing an axis the
        // linear scale in turn, on the one `Ticker` they share: the
        // locator ends bound to the last of them, whose length decides
        // how many ticks fit along every axes of the group.
        let fig = Figure::with_size(6.0, 4.0, 100.0);
        let wide = fig
            .add_axes([0.1, 0.55, 0.8, 0.4])
            .expect("a valid rectangle");
        let narrow = fig
            .add_axes([0.1, 0.1, 0.1, 0.4])
            .expect("a valid rectangle");
        narrow.share_x(&wide).unwrap();
        let locs = |ax: &Axes| -> Vec<f64> {
            ax.set_xlim(0.0, 24.0).expect("finite limits");
            fig.layout();
            let ticks = ax.ticks(AxisName::X).unwrap();
            ticks.major.iter().map(|t| t.loc).collect()
        };
        let wide_ticks = locs(&wide);
        narrow.clear();
        assert_eq!(locs(&narrow), wide_ticks, "bound to the wide axis");
        wide.clear();
        let narrow_ticks = locs(&wide);
        assert!(
            narrow_ticks.len() < wide_ticks.len(),
            "bound to the narrow axis"
        );
        assert_eq!(locs(&narrow), narrow_ticks);
    }

    #[test]
    fn a_clear_keeps_the_data_limits_for_the_next_data_to_replace() {
        // matplotlib's clear leaves `dataLim`, which autoscaling and a
        // data-limited aspect read until data replace it.
        let ax = Figure::new().subplot();
        ax.plot([2.0, 5.0], [1.0, 3.0]).unwrap();
        let limits = ax.data_lim();
        ax.clear();
        assert_eq!(ax.data_lim(), limits);
        assert!(ax.ignores_data_limits());
        assert!(
            !ax.xaxis().follows_units(),
            "the clear disconnects the handler"
        );
        ax.plot([7.0, 8.0], [7.0, 9.0]).unwrap();
        assert_eq!(ax.xaxis().data_interval(), (7.0, 8.0), "replaced");
    }

    #[test]
    fn titles_keep_their_place_through_a_clear() {
        // Found by fuzzing: a title reset by `cla` lost its id, so a draw
        // reporting malformed mathtext in it panicked.
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.clear();
        ax.set_title("$$");
        assert!(matches!(fig.to_png(), Err(SaveError::Draw(_))));
        ax.set_title("fine");
        fig.to_png().expect("draws");
    }

    #[test]
    fn titles_the_program_places_stay_its_own() {
        let ax = Figure::new().subplot();
        ax.set_title_placement(TitlePlacement::Program);
        ax.clear();
        assert_eq!(ax.title_placement(), TitlePlacement::Program);
        crate::rc::context(|| {
            crate::rc::set("axes.titley", "1.1").expect("a valid height");
            ax.clear();
        });
        assert_eq!(ax.title_placement(), TitlePlacement::Fixed);
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
        ax.set_rlabel_position(45.0);
        ax.plot([0.0, 1.0], [1.0, 2.0]).unwrap();
        ax.clear();
        assert_eq!(ax.projection(), ProjectionKind::Polar);
        assert_eq!(ax.theta_offset(), 0.0);
        assert!(
            (ax.rlabel_position() - 45.0).abs() < 1e-12,
            "the labels stay"
        );
        assert_eq!(ax.lines(), Vec::<crate::lines::Line2D>::new());
    }
}
