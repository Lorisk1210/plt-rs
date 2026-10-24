//! Replaying a scenario: the figure, its numbered axes and the objects
//! earlier steps produced.

use plt::artist::RawHandle;
use plt::axes::{Axes, BarContainer};
use plt::collections::Collection;
use plt::colorbar::Colorbar;
use plt::contour::ContourSet;
use plt::figure::Figure;
use plt::image::AxesImage;
use plt::layout::LayoutEngine;
use plt::legend::{Legend, LegendHandle};

use super::call::Call;
use super::{axes_steps, colorbar_steps, figure_steps, legend_steps, object_steps};
use crate::json::Json;
use crate::probes::ProbeError;

/// Something a step returned that later steps can act on (`"@k"`).
#[derive(Clone, Debug)]
pub(crate) enum Object {
    /// The step returned nothing usable.
    Nothing,
    /// An axes (`twinx`, `add_subplot`, ...).
    Axes(Axes),
    /// An artist with only the properties every artist shares.
    Artist(RawHandle),
    /// A collection (`scatter`).
    Collection(Collection),
    /// The bars of `bar`/`barh`.
    Bars(BarContainer),
    /// A legend.
    Legend(Legend),
    /// An image (`imshow`).
    Image(AxesImage),
    /// A contour set (`contour`, `contourf`).
    Contours(ContourSet),
    /// A colorbar.
    Colorbar(Colorbar),
}

impl Object {
    /// The legend entry the object makes, if it can be one.
    fn legend_handle(&self) -> Option<LegendHandle> {
        match self {
            Self::Artist(artist) => Some(LegendHandle::from(artist)),
            Self::Collection(collection) => Some(LegendHandle::from(collection)),
            Self::Bars(bars) => Some(LegendHandle::from(bars)),
            Self::Image(image) => Some(LegendHandle::from(image)),
            Self::Nothing
            | Self::Axes(_)
            | Self::Legend(_)
            | Self::Contours(_)
            | Self::Colorbar(_) => None,
        }
    }

    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Axes(a), Self::Axes(b)) => a == b,
            (Self::Legend(a), Self::Legend(b)) => a == b,
            _ => self.legend_handle().is_some() && self.legend_handle() == other.legend_handle(),
        }
    }
}

/// What a step produced.
#[derive(Debug)]
pub(crate) enum Produced {
    /// A recorded value (queries).
    Value(Json),
    /// An object for later steps; records `null`.
    Object(Object),
    /// Nothing; records `null`.
    Nothing,
}

impl Produced {
    /// A recorded pair of numbers.
    pub(crate) fn pair((a, b): (f64, f64)) -> Self {
        Self::Value(Json::from(vec![a, b]))
    }

    /// A recorded list of numbers.
    pub(crate) fn numbers(values: Vec<f64>) -> Self {
        Self::Value(Json::from(values))
    }
}

/// Result of one step handler; `None` when the handler does not know the
/// method.
pub(crate) type Handled = Option<Result<Produced, ProbeError>>;

/// The state of a scenario being replayed.
#[derive(Debug)]
pub(crate) struct Scene {
    /// The figure.
    pub(crate) fig: Figure,
    axes: Vec<Axes>,
    removed: Vec<Object>,
    objects: Vec<Object>,
}

impl Scene {
    /// A scene drawing into `Figure(**figure)`: `figsize` and `layout`
    /// (`"constrained"` or `"tight"`) are understood.
    pub(crate) fn new(figure: Option<&Json>) -> Result<Self, ProbeError> {
        let options = figure.and_then(Json::as_object).unwrap_or_default();
        let bad = || ProbeError::BadArgs(format!("unsupported figure options {figure:?}"));
        let mut fig = Figure::new();
        let mut engine = None;
        for (key, value) in options {
            match (key.as_str(), value) {
                ("figsize", size) => match size.as_f64_vec().as_deref() {
                    Some(&[width, height]) => {
                        fig = Figure::with_size(width, height, plt::rc::params().figure_dpi);
                    }
                    _ => return Err(bad()),
                },
                ("layout", Json::String(name)) if name == "constrained" => {
                    engine = Some(LayoutEngine::constrained());
                }
                ("layout", Json::String(name)) if name == "tight" => {
                    engine = Some(LayoutEngine::tight());
                }
                _ => return Err(bad()),
            }
        }
        fig.set_layout_engine(engine)
            .map_err(ProbeError::rejected)?;
        Ok(Self {
            fig,
            axes: Vec::new(),
            removed: Vec::new(),
            objects: Vec::new(),
        })
    }

    /// Replays `steps`, returning one recorded value (or `null`) per step.
    pub(crate) fn run(&mut self, steps: &[Json]) -> Result<Json, ProbeError> {
        steps
            .iter()
            .map(|step| self.step(step))
            .collect::<Result<_, _>>()
            .map(Json::Array)
    }

