//! A plotting library for Rust, ported from matplotlib 3.11.
//!
//! pltrs draws figures: lines, markers, bars, histograms, images, contours,
//! vector fields, 3-D surfaces, text and more. It writes them to image files
//! and documents, and it shows them in windows. It has the model and the
//! names of matplotlib. Where matplotlib takes keyword arguments, strings and
//! exceptions, pltrs takes option structs, enums and `Result`s.
//!
//! Add pltrs to your project:
//!
//! ```sh
//! cargo add pltrs
//! ```
//!
//! pltrs has no dependencies on crates.io. The code for fonts, image formats,
//! drawing, text layout and windows is in the `pltrs-*` crates of its
//! workspace.
//!
//! # A first figure
//!
//! This program draws a sine wave with a title, labels and a legend:
//!
//! 1. [`Figure::with_size`] makes a figure of 6.4 × 4 inches at 100 dots per
//!    inch.
//! 2. [`Figure::subplot`] adds one [`Axes`], the area that holds the plot.
//! 3. [`linspace`](numeric::linspace) makes 200 x values.
//! 4. [`Axes::plot_with`] draws the line with the label "sin".
//! 5. [`Axes::set_title`], [`Axes::set_xlabel`] and [`Axes::set_ylabel`] add
//!    the texts.
//! 6. [`Axes::legend`] adds a legend for the labeled line.
//! 7. [`Figure::to_png`] encodes the figure as PNG bytes in memory.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let fig = Figure::with_size(6.4, 4.0, 100.0);
//! let ax = fig.subplot();
//! let x = linspace(0.0, 10.0, 200);
//! let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
//! ax.plot_with(&x, y, LineProps::default().label("sin"))?;
//! ax.set_title("A sine wave");
//! ax.set_xlabel("x").set_ylabel("sin x");
//! ax.legend(LegendOptions::default())?;
//! let png = fig.to_png()?;
//! assert!(png.starts_with(b"\x89PNG"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! To write a file, call `fig.savefig("sine.png")?`
//! ([`Figure::savefig`]). To open a window, call
//! `pltrs::gui::show(&[fig])?` ([`gui::show`]).
//!
//! # What you can do
//!
//! ## Figures
//!
//! A [`Figure`] is the whole image. It holds axes, subfigures, texts and
//! images, and it has a size in inches and a resolution in dots per inch.
//!
//! | Task                                | Item |
//! |-------------------------------------|------|
//! | Make a figure with the default size | [`Figure::new`] |
//! | Make a figure with a size and a resolution | [`Figure::with_size`], [`Figure::with_figsize`] with a [`FigSize`](figure::FigSize) in inches, centimeters or pixels |
//! | Change the size or the resolution   | [`Figure::set_size_inches`], [`Figure::set_dpi`] |
//! | Add one axes                        | [`Figure::subplot`], [`Figure::add_subplot`], [`Figure::add_axes`] |
//! | Add a grid of axes                  | [`Figure::subplots`], [`Figure::subplots_shared`] |
//! | Add axes that span grid cells       | [`Figure::add_gridspec`], [`GridSpec`](gridspec::GridSpec), [`Figure::add_subplot_spec`] |
//! | Add axes from a text layout         | [`Figure::subplot_mosaic`] with a [`Mosaic`](figure::Mosaic) such as `"AB;CC"` |
//! | Add figures inside a figure         | [`Figure::subfigures`], [`Figure::add_subfigure`] |
//! | Add figure titles and texts         | [`Figure::suptitle`], [`Figure::supxlabel`], [`Figure::supylabel`], [`Figure::text`] |
//! | Add a figure legend or a colorbar   | [`Figure::legend`], [`Figure::colorbar`] |
//! | Draw an image in pixels             | [`Figure::figimage`] |
//! | Place the axes so that labels fit   | [`Figure::set_layout_engine`] with a [`LayoutEngine`](layout::LayoutEngine), [`Figure::tight_layout`] |
//! | Set the margins between subplots    | [`Figure::subplots_adjust`] |
//! | Align the labels of several axes    | [`Figure::align_labels`], [`Figure::align_titles`] |
//! | Rotate date labels                  | [`Figure::autofmt_xdate`] |
//! | Save the figure                     | [`Figure::savefig`], see [Output](#output) |
//! | Show the figure                     | [`gui::show`], see [Windows and interaction](#windows-and-interaction) |
//!
//! The [`axes_grid1`] module places axes with dividers and image grids. The
//! [`axisartist`] module draws axes with curved grids and floating axes.
//!
//! ## Plot types
//!
//! Each plotting method of [`Axes`] adds artists and returns a handle to
//! them. Methods that take keywords in matplotlib take an option struct.
//!
//! Lines and markers:
//!
//! | Method | What it draws |
//! |--------|---------------|
//! | [`plot`](axes::Axes::plot), [`plot_y`](axes::Axes::plot_y), [`plot_fmt`](axes::Axes::plot_fmt), [`plot_with`](axes::Axes::plot_with) | a line through points, with markers or a format string such as `"r--"` |
//! | [`loglog`](axes::Axes::loglog), [`semilogx`](axes::Axes::semilogx), [`semilogy`](axes::Axes::semilogy) | a line on logarithmic axes |
//! | [`scatter`](axes::Axes::scatter), [`scatter_with`](axes::Axes::scatter_with) | markers with a size and a color for each point |
//! | [`step`](axes::Axes::step), [`stairs`](axes::Axes::stairs) | a step function |
//! | [`errorbar`](axes::Axes::errorbar) | points with error bars |
//! | [`stem`](axes::Axes::stem) | vertical lines from a baseline to each point |
//! | [`eventplot`](axes::Axes::eventplot) | rows of short lines at event positions |
//! | [`hlines`](axes::Axes::hlines), [`vlines`](axes::Axes::vlines) | horizontal or vertical lines in data coordinates |
//! | [`axhline`](axes::Axes::axhline), [`axvline`](axes::Axes::axvline), [`axline`](axes::Axes::axline) | reference lines across the axes |
//! | [`axhspan`](axes::Axes::axhspan), [`axvspan`](axes::Axes::axvspan) | bands across the axes |
//!
//! Bars and histograms:
//!
//! | Method | What it draws |
//! |--------|---------------|
//! | [`bar`](axes::Axes::bar), [`barh`](axes::Axes::barh) | vertical or horizontal bars |
//! | [`bar_label`](axes::Axes::bar_label) | value labels on bars |
//! | [`grouped_bar`](axes::Axes::grouped_bar) | groups of bars side by side |
//! | [`broken_barh`](axes::Axes::broken_barh) | horizontal ranges, for example a timeline |
//! | [`hist`](axes::Axes::hist) | a histogram of one or more datasets |
//! | [`hist2d`](axes::Axes::hist2d), [`hexbin`](axes::Axes::hexbin) | a 2-D histogram on rectangles or hexagons |
//!
//! Statistics:
//!
//! | Method | What it draws |
//! |--------|---------------|
//! | [`boxplot`](axes::Axes::boxplot), [`bxp`](axes::Axes::bxp) | box plots from data or from computed statistics |
//! | [`violinplot`](axes::Axes::violinplot), [`violin`](axes::Axes::violin) | violin plots from data or from computed statistics |
//! | [`ecdf`](axes::Axes::ecdf) | an empirical cumulative distribution |
//! | [`psd`](axes::Axes::psd), [`csd`](axes::Axes::csd), [`cohere`](axes::Axes::cohere) | power spectral density, cross spectral density and coherence |
//! | [`specgram`](axes::Axes::specgram) | a spectrogram |
//! | [`magnitude_spectrum`](axes::Axes::magnitude_spectrum), [`angle_spectrum`](axes::Axes::angle_spectrum), [`phase_spectrum`](axes::Axes::phase_spectrum) | the spectrum of a signal |
//! | [`xcorr`](axes::Axes::xcorr), [`acorr`](axes::Axes::acorr) | cross correlation and autocorrelation |
//!
//! The [`stats`] and [`mlab`] modules compute the statistics and spectra
//! without a figure.
//!
//! Areas:
//!
//! | Method | What it draws |
//! |--------|---------------|
//! | [`fill`](axes::Axes::fill) | filled polygons |
//! | [`fill_between`](axes::Axes::fill_between), [`fill_betweenx`](axes::Axes::fill_betweenx) | the area between two curves |
//! | [`stackplot`](axes::Axes::stackplot) | stacked areas |
//!
//! Images and meshes:
//!
//! | Method | What it draws |
//! |--------|---------------|
//! | [`imshow`](axes::Axes::imshow) | an image from an array of values or colors |
//! | [`matshow`](axes::Axes::matshow) | a matrix as an image |
//! | [`pcolormesh`](axes::Axes::pcolormesh), [`pcolor`](axes::Axes::pcolor), [`pcolorfast`](axes::Axes::pcolorfast) | a grid of colored cells |
//! | [`nonuniform_image`](axes::Axes::nonuniform_image) | an image on a grid with unequal spacing |
//! | [`spy`](axes::Axes::spy) | the nonzero entries of a matrix |
//!
//! Contours and triangulations:
//!
//! | Method | What it draws |
//! |--------|---------------|
//! | [`contour`](axes::Axes::contour), [`contourf`](axes::Axes::contourf) | contour lines and filled contours |
//! | [`ContourSet::clabel`](contour::ContourSet::clabel) | labels on contour lines |
//! | [`tricontour`](axes::Axes::tricontour), [`tricontourf`](axes::Axes::tricontourf) | contours on a triangular grid |
//! | [`tripcolor`](axes::Axes::tripcolor) | colored triangles |
//! | [`triplot`](axes::Axes::triplot) | the edges of a triangulation |
//!
//! The [`tri`] module makes Delaunay triangulations, and it interpolates
//! and refines values on them.
//!
//! Vector fields:
//!
//! | Method | What it draws |
//! |--------|---------------|
//! | [`quiver`](axes::Axes::quiver), [`quiverkey`](axes::Axes::quiverkey) | arrows and a key for their length |
//! | [`barbs`](axes::Axes::barbs) | wind barbs |
//! | [`streamplot`](axes::Axes::streamplot) | streamlines |
//!
//! Pies and others:
//!
//! | Item | What it draws |
//! |------|---------------|
//! | [`pie`](axes::Axes::pie), [`pie_label`](axes::Axes::pie_label) | a pie chart and its labels |
//! | [`table`](axes::Axes::table) | a table of text cells |
//! | [`Sankey`](sankey::Sankey) | a flow diagram |
//! | [`arrow`](axes::Axes::arrow) | an arrow in data coordinates |
//! | [`add_patch`](axes::Axes::add_patch), [`add_collection`](axes::Axes::add_collection) | a shape or a collection that you make |
//!
//! Plotting methods take numbers of any primitive type, category names,
//! dates, and values of your own types. [`porting::conventions`] explains
//! these data types.
//!
//! ## 3-D, polar and geographic axes
//!
//! These axes have their own types. They have the plotting methods of
//! [`Axes`] that make sense for them.
//!
//! | Axes | Make it with | What it has |
//! |------|--------------|-------------|
//! | 3-D | [`Figure::subplot_3d`], [`Figure::add_subplot_3d`] | [`Axes3D`](mplot3d::Axes3D): [`plot_surface`](mplot3d::Axes3D::plot_surface), [`plot_wireframe`](mplot3d::Axes3D::plot_wireframe), [`plot_trisurf`](mplot3d::Axes3D::plot_trisurf), [`scatter`](mplot3d::Axes3D::scatter), [`bar3d`](mplot3d::Axes3D::bar3d), [`voxels`](mplot3d::Axes3D::voxels), [`quiver`](mplot3d::Axes3D::quiver), [`contour`](mplot3d::Axes3D::contour), and the camera ([`view_init`](mplot3d::Axes3D::view_init)) |
//! | polar | [`Figure::polar_subplot`], [`Figure::add_polar_subplot`] | [`PolarAxes`](polar::PolarAxes): angle and radius in place of x and y, a full circle or an annular sector |
//! | geographic | [`Figure::geo_subplot`] | [`GeoAxes`](geo::GeoAxes): the Aitoff, Hammer, Lambert and Mollweide map projections |
//!
//! [`mplot3d::LightSource`] shades surfaces, bars and voxels. A 3-D figure
//! rotates with the mouse in a window.
//!
//! To add a projection of your own, implement the
//! [`Projection`](projections::Projection) trait, register it with
//! [`projections::register`], and make axes with
//! [`Figure::subplot_with_projection`].
//!
//! ## Axes
//!
//! An [`Axes`] is one plotting area with an x axis and a y axis. Each
//! [`Axis`](axes::Axis) has limits, a scale, ticks and a label.
//!
//! | Task | Item |
//! |------|------|
//! | Set the limits | [`Axes::set_xlim`], [`Axes::set_ylim`], [`Axes::set_margins`], [`Axes::autoscale`] |
//! | Invert an axis | [`Axis::set_inverted`](axes::Axis::set_inverted) |
//! | Set the scale | [`Axes::set_xscale`] with linear, log, symlog, logit, asinh or a function scale ([`scale`]) |
//! | Set the ticks and tick labels | [`Axes::set_xticks`], [`Axes::set_xticklabels`], [`Axes::minorticks_on`] |
//! | Select a tick locator or formatter | [`Axis::set_major_locator`](axes::Axis::set_major_locator), [`Axis::set_major_formatter`](axes::Axis::set_major_formatter), see [`ticker`] |
//! | Change the look of the ticks | [`Axes::tick_params`], [`Axes::ticklabel_format`], [`Axes::locator_params`] |
//! | Plot dates and times | [`dates`]: [`DateTime`](dates::DateTime), date locators such as [`AutoDateLocator`](dates::AutoDateLocator), [`ConciseDateFormatter`](dates::ConciseDateFormatter), [`Axes::xaxis_date`] |
//! | Plot categories | strings as data, see [`units::Categories`] |
//! | Share axes between subplots | [`Figure::subplots_shared`], [`Axes::share_x`], [`Axes::share_y`] |
//! | Add a twin axes | [`Axes::twinx`], [`Axes::twiny`] |
//! | Add a secondary axis | [`Axes::secondary_xaxis`], [`Axes::secondary_yaxis`] |
//! | Set the aspect ratio | [`Axes::set_aspect`], [`Axes::set_box_aspect`] |
//! | Change or hide the spines | [`Axes::spines`], [`spines`] |
//! | Show a grid | [`Axes::grid`] |
//! | Hide inner labels in a grid | [`Axes::label_outer`] |
//! | Hide the axes | [`Axes::set_axis_off`] |
//! | Set the color cycle | [`Axes::set_prop_cycle`] |
//!
//! ## Text
//!
//! Each text is a [`Text`](text::Text) artist with a font, a size, a color,
//! an alignment and a rotation. Text between `$` signs is mathtext.
//!
//! | Task | Item |
//! |------|------|
//! | Titles and axis labels | [`Axes::set_title`], [`Axes::set_xlabel`], [`Axes::set_ylabel`], [`Figure::suptitle`] |
//! | Text at a position | [`Axes::text`], [`Figure::text`] |
//! | Annotations with arrows | [`Axes::annotate`] with [`AnnotateOptions`](text::AnnotateOptions) |
//! | Coordinate systems for annotations | [`coords`] |
//! | Text in a box | [`Text::set_bbox`](text::Text::set_bbox) |
//! | Math notation such as `$\alpha^2$` | mathtext in the `DejaVu`, Computer Modern and STIX font sets ([`Fontset`](fonts::Fontset)), [`text::math_to_image`] |
//! | Text that LaTeX typesets | the rcParam `text.usetex`, [`text::check_usetex`] |
//! | Fonts | [`FontProperties`](fonts::FontProperties), [`FontManager`](fonts::FontManager) |
//! | OpenType features and languages | [`Text::set_fontfeatures`](text::Text::set_fontfeatures), [`Text::set_language`](text::Text::set_language) |
//! | Measure a text | [`text::measure_text`] |
//!
//! pltrs shapes text with OpenType, including right-to-left and Arabic
//! text. If TeX is not installed, a figure with `usetex` text fails to draw
//! and returns an error.
//!
//! ## Colors
//!
//! pltrs takes colors as names, hex strings, gray levels, RGB and RGBA
//! tuples, and cycle references such as `"C1"`. A colormap maps numbers to
//! colors, and a norm maps data to the range of the colormap.
//!
//! | Task | Item |
//! |------|------|
//! | Give a color | [`Color`](colors::Color), [`colors::to_rgba`], [`colors::named_colors`] |
//! | Use a colormap by name | [`colors::colormaps::get`], [`Colorize::cmap`](colors::Colorize::cmap) |
//! | Make a colormap | [`Colormap`](colors::Colormap), [`colors::from_levels_and_colors`] |
//! | Map data to colors | [`Normalize`](colors::Normalize), [`LogNorm`](colors::LogNorm), [`SymLogNorm`](colors::SymLogNorm), [`PowerNorm`](colors::PowerNorm), [`TwoSlopeNorm`](colors::TwoSlopeNorm), [`BoundaryNorm`](colors::BoundaryNorm), [`CenteredNorm`](colors::CenteredNorm) |
//! | Use a color sequence | [`colors::color_sequence`] |
//! | Map two or three values to one color | [`colors::multivar`] |
//! | Shade a surface with light | [`colors::LightSource`] |
//! | Add a colorbar | [`Figure::colorbar`], [`colorbar::Colorbar`] |
//!
//! ## Legends, insets, patches and path effects
//!
//! | Task | Item |
//! |------|------|
//! | Add a legend | [`Axes::legend`] with [`LegendOptions`](legend::LegendOptions), [`Figure::legend`] |
//! | Drag a legend with the mouse | [`Legend::set_draggable`](legend::Legend::set_draggable) |
//! | Add a table | [`Axes::table`], [`table::Table`] |
//! | Add an inset axes | [`Axes::inset_axes`], [`Axes::indicate_inset_zoom`] |
//! | Draw shapes | [`patches`]: rectangles, circles, ellipses, polygons, wedges, arrows and fancy boxes |
//! | Connect two points with an arrow | [`patches::FancyArrowPatch`], [`patches::ConnectionPatch`] |
//! | Draw a path of lines and curves | [`path::Path`], [`Axes::add_patch`] |
//! | Add strokes and shadows to lines and text | [`patheffects`], [`ArtistHandle::set_path_effects`](artist::ArtistHandle::set_path_effects) |
//! | Pack artists into anchored boxes | [`offsetbox`] |
//! | Filter the pixels of an artist | [`ArtistHandle::set_agg_filter`](artist::ArtistHandle::set_agg_filter) |
//! | Draw in a hand-drawn style | [`ArtistHandle::set_sketch_params`](artist::ArtistHandle::set_sketch_params), [`pyplot::xkcd`] |
//!
//! The [`ArtistHandle`](artist::ArtistHandle) trait gives every artist the
//! common methods: visibility, alpha, z-order, clipping, label, transform
//! and picking.
//!
//! ## Looks and styling
//!
//! pltrs has two looks. The modern look is the default. It uses the Inter
//! font, a light grid, no top or right spine, left-aligned titles and
//! constrained layout. The matplotlib look draws figures as matplotlib
//! does. In the matplotlib look, the PNG output has the same pixels as the
//! matplotlib output.
//!
//! | Task | Item |
//! |------|------|
//! | Select a look in code | [`look::set_appearance`], [`look::set_figure_look`], [`look::set_chrome_look`] |
//! | Apply a style sheet | [`style::use_style`], [`style::context`] |
//! | Change one setting | [`rc::set`], [`rc::update`], [`rc::context`], [`RcParams`](rc::RcParams) |
//! | Restore the matplotlib defaults | [`rc::reset`] |
//! | Find the configuration files | [`config::config_dir`], [`config::rc_file_path`] |
//!
//! These environment variables change the defaults:
//!
//! | Variable | Values | Effect |
//! |----------|--------|--------|
//! | `PLTRS_LOOK` | `modern`, `matplotlib` | selects the look of figures and windows |
//! | `PLTRS_FIGURE_LOOK` | `modern`, `modern-dark`, `matplotlib` | selects the look of figures |
//! | `PLTRS_CHROME_LOOK` | `modern`, `matplotlib` | selects the look of windows |
//! | `PLTRSCONFIGDIR` | a directory | replaces the configuration directory of [`config::config_dir`] |
//! | `PLTRSRC` | a file or a directory | gives the `pltrsrc` file of settings |
//! | `MPLBACKEND` | `agg`, `webagg`, or a GUI backend name such as `qtagg` | selects how [`pyplot::show`] shows figures |
//! | `PLTRS_PLATFORM` | `wayland`, `x11`, `macos`, `windows`, `headless` | selects the window system |
//!
//! The `pltrsrc` file has the syntax of `matplotlibrc`. The `stylelib`
//! directory in the configuration directory holds your style sheets.
//!
//! ## Output
//!
//! [`Figure::savefig`] selects the format from the file extension.
//! [`SaveOptions::format`](figure::SaveOptions::format) overrides it.
//!
//! | Format | Extensions | In memory |
//! |--------|------------|-----------|
//! | PNG | `png` | [`Figure::to_png`] |
//! | JPEG | `jpg`, `jpeg` | [`Figure::to_format`] |
//! | TIFF | `tif`, `tiff` | [`Figure::to_format`] |
//! | GIF | `gif` | [`Figure::to_format`] |
//! | WebP, lossless | `webp` | [`Figure::to_format`] |
//! | AVIF | `avif` | [`Figure::to_format`] |
//! | raw RGBA bytes | `raw`, `rgba` | [`Figure::to_format`] |
//! | SVG and compressed SVG | `svg`, `svgz` | [`Figure::to_svg`] |
//! | PDF | `pdf` | [`Figure::to_pdf`] |
//! | PostScript and EPS | `ps`, `eps` | [`Figure::to_ps`], [`Figure::to_eps`] |
//! | PGF for LaTeX | `pgf` | [`Figure::to_pgf`] |
//!
//! [`SaveFormat`](figure::SaveFormat) names each format.
//! [`Figure::savefig_with`] and the `_with` functions take
//! [`SaveOptions`](figure::SaveOptions):
//!
//! * `dpi`: the resolution of raster output, and of images in vector
//!   output.
//! * `bbox_inches` and `pad_inches`: a tight bounding box around the
//!   drawn artists.
//! * `transparent`, `facecolor` and `edgecolor`: the background.
//! * `metadata`: the document information of vector formats and the text
//!   entries of PNG files.
//! * `jpeg_quality`, `avif_quality`, `orientation` and `papertype`.
//!
//! [`PdfPages`](figure::PdfPages) writes several figures into one PDF file.
//! [`Figure::canvas`] gives the canvas, which keeps the pixels of the last
//! draw.
//!
//! [`image::imread`] reads PNG, JPEG, TIFF, GIF and WebP files.
//! [`image::imsave`] writes an array as an image file.
//!
//! ## Windows and interaction
//!
//! [`gui::show`] opens one window for each figure and returns when the user
//! closes them. The toolbar has home, back, forward, pan, zoom, subplot
//! configuration, figure options and save. The window shows the data
//! coordinates under the pointer.
//!
//! The keys follow the `keymap.*` rcParams, for example `h` for home, `p`
//! for pan, `o` for zoom and `s` for save. The save dialog offers every
//! output format.
//!
//! | Task | Item |
//! |------|------|
//! | Show figures in windows | [`gui::show`], [`pyplot::show`] |
//! | Show figures in a web browser | [`web::serve`] |
//! | Pan and zoom from code | [`navigation`] |
//! | React to the mouse, keys and draws | [`Figure::connect`], [`events::Event`] |
//! | Select artists with the mouse | [`ArtistHandle::set_picker`](artist::ArtistHandle::set_picker), [`events::PickEvent`] |
//! | Run code at intervals | [`Figure::add_timer`] |
//! | Add a toolbar tool | [`Figure::add_tool`] |
//! | Add buttons, sliders and selectors | [`widgets`] |
//! | Draw only the changed artists | [`FigureCanvas::copy_from_bbox`](figure::FigureCanvas::copy_from_bbox), [`FigureCanvas::blit`](figure::FigureCanvas::blit) |
//!
//! The [`widgets`] module has buttons, sliders, range sliders, check
//! buttons, radio buttons, text boxes, cursors, and span, rectangle,
//! ellipse, lasso and polygon selectors.
//!
//! ## Animation
//!
//! [`FuncAnimation`](animation::FuncAnimation) calls a function for each
//! frame. [`FuncAnimation::from_artists`](animation::FuncAnimation::from_artists)
//! shows prepared artists in turn. A window plays the frames.
//!
//! | Output | Item |
//! |--------|------|
//! | a file in the format of its extension | [`FuncAnimation::save`](animation::FuncAnimation::save) |
//! | animated PNG, GIF or WebP in memory | [`FuncAnimation::to_apng`](animation::FuncAnimation::to_apng), [`FuncAnimation::to_gif`](animation::FuncAnimation::to_gif), [`FuncAnimation::to_webp`](animation::FuncAnimation::to_webp) |
//! | an HTML page with a player | [`FuncAnimation::to_jshtml`](animation::FuncAnimation::to_jshtml) |
//! | a video through `ffmpeg` | [`FuncAnimation::save_movie`](animation::FuncAnimation::save_movie), [`FuncAnimation::to_html5_video`](animation::FuncAnimation::to_html5_video) |
//!
//! ## The pyplot interface
//!
//! The [`pyplot`] module has the functions of `matplotlib.pyplot`. They act
//! on a current figure and a current axes for each thread. Use it for short
//! scripts. Use [`Figure`] and [`Axes`] for programs that make more than one
//! figure.
//!
//! ## Extending
//!
//! | Extension | Item |
//! |-----------|------|
//! | Your own artist | the [`Artist`](artist::Artist) trait, [`Axes::add_custom_artist`] |
//! | Your own projection | the [`Projection`](projections::Projection) trait, [`projections::register`] |
//! | Your own scale | the [`Scale`](scale::Scale) trait, [`Axes::set_scale`] |
//! | Your own colormap | [`colors::colormaps::register`] |
//! | Your own color sequence | [`colors::register_color_sequence`] |
//! | Plot values of your own types | the [`ConversionInterface`](units::ConversionInterface) and [`Convertible`](units::Convertible) traits |
//! | Your own legend entry | the [`LegendHandler`](legend::LegendHandler) trait |
//! | Your own toolbar tool | [`Figure::add_tool`] |
//!
//! ## Testing your figures
//!
//! The [`testing`] module tests the figures of your program as
//! `matplotlib.testing` does:
//!
//! * [`testing::setup`] selects the settings that image tests use.
//! * [`testing::compare_images`] compares two image files by their RMS
//!   difference. If they differ, it writes a difference image.
//! * [`testing::check_figures_equal`] compares two figures that your code
//!   draws.
//!
//! # Errors
//!
//! A function that can fail returns a `Result`. Each module has its own
//! error enum, for example [`PlotError`](axes::PlotError),
//! [`SaveError`](figure::SaveError) and [`LegendError`](legend::LegendError).
//! All error enums implement [`std::error::Error`], so `?` into
//! `Box<dyn Error>` works.
//!
//! pltrs does not panic on user input. Invalid data, an unknown name or too
//! large a request returns an error. Indexing panics on a missing entry, as
//! a slice does. [`porting::conventions`] describes these rules.
//!
//! # Platforms
//!
//! * Linux: windows on Wayland and X11.
//! * macOS: windows through `AppKit`.
//! * Windows: windows through Win32.
//!
//! pltrs needs Rust 1.99 or later. If no window system is available,
//! [`gui::show`] does not show the figures on Linux. On macOS and Windows,
//! it shows them in a web browser. Files and in-memory output work on all
//! systems without a display.
//!
//! # Where to go next
//!
//! * The [`tutorial`] teaches pltrs step by step, for readers who do not
//!   know matplotlib.
//! * The [`porting`] guide gives the pltrs call for each matplotlib call.
//! * The `examples/` directory of the repository has 20 graphs, each in Rust
//!   and in Python.
//! * `cargo run -p pltrs --example gallery` shows many more figures.
//! * pltrs (Python) is the package `pltrs` on the Python Package Index. It has the matplotlib
//!   3.11 API, and the `matplotlib` alias package lets pandas and seaborn
//!   draw with pltrs.
//!
//! # Modules
//!
//! The modules follow the matplotlib modules.
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`figure`] | figures, subfigures, mosaics, saving and the canvas |
//! | [`axes`] | axes, their plotting methods, axis, ticks and spines |
//! | [`gridspec`] | grids of subplots |
//! | [`layout`] | constrained and tight layout, and aligned labels |
//! | [`prelude`] | the common names |
//! | [`artist`] | the artist model, the handle trait and your own artists |
//! | [`lines`] | lines, line styles, draw styles and marker subsampling |
//! | [`markers`] | marker shapes |
//! | [`collections`] | many similar shapes drawn in one batch |
//! | [`patches`] | rectangles, circles, polygons, arrows and fancy boxes |
//! | [`path`] | polylines and Bézier curves |
//! | [`patheffects`] | strokes, shadows and other effects around paths |
//! | [`text`] | text, annotations, mathtext and `usetex` |
//! | [`fonts`] | font properties and the font manager |
//! | [`coords`] | coordinate systems for annotations |
//! | [`spines`] | the lines at the edges of the data area |
//! | [`orientation`] | the direction of bars, histograms, colorbars and sliders |
//! | [`transforms`] | coordinate systems and the transforms between them |
//! | [`numeric`] | numpy-style ranges and the 2-D array [`Grid`](numeric::Grid) |
//! | [`colors`] | color specifications, colormaps and norms |
//! | [`colorbar`] | colorbars for images, collections and contour sets |
//! | [`ticker`] | tick locators and formatters |
//! | [`scale`] | axis scales |
//! | [`dates`] | dates and times, and their locators and formatters |
//! | [`units`] | the conversion of categories, dates and your own types |
//! | [`stats`] | statistics for box plots, violin plots and histograms |
//! | [`mlab`] | spectral analysis |
//! | [`contour`] | contour lines and filled contours |
//! | [`tri`] | triangular grids |
//! | [`quiver`] | arrows and wind barbs |
//! | [`streamplot`] | streamlines |
//! | [`image`] | images, their resampling, `imread` and `imsave` |
//! | [`polar`] | polar axes |
//! | [`geo`] | geographic axes |
//! | [`projections`] | your own projections |
//! | [`mplot3d`] | 3-D axes |
//! | [`axes_grid1`] | axes placed by dividers, image grids and anchored artists |
//! | [`axisartist`] | axes drawn by axis artists, with curved grids |
//! | [`legend`] | legends and legend handlers |
//! | [`offsetbox`] | box packers, anchored boxes and annotation boxes |
//! | [`container`] | the artists of bar, error bar and stem plots |
//! | [`table`] | tables |
//! | [`inset`] | the indicators of inset axes |
//! | [`sankey`] | Sankey flow diagrams |
//! | [`look`] | the modern look and the matplotlib look |
//! | [`style`] | style sheets and the xkcd style |
//! | [`rc`] | runtime settings (rcParams) |
//! | [`config`] | the configuration directory, `pltrsrc` and `stylelib` |
//! | [`pyplot`] | the stateful interface of `matplotlib.pyplot` |
//! | [`events`] | figure events, callbacks and timers |
//! | [`navigation`] | pan, zoom and view history |
//! | [`widgets`] | buttons, sliders, text boxes, cursors and selectors |
//! | [`animation`] | animations |
//! | [`gui`] | figure windows with a toolbar |
//! | [`web`] | figures in a web browser |
//! | [`testing`] | tests of your figures |
//! | [`warnings`] | the warnings of pltrs |
//! | [`diagnostics`] | counts of the figures and artists that the process keeps |
//! | [`clock`] | the performance counter for slow work |
//! | [`tutorial`] | the tutorial |
//! | [`porting`] | the porting guide from matplotlib |
//!
//! [`Figure`]: figure::Figure
//! [`Axes`]: axes::Axes
//! [`Figure::new`]: figure::Figure::new
//! [`Figure::with_size`]: figure::Figure::with_size
//! [`Figure::with_figsize`]: figure::Figure::with_figsize
//! [`Figure::set_size_inches`]: figure::Figure::set_size_inches
//! [`Figure::set_dpi`]: figure::Figure::set_dpi
//! [`Figure::subplot`]: figure::Figure::subplot
//! [`Figure::add_subplot`]: figure::Figure::add_subplot
//! [`Figure::add_axes`]: figure::Figure::add_axes
//! [`Figure::subplots`]: figure::Figure::subplots
//! [`Figure::subplots_shared`]: figure::Figure::subplots_shared
//! [`Figure::add_gridspec`]: figure::Figure::add_gridspec
//! [`Figure::add_subplot_spec`]: figure::Figure::add_subplot_spec
//! [`Figure::subplot_mosaic`]: figure::Figure::subplot_mosaic
//! [`Figure::subfigures`]: figure::Figure::subfigures
//! [`Figure::add_subfigure`]: figure::Figure::add_subfigure
//! [`Figure::suptitle`]: figure::Figure::suptitle
//! [`Figure::supxlabel`]: figure::Figure::supxlabel
//! [`Figure::supylabel`]: figure::Figure::supylabel
//! [`Figure::text`]: figure::Figure::text
//! [`Figure::legend`]: figure::Figure::legend
//! [`Figure::colorbar`]: figure::Figure::colorbar
//! [`Figure::figimage`]: figure::Figure::figimage
//! [`Figure::set_layout_engine`]: figure::Figure::set_layout_engine
//! [`Figure::tight_layout`]: figure::Figure::tight_layout
//! [`Figure::subplots_adjust`]: figure::Figure::subplots_adjust
//! [`Figure::align_labels`]: figure::Figure::align_labels
//! [`Figure::align_titles`]: figure::Figure::align_titles
//! [`Figure::autofmt_xdate`]: figure::Figure::autofmt_xdate
//! [`Figure::savefig`]: figure::Figure::savefig
//! [`Figure::savefig_with`]: figure::Figure::savefig_with
//! [`Figure::to_png`]: figure::Figure::to_png
//! [`Figure::to_format`]: figure::Figure::to_format
//! [`Figure::to_svg`]: figure::Figure::to_svg
//! [`Figure::to_pdf`]: figure::Figure::to_pdf
//! [`Figure::to_ps`]: figure::Figure::to_ps
//! [`Figure::to_eps`]: figure::Figure::to_eps
//! [`Figure::to_pgf`]: figure::Figure::to_pgf
//! [`Figure::canvas`]: figure::Figure::canvas
//! [`Figure::subplot_3d`]: figure::Figure::subplot_3d
//! [`Figure::add_subplot_3d`]: figure::Figure::add_subplot_3d
//! [`Figure::polar_subplot`]: figure::Figure::polar_subplot
//! [`Figure::add_polar_subplot`]: figure::Figure::add_polar_subplot
//! [`Figure::geo_subplot`]: figure::Figure::geo_subplot
//! [`Figure::subplot_with_projection`]: figure::Figure::subplot_with_projection
//! [`Figure::connect`]: figure::Figure::connect
//! [`Figure::add_timer`]: figure::Figure::add_timer
//! [`Figure::add_tool`]: figure::Figure::add_tool
//! [`Axes::plot_with`]: axes::Axes::plot_with
//! [`Axes::set_title`]: axes::Axes::set_title
//! [`Axes::set_xlabel`]: axes::Axes::set_xlabel
//! [`Axes::set_ylabel`]: axes::Axes::set_ylabel
//! [`Axes::legend`]: axes::Axes::legend
//! [`Axes::set_xlim`]: axes::Axes::set_xlim
//! [`Axes::set_ylim`]: axes::Axes::set_ylim
//! [`Axes::set_margins`]: axes::Axes::set_margins
//! [`Axes::autoscale`]: axes::Axes::autoscale
//! [`Axes::set_xscale`]: axes::Axes::set_xscale
//! [`Axes::set_xticks`]: axes::Axes::set_xticks
//! [`Axes::set_xticklabels`]: axes::Axes::set_xticklabels
//! [`Axes::minorticks_on`]: axes::Axes::minorticks_on
//! [`Axes::tick_params`]: axes::Axes::tick_params
//! [`Axes::ticklabel_format`]: axes::Axes::ticklabel_format
//! [`Axes::locator_params`]: axes::Axes::locator_params
//! [`Axes::xaxis_date`]: axes::Axes::xaxis_date
//! [`Axes::share_x`]: axes::Axes::share_x
//! [`Axes::share_y`]: axes::Axes::share_y
//! [`Axes::twinx`]: axes::Axes::twinx
//! [`Axes::twiny`]: axes::Axes::twiny
//! [`Axes::secondary_xaxis`]: axes::Axes::secondary_xaxis
//! [`Axes::secondary_yaxis`]: axes::Axes::secondary_yaxis
//! [`Axes::set_aspect`]: axes::Axes::set_aspect
//! [`Axes::set_box_aspect`]: axes::Axes::set_box_aspect
//! [`Axes::spines`]: axes::Axes::spines
//! [`Axes::grid`]: axes::Axes::grid
//! [`Axes::label_outer`]: axes::Axes::label_outer
//! [`Axes::set_axis_off`]: axes::Axes::set_axis_off
//! [`Axes::set_prop_cycle`]: axes::Axes::set_prop_cycle
//! [`Axes::text`]: axes::Axes::text
//! [`Axes::annotate`]: axes::Axes::annotate
//! [`Axes::table`]: axes::Axes::table
//! [`Axes::inset_axes`]: axes::Axes::inset_axes
//! [`Axes::indicate_inset_zoom`]: axes::Axes::indicate_inset_zoom
//! [`Axes::add_patch`]: axes::Axes::add_patch
//! [`Axes::add_custom_artist`]: axes::Axes::add_custom_artist
//! [`Axes::set_scale`]: axes::Axes::set_scale
#![forbid(unsafe_code)]
#![expect(
    clippy::needless_pass_by_value,
    reason = "option structs are passed by value by convention (see `porting::conventions`), \
              whether the function keeps them or only reads them"
)]

