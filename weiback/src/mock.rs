//! This module provides mock implementations for various core components of the application.
//!
//! These mocks are primarily used in unit and integration tests to isolate the code
//! under test from external dependencies like the Weibo API, file system operations,
//! or complex asynchronous behaviors. They allow for controlled testing scenarios
//! and predictable outcomes.

pub mod api;
pub mod exporter;
pub mod media_downloader;

pub use api::MockApi;
pub use media_downloader::MockMediaDownloader;
