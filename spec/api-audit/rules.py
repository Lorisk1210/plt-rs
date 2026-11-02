"""Manual classifications, applied before the automatic matching.

RULES: (regex on "module:Owner.name", (status, rust item, note)).
OWNER_RULES: (regex on "module:Owner", ...) for every member of a class.
KWARG_RULES: (regex on "module:Owner.method(param)" or on the parameter
name, ...).
"""


import glob
import importlib
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rules_base import *  # noqa: F401,F403

RULES = [
    # matplotlib top level
    (r"matplotlib:ExecutableNotFoundError", na("pltrs runs no external executables for its outputs (LaTeX, Ghostscript, Inkscape)")),
    (r"matplotlib:get_data_path", cov("pltrs::config::data_dir", "the bundled fonts, style sheets and rc file, written out the first time they are asked for")),
    (r"matplotlib:interactive", cov("pyplot::ion / pyplot::ioff")),
    (r"matplotlib:is_interactive", cov("pyplot::isinteractive")),
    (r"matplotlib:rcParams", cov("rc::params / rc::set / pyplot::rc", "typed RcParams; string keys through rc::set")),
    (r"matplotlib:rcParamsDefault", cov("RcParams::matplotlib")),
    (r"matplotlib:rcParamsOrig", cov("RcParams::startup")),
    (r"matplotlib:rc_file_defaults", cov("rc::restore_startup")),
    (r"matplotlib:rc_params", cov("RcParams::startup", "the parameters read from the default rc file")),
    (r"matplotlib:rc_params_from_file", cov("RcFile::parse, RcParams::apply_rc_text")),
    (r"matplotlib:set_loglevel", na("pltrs does not log; warnings are returned as values (RcWarning, ContourWarning)")),
    (r"matplotlib:use", cov("pyplot::switch_backend (Backend)")),
    (r"matplotlib:RcParams\.validate", na("validator table of the rcParams dict; typed fields validate on parse")),
    (r"matplotlib:RcParams\..*", diff("RcParams, rc::set, rc::params", "a typed struct instead of a validating dict")),
    # pyplot
    (r"matplotlib\.pyplot:(getp|setp)", diff("typed getters, update_props", "conventions: introspection is replaced by typed getters and setters")),
    (r"matplotlib\.(artist):(getp|setp|kwdoc)", diff("typed getters, update_props", "conventions: introspection is replaced by typed getters and setters")),
    (r"matplotlib\.artist:allow_rasterization", na("decorator for draw methods; DrawContext handles rasterization")),
    (r"matplotlib\.pyplot:(install|uninstall)_repl_displayhook", na("Python REPL integration")),
    (r"matplotlib\.pyplot:new_figure_manager", BACKEND_PLUMBING),
    (r"matplotlib\.pyplot:set_loglevel", na("pltrs does not log")),
    (r"matplotlib\.axes:subplot_class_factory", PY_MACHINERY),
    # Axes
    (r"matplotlib\.axes:_AxesBase\.set_subplotspec", miss("moving an axes to another grid cell")),
    (r"matplotlib\.axes:_AxesBase\.get_[xy]axis_transform", diff("Transform::blended, Projection::axis_transform", "build the blended data/axes transform; projections provide axis_transform")),
    (r"matplotlib\.axes:_AxesBase\.get_[xy]axis_text[12]_transform", diff("Projection::tick_label", "tick label placement hook of the Projection trait")),
    (r"matplotlib\.axes:_AxesBase\.has_data", miss("whether anything was plotted that has data limits")),
    (r"matplotlib\.axes:_AxesBase\.add_child_axes", diff("Axes::inset_axes, Axes::inset_axes_in", "child axes are made by inset_axes")),
    (r"matplotlib\.axes:_AxesBase\.add_table", diff("Axes::table, Axes::attach", "tables are made by Axes::table; attach re-adds a removed one")),
    (r"matplotlib\.axes:_AxesBase\.in_axes", cov("ArtistHandle::contains on the Axes, Axes::contains_point")),
    (r"matplotlib\.axes:_AxesBase\.get_autoscaley_on", cov("Axes::autoscale_on(Some(AxisName::Y))")),
    (r"matplotlib\.axes:_AxesBase\.get_[xy](major|minor)ticklabels", cov("Axis::ticklabels(minor) on xaxis()/yaxis()")),
    (r"matplotlib\.axes:_AxesBase\.get_navigate_mode", cov("Navigation::mode (NavMode)")),
    (r"matplotlib\.axes:_AxesBase\.(start_pan|end_pan|drag_pan)", diff("Navigation::press/drag/release, Projection::drag_pan", "pan hooks of the Projection trait, driven by Navigation")),
    (r"matplotlib\.axes:_AxesBase\.get_children", diff("Artist::children, ArtistHandle::findobj", "the artist tree")),
    (r"matplotlib\.axes:_AxesBase\.get_default_bbox_extra_artists", diff("Figure::tight_bbox", "used by bbox_inches='tight', which pltrs computes itself")),
    (r"matplotlib\.axes:_AxesBase\.get_shared_y_axes", cov("Axes::shared_axes(AxisName::Y)")),
    (r"matplotlib\.axes:_AxesBase\.(get_fc|set_fc)", ALIAS),
    (r"matplotlib\.axes:Axes\.axison", cov("Axes::axis_on")),
    (r"matplotlib\.axes:Axes\.callbacks", miss("limit-change callbacks: `ax.callbacks.connect('xlim_changed', f)`")),
    (r"matplotlib\.axes:Axes\.dataLim", cov("Axes::data_lim")),
    (r"matplotlib\.axes:Axes\.ignore_existing_data_limits", na("internal autoscale flag")),
    (r"matplotlib\.axes:Axes\.titleOffsetTrans", na("internal offset of the title text")),
    (r"matplotlib\.axes:Axes\.transData", cov("Transform::Data")),
    (r"matplotlib\.axes:Axes\.transAxes", cov("Transform::Axes")),
    (r"matplotlib\.axes:Axes\.(transLimits|transScale)", diff("ProjectionView::rectilinear", "the parts of transData; pltrs resolves transData in one step")),
    # axis
    (r"matplotlib\.axis:Axis\.OFFSETTEXTPAD", PY_CONST),
    (r"matplotlib\.axis:Axis\.(majorTicks|minorTicks)", cov("Axis::ticklocs / Axis::ticklines / Axis::ticklabels", "the tick list behind get_major_ticks")),
    (r"matplotlib\.axis:.*\.axis_name", cov("Axis::name")),
    (r"matplotlib\.axis:XAxis\.callbacks", miss("limit-change callbacks of the axis (`units`/`*lim_changed`)")),
    (r"matplotlib\.axis:XAxis\.isDefault_label", na("internal flag")),
    (r"matplotlib\.axis:XAxis\.(major|minor)", cov("Axis::major_locator/major_formatter (and minor)", "the Ticker holding locator and formatter")),
    (r"matplotlib\.axis:XAxis\.offset_text_position", miss("which side the offset text sits on (`set_offset_position`)")),
    (r".*\.stale_callback", CHANGE_TRACKING),
    (r".*\.stale", CHANGE_TRACKING),
    (r".*\.clipbox", cov("ArtistHandle::clip_box")),
    (r".*\.(input_dims|output_dims|is_separable|has_inverse|is_affine|pass_through)", TRANSFORM_META),
    # lines
    (r"matplotlib\.lines:Line2D\.(lineStyles|drawStyles|drawStyleKeys|markers|filled_markers|fillStyles)", diff("LineStyle, DrawStyle::all, MarkerSymbol::all, FillStyle", "enums list the accepted names (FromStr)")),
    (r"matplotlib\.lines:Line2D\.ind_offset", na("internal picking offset")),
    (r"matplotlib\.lines:segment_hits", diff("ArtistHandle::contains, Line2D::pickradius", "the line hit test behind picking")),
    (r"matplotlib\.markers:MarkerStyle\.(filled_markers|fillstyles)", diff("MarkerSymbol::all, FillStyle", "enums list the accepted names")),
    # patches
    (r"matplotlib\.patches:ArrowStyle\.(Curve|CurveA|CurveB|CurveAB|CurveFilledA|CurveFilledB|CurveFilledAB|BracketA|BracketB|BracketAB|BarAB|BracketCurve|CurveBracket)", cov("ArrowStyle::Curve(CurveArrow)", "one curve style with configurable ends; parse matplotlib's names and '->' forms")),
    (r"matplotlib\.patches:ArrowStyle\.(Simple|Fancy|Wedge)", cov("ArrowStyle (Simple, Fancy, Wedge variants)")),
    (r"matplotlib\.patches:BoxStyle\.(Square|Circle|Ellipse|RArrow|LArrow|DArrow|Round|Round4|Sawtooth|Roundtooth)", cov("BoxShape variants (BoxStyle)")),
    (r"matplotlib\.patches:ConnectionStyle\.(Arc3|Angle3|Angle|Arc|Bar)", cov("ConnectionStyle variants")),
    (r"matplotlib\.patches:(bbox_artist|draw_bbox)", na("debugging helpers of matplotlib's layout code")),
    # image
    (r"matplotlib\.image:composite_images", diff("SvgOptions/PdfOptions/PsOptions::composite_images", "backends composite images themselves")),
    (r"matplotlib\.image:pil_to_array", na("PIL interoperability")),
    (r"matplotlib\.image:AxesImage\.colorbar_cid", na("internal connection id")),
    # colors
    (r"matplotlib\.colors:colorConverter", diff("Color::parse, to_rgba", "the converter object is replaced by functions")),
    (r"matplotlib\.colors:(hex2color|to_rgb)", diff("to_rgba / Color::resolve", "take the RGB channels of the RGBA result")),
    (r"matplotlib\.colors:rgb2hex", cov("to_hex / Rgba::to_hex")),
    (r"matplotlib\.colors:get_named_colors_mapping", cov("named_color_table")),
    (r"matplotlib\.colors:make_norm_from_scale", diff("ScaleNorm", "a norm in any scale's space")),
    # colorbar
    (r"matplotlib\.colorbar:Colorbar\.n_rasterize", PY_CONST),
    (r"matplotlib\.colorbar:Colorbar\.(solids|solids_patches)", miss("the colorbar's colour mesh (or patches) as an artist")),
    (r"matplotlib\.colorbar:make_axes", diff("ColorbarAxes::Parents (ColorbarOptions::axes)", "colorbar placement options")),
    (r"matplotlib\.colorbar:make_axes_gridspec", diff("ColorbarAxes::Parent with use_gridspec", "colorbar placement options")),
    # ticker / dates / scale
    (r"matplotlib\.ticker:EngFormatter\.ENG_PREFIXES", PY_CONST),
    (r"matplotlib\.ticker:Locator\.MAXTICKS", cov("ticker::MAXTICKS")),
    (r"matplotlib\.ticker:MaxNLocator\.default_params", cov("MaxNLocator::new, MaxNLocator::DEFAULT_STEPS")),
    (r"matplotlib\.dates:DateLocator\.hms0d", PY_CONST),
    (r"matplotlib\.scale:AsinhScale\.auto_tick_multipliers", PY_CONST),
    (r"matplotlib\.scale:register_scale", cov("ScaleRegistry::register")),
    # transforms / path
    (r"matplotlib\.transforms:blended_transform_factory", cov("Transform::blended")),
    (r"matplotlib\.transforms:composite_transform_factory", cov("Transform::then")),
    (r"matplotlib\.transforms:offset_copy", cov("Transform::offset_copy")),
    (r"matplotlib\.transforms:BboxBase\.coefs", cov("Anchor::coefficients")),
    (r"matplotlib\.path:Path\.(MOVETO|LINETO|CURVE3|CURVE4|CLOSEPOLY)", cov("PathCode variants")),
    (r"matplotlib\.path:Path\.STOP", cov("PathCode (paths end with their vertices)")),
    (r"matplotlib\.path:Path\.NUM_VERTICES_FOR_CODE", cov("PathCode::vertex_count")),
    (r"matplotlib\.path:get_path_collection_extents", cov("CollectionGeometry::extents")),
    # legend / offsetbox
    (r"matplotlib\.legend:Legend\.codes", cov("LegendLoc::code / LegendLoc::from_code")),
    (r"matplotlib\.legend:Legend\.isaxes", na("internal flag")),
    (r"matplotlib\.legend:Legend\.legendPatch", cov("Legend::frame")),
    (r"matplotlib\.legend:Legend\.parent", cov("ArtistHandle::axes / ArtistHandle::parent_figure")),
    (r"matplotlib\.legend_handler:update_from_first_child", na("copies Python artist properties between handles")),
    (r"matplotlib\.offsetbox:AnchoredOffsetbox\.codes", cov("AnchorLoc")),
    # animation
    (r"matplotlib\.animation:(FFMpegWriter|FFMpegBase|MovieWriter|AbstractMovieWriter)", diff("FuncAnimation::save_movie (MovieOptions)", "frames piped to ffmpeg; FuncAnimation::save picks the writer from the extension")),
    (r"matplotlib\.animation:(FFMpegFileWriter|FileMovieWriter)", diff("FuncAnimation::save_movie", "pltrs pipes frames instead of writing temporary files")),
    (r"matplotlib\.animation:(ImageMagickBase|ImageMagickFileWriter|ImageMagickWriter)", diff("FuncAnimation::to_gif / save", "GIF written natively; no ImageMagick needed")),
    (r"matplotlib\.animation:PillowWriter", cov("FuncAnimation::to_gif / to_apng / to_webp / save")),
    (r"matplotlib\.animation:HTMLWriter", cov("FuncAnimation::to_jshtml / save (.html)")),
    (r"matplotlib\.animation:(MovieWriterRegistry|writers)", diff("FuncAnimation::save", "the writer follows the file extension; no registry")),
    (r"matplotlib\.animation:adjusted_figsize", na("rounds figure sizes for ffmpeg's even-pixel codecs; save_movie handles frame sizes")),
    (r"matplotlib\.animation:.*\.(supported_formats|input_names)", diff("FuncAnimation::save", "formats chosen by extension")),
    # backend_bases
    (r"matplotlib\.backend_bases:FigureCanvasBase\.(required_interactive_framework|manager_class|fixed_dpi)", BACKEND_PLUMBING),
    (r"matplotlib\.backend_bases:FigureCanvasBase\.events", cov("EventKind")),
    (r"matplotlib\.backend_bases:FigureCanvasBase\.filetypes", cov("SaveFormat")),
    (r"matplotlib\.backend_bases:FigureCanvasBase\.supports_blit", diff("FigureCanvas::blit", "every canvas blits")),
    (r"matplotlib\.backend_bases:FigureCanvasBase\.manager", cov("pyplot::get_current_fig_manager, FigureManager")),
    (r"matplotlib\.backend_bases:MouseButton\.(LEFT|MIDDLE|RIGHT)", cov("PointerButton (Left, Middle, Right)")),
    (r"matplotlib\.backend_bases:MouseButton\.(BACK|FORWARD)", miss("the back and forward mouse buttons (X11 buttons 8 and 9)")),
    (r"matplotlib\.backend_bases:NavigationToolbar2\.toolitems", diff("pltrs::gui toolbar", "the window's built-in toolbar")),
    (r"matplotlib\.backend_bases:NavigationToolbar2\.UNKNOWN_SAVED_STATUS", PY_CONST),
    (r"matplotlib\.backend_bases:(button_press_handler|key_press_handler|scroll_handler)", diff("pltrs::gui (keymap.* rcParams), Navigation", "the windows' default bindings")),
    (r"matplotlib\.backend_bases:(get_registered_canvas_class|register_backend)", BACKEND_PLUMBING),
    (r"matplotlib\.backend_bases:_Backend\..*", BACKEND_PLUMBING),
    # backend_tools
    (r"matplotlib\.backend_tools:(ToolBase|ToolToggleBase)", cov("ToolSpec with ToolKind::Trigger / ToolKind::Toggle, Figure::add_tool")),
    (r"matplotlib\.backend_tools:Cursors", cov("CursorShape")),
    (r"matplotlib\.backend_tools:Cursors\..*", cov("CursorShape variants")),
    (r"matplotlib\.backend_tools:(add_tools_to_container|add_tools_to_manager)", diff("Figure::add_tool", "tools are added one by one")),
    (r"matplotlib\.backend_tools:(ToolBase|ToolToggleBase)\.(description|default_keymap)", cov("ToolSpec::description / ToolSpec::keys")),
    (r"matplotlib\.backend_tools:(ToolBase|ToolToggleBase)\.(radio_group|cursor|default_toggled)", miss("tool radio groups, cursors and initial toggle state for user tools")),
    (r"matplotlib\.backend_tools:.*", BUILTIN_TOOLS),
    # font_manager
    (r"matplotlib\.font_manager:FontEntry\.fname", cov("FontListEntry::file")),
    (r"matplotlib\.font_manager:(afmFontProperty|ttfFontProperty)", cov("FontManager::add_font (FontListEntry)")),
    (r"matplotlib\.font_manager:findfont", cov("FontManager::find_font")),
    (r"matplotlib\.font_manager:fontManager", cov("FontManager::global")),
    (r"matplotlib\.font_manager:get_font_names", cov("FontManager::font_names")),
    (r"matplotlib\.font_manager:get_font", diff("pltrs_font (FontManager::find_font)", "FT2Font objects are internal; pltrs parses fonts itself")),
    (r"matplotlib\.font_manager:(get_fontext_synonyms|win32FontDirectory)", na("file-extension and Windows directory helpers of matplotlib's font search")),
    (r"matplotlib\.font_manager:(json_dump|json_load)", na("matplotlib's font cache file")),
    (r"matplotlib\.font_manager:(findSystemFonts|list_fonts)", miss("listing the font files installed on the system")),
    # mathtext
    (r"matplotlib\.mathtext:RasterParse\.(ox|oy|image)", cov("MathLayout::raster_extents, MathImage")),
    (r"matplotlib\.mathtext:VectorParse\.(glyphs|rects)", cov("MathLayout::glyphs / MathLayout::rects")),
    (r"matplotlib\.mathtext:get_unicode_index", cov("pltrs_text::mathtext (symbol table)", "TeX symbol names resolve inside the mathtext parser")),
    # quiver / streamplot / table
    (r"matplotlib\.quiver:QuiverKey\.(halign|valign)", PY_CONST),
    (r"matplotlib\.streamplot:interpgrid", na("streamplot integration internals")),
    (r"matplotlib\.table:Cell\.PAD", cov("TableCell padding (plt.table rcParams)")),
    (r"matplotlib\.table:Table\.(FONTSIZE|AXESPAD)", PY_CONST),
    (r"matplotlib\.table:Table\.codes", cov("TableLoc")),
    # units / style / rcsetup
    (r"matplotlib\.units:registry", diff("Convertible / Axis::set_converter", "converters are picked by type or set on the axis")),
    (r"matplotlib\.style:use", cov("style::use_style")),
    (r"matplotlib\.rcsetup:cycler", cov("Cycler::new")),
    (r"matplotlib\.rcsetup:validate_.*", diff("RcParams, rc::set", "rc values parse into typed fields and enums; rc::set validates by key")),
    # cbook
    (r"matplotlib\.cbook:(file_requires_unicode|is_writable_file_like|open_file_cm|to_filehandle|is_scalar_or_string|sanitize_sequence|safe_first_element|normalize_kwargs|print_cycles|flatten)", PY_MACHINERY),
    (r"matplotlib\.cbook:get_sample_data", na("sample data files are not shipped")),
    (r"matplotlib\.cbook:(delete_masked_points|safe_masked_invalid)", diff("ScalarField masks, AxisValues", "masked values are explicit (Option / masks)")),
    (r"matplotlib\.cbook:index_of", cov("Axes::plot_y", "plotting y against its indices")),
    (r"matplotlib\.cbook:simple_linear_interpolation", cov("Path::interpolated")),
    # projections
    (r"matplotlib\.projections:(Aitoff|Hammer|Lambert|Mollweide)Axes\.(Inverted)?(Aitoff|Hammer|Lambert|Mollweide)Transform", cov("GeoProjection::forward / GeoProjection::inverse")),
    (r"matplotlib\.projections:PolarAxes\.(PolarTransform|PolarAffine|InvertedPolarTransform)", diff("PolarAxes (polar transform), AuxTransform::polar", "the polar mapping is resolved by the polar axes")),
    (r"matplotlib\.projections:PolarAxes\.ThetaFormatter", cov("ThetaFormatter")),
    (r"matplotlib\.projections:PolarAxes\.(RadialLocator|ThetaLocator)", diff("PolarAxes::set_rticks / set_thetagrids, Locator", "polar axes wrap the axis locators themselves")),
    (r"matplotlib\.projections:get_projection_class", diff("projection_names, ProjectionSpec", "projections are looked up by name when an axes is made")),
    (r"matplotlib\.projections:projection_registry", cov("projection_names, register")),
    (r"matplotlib\.projections:register_projection", cov("projections::register / register_with")),
    (r"matplotlib\.projections\.geo:GeoAxes\.ThetaFormatter", cov("GeoThetaFormatter")),
    (r"matplotlib\.projections\.geo:GeoAxes\.RESOLUTION", PY_CONST),
    # hatch / bezier / textpath / mlab
    (r"matplotlib\.hatch:(Circles|HatchPatternBase|HorizontalHatch|LargeCircles|NorthEastHatch|Shapes|SmallCircles|SmallFilledCircles|SouthEastHatch|Stars|VerticalHatch)", diff("pltrs_core::hatch::pattern_path", "hatch patterns are built by one function from the hatch string")),
    (r"matplotlib\.hatch:.*\..*", diff("pltrs_core::hatch::pattern_path", "hatch patterns are built by one function from the hatch string")),
    (r"matplotlib\.hatch:get_path", cov("pltrs_core::hatch::pattern_path")),
    (r"matplotlib\.bezier:NonIntersectingPathException", cov("BezierError::NotIntersecting")),
    (r"matplotlib\.bezier:split_de_casteljau", cov("BezierSegment::split")),
    (r"matplotlib\.textpath:TextToPath\.(FONT_SCALE|DPI)", PY_CONST),
    (r"matplotlib\.textpath:text_to_path", cov("text::layout::text_path")),
    (r"matplotlib\.mlab:detrend", cov("Detrend, detrend_mean / detrend_linear / detrend_none")),
    # mplot3d
    (r"mpl_toolkits\.mplot3d\.axes3d:get_test_data", cov("pltrs::mplot3d::test_data::test_data")),
    # toolkits
    (r"mpl_toolkits\.axes_grid1\.axes_size:from_any", cov("SizeSpec (conversions)")),
    (r"mpl_toolkits\..*class_factory", PY_MACHINERY),
    (r"mpl_toolkits\.axisartist\.angle_helper:Formatter(DMS|HMS)\.(deg_mark|min_mark|sec_mark|fmt_.*)", PY_CONST),
    # Figure attributes
    (r"matplotlib\.figure:Figure\.(artists|images|patches|lines)", miss("the lists of artists placed on the figure itself")),
    (r"matplotlib\.figure:Figure\.patch", diff("Figure::facecolor / Figure::edgecolor / Figure::linewidth / Figure::frameon", "the background patch's properties are figure properties")),
    (r"matplotlib\.figure:Figure\.dpi_scale_trans", cov("Transform::Inches")),
    (r"matplotlib\.figure:Figure\.(figbbox|bbox_inches)", cov("Figure::bbox, Figure::size_inches")),
    (r"matplotlib\.figure:Figure\.(transFigure|transSubfigure)", cov("Transform::Figure")),
    (r"matplotlib\.text:Text\..*clipbox", cov("ArtistHandle::clip_box")),
]