pub mod animation;
#[macro_use]
pub mod artist;
pub mod axes;
pub mod axes_grid1;
pub mod axisartist;
pub mod clock;
pub mod collections;
pub mod colorbar;
mod colorizer;
pub mod colors;
pub mod config;
pub mod container;
pub mod contour;
pub mod coords;
pub mod dates;
pub mod diagnostics;
pub mod events;
pub mod figure;
pub mod fonts;
pub mod geo;
pub mod gridspec;
pub mod gui;
pub mod image;
pub mod inset;
pub mod layout;
pub mod legend;
pub mod lines;
pub mod look;
pub mod markers;
pub mod mlab;
pub mod mplot3d;
pub mod navigation;
pub mod numeric;
pub mod offsetbox;
pub mod orientation;
pub mod patches;
pub mod path;
pub mod patheffects;
pub mod polar;
pub mod porting;
pub mod prelude;
pub mod projections;
pub mod pyplot;
pub mod quiver;
pub mod rc;
pub mod sankey;
pub mod scale;
pub mod spines;
pub mod stats;
pub mod streamplot;
pub mod style;
pub mod table;
pub mod testing;
pub mod text;
pub mod ticker;
pub mod transforms;
pub mod tri;
pub mod tutorial;
pub mod units;
pub mod warnings;
pub mod web;
pub mod widgets;

/// The harness of the fuzz tests (`pltrs_core::fuzz`), also driving the unit
/// tests that fuzz what the crate keeps private: the web server's
/// parsers, the configuration lookup and the dialogs of figure windows.
#[cfg(test)]
use pltrs_core::fuzz;
/// `pltrs-font`'s test fixtures, for the unit tests that read them.
#[cfg(test)]
#[path = "../tests/support/font_fixtures.rs"]
mod font_fixtures;
mod process;
/// Scratch directories for the unit tests that write files.
#[cfg(test)]
#[path = "../tests/support/scratch.rs"]
mod scratch;
