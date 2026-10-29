//! `figures/cases_basic.py`.

use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("basic/empty_axes", |fig| {
        fig.subplot();
        Ok(())
    }),
    FigureCase::new("basic/line", |fig| {
        let ax = fig.subplot();
        ax.plot([1.0, 2.0, 3.0, 4.0], [1.0, 4.0, 2.0, 3.0])?;
        Ok(())
    }),
];
