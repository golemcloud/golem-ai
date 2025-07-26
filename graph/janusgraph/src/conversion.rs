use crate::client::GremlinResponse;
use golem_graph::golem::graph::types::*;
use golem_graph::golem::graph::errors::GraphError;
use golem_graph::golem::graph::schema::PropertyType;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use base64::Engine;

/// Convert  PropertyMap to Gremlin bindings
pub fn property_map_to_gremlin_bindings(
    properties: &PropertyMap
) -> Result<HashMap<String, JsonValue>, GraphError> {
    let mut bindings = HashMap::new();

    for (key, value) in properties {
        let json_value = property_value_to_json(value)?;
        bindings.insert(key.clone(), json_value);
    }

    Ok(bindings)
}

/// Convert JSON to  Vertex
fn json_to_vertex(value: &JsonValue) -> Result<Vertex, GraphError> {
    let obj = value
        .as_object()
        .ok_or_else(|| GraphError::InternalError("Expected object for vertex".to_string()))?;

    let id = obj
        .get("id")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(ElementId::Int64(i))
            } else if let Some(s) = v.as_str() {
                Some(ElementId::StringValue(s.to_string()))
            } else {
                None
            }
        })
        .ok_or_else(|| GraphError::InternalError("Missing or invalid vertex ID".to_string()))?;

    let vertex_type = obj
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let mut properties = Vec::new();
    let mut additional_labels = Vec::new();

    for (key, value) in obj {
        if key.starts_with("additional_label_") {
            if let Some(label) = value.as_str() {
                additional_labels.push(label.to_string());
            }
        } else if key != "id" && key != "label" {
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

/// Convert JSON to  Edge
fn json_to_edge(value: &JsonValue) -> Result<Edge, GraphError> {
    let obj = value
        .as_object()
        .ok_or_else(|| GraphError::InternalError("Expected object for edge".to_string()))?;

    let id = obj
        .get("id")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(ElementId::Int64(i))
            } else if let Some(s) = v.as_str() {
                Some(ElementId::StringValue(s.to_string()))
            } else {
                None
            }
        })
        .ok_or_else(|| GraphError::InternalError("Missing or invalid edge ID".to_string()))?;

    let edge_type = obj
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let from_vertex = obj
        .get("outV")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(ElementId::Int64(i))
            } else if let Some(s) = v.as_str() {
                Some(ElementId::StringValue(s.to_string()))
            } else {
                None
            }
        })
        .ok_or_else(|| GraphError::InternalError("Missing or invalid outV vertex ID".to_string()))?;

    let to_vertex = obj
        .get("inV")
        .and_then(|v| {
            if let Some(i) = v.as_i64() {
                Some(ElementId::Int64(i))
            } else if let Some(s) = v.as_str() {
                Some(ElementId::StringValue(s.to_string()))
            } else {
                None
            }
        })
        .ok_or_else(|| GraphError::InternalError("Missing or invalid inV vertex ID".to_string()))?;

    let mut properties = Vec::new();

    for (key, value) in obj {
        if key != "id" && key != "label" && key != "outV" && key != "inV" {
            let prop_value = json_to_property_value(value)?;
            properties.push((key.clone(), prop_value));
        }
    }

    Ok(Edge {
        id,
        edge_type,
        from_vertex,
        to_vertex,
        properties,
    })
}

// Helper functions for parsing temporal and geospatial types
fn parse_date(s: &str) -> Result<Date, GraphError> {
    // Simple date parsing for YYYY-MM-DD format
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 3 {
        let year = parts[0]
            .parse::<u32>()
            .map_err(|_| GraphError::InvalidPropertyType("Invalid year".to_string()))?;
        let month = parts[1]
            .parse::<u8>()
            .map_err(|_| GraphError::InvalidPropertyType("Invalid month".to_string()))?;
        let day = parts[2]
            .parse::<u8>()
            .map_err(|_| GraphError::InvalidPropertyType("Invalid day".to_string()))?;

        Ok(Date { year, month, day })
    } else {
        Err(GraphError::InvalidPropertyType("Invalid date format".to_string()))
    }
}

