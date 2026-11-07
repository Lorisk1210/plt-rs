use std::any::{Any, TypeId};
use std::rc::Rc;

use pltrs_core::archive::{self, ArchiveError, External, NoProgram, ProgramKinds};

use super::Figure;
use crate::artist::{ArtistHandle, ArtistId, FigureRef, Picker, RawHandle};
use crate::axes::AxesState;
use crate::axes::ImshowOptions;
use crate::colors::colormaps;
use crate::events::EventSource;
use crate::figure::{FigureState, SubfigureOptions};
use crate::legend::LegendOptions;
use crate::lines::{Line2D, LineProps};
use crate::navigation::PointerButton;
use crate::numeric::Grid;
use crate::ticker::FuncFormatter;

/// The figure `fig` archived and read back.
fn round_trip(fig: &Figure) -> Figure {
    let (bytes, externals) = fig.to_archive(&NoProgram).expect("archivable");
    Figure::from_archive(&bytes, &externals, &NoProgram).expect("readable")
}

#[test]
fn a_figure_read_back_draws_the_same() {
    let fig = Figure::with_size(4.0, 3.0, 50.0);
    let ax = fig.subplot();
    ax.plot([0.0, 1.0, 2.0], [1.0, 3.0, 2.0])
        .expect("a line")
        .set_label("rising");
    let data = Grid::from_rows([vec![0.0, 1.0], vec![2.0, 3.0]]).expect("a grid");
    ax.imshow(data, ImshowOptions::default())
        .expect("an image")
        .set_cmap(colormaps::get("gnuplot").expect("a built-in colormap"));
    ax.legend(LegendOptions::default())
        .expect("a legend")
        .set_draggable(true);
    ax.set_title("title");
    let back = round_trip(&fig);
    assert_eq!(back.to_png().expect("drawn"), fig.to_png().expect("drawn"));
    assert_eq!(back.size_inches(), (4.0, 3.0));
}

#[test]
fn artists_keep_their_ids_removed_ones_too() {
    let fig = Figure::new();
    let ax = fig.subplot();
    let kept = ax.plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
    let removed = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("a line");
    removed.remove();
    let back = round_trip(&fig);
    let root = back.raw().clone();
    let line = |id: ArtistId| root.sibling(id).downcast::<Line2D>().expect("a line");
    assert_eq!(line(kept.id()).ydata(), vec![0.0, 1.0]);
    let gone = line(removed.id());
    assert_eq!(gone.ydata(), vec![1.0, 0.0]);
    assert!(!gone.raw().is_attached());
}

#[test]
fn a_figure_restored_in_place_serves_the_handles_made_before() {
    let fig = Figure::new();
    let line = fig.subplot().plot([0.0, 1.0], [2.0, 4.0]).expect("a line");
    let (bytes, externals) = fig.to_archive(&NoProgram).expect("archivable");
    let target = Figure::with_size(1.0, 1.0, 72.0);
    let early = target.raw().sibling(line.id());
    target
        .raw()
        .restore_figure(&bytes, &externals, &NoProgram)
        .expect("readable");
    assert_eq!(target.size_inches(), fig.size_inches());
    assert_eq!(
        early.downcast::<Line2D>().map(|line| line.ydata()),
        Some(vec![2.0, 4.0])
    );
}

#[test]
fn a_loose_artist_is_archived_with_its_holder() {
    let line = Line2D::loose(vec![0.0, 1.0], vec![5.0, 6.0], LineProps::default());
    let (bytes, externals) = line.raw().archive_figure(&NoProgram).expect("archivable");
    let back = RawHandle::from_figure_archive(&bytes, &externals, &NoProgram, line.id())
        .expect("readable");
    assert!(back.is_loose());
    assert_eq!(
        back.downcast::<Line2D>().map(|l| l.ydata()),
        Some(vec![5.0, 6.0])
    );
    assert!(Figure::from_archive(&bytes, &externals, &NoProgram).is_err());
}

#[test]
fn handles_archive_as_their_figure_and_id() {
    let fig = Figure::new();
    let line = fig.subplot().plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
    let (bytes, externals) = archive::to_bytes(line.raw(), &NoProgram).expect("archivable");
    assert_eq!(externals.len(), 1);
    let figure = externals[0]
        .downcast_ref::<FigureRef>()
        .expect("the figure");
    assert_eq!(figure.handle(line.id()), *line.raw());
    let back: RawHandle = archive::from_bytes(&bytes, &externals, &NoProgram).expect("readable");
    assert_eq!(back, *line.raw());
}

#[test]
fn functions_without_an_origin_are_not_archived() {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.xaxis()
        .set_major_formatter(FuncFormatter::new(|x, _| format!("{x}!")));
    assert!(matches!(
        fig.to_archive(&NoProgram),
        Err(ArchiveError::Unsupported(_))
    ));
}

/// A program making the functions of formatters from the number of
/// exclamation marks they append.
struct Exclaim;

impl ProgramKinds for Exclaim {
    fn function(&self, function: TypeId, origin: &External) -> Result<Box<dyn Any>, ArchiveError> {
        type Labels = Rc<dyn Fn(f64, Option<usize>) -> String>;
        assert_eq!(function, TypeId::of::<Labels>());
        let marks = "!".repeat(*origin.downcast_ref::<usize>().expect("a count"));
        let labels: Labels = Rc::new(move |x, _| format!("{x}{marks}"));
        Ok(Box::new(labels))
    }
}

