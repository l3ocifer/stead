# Contract: `georef.json`

**Frozen.** Field names intentionally match
[mazzap/VEIL](https://github.com/zymazza/mazzap)'s `georef.json` so a
stead site and a mazzap land twin of the same property share one
coordinate frame.

```json
{
  "analysis_crs": "EPSG:26918",
  "proj4": "+proj=utm +zone=18 +datum=NAD83 +units=m +no_defs",
  "geographic_crs": "EPSG:4326",
  "origin_utm": [322500.0, 4308000.0]
}
```

| Field | Meaning |
|---|---|
| `analysis_crs` | Projected working CRS (EPSG code string) |
| `proj4` | proj4 string for client-side conversion (viewers) |
| `geographic_crs` | CRS that lat/lon outputs are expressed in |
| `origin_utm` | `[easting, northing]` scene origin in `analysis_crs` |

Scene-local coordinates are meters offset from `origin_utm`:
`x = easting − origin.0` (east positive), `y = northing − origin.1`
(north positive), `z` = meters above the site's vertical datum.

`georef.json` lives in the **site directory** (private, per-home data)
— never in this repository.
