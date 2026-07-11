//! Coordinate frames. "Coordinates are data, not code": the site's
//! CRS and origin live in a serialized [`GeoRef`], never in constants.

use serde::{Deserialize, Serialize};

/// A point in scene-local meters: x = east, y = north, z = up (meters
/// above the site's vertical datum).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalPoint {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub z: f64,
}

/// Georeferencing anchor, serialized as `georef.json`.
///
/// Field names intentionally match mazzap/VEIL's `georef.json` so the
/// two systems can share a frame for the same property.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoRef {
    /// Projected working CRS, e.g. "EPSG:26918".
    pub analysis_crs: String,
    /// proj4 string for browser-side conversion.
    pub proj4: String,
    /// Geographic CRS lon/lat is expressed in, e.g. "EPSG:4269".
    pub geographic_crs: String,
    /// (easting, northing) scene origin in the projected CRS.
    pub origin_utm: (f64, f64),
}

/// The site frame: georef plus indoor floor levels. Floors share the
/// site's x/y; each floor pins a z datum (meters above origin ground).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteFrame {
    pub georef: GeoRef,
    #[serde(default)]
    pub floors: Vec<Floor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Floor {
    /// Stable id, e.g. "basement", "main", "upstairs".
    pub id: String,
    pub name: String,
    /// Floor z datum in scene-local meters.
    pub z: f64,
    /// Ceiling height in meters, if known.
    pub height_m: Option<f64>,
}