fn parse_time(s: &str) -> Result<Time, GraphError> {
    // Simple time parsing for HH:MM:SS format
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() >= 2 {
        let hour = parts[0]
            .parse::<u8>()
            .map_err(|_| GraphError::InvalidPropertyType("Invalid hour".to_string()))?;
        let minute = parts[1]
            .parse::<u8>()
            .map_err(|_| GraphError::InvalidPropertyType("Invalid minute".to_string()))?;
        let second = parts.get(2).unwrap_or(&"0").parse::<u8>().unwrap_or(0);
        let nanosecond = 0; // Default to 0 for simplicity

        Ok(Time { hour, minute, second, nanosecond })
    } else {
        Err(GraphError::InvalidPropertyType("Invalid time format".to_string()))
    }
}

fn parse_datetime(s: &str) -> Result<Datetime, GraphError> {
    // Simple datetime parsing for YYYY-MM-DDTHH:MM:SS format
    if let Some(t_pos) = s.find('T') {
        let date_part = &s[..t_pos];
        let time_part = &s[t_pos + 1..];

        let date = parse_date(date_part)?;
        let time = parse_time(time_part)?;

        Ok(Datetime {
            date,
            time,
            timezone_offset_minutes: None, // Default to None for simplicity
        })
    } else {
        Err(GraphError::InvalidPropertyType("Invalid datetime format".to_string()))
    }
}

fn parse_duration(s: &str) -> Result<Duration, GraphError> {
    // Simple duration parsing for PTnS format (ISO 8601)
    if s.starts_with("PT") && s.ends_with('S') {
        if let Ok(seconds) = s[2..s.len() - 1].parse::<i64>() {
            Ok(Duration { seconds, nanoseconds: 0 })
        } else {
            Err(GraphError::InvalidPropertyType("Invalid duration format".to_string()))
        }
    } else {
        Err(GraphError::InvalidPropertyType("Invalid duration format".to_string()))
    }
}

fn parse_point(s: &str) -> Result<Point, GraphError> {
    // Simple point parsing for "longitude,latitude" format
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() >= 2 {
        let longitude = parts[0]
            .parse::<f64>()
            .map_err(|_| GraphError::InvalidPropertyType("Invalid longitude".to_string()))?;
        let latitude = parts[1]
            .parse::<f64>()
            .map_err(|_| GraphError::InvalidPropertyType("Invalid latitude".to_string()))?;
        let altitude = parts.get(2).and_then(|a| a.parse::<f64>().ok());

        Ok(Point { longitude, latitude, altitude })
    } else {
        Err(GraphError::InvalidPropertyType("Invalid point format".to_string()))
    }
}

/// Convert  PropertyValue to JSON
pub fn property_value_to_json(value: &PropertyValue) -> Result<JsonValue, GraphError> {
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
        PropertyValue::Float32Value(f) => {
            serde_json::Number
                ::from_f64(*f as f64)
                .map(JsonValue::Number)
                .ok_or_else(|| GraphError::InternalError("Invalid float value".to_string()))
        }
        PropertyValue::Float64Value(f) => {
            serde_json::Number
                ::from_f64(*f)
                .map(JsonValue::Number)
                .ok_or_else(|| GraphError::InternalError("Invalid float value".to_string()))
        }
        PropertyValue::Boolean(b) => Ok(JsonValue::Bool(*b)),
        PropertyValue::Bytes(b) =>
            Ok(JsonValue::String(base64::engine::general_purpose::STANDARD.encode(b))),
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
        PropertyValue::Linestring(_) => {
            Err(
                GraphError::UnsupportedOperation(
                    "Linestring not supported in JanusGraph".to_string()
                )
            )
        }
        PropertyValue::Polygon(_) => {
            Err(GraphError::UnsupportedOperation("Polygon not supported in JanusGraph".to_string()))
        }
        PropertyValue::NullValue => Ok(JsonValue::Null),
    }
}

