//! Custom-painted widgets. Each takes a `Ui` (or painter + rect) and the
//! relevant slice of engine state; none of them mutate the engine directly —
//! interactions are returned to the app as values.

pub mod icons;
pub mod info;
pub mod meters;
pub mod pattern;
pub mod queue;
pub mod viz;
