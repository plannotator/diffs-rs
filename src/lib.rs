pub mod app;
mod cleanup;
pub mod engine;
pub mod geometry;
pub mod highlight;
pub mod ingest;
pub mod input;
pub mod model;
pub mod projection;
pub mod reading;
pub mod render;
pub mod source;
mod source_spans;
pub mod storage;
pub mod text;
pub mod theme;
mod tree;
pub mod view;
mod wake;

#[cfg(feature = "regex-census")]
mod regex_census;

#[cfg(feature = "owner-census")]
pub mod owner_census;

#[cfg(feature = "delivery-trace")]
pub mod delivery_trace;

#[macro_export]
macro_rules! delivery_event {
    ($kind:literal, $a:expr, $b:expr) => {
        #[cfg(feature = "delivery-trace")]
        $crate::delivery_trace::record($kind, $a as u64, $b as u64);
    };
}
