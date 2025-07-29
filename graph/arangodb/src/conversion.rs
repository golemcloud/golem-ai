use crate::client::ArangoResponse;
use base64::Engine;
use golem_graph::exports::golem::graph::errors::GraphError;
use golem_graph::exports::golem::graph::types::*;
use serde_json::Value as JsonValue;

/// Convert PropertyMap to ArangoDB document
pub fn property_map_to_arango_doc(properties: &PropertyMap) -> Result<JsonValue, GraphError> {
    let mut doc = serde_json::Map::new();

    for (key, value) in properties {
        let json_value = property_value_to_json(value)?;
        doc.insert(key.clone(), json_value);
    }

    Ok(JsonValue::Object(doc))
}

/// Convert ArangoDB response to Vertex
pub fn parse_vertex_from_response(response: &ArangoResponse) -> Result<Vertex, GraphError> {
    if response.error {
        let error_msg = response
            .error_message
            .as_ref()
            .unwrap_or(&"Unknown error".to_string())
            .clone();
        return Err(GraphError::InvalidQuery(error_msg));
    }

    if response.result.is_empty() {
        return Err(GraphError::InvalidQuery(
            "No vertex data in response".to_string(),
        ));
    }

    let value = &response.result[0];
    json_to_vertex(value)
}

/// Convert ArangoDB response to  Vertices
pub fn parse_vertices_from_response(response: &ArangoResponse) -> Result<Vec<Vertex>, GraphError> {
    if response.error {
        let error_msg = response
            .error_message
            .as_ref()
            .unwrap_or(&"Unknown error".to_string())
            .clone();
        return Err(GraphError::InvalidQuery(error_msg));
    }

    let mut vertices = Vec::new();
    for value in &response.result {
        let vertex = json_to_vertex(value)?;
        vertices.push(vertex);
    }

    Ok(vertices)
}

/// Convert ArangoDB response to  Edge
pub fn parse_edge_from_response(response: &ArangoResponse) -> Result<Edge, GraphError> {
    if response.error {
        let error_msg = response
            .error_message
            .as_ref()
            .unwrap_or(&"Unknown error".to_string())
            .clone();
        return Err(GraphError::InvalidQuery(error_msg));
    }

    if response.result.is_empty() {
        return Err(GraphError::InvalidQuery(
            "No edge data in response".to_string(),
        ));
    }

    let value = &response.result[0];
    json_to_edge(value)
}

/// Convert ArangoDB response to Edges
pub fn parse_edges_from_response(response: &ArangoResponse) -> Result<Vec<Edge>, GraphError> {
    if response.error {
        let error_msg = response
            .error_message
            .as_ref()
            .unwrap_or(&"Unknown error".to_string())
            .clone();
        return Err(GraphError::InvalidQuery(error_msg));
    }

    let mut edges = Vec::new();
    for value in &response.result {
        let edge = json_to_edge(value)?;
        edges.push(edge);
    }

    Ok(edges)
}

/// Convert ArangoDB response to string list
pub fn _parse_string_list_from_response(
    response: &ArangoResponse,
) -> Result<Vec<String>, GraphError> {
    if response.error {
        let error_msg = response
            .error_message
            .as_ref()
            .unwrap_or(&"Unknown error".to_string())
            .clone();
        return Err(GraphError::InvalidQuery(error_msg));
    }

    let mut strings = Vec::new();
    for value in &response.result {
        if let Some(s) = value.as_str() {
            strings.push(s.to_string());
        }
    }

    Ok(strings)
}

