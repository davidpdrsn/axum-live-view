use crate::{
    event_data::EventData, html::Html, life_cycle::SelfHandle, live_view::Updated, LiveView,
};
use async_trait::async_trait;
use axum::http::{HeaderMap, Uri};
use serde::{Deserialize, Serialize};

pub fn combine<V, F>(views: V, render: F) -> Combine<V, F> {
    Combine { views, render }
}

#[allow(missing_debug_implementations)]
pub struct Combine<V, F> {
    views: V,
    render: F,
}

include!(concat!(env!("OUT_DIR"), "/combine_impls.rs"));
