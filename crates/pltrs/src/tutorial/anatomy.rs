//! Chapter 2: the anatomy of a figure, and the handles to its parts.
//!
//! A figure is a tree of *artists*, the things that pltrs draws. From the
//! top down:
//!
//! * The [`Figure`] has a size, a resolution and a background color. It
//!   holds axes, and texts and legends for the whole figure
//!   ([`Figure::suptitle`], [`Figure::legend`]).
//! * An [`Axes`] maps data coordinates to a rectangle of the figure and
//!   holds what you plot in it.
//! * An [`Axis`] ([`Axes::xaxis`], [`Axes::yaxis`]) is the scale along one
//!   direction, with limits, a scale type, ticks and a label.
//! * A tick is a mark at a value on an axis, with a label. A *locator*
//!   selects the tick positions, and a *formatter* writes the labels
//!   (chapter [`scales`](super::scales)). The grid is a line at each tick.
//! * The four spines ([`Axes::spines`]) are the lines at the edges of the
//!   data area. They carry the ticks. You can hide or move them.
//! * The legend ([`Axes::legend`]) lists the labeled artists of the axes.
//! * The other artists are [lines](crate::lines::Line2D),
//!   [patches](crate::patches::Patch) (shapes),
//!   [collections](crate::collections::Collection) (many shapes, such as
//!   the markers of a scatter plot), [texts](crate::text::Text) and
//!   [images](crate::image::AxesImage).
//!
//! This example builds a small figure and then walks down its tree.
//!
//! This example builds a small figure and then walks down its tree.
//!
//! The example builds a small figure and then walks down its tree.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0);
//! let ax = fig.subplot();
//! let props = LineProps::default().label("measured");
//! let line = ax.plot_with([0.0, 1.0, 2.0, 3.0], [1.0, 3.0, 2.0, 4.0], props)?;
//! ax.set_title("Parts of a figure");
//! ax.set_xlabel("time").set_ylabel("value");
//! ax.set_xticks([0.0, 1.0, 2.0, 3.0]);
//! ax.legend(LegendOptions::default())?;
//!
//! // The figure holds one axes, which holds the line, a title and a legend.
//! assert_eq!(fig.axes(), [ax.clone()]);
//! assert_eq!(ax.lines(), [line]);
//! assert_eq!(ax.shown_title().1, "Parts of a figure");
//! assert!(ax.current_legend().is_some());
//!
//! // The x axis has the ticks set above, their labels and the axis label.
//! let xaxis = ax.xaxis();
//! assert_eq!(xaxis.ticklocs(false)?, [0.0, 1.0, 2.0, 3.0]);
//! assert_eq!(xaxis.ticklabels(false)?, ["0", "1", "2", "3"]);
//! assert_eq!(xaxis.label_text(), "time");
//!
//! // The spines frame the data area.
//! ax.spines().top.set_visible(false);
//! ax.spines().right.set_visible(false);
//! assert!(!ax.spines().top.visible());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The argument `false` selects the major ticks. The minor ticks between
//! them are smaller and have no labels.
//!
//! The modern look hides the top and right spines and the tick marks, and
//! draws a grid. The matplotlib look draws the full frame with ticks and no
//! grid ([`look`](crate::look)). The tree is the same.
//!
//! # Artists share their properties
//!
//! All artists have visibility, opacity (alpha), z-order, a label and
//! clipping. An artist with a higher z-order is drawn on top. The trait
//! [`ArtistHandle`] in the prelude gives these methods to every handle.
//! Each artist type adds its own properties, for example the width of a
//! [`Line2D`] or the font of a [`Text`](crate::text::Text).
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let line = ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.0])?;
//! let note = ax.text(1.0, 1.0, "the peak");
//!
//! line.set_zorder(3.0).set_alpha(Some(0.5)); // ArtistHandle, on any artist
//! note.set_visible(false);
//! line.set_linewidth(3.0); // Line2D's own
//! note.set_fontsize(14.0); // Text's own
//!
//! assert_eq!((line.zorder(), line.alpha()), (3.0, Some(0.5)));
//! assert!(!note.visible());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Handles
//!
//! The figure owns all its artists. A [`Figure`], [`Axes`] or [`Line2D`]
//! that you hold is a *handle*: a reference-counted pointer to the figure
//! plus the id of the artist. Thus:
//!
//! * A clone of a handle refers to the same artist. A change through one
//!   handle shows in all others. Handles to the same artist are equal.
//! * Methods take `&self`, because handles use interior mutability. You
//!   never need `mut` or `&mut` to change a figure. Setters return the
//!   handle, so you can chain calls.
//! * Handles have no lifetime. You can return them, store them and move
//!   them into closures. A handle keeps its figure alive.
//! * Handles are not `Send`. Build, draw and save a figure on one thread.
//!   To draw in parallel, give each thread its own figures. Plotting
//!   settings are also per thread.
//! * Getters return copies, so a value that you hold never borrows the
//!   figure.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! /// Adds a dashed horizontal line at `y` across `ax`, and returns it.
//! fn baseline(ax: &Axes, y: f64) -> Line2D {
//!     ax.axhline(y, LineProps::default().linestyle(LineStyle::Dashed))
//! }
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! let zero = baseline(&ax, 0.0);
//!
//! let same = line.clone(); // a second handle, not a second line
//! same.set_linewidth(4.0);
//! assert_eq!(line.linewidth(), 4.0);
//! assert_eq!(line, same);
//! assert_ne!(line, zero);
//!
//! // Every handle knows where it lives.
//! assert_eq!(line.axes(), Some(ax.clone()));
//! assert_eq!(line.figure(), fig);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Removing and putting back
//!
//! [`ArtistHandle::remove`] takes an artist out of its figure. pltrs stops
//! drawing it, and its axes stop using it for their limits. The handle of
//! the *detached* artist still works, and [`Axes::attach`] attaches it
//! again. A figure keeps its detached artists until you drop it. Thus, an
//! animation must change its artists in place, not remove and add them in
//! each frame.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! line.remove();
//! assert!(ax.lines().is_empty() && line.axes().is_none());
//!
//! line.set_color("tab:red"); // still usable
//! ax.attach(&line)?;
//! assert_eq!(ax.lines(), [line]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Finding artists
//!
//! An axes lists its artists by kind ([`Axes::lines`],
//! [`Axes::patches`], [`Axes::collections`], [`Axes::texts`],
//! [`Axes::images`]). [`ArtistHandle::findobj`] searches all artists below
//! an artist and returns untyped handles.
//! [`RawHandle::downcast`](crate::artist::RawHandle::downcast) makes them
//! typed again.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let axs = fig.subplots(1, 2)?;
//! axs[(0, 0)].plot([0.0, 1.0], [0.0, 1.0])?;
//! axs[(0, 1)].plot([0.0, 1.0], [1.0, 0.0])?;
//! let lines: Vec<Line2D> = fig.findobj(|_| true).iter().filter_map(|a| a.downcast()).collect();
//! assert_eq!(lines.len(), 2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The page [`porting::artists`](crate::porting::artists) lists the
//! properties of each kind of artist.
//!
//! Next: [plot types](super::plot_types).
//!
//! [`Figure`]: crate::figure::Figure
//! [`Figure::suptitle`]: crate::figure::Figure::suptitle
//! [`Figure::legend`]: crate::figure::Figure::legend
//! [`Axes`]: crate::axes::Axes
//! [`Axes::xaxis`]: crate::axes::Axes::xaxis
//! [`Axes::yaxis`]: crate::axes::Axes::yaxis
//! [`Axes::spines`]: crate::axes::Axes::spines
//! [`Axes::legend`]: crate::axes::Axes::legend
//! [`Axes::attach`]: crate::axes::Axes::attach
//! [`Axes::lines`]: crate::axes::Axes::lines
//! [`Axes::patches`]: crate::axes::Axes::patches
//! [`Axes::collections`]: crate::axes::Axes::collections
//! [`Axes::texts`]: crate::axes::Axes::texts
//! [`Axes::images`]: crate::axes::Axes::images
//! [`Axis`]: crate::axes::Axis
//! [`ArtistHandle`]: crate::artist::ArtistHandle
//! [`ArtistHandle::remove`]: crate::artist::ArtistHandle::remove
//! [`ArtistHandle::findobj`]: crate::artist::ArtistHandle::findobj
//! [`Line2D`]: crate::lines::Line2D