/// Convert JSON to Vertex
fn json_to_vertex(value: &JsonValue) -> Result<Vertex, GraphError> {
    let obj = value
        .as_object()
        .ok_or_else(|| GraphError::InternalError("Expected object for vertex".to_string()))?;

    let id_value = obj
        .get("_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GraphError::InternalError("Missing vertex _id".to_string()))?;
    let id = ElementId::StringValue(id_value.to_string());

    let vertex_type = obj
        .get("_vertex_type")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let additional_labels = obj
        .get("_additional_labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let mut properties = Vec::new();
    for (key, value) in obj {
        if !key.starts_with('_') {
            let prop_value = json_to_property_value(value)?;
            properties.push((key.clone(), prop_value));
        }
    }

    Ok(Vertex {
        id,
        vertex_type,
        additional_labels,
        properties,
    })
}

/// Convert JSON to Edge
fn json_to_edge(value: &JsonValue) -> Result<Edge, GraphError> {
    let obj = value
        .as_object()
        .ok_or_else(|| GraphError::InternalError("Expected object for edge".to_string()))?;

    let id_value = obj
        .get("_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GraphError::InternalError("Missing edge _id".to_string()))?;
    let id = ElementId::StringValue(id_value.to_string());

    let edge_type = obj
        .get("_edge_type")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let from_vertex = obj
        .get("_from")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GraphError::InternalError("Missing edge _from".to_string()))?;
    let from_id = ElementId::StringValue(from_vertex.to_string());

    let to_vertex = obj
        .get("_to")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GraphError::InternalError("Missing edge _to".to_string()))?;
    let to_id = ElementId::StringValue(to_vertex.to_string());

    let mut properties = Vec::new();
    for (key, value) in obj {
        if !key.starts_with('_') {
            let prop_value = json_to_property_value(value)?;
            properties.push((key.clone(), prop_value));
        }
    }

    Ok(Edge {
        id,
        edge_type,
        from_vertex: from_id,
        to_vertex: to_id,
        properties,
    })
}

/// Convert PropertyValue to JSON
fn property_value_to_json(value: &PropertyValue) -> Result<JsonValue, GraphError> {
    match value {
        PropertyValue::StringValue(s) => Ok(JsonValue::String(s.clone())),
        PropertyValue::Int8(i) => Ok(JsonValue::Number(serde_json::Number::from(*i))),
        PropertyValue::Int16(i) => Ok(JsonValue::Number(serde_json::Number::from(*i))),
        PropertyValue::Int32(i) => Ok(JsonValue::Number(serde_json::Number::from(*i))),
        PropertyValue::Int64(i) => Ok(JsonValue::Number(serde_json::Number::from(*i))),
        PropertyValue::Uint8(u) => Ok(JsonValue::Number(serde_json::Number::from(*u))),
        PropertyValue::Uint16(u) => Ok(JsonValue::Number(serde_json::Number::from(*u))),
        PropertyValue::Uint32(u) => Ok(JsonValue::Number(serde_json::Number::from(*u))),
        PropertyValue::Uint64(u) => Ok(JsonValue::Number(serde_json::Number::from(*u))),
        PropertyValue::Float32Value(f) => serde_json::Number::from_f64(*f as f64)
            .map(JsonValue::Number)
            .ok_or_else(|| GraphError::InternalError("Invalid float value".to_string())),
        PropertyValue::Float64Value(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .ok_or_else(|| GraphError::InternalError("Invalid float value".to_string())),
        PropertyValue::Boolean(b) => Ok(JsonValue::Bool(*b)),
        PropertyValue::Bytes(b) => Ok(JsonValue::String(
            base64::engine::general_purpose::STANDARD.encode(b),
        )),
        PropertyValue::Date(date) => {
            let date_str = format!("{}-{:02}-{:02}", date.year, date.month, date.day);
            Ok(JsonValue::String(date_str))
        }
        PropertyValue::Time(time) => {
            let time_str = format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second);
            Ok(JsonValue::String(time_str))
        }
        PropertyValue::Datetime(datetime) => {
            let datetime_str = format!(
                "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                datetime.date.year,
                datetime.date.month,
                datetime.date.day,
                datetime.time.hour,
                datetime.time.minute,
                datetime.time.second
            );
            Ok(JsonValue::String(datetime_str))
        }
        PropertyValue::Duration(duration) => {
            let duration_str = format!("PT{}S", duration.seconds);
            Ok(JsonValue::String(duration_str))
        }
        PropertyValue::Point(point) => {
            let point_str = format!("POINT({} {})", point.longitude, point.latitude);
            Ok(JsonValue::String(point_str))
        }
        PropertyValue::Linestring(linestring) => {
            let coordinates: Vec<Vec<f64>> = linestring
                .coordinates
                .iter()
                .map(|p| vec![p.longitude, p.latitude])
                .collect();
            let line_obj = serde_json::json!({
                "type": "LineString",
                "coordinates": coordinates
            });
            Ok(line_obj)
        }
        PropertyValue::Polygon(polygon) => {
            let exterior: Vec<Vec<f64>> = polygon
                .exterior
                .iter()
                .map(|p| vec![p.longitude, p.latitude])
                .collect();
            let holes = polygon.holes.as_ref().map(|holes| {
                holes
                    .iter()
                    .map(|hole| {
                        hole.iter()
                            .map(|p| vec![p.longitude, p.latitude])
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            });
            let polygon_obj = serde_json::json!({
                "type": "Polygon",
                "coordinates": [exterior, holes.unwrap_or_default()]
            });
            Ok(polygon_obj)
        }
        PropertyValue::NullValue => Ok(JsonValue::Null),
    }
}

/// Convert JSON to  PropertyValue
pub fn json_to_property_value(value: &JsonValue) -> Result<PropertyValue, GraphError> {
    match value {
        JsonValue::String(s) => {
            // Try to parse as date/time types first
            if let Ok(date) = parse_date(s) {
                return Ok(PropertyValue::Date(date));
            }
            if let Ok(time) = parse_time(s) {
                return Ok(PropertyValue::Time(time));
            }
            if let Ok(datetime) = parse_datetime(s) {
                return Ok(PropertyValue::Datetime(datetime));
            }
            if let Ok(duration) = parse_duration(s) {
                return Ok(PropertyValue::Duration(duration));
            }
            if let Ok(point) = parse_point(s) {
                return Ok(PropertyValue::Point(point));
            }
            Ok(PropertyValue::StringValue(s.clone()))
        }
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= (i8::MIN as i64) && i <= (i8::MAX as i64) {
                    Ok(PropertyValue::Int8(i as i8))
                } else if i >= (i16::MIN as i64) && i <= (i16::MAX as i64) {
                    Ok(PropertyValue::Int16(i as i16))
                } else if i >= (i32::MIN as i64) && i <= (i32::MAX as i64) {
                    Ok(PropertyValue::Int32(i as i32))
                } else {
                    Ok(PropertyValue::Int64(i))
                }
            } else if let Some(u) = n.as_u64() {
                if u <= (u8::MAX as u64) {
                    Ok(PropertyValue::Uint8(u as u8))
                } else if u <= (u16::MAX as u64) {
                    Ok(PropertyValue::Uint16(u as u16))
                } else if u <= (u32::MAX as u64) {
                    Ok(PropertyValue::Uint32(u as u32))
                } else {
                    Ok(PropertyValue::Uint64(u))
                }
            } else if let Some(f) = n.as_f64() {
                Ok(PropertyValue::Float64Value(f))
            } else {
                Err(GraphError::InternalError(
                    "Invalid number value".to_string(),
                ))
            }
        }
        JsonValue::Bool(b) => Ok(PropertyValue::Boolean(*b)),
        JsonValue::Array(_) => {
            // Arrays not supported in current PropertyValue enum
            Err(GraphError::InternalError(
                "Arrays not supported".to_string(),
            ))
        }
        JsonValue::Object(obj) => {
            // Check if it's a GeoJSON object
            if let Some(geo_type) = obj.get("type").and_then(|t| t.as_str()) {
                match geo_type {
                    "Point" => {
                        if let Some(coords) = obj.get("coordinates").and_then(|c| c.as_array()) {
                            if coords.len() >= 2 {
                                let longitude = coords[0].as_f64().unwrap_or(0.0);
                                let latitude = coords[1].as_f64().unwrap_or(0.0);
                                let altitude = coords.get(2).and_then(|a| a.as_f64());
                                return Ok(PropertyValue::Point(Point {
                                    longitude,
                                    latitude,
                                    altitude,
                                }));
                            }
                        }
                    }
                    "LineString" => {
                        if let Some(coords) = obj.get("coordinates").and_then(|c| c.as_array()) {
                            let coordinates: Result<Vec<Point>, _> = coords
                                .iter()
                                .map(|coord| {
                                    if let Some(coord_arr) = coord.as_array() {
                                        if coord_arr.len() >= 2 {
                                            Ok(Point {
                                                longitude: coord_arr[0].as_f64().unwrap_or(0.0),
                                                latitude: coord_arr[1].as_f64().unwrap_or(0.0),
                                                altitude: coord_arr.get(2).and_then(|a| a.as_f64()),
                                            })
                                        } else {
                                            Err(GraphError::InternalError(
                                                "Invalid coordinate".to_string(),
                                            ))
                                        }
                                    } else {
                                        Err(GraphError::InternalError(
                                            "Invalid coordinate".to_string(),
                                        ))
                                    }
                                })
                                .collect();
                            return Ok(PropertyValue::Linestring(Linestring {
                                coordinates: coordinates?,
                            }));
                        }
                    }
                    "Polygon" => {
                        if let Some(coords) = obj.get("coordinates").and_then(|c| c.as_array()) {
                            if let Some(exterior) = coords.first().and_then(|e| e.as_array()) {
                                let exterior_points: Result<Vec<Point>, _> = exterior
                                    .iter()
                                    .map(|coord| {
                                        if let Some(coord_arr) = coord.as_array() {
                                            if coord_arr.len() >= 2 {
                                                Ok(Point {
                                                    longitude: coord_arr[0].as_f64().unwrap_or(0.0),
                                                    latitude: coord_arr[1].as_f64().unwrap_or(0.0),
                                                    altitude: coord_arr
                                                        .get(2)
                                                        .and_then(|a| a.as_f64()),
                                                })
                                            } else {
                                                Err(GraphError::InternalError(
                                                    "Invalid coordinate".to_string(),
                                                ))
                                            }
                                        } else {
                                            Err(GraphError::InternalError(
                                                "Invalid coordinate".to_string(),
                                            ))
                                        }
                                    })
                                    .collect();

                                let holes = if coords.len() > 1 {
                                    let holes_result: Result<Vec<Vec<Point>>, _> = coords
                                        .iter()
                                        .skip(1)
                                        .map(|hole| {
                                            if let Some(hole_arr) = hole.as_array() {
                                                let hole_points: Result<Vec<Point>, _> = hole_arr
                                                    .iter()
                                                    .map(|coord| {
                                                        if let Some(coord_arr) = coord.as_array() {
                                                            if coord_arr.len() >= 2 {
                                                                Ok(Point {
                                                                    longitude: coord_arr[0]
                                                                        .as_f64()
                                                                        .unwrap_or(0.0),
                                                                    latitude: coord_arr[1]
                                                                        .as_f64()
                                                                        .unwrap_or(0.0),
                                                                    altitude: coord_arr
                                                                        .get(2)
                                                                        .and_then(|a| a.as_f64()),
                                                                })
                                                            } else {
                                                                Err(GraphError::InternalError(
                                                                    "Invalid coordinate"
                                                                        .to_string(),
                                                                ))
                                                            }
                                                        } else {
                                                            Err(GraphError::InternalError(
                                                                "Invalid coordinate".to_string(),
                                                            ))
                                                        }
                                                    })
                                                    .collect();
                                                hole_points
                                            } else {
                                                Err(GraphError::InternalError(
                                                    "Invalid hole".to_string(),
                                                ))
                                            }
                                        })
                                        .collect();
                                    Some(holes_result?)
                                } else {
                                    None
                                };

                                return Ok(PropertyValue::Polygon(Polygon {
                                    exterior: exterior_points?,
                                    holes,
                                }));
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Regular object not supported in current PropertyValue enum
            Err(GraphError::InternalError(
                "Objects not supported".to_string(),
            ))
        }
        JsonValue::Null => Ok(PropertyValue::NullValue),
    }
}

/// Convert ElementId to string
pub fn _element_id_to_string(id: &ElementId) -> String {
    match id {
        ElementId::StringValue(s) => s.clone(),
        ElementId::Int64(i) => i.to_string(),
        ElementId::Uuid(u) => u.clone(),
    }
}

/// Parse date string
fn parse_date(s: &str) -> Result<Date, GraphError> {
    // Simple date parsing for YYYY-MM-DD format
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 3 {
        let year = parts[0]
            .parse::<u32>()
            .map_err(|_| GraphError::InternalError("Invalid year".to_string()))?;
        let month = parts[1]
            .parse::<u8>()
            .map_err(|_| GraphError::InternalError("Invalid month".to_string()))?;
        let day = parts[2]
            .parse::<u8>()
            .map_err(|_| GraphError::InternalError("Invalid day".to_string()))?;

        Ok(Date { year, month, day })
    } else {
        Err(GraphError::InternalError("Invalid date format".to_string()))
    }
}

/// Parse time string
fn parse_time(s: &str) -> Result<Time, GraphError> {
    // Simple time parsing for HH:MM:SS format
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        let hour = parts[0]
            .parse::<u8>()
            .map_err(|_| GraphError::InternalError("Invalid hour".to_string()))?;
        let minute = parts[1]
            .parse::<u8>()
            .map_err(|_| GraphError::InternalError("Invalid minute".to_string()))?;
        let second = parts[2]
            .parse::<u8>()
            .map_err(|_| GraphError::InternalError("Invalid second".to_string()))?;

        Ok(Time {
            hour,
            minute,
            second,
            nanosecond: 0,
        })
    } else {
        Err(GraphError::InternalError("Invalid time format".to_string()))
    }
}

/// Parse datetime string
fn parse_datetime(s: &str) -> Result<Datetime, GraphError> {
    // Simple datetime parsing for YYYY-MM-DDTHH:MM:SSZ format
    if let Some(t_index) = s.find('T') {
        let date_part = &s[..t_index];
        let time_part = &s[t_index + 1..s.len() - 1]; // Remove 'T' and 'Z'

        let date = parse_date(date_part)?;
        let time = parse_time(time_part)?;

        Ok(Datetime {
            date,
            time,
            timezone_offset_minutes: None,
        })
    } else {
        Err(GraphError::InternalError(
            "Invalid datetime format".to_string(),
        ))
    }
}

/// Parse duration string
fn parse_duration(s: &str) -> Result<Duration, GraphError> {
    // Simple duration parsing for PTnS format (ISO 8601)
    if s.starts_with("PT") && s.ends_with('S') {
        if let Ok(seconds) = s[2..s.len() - 1].parse::<i64>() {
            Ok(Duration {
                seconds,
                nanoseconds: 0,
            })
        } else {
            Err(GraphError::InternalError(
                "Invalid duration format".to_string(),
            ))
        }
    } else {
        Err(GraphError::InternalError(
            "Invalid duration format".to_string(),
        ))
    }
}

/// Parse point string
fn parse_point(s: &str) -> Result<Point, GraphError> {
    // Simple point parsing for POINT(lon lat) format
    if s.starts_with("POINT(") && s.ends_with(')') {
        let coords = &s[6..s.len() - 1]; // Remove "POINT(" and ")"
        let parts: Vec<&str> = coords.split_whitespace().collect();
        if parts.len() == 2 {
            let longitude = parts[0]
                .parse::<f64>()
                .map_err(|_| GraphError::InternalError("Invalid longitude".to_string()))?;
            let latitude = parts[1]
                .parse::<f64>()
                .map_err(|_| GraphError::InternalError("Invalid latitude".to_string()))?;

            Ok(Point {
                longitude,
                latitude,
                altitude: None,
            })
        } else {
            Err(GraphError::InternalError(
                "Invalid point format".to_string(),
            ))
        }
    } else {
        Err(GraphError::InternalError(
            "Invalid point format".to_string(),
        ))
    }
}
