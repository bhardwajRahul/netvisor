use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 2D unsigned coordinate. Used for node positions and sizes.
/// Element node sizes are computed by the frontend (elkjs); the backend
/// sets `Uxy::default()` for element nodes.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, Hash, ToSchema)]
pub struct Uxy {
    /// Horizontal position.
    pub x: usize,
    /// Vertical position.
    pub y: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, Hash, ToSchema)]
pub struct Ixy {
    /// Horizontal position, which may be negative.
    pub x: isize,
    /// Vertical position, which may be negative.
    pub y: isize,
}
