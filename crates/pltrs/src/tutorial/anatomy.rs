//! Chapter 2: the anatomy of a figure, and the handles that reach its
//! parts.
//!
//! A figure is a tree. The [`Figure`] holds its axes, plus texts and
//! legends of its own. Each [`Axes`] holds what is drawn in it: lines,
//! shapes, texts, images, its title and its legend. Each axes also has
//! two [`Axis`] values, x and y, which own the limits, the ticks and the
//! axis label, and four spines, the lines around the data area. matplotlib
//! calls every node of this tree that can be drawn an *artist*.
//!
//! # The parts
//!
//! From the outside in:
//!
//! * The **figure** is the canvas: its size, resolution and background
//!   colour. It holds axes, and figure-wide texts and legends
//!   ([`Figure::suptitle`], [`Figure::legend`]).
//! * An **axes** is one plotting area. It maps data coordinates to a
//!   rectangle of the figure and holds what you plot in it.
//! * An **axis** ([`Axes::xaxis`], [`Axes::yaxis`]) is the scale along
//!   one direction: its limits, how values map to positions (linear,
//!   logarithmic, ...), its ticks and its label.
//! * A **tick** is a mark at a value along an axis, with a tick label
//!   writing the value. A *locator* decides where ticks go and a
//!   *formatter* writes their labels (chapter [`scales`](super::scales)).
//!   The grid is a line across the axes at each tick.
//! * The **spines** ([`Axes::spines`]) are the four lines bounding the
//!   data area. They carry the ticks, can be hidden or moved, and give
//!   the axes their frame.
//! * The **legend** ([`Axes::legend`]) is a box listing the labelled
//!   artists of the axes, each with a small sample of how it is drawn.
//! * **Artists** are everything drawn: [lines](crate::lines::Line2D),
//!   [patches](crate::patches::Patch) (shapes: rectangles, circles,
//!   polygons, ...), [collections](crate::collections::Collection) (many
//!   shapes drawn at once, such as the markers of a scatter plot),
//!   [texts](crate::text::Text) and [images](crate::image::AxesImage).
//!   The figure and the axes are artists too.
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
//! The `false` given to `ticklocs` and `ticklabels` asks for the major
//! ticks: an axis also has minor ticks, smaller and unlabelled, between
//! them.
//!
//! The two looks differ in which parts they draw by default: pltrs'
//! modern look hides the top and right spines and the tick marks and
//! draws a light grid, where matplotlib's look draws the full frame with
//! ticks and no grid (chapter [`styling`](super::styling), and
//! [`look`](crate::look) for the full list). The tree is the same in
//! both.
//!
//! # Artists share their properties
//!
//! Every artist has the same basic properties: whether it is visible, its
//! opacity (alpha), its z-order (artists with a higher one are drawn on
//! top), its label, and how it is clipped to the axes. The
//! [`ArtistHandle`] trait, in the prelude, gives every handle these
//! methods. Each artist type adds its own: a [`Line2D`] has a colour,
//! a width, a dash pattern and markers; a [`Text`](crate::text::Text) has
//! a string, a font and an alignment.
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
//! The figure owns all its artists. What you hold, a [`Figure`], an
//! [`Axes`], a [`Line2D`], is a *handle*: a reference-counted pointer to
//! the figure plus the artist's id. This shapes how pltrs code reads:
//!
//! * **Cloning a handle is cheap and makes another reference to the same
//!   artist**, never a copy of it. A change through one handle shows
//!   through every other. Handles compare equal when they refer to the
//!   same artist.
//! * **Methods take `&self`.** Handles share the figure's state through
//!   interior mutability, so setters work on a shared reference and you
//!   never need `mut` bindings or `&mut` borrows to change a figure.
//!   Setters return the handle again, so calls chain.
//! * **Handles have no lifetime.** Return them from functions, keep them
//!   in your own structs, move clones into closures. Any handle keeps its
//!   figure alive.
//! * **Handles stay on their thread.** They are not `Send`: build, draw
//!   and save a figure on one thread. To draw in parallel, give each
//!   thread figures of its own; plotting settings are per thread too.
//! * **Getters return copies.** `line.linewidth()` copies the width out,
//!   `line.xdata()` clones the data. Nothing you hold borrows the figure,
//!   so nothing you hold can block a change.
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
//! [`ArtistHandle::remove`] takes an artist out of its figure: the figure
//! no longer draws it, and its axes no longer list it or fit their limits
//! to it. The artist is now *detached*, but its handle keeps working:
//! read and change it as before, and attach it again with
//! [`Axes::attach`]. A figure keeps its detached artists until the figure
//! itself is dropped, so a program that removes thousands of artists
//! (an animation replotting every frame, say) should update its artists
//! in place instead.
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
//! The axes list their artists by kind ([`Axes::lines`],
//! [`Axes::patches`], [`Axes::collections`], [`Axes::texts`],
//! [`Axes::images`]). [`ArtistHandle::findobj`] searches everything an
//! artist owns and returns untyped handles, which
//! [`RawHandle::downcast`](crate::artist::RawHandle::downcast) turns back
//! into typed ones.
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
//! The [`porting::artists`](crate::porting::artists) page lists the
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
