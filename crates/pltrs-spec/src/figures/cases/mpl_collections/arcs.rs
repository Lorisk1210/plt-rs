//! `test_patches.py`: arcs far larger than the view, skewed arcs in every
//! quadrant, annuli built directly and through their setters, autoscaling
//! to arcs, an arc in a collection and an arc changed after it was added.

use pltrs::axes::AspectError;
use pltrs::collections::{CollectionPatch, CollectionProps, CollectionSpec};
use pltrs::patches::{Annulus, Arc, Patch};
use pltrs::prelude::*;
use pltrs::rc::Aspect;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

pub(super) fn large_arc(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    for ax in axs.as_slice() {
        ax.add_patch(Arc::new((210.0, -2115.0), 4261.0, 4261.0))
            .update_props(PatchProps::default().linewidth(2.0).color("k"));
        ax.set_axis_off();
        ax.set_aspect(Aspect::Equal)?;
    }
    // The high accuracy case, then the low accuracy one.
    axs[(0, 0)].set_xlim(7.0, 8.0)?.set_ylim(5.0, 6.0)?;
    axs[(0, 1)]
        .set_xlim(-25000.0, 18000.0)?
        .set_ylim(-20000.0, 6600.0)?;
    Ok(())
}

pub(super) fn rotated_arcs(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(2, 2)?;
    let scale = 10_000_000.0;
    let diag_centers = [(-1.0, -1.0), (-1.0, 1.0), (1.0, 1.0), (1.0, -1.0)];
    let on_axis_centers = [(0.0, 1.0), (1.0, 0.0), (0.0, -1.0), (-1.0, 0.0)];
    let skews = [(2.0, 2.0), (2.0, 0.1), (2.0, 0.01), (2.0, 0.001)];
    let prescale = 1.0 - 0.0001;
    let groups = [
        (prescale, on_axis_centers),
        (prescale / 2f64.sqrt(), diag_centers),
    ];
    for (ax, (sx, sy)) in axs.as_slice().iter().zip(skews) {
        let centers = groups
            .iter()
            .flat_map(|&(prescale, centers)| centers.map(|c| (prescale, c)));
        for (j, (prescale, (x_sign, y_sign))) in centers.enumerate() {
            let angle = f64::atan2(y_sign, x_sign).to_degrees().rem_euclid(360.0);
            let center = (x_sign * scale * prescale, y_sign * scale * prescale);
            let label = format!("big {j}");
            let mut props = PatchProps::default()
                .linewidth(4.0)
                .color(format!("C{j}"))
                .zorder(1.0 + j as f64)
                .label(label.as_str());
            props.base.gid = Some(label);
            ax.add_patch(Arc::new(center, scale * sx, scale * sy).with_angle(angle))
                .update_props(props);
        }
        let limit = scale / 4000.0;
        ax.set_xlim(-limit, limit)?.set_ylim(-limit, limit)?;
        ax.axhline(0.0, LineProps::default().color("k"));
        ax.axvline(0.0, LineProps::default().color("k"));
        ax.set_axis_off();
        ax.set_aspect(Aspect::Equal)?;
    }
    Ok(())
}

/// The two annuli of the annulus tests.
type Annuli = (Patch<Annulus>, Patch<Annulus>);

/// A green circular annulus and a half-transparent, hatched magenta
/// elliptical one on equal axes.
fn annuli(fig: &Figure, circular: Annulus, elliptical: Annulus) -> Result<Annuli, AspectError> {
    let ax = fig.subplot();
    let cir = ax.add_patch(circular);
    cir.set_facecolor("g");
    let ell = ax.add_patch(elliptical);
    ell.update_props(
        PatchProps::default()
            .facecolor("m")
            .edgecolor("b")
            .alpha(0.5)
            .hatch("xxx"),
    );
    ax.set_aspect(Aspect::Equal)?;
    Ok((cir, ell))
}

pub(super) fn annulus(fig: &Figure) -> CaseResult {
    annuli(
        fig,
        Annulus::new((0.5, 0.5), 0.2, 0.05)?,
        Annulus::elliptical((0.5, 0.5), (0.5, 0.3), 0.1, 45.0)?,
    )?;
    Ok(())
}

/// The annuli of `test_annulus_setters`, before their setters run.
fn annuli_to_set(fig: &Figure) -> Result<Annuli, Box<dyn std::error::Error>> {
    Ok(annuli(
        fig,
        Annulus::new((0.0, 0.0), 0.2, 0.01)?,
        Annulus::elliptical((0.0, 0.0), (1.0, 2.0), 0.1, 0.0)?,
    )?)
}

pub(super) fn annulus_setters(fig: &Figure) -> CaseResult {
    let (cir, ell) = annuli_to_set(fig)?;
    let mut circular = cir.shape();
    circular.set_center((0.5, 0.5));
    circular.set_radii((0.2, 0.2));
    circular.set_width(0.05)?;
    cir.set_shape(circular);
    let mut elliptical = ell.shape();
    elliptical.set_center((0.5, 0.5));
    elliptical.set_radii((0.5, 0.3));
    elliptical.set_width(0.1)?;
    elliptical.set_angle(45.0);
    ell.set_shape(elliptical);
    Ok(())
}

pub(super) fn annulus_setters2(fig: &Figure) -> CaseResult {
    let (cir, ell) = annuli_to_set(fig)?;
    let mut circular = cir.shape();
    circular.set_center((0.5, 0.5));
    circular.set_semimajor(0.2);
    circular.set_semiminor(0.2);
    circular.set_width(0.05)?;
    cir.set_shape(circular);
    let mut elliptical = ell.shape();
    elliptical.set_center((0.5, 0.5));
    elliptical.set_semimajor(0.5);
    elliptical.set_semiminor(0.3);
    elliptical.set_width(0.1)?;
    elliptical.set_angle(45.0);
    ell.set_shape(elliptical);
    Ok(())
}

pub(super) fn autoscale_arc(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    // Ten-degree arcs around (0.5, 0.5), each a unit wider than the last.
    let narrow = |i: u8| {
        let i = f64::from(i);
        Arc::new((0.5, 0.5), 1.5 + i, 0.5).between(10.0 + 100.0 * i, 20.0 + 100.0 * i)
    };
    let arc_lists = [
        vec![Arc::new((0.0, 0.0), 1.0, 1.0).between(0.0, 90.0)],
        vec![narrow(0)],
        (0..4).map(narrow).collect(),
    ];
    for (ax, arcs) in axs.as_slice().iter().zip(arc_lists) {
        for arc in arcs {
            ax.add_patch(arc);
        }
        ax.autoscale(None, None);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn arc_in_collection(fig: &Figure) -> CaseResult {
    let arc = Arc::new((0.5, 0.5), 0.5, 1.0)
        .between(0.0, 60.0)
        .with_angle(20.0);
    let arc = CollectionPatch::new(&arc, PatchProps::default());
    let props = CollectionProps::default().facecolor("none").edgecolor("k");
    fig.subplot()
        .add_collection(CollectionSpec::patches(&[arc], false)?.with_props(props));
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn modifying_arc(fig: &Figure) -> CaseResult {
    let arc = Arc::new((0.5, 0.5), 1.5, 1.0)
        .between(0.0, 60.0)
        .with_angle(10.0);
    let patch = fig.subplot().add_patch(arc);
    patch.update_shape(|arc| {
        arc.width = 0.5;
        arc.angle = 20.0;
    });
    Ok(())
}
