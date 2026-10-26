//! Handles stay usable after `remove()`: for every family of artists, a
//! removed artist's properties read back as set, the figure draws to every
//! format without it, and nothing panics.

use std::panic::{AssertUnwindSafe, catch_unwind};

use plt::artist::AttachError;
use plt::axes::{ImshowOptions, PieOptions, SecondaryLocation, StemOptions, TitleLocation};
use plt::axes_grid1::host_subplot;
use plt::axisartist::AxisArtistAxes;
use plt::colorbar::ColorbarOptions;
use plt::contour::{ClabelOptions, ContourOptions, QuadField};
use plt::figure::{SaveFormat, SaveOptions, SubfigureOptions};
use plt::geo::GeoProjection;
use plt::image::{FigimageOptions, ImageData};
use plt::legend::LegendOptions;
use plt::lines::LineProps;
use plt::numeric::{Grid, linspace};
use plt::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, AnnotationBboxOptions, TextArea};
use plt::patches::Rectangle;
use plt::prelude::*;
use plt::quiver::{QuiverKeyOptions, QuiverOptions, VectorField};
use plt::spines::SpineSide;
use plt::streamplot::{StreamField, StreamplotOptions};
use plt::table::TableOptions;
use plt::text::{AnnotateOptions, TextProps};
use plt::ticker::AxisName;
use plt::widgets::{
    Button, ButtonOptions, CheckButtons, Orientation, RectangleSelector, RectangleSelectorOptions,
    Slider, SliderOptions, SpanSelector, SpanSelectorOptions,
};

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// A family of artists: made, removed and used in a figure.
type Case = fn(&Figure) -> Outcome;

/// `z = r + c` on a 4 x 5 grid.
fn field() -> Grid<f64> {
    Grid::from_fn(4, 5, |r, c| (r + c) as f64)
}

/// Draws `fig` to every format; each must succeed.
fn draw(fig: &Figure) -> Outcome {
    for format in [
        SaveFormat::Png,
        SaveFormat::Svg,
        SaveFormat::Pdf,
        SaveFormat::Eps,
    ] {
        fig.to_format(format, SaveOptions::default())?;
    }
    Ok(())
}

/// Reads and sets the properties every artist has on `artist`, which must
/// have been removed, and checks they read back.
fn exercise(artist: &impl ArtistHandle) {
    assert!(!artist.raw().is_attached(), "the artist was removed");
    assert_eq!(artist.axes(), None, "a removed artist has no axes");
    artist
        .set_alpha(Some(0.5))
        .set_zorder(7.0)
        .set_label("removed")
        .set_visible(true)
        .set_gid("gone");
    assert_eq!(artist.alpha(), Some(0.5));
    assert_eq!(artist.zorder(), 7.0);
    assert_eq!(artist.label(), "removed");
    assert_eq!(artist.gid().as_deref(), Some("gone"));
    assert!(artist.window_extent().is_none());
    assert_eq!(artist.findobj(|_| true).last(), Some(artist.raw()));
    artist.remove();
    assert!(
        !artist.raw().is_attached(),
        "removing twice changes nothing"
    );
}

fn lines(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    line.remove();
    exercise(&line);
    line.set_data([0.0, 2.0], [0.0, 2.0])
        .set_linewidth(4.0)
        .set_color("tab:red");
    assert_eq!(line.data(), (vec![0.0, 2.0], vec![0.0, 2.0]));
    assert_eq!(line.linewidth(), 4.0);
    assert!(line.path().is_ok() && line.rgba().is_ok());
    let _ = line.marker_indices();
    assert!(ax.lines().is_empty(), "the axes no longer holds the line");
    draw(fig)
}

fn patches(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let patch = ax.add_patch(Rectangle::new((0.0, 0.0), 1.0, 1.0));
    let note = fig.add_artist(Rectangle::new((0.1, 0.1), 0.2, 0.2));
    patch.remove();
    note.remove();
    for patch in [&patch, &note] {
        exercise(patch);
        patch.set_facecolor("tab:green");
    }
    assert!(
        ax.patches().is_empty(),
        "the axes no longer holds the patch"
    );
    draw(fig)
}

