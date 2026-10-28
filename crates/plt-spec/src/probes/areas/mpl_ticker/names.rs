//! matplotlib's class names of the tick locators and formatters that
//! probes report (`type(ticker).__name__`).

use std::any::Any;
use std::rc::Rc;

use plt::dates::{AutoDateFormatter, AutoDateLocator, ConciseDateFormatter};
use plt::ticker::{
    AutoLocator, FixedLocator, Formatter, Locator, LogFormatter, LogLabelStyle, LogLocator,
    ScalarFormatter, StrMethodFormatter,
};

use crate::probes::ProbeError;

/// matplotlib's class name of the locators the probes meet.
pub(crate) fn locator_name(locator: &Rc<dyn Locator>) -> Result<&'static str, ProbeError> {
    let any = locator.as_ref() as &dyn Any;
    if any.is::<FixedLocator>() {
        Ok("FixedLocator")
    } else if any.is::<LogLocator>() {
        Ok("LogLocator")
    } else if any.is::<AutoLocator>() {
        Ok("AutoLocator")
    } else if any.is::<AutoDateLocator>() {
        Ok("AutoDateLocator")
    } else {
        Err(ProbeError::rejected(format!(
            "unexpected locator {locator:?}"
        )))
    }
}

/// matplotlib's class name of the formatters the probes meet.
pub(crate) fn formatter_name(formatter: &Rc<dyn Formatter>) -> Result<&'static str, ProbeError> {
    let any = formatter.as_ref() as &dyn Any;
    match any.downcast_ref::<LogFormatter>() {
        Some(log) if log.style() == LogLabelStyle::Plain => Ok("LogFormatter"),
        _ if any.is::<StrMethodFormatter>() => Ok("StrMethodFormatter"),
        _ if any.is::<ScalarFormatter>() => Ok("ScalarFormatter"),
        _ if any.is::<AutoDateFormatter>() => Ok("AutoDateFormatter"),
        _ if any.is::<ConciseDateFormatter>() => Ok("ConciseDateFormatter"),
        _ => Err(ProbeError::rejected(format!(
            "unexpected formatter {formatter:?}"
        ))),
    }
}
