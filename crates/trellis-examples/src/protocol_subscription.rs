//! Wrapper-owned protocol subscription proof.
//!
//! The public API in this module is intentionally application-shaped. It
//! exposes typed session handles, subscription effects, and feed frames while
//! Trellis remains a private reconciliation engine.

mod engine;
mod shape;
mod types;

pub use engine::ArticleFeedApp;
pub use types::{
    ArticleFeedFrame, ArticleFeedHandle, ArticleFeedParams, ArticleRow, LiveSubscription,
    SubscriptionEffect, SubscriptionTarget,
};

#[cfg(test)]
mod tests;