fn texts(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let text = ax.text(0.5, 0.5, "text");
    let note = fig.text(0.1, 0.1, "note");
    let title = fig.suptitle("title");
    let annotation = ax.annotate("a", (0.5, 0.5), AnnotateOptions::default())?;
    for artist in [&text, &note, &title] {
        artist.remove();
        exercise(artist);
        artist.set_text("changed");
        assert_eq!(artist.text(), "changed");
    }
    annotation.remove();
    exercise(&annotation);
    annotation.set_text("moved");
    let _ = annotation.xy();
    assert!(ax.texts().is_empty(), "the axes no longer holds the texts");
    draw(fig)
}

fn collections(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let points = ax.scatter(vec![0.0, 1.0], vec![1.0, 0.0])?;
    points.remove();
    exercise(&points);
    points.set_facecolor("tab:blue");
    assert_eq!(points.offsets().len(), 2);
    assert!(
        ax.collections().is_empty(),
        "the axes no longer holds the points"
    );
    draw(fig)
}

fn images(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let image = ax.imshow(ImageData::from(field()), ImshowOptions::default())?;
    image.remove();
    exercise(&image);
    image.set_cmap(plt::colors::colormaps::get("gray")?);
    let _ = image.extent();
    assert!(ax.images().is_empty(), "the axes no longer holds the image");
    draw(fig)
}

fn axes(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    ax.set_title("title");
    ax.remove();
    exercise(&ax);
    assert_eq!(line.axes(), Some(ax.clone()), "its artists keep their axes");
    assert_eq!(ax.lines(), std::slice::from_ref(&line));
    ax.set_xlabel("x").set_facecolor("tab:gray");
    let _ = (
        ax.xlim(),
        ax.ylim(),
        ax.position(),
        ax.title(TitleLocation::default()),
    );
    let later = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    assert!(!later.raw().is_attached(), "made in a removed axes");
    assert!(fig.axes().is_empty(), "the figure holds no axes");
    let twin = ax.twinx()?;
    assert!(!twin.raw().is_attached());
    ax.relim(false).autoscale_view(None);
    let _ = ax.legend(LegendOptions::default());
    ax.clear();
    assert!(ax.lines().is_empty(), "clearing a removed axes empties it");
    draw(fig)
}

fn legends(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    ax.plot_with([0.0, 1.0], [1.0, 0.0], LineProps::default().label("line"))?;
    let legend = ax.legend(LegendOptions::default())?;
    legend.remove();
    exercise(&legend);
    legend.set_title("title");
    assert_eq!(ax.current_legend(), None);
    let figure_legend = fig.legend(LegendOptions::default())?;
    figure_legend.remove();
    exercise(&figure_legend);
    draw(fig)
}

fn colorbars(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let image = ax.imshow(ImageData::from(field()), ImshowOptions::default())?;
    let colorbar = fig.colorbar(&image, ColorbarOptions::default())?;
    colorbar.remove();
    let cax = colorbar.ax().clone();
    exercise(&cax);
    colorbar.set_label("label", None);
    let _ = (colorbar.vmin(), colorbar.vmax(), colorbar.outline());
    image.set_cmap(plt::colors::colormaps::get("gray")?);
    assert_eq!(fig.axes(), std::slice::from_ref(&ax));
    draw(fig)
}

fn containers(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let bars = ax.bar(vec![0.0, 1.0], [1.0, 2.0], BarOptions::default())?;
    let errors = ax.errorbar([0.0, 1.0], [1.0, 2.0], ErrorbarOptions::default())?;
    let stems = ax.stem([0.0, 1.0], [1.0, 2.0], StemOptions::default())?;
    let pie = ax.pie([1.0, 2.0], PieOptions::default())?;
    bars.remove();
    errors.remove();
    stems.remove();
    pie.remove();
    for patch in &bars.patches {
        exercise(patch);
    }
    if let Some(line) = &errors.data_line {
        exercise(line);
    }
    exercise(&stems.markerline);
    for wedge in &pie.wedges {
        exercise(wedge);
    }
    assert!(ax.containers().is_empty() && ax.patches().is_empty());
    draw(fig)
}