OWNER_RULES = []

KWARG_RULES = [
    (r"data", LABELLED_DATA),
    (r"matplotlib\.axes:Axes\.set\(.*\)", diff("chained setters (Axes::set_xlim, set_xlabel, ...)", "conventions: `ax.set(...)` becomes chained setters")),
    (r"matplotlib\.axes:Axes\.get_title\(loc\)", cov("Axes::title(TitleLocation)")),
    (r"matplotlib\.axes:Axes\.set_title\(label\)", cov("Axes::set_title(text)")),
    (r"matplotlib\.axes:Axes\.set_title\(loc\)", cov("Axes::set_title_at(TitleLocation, text)")),
    (r"fontdict", diff("setters of the returned Text (Text::update_props, TextProps)", "the text's properties are set on its handle")),
    (r"matplotlib\.axes:Axes\.get_legend_handles_labels\(legend_handler_map\)", diff("LegendOptions::handler_map", "the handler map is consulted when the legend is made")),
    (r"matplotlib\.axes:Axes\.inset_axes\(zorder\)", diff("ArtistHandle::set_zorder on the inset", "set on the returned axes")),
    (r"matplotlib\.axes:Axes\.indicate_inset(_zoom)?\(inset_ax\)", cov("Axes::indicate_inset(.., inset, ..)")),
    (r"matplotlib\.axes:Axes\.secondary_[xy]axis\(transform\)", cov("SecondaryLocation (Data variant)", "where the secondary axis sits")),
    (r"matplotlib\.axes:Axes\.text\(s\)", cov("Axes::text(x, y, text)")),
    (r"matplotlib\.axes:Axes\.axline\((xy2|slope)\)", cov("AxLineThrough (a point or a slope)")),
    (r"matplotlib\.axes:Axes\.[hv]lines\((colors|linestyles|label)\)", diff("Collection::set_edgecolors / Collection::style / ArtistHandle::set_label on the returned collection", "set on the returned handle")),
    (r"matplotlib\.axes:Axes\.plot\((scalex|scaley)\)", diff("Axes::set_axis_autoscale_on", "turn autoscaling of that axis off around the call")),
    (r"matplotlib\.axes:Axes\.step\(where\)", cov("Axes::step(.., at: StepWhere, ..)")),
    (r"matplotlib\.axes:Axes\.barh\(height\)", cov("BarOptions::thickness")),
    (r"matplotlib\.axes:Axes\.broken_barh\(yrange\)", cov("Axes::broken_barh(.., band: BandRange, ..)")),
    (r"matplotlib\.axes:Axes\.pie\(counterclock\)", cov("PieOptions::direction (PieDirection)")),
    (r"matplotlib\.axes:Axes\.boxplot\(x\)", cov("Axes::boxplot(datasets, ..)")),
    (r"matplotlib\.axes:Axes\.(boxplot|bxp)\(notch\)", cov("BxpOptions::shownotches")),
    (r"matplotlib\.axes:Axes\.(boxplot|bxp|violinplot|violin)\(vert\)", cov("orientation (Orientation)")),
    (r"matplotlib\.axes:Axes\.(boxplot|bxp)\((patch_artist|boxprops)\)", cov("BxpOptions::boxes (BoxProps: line or patch boxes)")),
    (r"matplotlib\.axes:Axes\.boxplot\(bootstrap\)", miss("bootstrapped confidence intervals of the median")),
    (r"matplotlib\.axes:Axes\.bxp\(bxpstats\)", cov("Axes::bxp(stats, ..)")),
    (r"matplotlib\.axes:Axes\.scatter\(s\)", cov("ScatterOptions::sizes")),
    (r"matplotlib\.axes:Axes\.scatter\(c\)", cov("ScatterOptions::colors (ScatterColors)")),
    (r"matplotlib\.axes:Axes\.scatter\((cmap|norm|vmin|vmax|colorizer)\)", cov("ScatterColors::Mapped(MappedColors { colorizer })")),
    (r"matplotlib\.axes:Axes\.scatter\(plotnonfinite\)", cov("ScatterOptions::plot_nonfinite")),
    (r"matplotlib\.axes:Axes\.hexbin\(C\)", cov("HexbinOptions::values")),
    (r"matplotlib\.axes:Axes\.hexbin\(reduce_C_function\)", cov("HexbinOptions::reduce")),
    (r"matplotlib\.axes:Axes\.arrow\((dx|dy)\)", cov("FancyArrow::delta")),
    (r"matplotlib\.axes:Axes\.quiverkey\((Q|X|Y|U)\)", cov("Axes::quiverkey(quiver, position, length, ..)")),
    (r"matplotlib\.axes:Axes\.imshow\(X\)", cov("Axes::imshow(data, ..)")),
    (r"url", diff("ArtistHandle::set_url", "set on the returned artist")),
    (r"matplotlib\.axes:Axes\.clabel\((CS|levels)\)", cov("ContourSet::clabel (ClabelOptions::levels)")),
    (r"NFFT", cov("SpectralOptions::nfft")),
    (r"Fs", cov("SpectralOptions::fs / SpectrumOptions::fs")),
    (r"return_line", diff("SpectralPlotOptions::line", "the line is always returned with the spectrum")),
    (r"matplotlib\.axes:Axes\.(spy|matshow)\(Z\)", cov("first argument (Grid / ImageData)")),
    (r"matplotlib\.axes:Axes\.violin\(vpstats\)", cov("Axes::violin(stats, ..)")),
    (r"matplotlib\.axes:Axes\.table\(ax\)", na("the axes is the receiver")),
    (r"matplotlib\.axes:Axes\.(stackplot|streamplot)\(axes\)", na("the axes is the receiver")),
    (r"matplotlib\.axes:Axes\.(tricontour|tricontourf|tripcolor|triplot)\(ax\)", na("the axes is the receiver")),
    (r"matplotlib\.axes:Axes\.streamplot\((cmap|norm)\)", cov("StreamColors::Mapped(StreamColorMap { colorizer })")),
]


# The other rule files (rules_*.py) extend the lists.
for _path in sorted(glob.glob(os.path.join(os.path.dirname(os.path.abspath(__file__)), "rules_*.py"))):
    _name = os.path.basename(_path)[:-3]
    if _name == "rules_base":
        continue
    _module = importlib.import_module(_name)
    if _name == "rules_impl":
        # What this audit implemented overrides the earlier verdicts.
        RULES = getattr(_module, "RULES", []) + RULES
        KWARG_RULES = getattr(_module, "KWARG_RULES", []) + KWARG_RULES
        continue
    RULES = RULES + getattr(_module, "RULES", [])
    OWNER_RULES = OWNER_RULES + getattr(_module, "OWNER_RULES", [])
    KWARG_RULES = KWARG_RULES + getattr(_module, "KWARG_RULES", [])