#[test]
fn functions_with_an_origin_are_made_again_by_the_program() {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.set_xticks([0.0, 1.0]);
    let formatter = FuncFormatter::with_origin(|x, _| format!("{x}!!"), External::new(2usize));
    ax.xaxis().set_major_formatter(formatter);
    let (bytes, externals) = fig.to_archive(&Exclaim).expect("archivable");
    let back = Figure::from_archive(&bytes, &externals, &Exclaim).expect("readable");
    let labels = back.axes()[0].xaxis().ticklabels(false).expect("labels");
    assert_eq!(labels, ["0!!", "1!!"]);
}

#[test]
fn tick_labels_set_by_text_and_draggable_legends_archive() {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.plot([0.0, 1.0], [0.0, 1.0])
        .expect("a line")
        .set_label("a");
    ax.set_xticks([0.0, 0.5]);
    ax.set_xticklabels(["start", "middle"])
        .expect("as many labels");
    let legend = ax.legend(LegendOptions::default()).expect("a legend");
    legend.set_draggable(true);
    assert_eq!(
        legend.raw().with_base(|base| base.picker.clone()),
        Some(Picker::Press)
    );
    let back = round_trip(&fig);
    let ax = &back.axes()[0];
    let labels = ax.xaxis().ticklabels(false).expect("labels");
    assert_eq!(labels, ["start", "middle"]);

    // The legend read back is dragged by the callbacks of its new figure.
    let legend = ax.current_legend().expect("the legend");
    assert!(legend.draggable());
    let extent = legend.window_extent().expect("drawn");
    let (x, y) = (extent.x0() + 5.0, extent.y0() + 5.0);
    let mut source = EventSource::new();
    source.button_press(&back, x, y, PointerButton::Left, false);
    source.motion(&back, x + 40.0, y);
    source.button_release(&back, x + 40.0, y, PointerButton::Left);
    let moved = legend.window_extent().expect("drawn");
    assert!((moved.x0() - (extent.x0() + 40.0)).abs() < 1e-9);
}

#[test]
fn corrupted_archives_are_refused_without_panicking() {
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.plot([0.0, 1.0], [0.0, 1.0]).expect("a line");
    ax.set_title("t");
    let (bytes, externals) = fig.to_archive(&NoProgram).expect("archivable");
    let mut rng = pltrs_core::rng::SplitMix64::new(7);
    for _ in 0..300 {
        let mut corrupted = bytes.clone();
        for _ in 0..=rng.below(4) {
            let index = rng.below(corrupted.len());
            corrupted[index] = rng.next_u64().to_le_bytes()[0];
        }
        let _ = Figure::from_archive(&corrupted, &externals, &NoProgram);
    }
    for len in (0..bytes.len()).step_by(97) {
        assert!(Figure::from_archive(&bytes[..len], &externals, &NoProgram).is_err());
    }
}

/// The figure `fig` archived after `tamper` changed its state, as a
/// corrupted archive may hold it, then read back.
fn tampered(fig: &Figure, tamper: impl FnOnce(&mut FigureState)) -> Result<Figure, ArchiveError> {
    tamper(&mut fig.state.borrow_mut());
    let (bytes, externals) = fig.to_archive(&NoProgram).expect("archivable");
    Figure::from_archive(&bytes, &externals, &NoProgram)
}

#[test]
fn artists_owning_or_placed_in_themselves_are_refused() {
    // Found by fuzzing: a subfigure listed among the children of an axes
    // it holds, which drew the two in turn until the stack overflowed.
    let fig = Figure::new();
    let sub = fig
        .subfigures(1, 1, SubfigureOptions::default())
        .expect("a subfigure")[(0, 0)]
        .clone();
    let ax = sub.subplot();
    let (frame, axes) = (sub.root.id(), ax.id());
    let read = tampered(&fig, |state| {
        let ax = state.artist_as_mut::<AxesState>(axes).expect("axes");
        ax.children.push(frame);
    });
    assert!(matches!(read, Err(ArchiveError::Invalid(_))));
    // A frame placed in an axes of its own: placing either asks the other.
    let fig = Figure::new();
    let sub = fig
        .subfigures(1, 1, SubfigureOptions::default())
        .expect("a subfigure")[(0, 0)]
        .clone();
    let ax = sub.subplot();
    let (frame, axes) = (sub.root.id(), ax.id());
    let read = tampered(&fig, |state| {
        state.artist_mut(frame).expect("frame").base_mut().parent = Some(axes);
    });
    assert!(matches!(read, Err(ArchiveError::Invalid(_))));
    // Untampered, the same figure reads back.
    let fig = Figure::new();
    fig.subfigures(1, 1, SubfigureOptions::default())
        .expect("a subfigure")[(0, 0)]
        .subplot();
    assert!(tampered(&fig, |_| {}).is_ok());
}

#[test]
fn artists_drawn_as_their_own_parts_report_a_recursion() {
    // Found by fuzzing: the figure's frame among the titles of its axes
    // (drawn by id, but none of its children) drew the two in turn until
    // the stack overflowed.
    let fig = Figure::new();
    let ax = fig.subplot();
    ax.set_title("t");
    let (root, axes) = (fig.root.id(), ax.id());
    let read = tampered(&fig, |state| {
        let ax = state.artist_as_mut::<AxesState>(axes).expect("axes");
        ax.titles[1] = root;
    })
    .expect("no link cycle among parents and children");
    let err = read.to_png().expect_err("the frame is its own part");
    assert!(
        err.to_string().contains("maximum recursion depth exceeded"),
        "{err}"
    );
}
