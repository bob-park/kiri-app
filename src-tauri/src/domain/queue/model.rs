//! Queue types. The engine in `kiri_core` defines them; the domain re-exports them.

pub use kiri_core::{
    model::{Job, NewJob},
    queue::QueueState,
};