fn contour_sets(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let lines = ax.contour(&QuadField::new(field()), ContourOptions::default())?;
    let labels = lines.clabel(ClabelOptions::default())?;
    let filled = ax.contourf(&QuadField::new(field()), ContourOptions::default())?;
    lines.remove();
    filled.remove();
    for set in [&lines, &filled] {
        assert!(!set.levels().is_empty(), "the set keeps its levels");
        set.set_cmap(plt::colors::colormaps::get("gray")?)
            .set_linewidths(&[2.0]);
        for collection in set.collections() {
            exercise(collection);
        }
    }
    assert_eq!(lines.labels().len(), labels.len());
    for label in &labels {
        exercise(label);
    }
    assert!(ax.collections().is_empty() && ax.texts().is_empty());
    draw(fig)
}

fn tables_and_quivers(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let table = ax.table(TableOptions {
        cell_text: vec![vec!["a".into(), "b".into()]],
        ..TableOptions::default()
    })?;
    let quiver = ax.quiver(
        VectorField::new([0.0], [0.0], [1.0], [1.0]),
        QuiverOptions::default(),
    )?;
    let key = ax.quiverkey(
        &quiver,
        (0.9, 1.05),
        1.0,
        "1 m/s",
        QuiverKeyOptions::default(),
    );
    table.remove();
    key.remove();
    quiver.remove();
    exercise(&table);
    exercise(&key);
    exercise(&quiver);
    assert_eq!(key.text().text(), "1 m/s");
    draw(fig)
}

fn widgets(fig: &Figure) -> Outcome {
    let (one, two, three) = (
        fig.add_subplot(3, 1, 1)?,
        fig.add_subplot(3, 1, 2)?,
        fig.add_subplot(3, 1, 3)?,
    );
    let slider = Slider::new(&one, "s", 0.0, 1.0, SliderOptions::default())?;
    let button = Button::new(&two, "b", ButtonOptions::default());
    let checks = CheckButtons::new(&three, &["a", "b"], &[true, false]);
    slider.poly().remove();
    slider.valtext().remove();
    button.label().remove();
    checks.labels()[0].remove();
    slider.set_val(0.5);
    assert_eq!(slider.val(), 0.5);
    exercise(slider.poly());
    exercise(button.label());
    let _ = checks.status();
    one.remove();
    slider.set_val(0.25);
    draw(fig)
}

fn selectors_and_extras(fig: &Figure) -> Outcome {
    let (one, two) = (fig.add_subplot(2, 1, 1)?, fig.add_subplot(2, 1, 2)?);
    let span = SpanSelector::new(
        &one,
        Orientation::Horizontal,
        SpanSelectorOptions::default(),
    );
    let rectangle = RectangleSelector::new(&two, RectangleSelectorOptions::default());
    span.artist().remove();
    rectangle.artist().remove();
    span.set_extents((0.2, 0.6));
    assert_eq!(span.extents(), (0.2, 0.6));
    rectangle.set_extents((0.1, 0.5, 0.2, 0.4));
    exercise(&span.artist());
    exercise(&rectangle.artist());
    span.set_direction(Orientation::Vertical);
    let image = fig.figimage(ImageData::from(field()), FigimageOptions::default())?;
    image.remove();
    exercise(&image);
    assert_eq!(image.shape(), (4, 5));
    let table = two.table(TableOptions {
        cell_text: vec![vec!["a".into()]],
        ..TableOptions::default()
    })?;
    table.remove();
    let cell = table.cell(0, 0).ok_or("a cell")?;
    cell.set_text("b");
    assert_eq!(cell.text(), "b");
    let mapped = two.imshow(ImageData::from(field()), ImshowOptions::default())?;
    mapped.remove();
    assert!(
        fig.colorbar(&mapped, ColorbarOptions::default()).is_err(),
        "a removed mappable has no axes to take room from"
    );
    two.remove();
    assert!(fig.colorbar(&mapped, ColorbarOptions::default()).is_err());
    draw(fig)
}

