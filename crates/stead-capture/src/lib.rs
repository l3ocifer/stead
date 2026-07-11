//! stead-capture: import scans (phone walkthrough exports, LiDAR,
//! photogrammetry meshes from OpenDroneMap) and anchor them to the
//! site frame.
//!
//! v0.1: describes the capture manifest format; import pipelines land
//! next (glTF/GLB and PLY point clouds first).

use serde::{Deserialize, Serialize};

/// A registered capture artifact: one scan session's output, anchored
/// to the site frame with a rigid transform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureManifest {
    /// Stable id, e.g. "walkthrough-2026-07-12".
    pub id: String,
    /// Path to the mesh/point-cloud file, relative to the site dir.
    pub path: String,
    /// "gltf" | "glb" | "ply" | "las" | "laz" | "obj"
    pub format: String,
    /// Row-major 4x4 transform from capture-local into scene-local
    /// meters. Identity means the capture was exported pre-aligned.
    pub transform: [[f64; 4]; 4],
    pub captured_at: String,
    pub notes: Option<String>,
}

impl CaptureManifest {
    pub const IDENTITY: [[f64; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
}
