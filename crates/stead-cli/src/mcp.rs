//! `stead mcp <site>` — an MCP (Model Context Protocol) server over
//! one site, speaking JSON-RPC 2.0 on stdio. Register it with any MCP
//! client (Claude, Cursor, …) and agents can query the home with the
//! same region grammar mazzap uses for land.
//!
//! Hand-rolled on purpose: the protocol subset we need (initialize /
//! tools/list / tools/call / ping) is ~stable and small, and zero new
//! dependencies keeps the CLI light. Revisit the official SDK when we
//! need sampling or resources.

use std::io::{BufRead, Write};
use std::path::Path;

use serde_json::{json, Value};
use stead_core::{
    Journal, JournalEvent, LocalPoint, Observation, Provenance, RegionSpec, SiteState,
};

const PROTOCOL_VERSION: &str = "2024-11-05";

pub fn serve(site: &Path) -> anyhow::Result<()> {
    let journal_dir = site.join("journal");
    let mut state = SiteState::replay(&journal_dir)?;
    let mut journal = Journal::create(&journal_dir, "mcp")?;

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        // Notifications (no id) get no response.
        let Some(id) = id else { continue };
        let result = match method {
            "initialize" => json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "stead", "version": env!("CARGO_PKG_VERSION")},
            }),
            "ping" => json!({}),
            "tools/list" => json!({"tools": tool_catalog()}),
            "tools/call" => {
                let name = msg
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let args = msg
                    .pointer("/params/arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let outcome = call_tool(&mut state, &mut journal, name, args);
                match outcome {
                    Ok(value) => json!({
                        "content": [{"type": "text", "text": value.to_string()}],
                    }),
                    Err(err) => json!({
                        "content": [{"type": "text",
                                     "text": json!({"error": err}).to_string()}],
                        "isError": true,
                    }),
                }
            }
            _ => {
                write_response(
                    &mut stdout,
                    &json!({"jsonrpc": "2.0", "id": id,
                            "error": {"code": -32601,
                                      "message": format!("unknown method: {method}")}}),
                )?;
                continue;
            }
        };
        write_response(
            &mut stdout,
            &json!({"jsonrpc": "2.0", "id": id, "result": result}),
        )?;
    }
    Ok(())
}

fn write_response(stdout: &mut impl Write, value: &Value) -> std::io::Result<()> {
    writeln!(stdout, "{value}")?;
    stdout.flush()
}

fn tool_catalog() -> Value {
    let point = json!({"type": "object", "properties": {
        "x": {"type": "number"}, "y": {"type": "number"}},
        "required": ["x", "y"]});
    let region = json!({"type": "object", "description":
        "Exactly one of: {\"all\":true} | {\"bbox\":[minx,miny,maxx,maxy]} | \
         {\"within_m\":r,\"point\":{x,y}} | {\"polygon\":[{x,y},…]}"});
    json!([
        {"name": "describe_site",
         "description": "Site summary: entity/zone/feature/anchor counts.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "list_zones",
         "description": "All living zones (rooms, garden beds, decks, event spaces) with kind, floor, tags, and expiry.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "zone_at",
         "description": "The innermost zone containing a point (scene-local meters); expired temporary zones are skipped.",
         "inputSchema": {"type": "object", "properties": {
             "x": {"type": "number"}, "y": {"type": "number"},
             "floor": {"type": "string"}},
             "required": ["x", "y"]}},
        {"name": "query_features",
         "description": "Living features (devices, plants, fixtures) inside a region, with their Home Assistant bindings.",
         "inputSchema": {"type": "object",
             "properties": {"region": region}, "required": ["region"]}},
        {"name": "get_entity",
         "description": "One entity's record plus every current attribute with provenance (source, observed_at).",
         "inputSchema": {"type": "object",
             "properties": {"id": {"type": "string"}}, "required": ["id"]}},
        {"name": "list_anchors",
         "description": "Relocalization anchors (QR, fiducial, VPS) with positions and payloads.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "observe",
         "description": "Record an observation on an entity (journaled write with provenance).",
         "inputSchema": {"type": "object", "properties": {
             "entity": {"type": "string"}, "attr": {"type": "string"},
             "value": {}, "source": {"type": "string"}},
             "required": ["entity", "attr", "value"]}},
        {"name": "locate_point",
         "description": "Everything true at a point: containing zone and features within 5 m.",
         "inputSchema": {"type": "object", "properties": {
             "point": point}, "required": ["point"]}},
    ])
}

fn now() -> String {
    humantime::format_rfc3339_seconds(std::time::SystemTime::now()).to_string()
}

fn call_tool(
    state: &mut SiteState,
    journal: &mut Journal,
    name: &str,
    args: Value,
) -> Result<Value, String> {
    match name {
        "describe_site" => Ok(json!(state.summary())),
        "list_zones" => Ok(json!(state.zones().collect::<Vec<_>>())),
        "list_anchors" => Ok(json!(state.anchors().collect::<Vec<_>>())),
        "zone_at" => {
            let point = point_from(&args)?;
            let floor = args.get("floor").and_then(Value::as_str);
            Ok(json!({"zone": state.zone_at(point, floor, Some(&now()))}))
        }
        "query_features" => {
            let spec: RegionSpec =
                serde_json::from_value(args.get("region").cloned().unwrap_or_default())
                    .map_err(|e| e.to_string())?;
            let region = spec.resolve()?;
            Ok(json!(state.features_in(&region)))
        }
        "get_entity" => {
            let id = args.get("id").and_then(Value::as_str).ok_or("missing id")?;
            let entity = state
                .entity(id)
                .ok_or_else(|| format!("unknown entity: {id} (try describe_site / list_zones)"))?;
            Ok(json!({"entity": entity, "attrs": state.attrs(id)}))
        }
        "observe" => {
            let entity = args
                .get("entity")
                .and_then(Value::as_str)
                .ok_or("missing entity")?
                .to_string();
            let attr = args
                .get("attr")
                .and_then(Value::as_str)
                .ok_or("missing attr")?
                .to_string();
            let value = args.get("value").cloned().ok_or("missing value")?;
            let source = args
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or("mcp")
                .to_string();
            let event = JournalEvent::Observe(Observation {
                entity_id: entity.clone(),
                attr: attr.clone(),
                value,
                provenance: Provenance {
                    source,
                    run_id: None,
                    confidence: None,
                    observed_at: now(),
                },
            });
            journal.append(&event).map_err(|e| e.to_string())?;
            state.apply(&event);
            Ok(json!({"observed": {"entity": entity, "attr": attr}}))
        }
        "locate_point" => {
            let point = args
                .get("point")
                .ok_or("missing point")
                .and_then(|p| point_from(p).map_err(|_| "point needs numeric x and y"))?;
            let nearby = RegionSpec {
                within_m: Some(5.0),
                point: Some(point),
                ..Default::default()
            }
            .resolve()?;
            Ok(json!({
                "zone": state.zone_at(point, None, Some(&now())),
                "features_within_5m": state.features_in(&nearby),
            }))
        }
        other => Err(format!(
            "unknown tool: {other}; available: describe_site, list_zones, zone_at, \
             query_features, get_entity, list_anchors, observe, locate_point"
        )),
    }
}

fn point_from(args: &Value) -> Result<LocalPoint, String> {
    let x = args
        .get("x")
        .and_then(Value::as_f64)
        .ok_or("missing numeric x")?;
    let y = args
        .get("y")
        .and_then(Value::as_f64)
        .ok_or("missing numeric y")?;
    Ok(LocalPoint { x, y, z: 0.0 })
}