fn three_d(fig: &Figure) -> Outcome {
    let ax = fig.subplot_3d();
    let line = ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    let points = ax.scatter(&[0.0, 1.0], &[1.0, 0.0], &[0.5, 0.5])?;
    line.remove();
    points.remove();
    exercise(&line);
    exercise(&points);
    let _ = line.data();
    let other = fig.subplot_3d();
    other.plot(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])?;
    other.remove();
    let _ = (other.position(), other.trans_axes());
    other.set_title("gone");
    let late = other.plot(&[0.0, 1.0], &[1.0, 0.0], &[0.0, 1.0])?;
    assert!(!late.raw().is_attached(), "made in a removed 3-D axes");
    draw(fig)
}

fn projections(fig: &Figure) -> Outcome {
    let polar = fig.add_polar_subplot(2, 2, 1)?;
    let geo = fig.add_geo_subplot(GeoProjection::Hammer, 2, 2, 2)?;
    let host = host_subplot(fig, 2, 2, 3)?;
    let parasite = host.twinx();
    let ax = AxisArtistAxes::new(fig.add_subplot(2, 2, 4)?);
    polar.axes().remove();
    geo.axes().remove();
    host.remove();
    ax.ax().remove();
    polar.set_theta_offset(1.0).set_thetamax(180.0);
    assert_eq!(polar.theta_offset(), 1.0);
    geo.set_longitude_grid_ends(60.0);
    assert!((geo.longitude_grid_ends() - 60.0).abs() < 1e-9);
    assert_eq!(host.parasites().len(), 1);
    parasite.plot([0.0, 1.0], [1.0, 0.0])?;
    assert_eq!(parasite.host().id(), host.id());
    let late = host.twinx();
    assert!(!late.raw().is_attached(), "made in a removed host");
    assert_eq!(ax.axis_artists().len(), 4);
    ax.new_floating_axis("x0", AxisName::X, 0.5);
    let twin = ax.twinx();
    assert!(!twin.ax().raw().is_attached());
    assert!(fig.axes().is_empty(), "the figure holds no axes");
    draw(fig)
}

fn axes_children(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let inset = ax.inset_axes([0.6, 0.6, 0.3, 0.3]);
    let secondary = ax.secondary_xaxis(SecondaryLocation::Side(SpineSide::Top), None)?;
    let twin = ax.twinx()?;
    let label = AnchoredBox::text(
        "label",
        TextProps::default(),
        AnchoredOptions::new(AnchorLoc::UpperLeft),
    )?;
    let anchored = ax.add_anchored_box(label);
    let note = ax.add_annotation_bbox(
        TextArea::from_text("peak", TextProps::default()),
        (0.5, 0.5),
        AnnotationBboxOptions::default(),
    )?;
    let x = linspace(0.0, 1.0, 10);
    let field = StreamField::new(
        x.clone(),
        x,
        Grid::filled(10, 10, 1.0),
        Grid::filled(10, 10, 0.5),
    );
    let streams = ax.streamplot(&field, StreamplotOptions::default())?;
    for axes in [&inset, &secondary, &twin] {
        axes.remove();
        exercise(axes);
        axes.xaxis().set_inverted(true);
        assert!(axes.xaxis().inverted(), "a removed axes takes new limits");
    }
    anchored.remove();
    note.remove();
    streams.lines.remove();
    exercise(&anchored);
    exercise(&note);
    exercise(&streams.lines);
    assert_eq!(anchored.texts()[0].text(), "label");
    note.set_xybox((0.2, 0.2));
    assert_eq!(note.xybox(), (0.2, 0.2).into());
    assert!(ax.child_axes().is_empty(), "the axes holds no child axes");
    let late = inset.inset_axes([0.1, 0.1, 0.5, 0.5]);
    assert!(
        !late.raw().is_attached(),
        "an inset of a removed axes is removed"
    );
    assert_eq!(inset.child_axes(), [late]);
    ax.spines().left.remove();
    exercise(&ax.spines().left);
    let title = ax.title_text(TitleLocation::Center);
    title.remove();
    exercise(&title);
    ax.set_title_at(TitleLocation::Center, "again");
    assert_eq!(
        title.text(),
        "again",
        "the title text goes to the removed title"
    );
    draw(fig)
}

