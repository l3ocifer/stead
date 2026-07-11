//! Scene-local geometry and the **region grammar** — the one shape
//! every spatial query accepts (adopted from mazzap so agents can
//! move between a land twin and a home twin without relearning).

use serde::{Deserialize, Serialize};

use crate::frame::LocalPoint;

/// Axis-aligned bounding box in scene-local meters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BBox {
    pub fn contains(&self, p: LocalPoint) -> bool {
        p.x >= self.min_x && p.x <= self.max_x && p.y >= self.min_y && p.y <= self.max_y
    }

    /// The bounding box of a point set; `None` for an empty slice.
    pub fn of(points: &[LocalPoint]) -> Option<BBox> {
        let first = points.first()?;
        let mut b = BBox {
            min_x: first.x,
            min_y: first.y,
            max_x: first.x,
            max_y: first.y,
        };
        for p in &points[1..] {
            b.min_x = b.min_x.min(p.x);
            b.min_y = b.min_y.min(p.y);
            b.max_x = b.max_x.max(p.x);
            b.max_y = b.max_y.max(p.y);
        }
        Some(b)
    }
}

/// Horizontal (x/y) distance in meters.
pub fn distance_2d(a: LocalPoint, b: LocalPoint) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// Ray-casting point-in-polygon test. The ring is auto-closed
/// (first vertex should not be repeated at the end).
pub fn point_in_polygon(p: LocalPoint, ring: &[LocalPoint]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (xi, yi) = (ring[i].x, ring[i].y);
        let (xj, yj) = (ring[j].x, ring[j].y);
        if (yi > p.y) != (yj > p.y) && p.x < (xj - xi) * (p.y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Wire form of a region argument. Exactly one of the shapes must be
/// present:
///
/// ```json
/// {"all": true}
/// {"bbox": [minx, miny, maxx, maxy]}
/// {"within_m": 5.0, "point": {"x": 12.0, "y": -3.5}}
/// {"polygon": [{"x":0,"y":0}, {"x":10,"y":0}, {"x":10,"y":8}]}
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point: Option<LocalPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polygon: Option<Vec<LocalPoint>>,
}

/// A resolved, queryable region.
#[derive(Debug, Clone)]
pub enum Region {
    All,
    BBox(BBox),
    Within { center: LocalPoint, radius_m: f64 },
    Polygon(Vec<LocalPoint>),
}

impl Region {
    pub fn contains(&self, p: LocalPoint) -> bool {
        match self {
            Region::All => true,
            Region::BBox(b) => b.contains(p),
            Region::Within { center, radius_m } => distance_2d(*center, p) <= *radius_m,
            Region::Polygon(ring) => point_in_polygon(p, ring),
        }
    }
}

impl RegionSpec {
    /// Validate that exactly one shape was supplied and resolve it.
    /// Errors are structured guidance, never a stack trace.
    pub fn resolve(self) -> Result<Region, String> {
        let shapes = [
            self.all.is_some(),
            self.bbox.is_some(),
            self.within_m.is_some() || self.point.is_some(),
            self.polygon.is_some(),
        ];
        if shapes.iter().filter(|s| **s).count() != 1 {
            return Err(
                "region must be exactly one of: {\"all\": true} | {\"bbox\": [minx,miny,maxx,maxy]} \
                 | {\"within_m\": r, \"point\": {\"x\",\"y\"}} | {\"polygon\": [{\"x\",\"y\"},…]}"
                    .into(),
            );
        }
        if self.all == Some(true) {
            return Ok(Region::All);
        }
        if let Some([min_x, min_y, max_x, max_y]) = self.bbox {
            if min_x > max_x || min_y > max_y {
                return Err("bbox must be [minx, miny, maxx, maxy] with min <= max".into());
            }
            return Ok(Region::BBox(BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            }));
        }
        if self.within_m.is_some() || self.point.is_some() {
            let (Some(radius_m), Some(center)) = (self.within_m, self.point) else {
                return Err("within_m and point must be supplied together".into());
            };
            if radius_m < 0.0 {
                return Err("within_m must be non-negative".into());
            }
            return Ok(Region::Within { center, radius_m });
        }
        if let Some(ring) = self.polygon {
            if ring.len() < 3 {
                return Err("polygon needs at least 3 vertices".into());
            }
            return Ok(Region::Polygon(ring));
        }
        Err("region {\"all\": false} is not a shape — use {\"all\": true}".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> LocalPoint {
        LocalPoint { x, y, z: 0.0 }
    }

    #[test]
    fn polygon_containment() {
        let ring = vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 8.0), p(0.0, 8.0)];
        assert!(point_in_polygon(p(5.0, 4.0), &ring));
        assert!(!point_in_polygon(p(11.0, 4.0), &ring));
        assert!(!point_in_polygon(p(-0.1, 0.0), &ring));
    }

    #[test]
    fn region_spec_resolution() {
        let r: RegionSpec =
            serde_json::from_str(r#"{"within_m": 5.0, "point": {"x":0,"y":0}}"#).unwrap();
        assert!(r.resolve().unwrap().contains(p(3.0, 4.0)));

        let r: RegionSpec = serde_json::from_str(r#"{"bbox": [0,0,10,10]}"#).unwrap();
        assert!(!r.resolve().unwrap().contains(p(11.0, 5.0)));

        let bad: RegionSpec = serde_json::from_str(r#"{"bbox": [0,0,1,1], "all": true}"#).unwrap();
        assert!(bad.resolve().is_err());

        let empty = RegionSpec::default();
        assert!(empty.resolve().is_err());
    }
}