    fn step(&mut self, step: &Json) -> Result<Json, ProbeError> {
        let (target, method, args) = match step.as_array() {
            Some([target, Json::String(method), args @ ..]) => (target, method.as_str(), args),
            _ => return Err(ProbeError::BadArgs(format!("malformed step {step}"))),
        };
        let produced = if target.as_str() == Some("fig") {
            let call = Call::new(method, args)?;
            let produced = figure_steps::call(self, &call)
                .or_else(|| legend_steps::figure_call(self, &call))
                .or_else(|| colorbar_steps::figure_call(self, &call));
            if matches!(method, "clear" | "clf") {
                self.removed
                    .extend(self.axes.iter().cloned().map(Object::Axes));
            }
            finished(&call, produced)?
        } else {
            // `cbar.ax.<method>` is a method of the colorbar's axes.
            let (object, method) = match (self.target(target)?, method.strip_prefix("ax.")) {
                (Object::Colorbar(colorbar), Some(rest)) => {
                    (Object::Axes(colorbar.ax().clone()), rest)
                }
                (object, _) => (object, method),
            };
            if self.removed.iter().any(|gone| gone.same(&object)) {
                return Err(ProbeError::rejected(
                    "the artist was removed; plt-rs handles of removed artists are invalid",
                ));
            }
            let call = Call::new(method, args)?;
            let produced = match &object {
                Object::Legend(legend) => legend_steps::legend_call(legend, &call),
                Object::Colorbar(colorbar) => colorbar_steps::colorbar_call(colorbar, &call),
                Object::Axes(ax) => axes_steps::call(self, ax, &call),
                object => object_steps::call(object, &call),
            };
            let produced = finished(&call, produced)?;
            if method == "remove" {
                self.removed.push(object);
            }
            produced
        };
        self.track_axes();
        Ok(match produced {
            Produced::Value(value) => {
                self.objects.push(Object::Nothing);
                value
            }
            Produced::Object(object) => {
                self.objects.push(object);
                Json::Null
            }
            Produced::Nothing => {
                self.objects.push(Object::Nothing);
                Json::Null
            }
        })
    }

    /// Numbers the axes the last step added to the figure.
    fn track_axes(&mut self) {
        for ax in self.fig.axes() {
            if !self.axes.contains(&ax) {
                self.axes.push(ax);
            }
        }
    }

    fn target(&self, target: &Json) -> Result<Object, ProbeError> {
        match target {
            Json::String(reference) => self.reference(reference),
            number => {
                let n = number
                    .as_i64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad target {target}")))?;
                self.axes(n).map(Object::Axes)
            }
        }
    }

    /// Axes number `n`.
    pub(crate) fn axes(&self, n: usize) -> Result<Axes, ProbeError> {
        self.axes
            .get(n)
            .cloned()
            .ok_or_else(|| ProbeError::BadArgs(format!("no axes {n}")))
    }

    /// The number of an axes of the scene.
    pub(crate) fn number(&self, ax: &Axes) -> Option<usize> {
        self.axes.iter().position(|known| known == ax)
    }

    /// The object a reference names: `"@k"` for step `k`, `"@axN"` for
    /// axes `N`.
    pub(crate) fn reference(&self, reference: &str) -> Result<Object, ProbeError> {
        let bad = || ProbeError::BadArgs(format!("bad reference {reference:?}"));
        let name = reference.strip_prefix('@').ok_or_else(bad)?;
        if let Some(n) = name.strip_prefix("ax") {
            return self.axes(n.parse().map_err(|_| bad())?).map(Object::Axes);
        }
        let step: usize = name.parse().map_err(|_| bad())?;
        self.objects.get(step).cloned().ok_or_else(bad)
    }

    /// The axes an argument refers to (`"@axN"` or a step that made one).
    pub(crate) fn axes_argument(&self, value: &Json) -> Result<Axes, ProbeError> {
        match value.as_str().map(|r| self.reference(r)) {
            Some(Ok(Object::Axes(ax))) => Ok(ax),
            _ => Err(ProbeError::BadArgs(format!("{value} is not an axes"))),
        }
    }

    /// The legend entry an argument refers to (a step that made an artist).
    pub(crate) fn legend_handle(&self, value: &Json) -> Result<LegendHandle, ProbeError> {
        value
            .as_str()
            .map(|r| self.reference(r))
            .transpose()?
            .and_then(|object| object.legend_handle())
            .ok_or_else(|| ProbeError::BadArgs(format!("{value} is not a legend handle")))
    }

    /// `"@k"` for the step that returned `object` (the first such step).
    pub(crate) fn reference_of(&self, object: &Object) -> Result<Json, ProbeError> {
        self.objects
            .iter()
            .position(|made| made.same(object))
            .map(|step| Json::from(format!("@{step}")))
            .ok_or_else(|| ProbeError::BadArgs(format!("{object:?} was not made by a step")))
    }

    /// `"@k"` for the step that made the artist of a legend entry.
    pub(crate) fn reference_of_handle(&self, handle: &LegendHandle) -> Result<Json, ProbeError> {
        self.objects
            .iter()
            .position(|made| made.legend_handle().as_ref() == Some(handle))
            .map(|step| Json::from(format!("@{step}")))
            .ok_or_else(|| ProbeError::BadArgs(format!("{handle:?} was not made by a step")))
    }
}

/// The outcome of a handled call, rejecting unknown methods and arguments
/// the handler did not read.
fn finished(call: &Call<'_>, produced: Handled) -> Result<Produced, ProbeError> {
    let produced = produced.unwrap_or_else(|| Err(call.bad("unsupported method")))?;
    call.finish()?;
    Ok(produced)
}