fn subfigures(fig: &Figure) -> Outcome {
    let subs = fig.subfigures(1, 2, SubfigureOptions::default())?;
    let sub = subs[(0, 1)].clone();
    let ax = sub.subplot();
    ax.plot([0.0, 1.0], [1.0, 0.0])?;
    sub.remove();
    sub.suptitle("gone");
    let later = sub.subplot();
    assert!(!later.raw().is_attached(), "made in a removed subfigure");
    assert!(fig.axes().is_empty(), "the figure holds no axes");
    let root = fig.clone();
    root.remove();
    assert!(root.raw().is_attached(), "the figure itself stays");
    draw(fig)
}

fn clearing(fig: &Figure) -> Outcome {
    let ax = fig.subplot();
    let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    ax.clear();
    exercise(&line);
    let kept = fig.subplot();
    let text = kept.text(0.5, 0.5, "t");
    fig.clear();
    exercise(&kept);
    let _ = text.text();
    draw(fig)
}

fn attaching(fig: &Figure) -> Outcome {
    let (left, right) = (fig.add_subplot(1, 2, 1)?, fig.add_subplot(1, 2, 2)?);
    let line = left.plot([0.0, 1.0], [1.0, 0.0])?;
    assert_eq!(right.attach(&line).err(), Some(AttachError::NotRemoved));
    line.remove();
    right.attach(&line)?;
    assert_eq!(right.lines(), std::slice::from_ref(&line));
    assert_eq!(line.axes(), Some(right.clone()));
    left.remove();
    let other = Figure::new();
    assert_eq!(other.attach(&left).err(), Some(AttachError::OtherFigure));
    let title = left.set_title("t");
    assert_eq!(
        right.attach(&title).err(),
        Some(AttachError::OwnedByRemoved)
    );
    fig.attach(&left)?;
    assert_eq!(fig.axes(), [left.clone(), right.clone()]);
    assert_eq!(fig.gca(), left);
    let sub = fig.subfigures(1, 1, SubfigureOptions::default())?[(0, 0)].clone();
    sub.remove();
    assert_eq!(right.attach(&sub).err(), Some(AttachError::Misplaced));
    let inner = sub.subplot();
    assert_eq!(inner.attach(&sub).err(), Some(AttachError::Misplaced));
    fig.attach(&sub)?;
    assert!(inner.raw().is_attached());
    draw(fig)
}

#[test]
fn removed_artists_stay_usable() {
    let cases: [(&str, Case); 19] = [
        ("lines", lines),
        ("patches", patches),
        ("texts", texts),
        ("collections", collections),
        ("images", images),
        ("axes", axes),
        ("legends", legends),
        ("colorbars", colorbars),
        ("containers", containers),
        ("contour sets", contour_sets),
        ("tables and quivers", tables_and_quivers),
        ("widgets", widgets),
        ("selectors and extras", selectors_and_extras),
        ("3-D artists", three_d),
        ("projections and toolkits", projections),
        ("children of axes", axes_children),
        ("subfigures", subfigures),
        ("clearing", clearing),
        ("attaching", attaching),
    ];
    let mut failures = Vec::new();
    for (name, case) in cases {
        let fig = Figure::with_size(4.0, 3.0, 50.0);
        match catch_unwind(AssertUnwindSafe(|| case(&fig))) {
            Ok(Ok(())) => {}
            Ok(Err(err)) => failures.push(format!("{name}: {err}")),
            Err(_) => failures.push(format!("{name}: panicked")),
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