/// Convert JSON to  PropertyValue
pub fn json_to_property_value(value: &JsonValue) -> Result<PropertyValue, GraphError> {
    match value {
        JsonValue::String(s) => {
            // Try to parse as different types
            if let Ok(date) = parse_date(s) {
                Ok(PropertyValue::Date(date))
            } else if let Ok(time) = parse_time(s) {
                Ok(PropertyValue::Time(time))
            } else if let Ok(datetime) = parse_datetime(s) {
                Ok(PropertyValue::Datetime(datetime))
            } else if let Ok(duration) = parse_duration(s) {
                Ok(PropertyValue::Duration(duration))
            } else if let Ok(point) = parse_point(s) {
                Ok(PropertyValue::Point(point))
            } else {
                Ok(PropertyValue::StringValue(s.clone()))
            }
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
                Err(GraphError::InvalidPropertyType("Invalid number".to_string()))
            }
        }
        JsonValue::Bool(b) => Ok(PropertyValue::Boolean(*b)),
        JsonValue::Array(_) => {
            Err(GraphError::InvalidPropertyType("Arrays not supported".to_string()))
        }
        JsonValue::Object(_) => {
            Err(GraphError::InvalidPropertyType("Objects not supported".to_string()))
        }
        JsonValue::Null => Ok(PropertyValue::NullValue),
    }
}

/// Convert  ElementId to string for Gremlin queries
pub fn element_id_to_string(id: &ElementId) -> String {
    match id {
        ElementId::StringValue(s) => s.clone(),
        ElementId::Int64(i) => i.to_string(),
        ElementId::Uuid(u) => u.clone(),
    }
}

/// Parse vertices from Gremlin response
pub fn parse_vertices_from_response(response: &GremlinResponse) -> Result<Vec<Vertex>, GraphError> {
    if response.status.code != 200 {
        return Err(GraphError::InvalidQuery(response.status.message.clone()));
    }

    let mut vertices = Vec::new();
    for value in &response.result.data {
        let vertex = json_to_vertex(value)?;
        vertices.push(vertex);
    }

    Ok(vertices)
}

/// Parse edges from Gremlin response
pub fn parse_edges_from_response(response: &GremlinResponse) -> Result<Vec<Edge>, GraphError> {
    if response.status.code != 200 {
        return Err(GraphError::InvalidQuery(response.status.message.clone()));
    }

    let mut edges = Vec::new();
    for value in &response.result.data {
        let edge = json_to_edge(value)?;
        edges.push(edge);
    }

    Ok(edges)
}

/// Parse vertex from Gremlin response
pub fn parse_vertex_from_response(response: &GremlinResponse) -> Result<Vertex, GraphError> {
    if response.status.code != 200 {
        return Err(GraphError::InvalidQuery(response.status.message.clone()));
    }

    let value = response.result.data
        .first()
        .ok_or_else(|| GraphError::InvalidQuery("No data in response".to_string()))?;

    json_to_vertex(value)
}

/// Parse edge from Gremlin response
pub fn parse_edge_from_response(response: &GremlinResponse) -> Result<Edge, GraphError> {
    if response.status.code != 200 {
        return Err(GraphError::InvalidQuery(response.status.message.clone()));
    }

    let value = response.result.data
        .first()
        .ok_or_else(|| GraphError::InvalidQuery("No data in response".to_string()))?;

    json_to_edge(value)
}

/// Parse string list from Gremlin response
pub fn parse_string_list_from_response(
    response: &GremlinResponse
) -> Result<Vec<String>, GraphError> {
    if response.status.code != 200 {
        return Err(GraphError::InvalidQuery(response.status.message.clone()));
    }

    let mut strings = Vec::new();
    for value in &response.result.data {
        if let Some(s) = value.as_str() {
            strings.push(s.to_string());
        }
    }

    Ok(strings)
}

/// Convert  PropertyType to Gremlin data type string
pub fn property_type_to_gremlin_type(property_type: &PropertyType) -> String {
    match property_type {
        PropertyType::StringType => "String.class".to_string(),
        PropertyType::Int32 => "Integer.class".to_string(),
        PropertyType::Int64 => "Long.class".to_string(),
        PropertyType::Float32Type => "Float.class".to_string(),
        PropertyType::Float64Type => "Double.class".to_string(),
        PropertyType::Boolean => "Boolean.class".to_string(),
        PropertyType::Bytes => "String.class".to_string(), // Store as base64 string
        PropertyType::Date => "String.class".to_string(), // Store as ISO date string
        PropertyType::Datetime => "String.class".to_string(), // Store as ISO datetime string
        PropertyType::Point => "String.class".to_string(), // Store as WKT point string
        PropertyType::ListType => "String.class".to_string(), // Store as JSON string
        PropertyType::MapType => "String.class".to_string(), // Store as JSON string
    }
}
