use crate::client::GremlinResponse;
use base64::Engine;
use golem_graph::golem::graph::errors::GraphError;
use golem_graph::golem::graph::schema::PropertyType;
use golem_graph::golem::graph::types::*;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Convert PropertyMap to Gremlin bindings
pub fn property_map_to_gremlin_bindings(
    properties: &PropertyMap,
) -> Result<HashMap<String, JsonValue>, GraphError> {
    let mut bindings = HashMap::new();

    for (key, value) in properties {
        let json_value = property_value_to_json(value)?;
        bindings.insert(key.clone(), json_value);
    }

    Ok(bindings)
}

/// Convert JSON to Vertex
fn json_to_vertex(value: &JsonValue) -> Result<Vertex, GraphError> {
    let obj = value
        .as_object()
        .ok_or_else(|| GraphError::InternalError("Expected object for vertex".to_string()))?;

    // Handle JanusGraph's nested format where the actual vertex data is in @value
    let vertex_obj = if obj.contains_key("@value") {
        obj.get("@value")
            .and_then(|v| v.as_object())
            .ok_or_else(|| GraphError::InternalError("Invalid @value structure".to_string()))?
    } else {
        obj
    };

    // Extract ID - handle JanusGraph's nested ID format
    let id = vertex_obj
        .get("id")
        .and_then(|id_obj| {
            // Handle nested ID format: {"@type": "g:Int64", "@value": 12424}
            if let Some(id_value) = id_obj.get("@value") {
                if let Some(i) = id_value.as_i64() {
                    Some(ElementId::Int64(i))
                } else {
                    id_value
                        .as_str()
                        .map(|s| ElementId::StringValue(s.to_string()))
                }
            } else {
                // Handle direct ID format
                if let Some(i) = id_obj.as_i64() {
                    Some(ElementId::Int64(i))
                } else {
                    id_obj
                        .as_str()
                        .map(|s| ElementId::StringValue(s.to_string()))
                }
            }
        })
        .ok_or_else(|| GraphError::InternalError("Missing or invalid vertex ID".to_string()))?;

    let vertex_type = vertex_obj
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let mut properties = Vec::new();
    let mut additional_labels = Vec::new();

    // Handle properties - JanusGraph stores properties as arrays of VertexProperty objects
    if let Some(props_obj) = vertex_obj.get("properties") {
        if let Some(props_map) = props_obj.as_object() {
            for (prop_name, prop_array) in props_map {
                if let Some(prop_array) = prop_array.as_array() {
                    for prop_item in prop_array {
                        if let Some(prop_obj) = prop_item.as_object() {
                            // Extract the actual property value from the VertexProperty structure
                            if let Some(prop_value_obj) = prop_obj.get("@value") {
                                if let Some(prop_value) = prop_value_obj.as_object() {
                                    if let Some(value_field) = prop_value.get("value") {
                                        let prop_value = json_to_property_value(value_field)?;
                                        properties.push((prop_name.clone(), prop_value));
                                        break; // Take the first property value for this property name
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Handle additional properties that might be directly on the vertex object
    for (key, value) in vertex_obj {
        if key.starts_with("additional_label_") {
            if let Some(label) = value.as_str() {
                additional_labels.push(label.to_string());
            }
        } else if key != "id" && key != "label" && key != "properties" {
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

    // Handle JanusGraph's nested format where the actual edge data is in @value
    let edge_obj = if obj.contains_key("@value") {
        obj.get("@value")
            .and_then(|v| v.as_object())
            .ok_or_else(|| GraphError::InternalError("Invalid @value structure".to_string()))?
    } else {
        obj
    };

    // Extract ID - handle JanusGraph's nested ID format
    let id = edge_obj
        .get("id")
        .and_then(|id_obj| {
            // Handle nested ID format: {"@type": "janusgraph:RelationIdentifier", "@value": {"relationId": "du6-pdc-hed-270w"}}
            if id_obj.is_object() && id_obj.get("@value").is_some() {
                let id_value = id_obj.get("@value").unwrap();
                if id_value.is_object() {
                    // Handle relationId format
                    if let Some(relation_id) = id_value.get("relationId").and_then(|v| v.as_str()) {
                        Some(ElementId::StringValue(relation_id.to_string()))
                    } else {
                        // Fallback to other ID formats
                        if let Some(i) = id_value.as_i64() {
                            Some(ElementId::Int64(i))
                        } else {
                            id_value
                                .as_str()
                                .map(|s| ElementId::StringValue(s.to_string()))
                        }
                    }
                } else {
                    // Handle simple nested value
                    if let Some(i) = id_value.as_i64() {
                        Some(ElementId::Int64(i))
                    } else {
                        id_value
                            .as_str()
                            .map(|s| ElementId::StringValue(s.to_string()))
                    }
                }
            } else {
                // Handle direct ID format
                if let Some(i) = id_obj.as_i64() {
                    Some(ElementId::Int64(i))
                } else {
                    id_obj
                        .as_str()
                        .map(|s| ElementId::StringValue(s.to_string()))
                }
            }
        })
        .ok_or_else(|| GraphError::InternalError("Missing or invalid edge ID".to_string()))?;

    let edge_type = edge_obj
        .get("label")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GraphError::InternalError("Missing edge label".to_string()))?
        .to_string();

    // Extract outV (from vertex)
    let from_vertex = edge_obj
        .get("outV")
        .and_then(|v| {
            if v.is_object() && v.get("@value").is_some() {
                v.get("@value").and_then(|vv| {
                    if let Some(i) = vv.as_i64() {
                        Some(ElementId::Int64(i))
                    } else {
                        vv.as_str().map(|s| ElementId::StringValue(s.to_string()))
                    }
                })
            } else if let Some(i) = v.as_i64() {
                Some(ElementId::Int64(i))
            } else {
                v.as_str().map(|s| ElementId::StringValue(s.to_string()))
            }
        })
        .ok_or_else(|| {
            GraphError::InternalError("Missing or invalid outV vertex ID".to_string())
        })?;

    // Extract inV (to vertex)
    let to_vertex = edge_obj
        .get("inV")
        .and_then(|v| {
            if v.is_object() && v.get("@value").is_some() {
                v.get("@value").and_then(|vv| {
                    if let Some(i) = vv.as_i64() {
                        Some(ElementId::Int64(i))
                    } else {
                        vv.as_str().map(|s| ElementId::StringValue(s.to_string()))
                    }
                })
            } else if let Some(i) = v.as_i64() {
                Some(ElementId::Int64(i))
            } else {
                v.as_str().map(|s| ElementId::StringValue(s.to_string()))
            }
        })
        .ok_or_else(|| GraphError::InternalError("Missing or invalid inV vertex ID".to_string()))?;

    // Parse properties
    let mut properties = PropertyMap::new();
    if let Some(props) = edge_obj.get("properties") {
        if let Some(props_obj) = props.as_object() {
            for (key, value) in props_obj {
                // Handle JanusGraph's nested property format
                let prop_value = if value.is_object() && value.get("@value").is_some() {
                    let prop_obj = value.get("@value").unwrap();
                    if prop_obj.is_object() {
                        // Extract the actual value from the property object
                        if let Some(actual_value) = prop_obj.get("value") {
                            json_to_property_value(actual_value)?
                        } else {
                            json_to_property_value(value)?
                        }
                    } else {
                        json_to_property_value(prop_obj)?
                    }
                } else {
                    json_to_property_value(value)?
                };
                properties.push((key.clone(), prop_value));
            }
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

/// Convert PropertyValue to JSON
pub fn property_value_to_json(value: &PropertyValue) -> Result<JsonValue, GraphError> {
    match value {
        PropertyValue::NullValue => Ok(JsonValue::Null),
        PropertyValue::Boolean(b) => Ok(JsonValue::Bool(*b)),
        PropertyValue::Int8(i) => Ok(JsonValue::Number((*i).into())),
        PropertyValue::Int16(i) => Ok(JsonValue::Number((*i).into())),
        PropertyValue::Int32(i) => Ok(JsonValue::Number((*i).into())),
        PropertyValue::Int64(i) => Ok(JsonValue::Number((*i).into())),
        PropertyValue::Uint8(u) => Ok(JsonValue::Number((*u).into())),
        PropertyValue::Uint16(u) => Ok(JsonValue::Number((*u).into())),
        PropertyValue::Uint32(u) => Ok(JsonValue::Number((*u).into())),
        PropertyValue::Uint64(u) => Ok(JsonValue::Number((*u).into())),
        PropertyValue::Float32Value(f) => serde_json::Number::from_f64(*f as f64)
            .map(JsonValue::Number)
            .ok_or_else(|| GraphError::InvalidPropertyType("Invalid float32 value".to_string())),
        PropertyValue::Float64Value(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .ok_or_else(|| GraphError::InvalidPropertyType("Invalid float64 value".to_string())),
        PropertyValue::StringValue(s) => Ok(JsonValue::String(s.clone())),
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
        PropertyValue::Linestring(_) => Err(GraphError::UnsupportedOperation(
            "Linestring not supported in JanusGraph".to_string(),
        )),
        PropertyValue::Polygon(_) => Err(GraphError::UnsupportedOperation(
            "Polygon not supported in JanusGraph".to_string(),
        )),
    }
}

/// Convert JSON to PropertyValue
pub fn json_to_property_value(value: &JsonValue) -> Result<PropertyValue, GraphError> {
    // Handle JanusGraph's nested format where values are wrapped in @type and @value
    let actual_value = if value.is_object() && value.get("@value").is_some() {
        // This is a JanusGraph nested value like {"@type": "g:Int32", "@value": 30}
        value.get("@value").unwrap()
    } else {
        value
    };

    match actual_value {
        JsonValue::String(s) => {
            // Try to parse as various types
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
            // Handle floating point numbers by converting to integers to avoid BigDecimal issues
            if let Some(f) = n.as_f64() {
                // Convert to integer if it's a whole number, otherwise use string
                if f.fract() == 0.0 {
                    if f >= (i8::MIN as f64) && f <= (i8::MAX as f64) {
                        Ok(PropertyValue::Int8(f as i8))
                    } else if f >= (i16::MIN as f64) && f <= (i16::MAX as f64) {
                        Ok(PropertyValue::Int16(f as i16))
                    } else if f >= (i32::MIN as f64) && f <= (i32::MAX as f64) {
                        Ok(PropertyValue::Int32(f as i32))
                    } else if f >= (i64::MIN as f64) && f <= (i64::MAX as f64) {
                        Ok(PropertyValue::Int64(f as i64))
                    } else {
                        Ok(PropertyValue::StringValue(f.to_string()))
                    }
                } else {
                    // For non-whole numbers, convert to string to avoid BigDecimal issues
                    Ok(PropertyValue::StringValue(f.to_string()))
                }
            } else if let Some(i) = n.as_i64() {
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
            } else {
                Ok(PropertyValue::StringValue(n.to_string()))
            }
        }
        JsonValue::Bool(b) => Ok(PropertyValue::Boolean(*b)),
        JsonValue::Null => Ok(PropertyValue::StringValue("null".to_string())),
        JsonValue::Array(_) => Err(GraphError::InternalError(
            "Arrays not supported".to_string(),
        )),
        JsonValue::Object(_) => Err(GraphError::InternalError(
            "Objects not supported".to_string(),
        )),
    }
}

/// Convert ElementId to string for Gremlin queries
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

    // Extract the @value array from the data
    let data_array = response
        .result
        .data
        .get("@value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            GraphError::InternalError("Invalid response format: missing @value array".to_string())
        })?;

    for value in data_array {
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

    // Extract the @value array from the data
    let data_array = response
        .result
        .data
        .get("@value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            GraphError::InternalError("Invalid response format: missing @value array".to_string())
        })?;

    for value in data_array {
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

    // Extract the @value array from the data
    let data_array = response
        .result
        .data
        .get("@value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            GraphError::InternalError("Invalid response format: missing @value array".to_string())
        })?;

    let value = data_array
        .first()
        .ok_or_else(|| GraphError::InvalidQuery("No data in response".to_string()))?;

    json_to_vertex(value)
}

/// Parse edge from Gremlin response
pub fn parse_edge_from_response(response: &GremlinResponse) -> Result<Edge, GraphError> {
    if response.status.code != 200 {
        return Err(GraphError::InvalidQuery(response.status.message.clone()));
    }

    // Extract the @value array from the data
    let data_array = response
        .result
        .data
        .get("@value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            GraphError::InternalError("Invalid response format: missing @value array".to_string())
        })?;

    let value = data_array
        .first()
        .ok_or_else(|| GraphError::InvalidQuery("No data in response".to_string()))?;

    json_to_edge(value)
}

/// Parse string list from Gremlin response
pub fn parse_string_list_from_response(
    response: &GremlinResponse,
) -> Result<Vec<String>, GraphError> {
    if response.status.code != 200 {
        return Err(GraphError::InvalidQuery(response.status.message.clone()));
    }

    let mut strings = Vec::new();

    // Extract the @value array from the data
    let data_array = response
        .result
        .data
        .get("@value")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            GraphError::InternalError("Invalid response format: missing @value array".to_string())
        })?;

    for value in data_array {
        if let Some(s) = value.as_str() {
            strings.push(s.to_string());
        }
    }

    Ok(strings)
}

/// Convert PropertyType to Gremlin data type string
pub fn property_type_to_gremlin_type(property_type: &PropertyType) -> String {
    match property_type {
        PropertyType::StringType => "String.class".to_string(),
        PropertyType::Int32 => "Integer.class".to_string(),
        PropertyType::Int64 => "Long.class".to_string(),
        PropertyType::Float32Type => "Float.class".to_string(),
        PropertyType::Float64Type => "Double.class".to_string(),
        PropertyType::Boolean => "Boolean.class".to_string(),
        PropertyType::Bytes => "String.class".to_string(), // Store as base64 string
        PropertyType::Date => "String.class".to_string(),  // Store as ISO date string
        PropertyType::Datetime => "String.class".to_string(), // Store as ISO datetime string
        PropertyType::Point => "String.class".to_string(), // Store as WKT point string
        PropertyType::ListType => "String.class".to_string(), // Store as JSON string
        PropertyType::MapType => "String.class".to_string(), // Store as JSON string
    }
}

/// Convert PropertyValue to Gremlin string representation
pub fn property_value_to_gremlin_string(value: &PropertyValue) -> Result<String, GraphError> {
    match value {
        PropertyValue::NullValue => Ok("null".to_string()),
        PropertyValue::Boolean(b) => Ok(b.to_string()),
        PropertyValue::Int8(i) => Ok(i.to_string()),
        PropertyValue::Int16(i) => Ok(i.to_string()),
        PropertyValue::Int32(i) => Ok(i.to_string()),
        PropertyValue::Int64(i) => Ok(i.to_string()),
        PropertyValue::Uint8(u) => Ok(u.to_string()),
        PropertyValue::Uint16(u) => Ok(u.to_string()),
        PropertyValue::Uint32(u) => Ok(u.to_string()),
        PropertyValue::Uint64(u) => Ok(u.to_string()),
        PropertyValue::Float32Value(f) => Ok(f.to_string()),
        PropertyValue::Float64Value(f) => Ok(f.to_string()),
        PropertyValue::StringValue(s) => Ok(format!("'{s}'")),
        PropertyValue::Bytes(b) => Ok(format!(
            "'{}'",
            base64::engine::general_purpose::STANDARD.encode(b)
        )),
        PropertyValue::Date(_) => Ok("'date'".to_string()),
        PropertyValue::Time(_) => Ok("'time'".to_string()),
        PropertyValue::Datetime(_) => Ok("'datetime'".to_string()),
        PropertyValue::Duration(_) => Ok("'duration'".to_string()),
        PropertyValue::Point(_) => Ok("'point'".to_string()),
        PropertyValue::Linestring(_) => Ok("'linestring'".to_string()),
        PropertyValue::Polygon(_) => Ok("'polygon'".to_string()),
    }
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
        Err(GraphError::InvalidPropertyType(
            "Invalid date format".to_string(),
        ))
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

        Ok(Time {
            hour,
            minute,
            second,
            nanosecond,
        })
    } else {
        Err(GraphError::InvalidPropertyType(
            "Invalid time format".to_string(),
        ))
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
        Err(GraphError::InvalidPropertyType(
            "Invalid datetime format".to_string(),
        ))
    }
}

fn parse_duration(s: &str) -> Result<Duration, GraphError> {
    // Simple duration parsing for PTnS format (ISO 8601)
    if s.starts_with("PT") && s.ends_with('S') {
        if let Ok(seconds) = s[2..s.len() - 1].parse::<i64>() {
            Ok(Duration {
                seconds,
                nanoseconds: 0,
            })
        } else {
            Err(GraphError::InvalidPropertyType(
                "Invalid duration format".to_string(),
            ))
        }
    } else {
        Err(GraphError::InvalidPropertyType(
            "Invalid duration format".to_string(),
        ))
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

        Ok(Point {
            longitude,
            latitude,
            altitude,
        })
    } else {
        Err(GraphError::InvalidPropertyType(
            "Invalid point format".to_string(),
        ))
    }
}
