use golem_graph::exports::golem::graph::types::{ PropertyMap, ElementId, PropertyValue };
use golem_graph::golem::graph::errors::GraphError;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use base64::Engine;

/// Convert  PropertyMap to Neo4j parameters with proper escaping
pub fn property_map_to_neo4j_params(
    properties: &PropertyMap
) -> Result<HashMap<String, JsonValue>, GraphError> {
    let mut params = HashMap::new();

    for (key, value) in properties {
        // Validate property key (Neo4j property names have restrictions)
        if key.is_empty() || key.starts_with('_') {
            return Err(GraphError::InvalidPropertyType(format!("Invalid property key: {}", key)));
        }

        let json_value = property_value_to_neo4j_json(value)?;
        params.insert(key.clone(), json_value);
    }

    Ok(params)
}

/// Convert  PropertyValue to Neo4j-compatible JSON
pub fn property_value_to_neo4j_json(value: &PropertyValue) -> Result<JsonValue, GraphError> {
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
        PropertyValue::Float32Value(f) => {
            serde_json::Number
                ::from_f64(*f as f64)
                .map(JsonValue::Number)
                .ok_or_else(|| GraphError::InvalidPropertyType("Invalid float32 value".to_string()))
        }
        PropertyValue::Float64Value(f) => {
            serde_json::Number
                ::from_f64(*f)
                .map(JsonValue::Number)
                .ok_or_else(|| GraphError::InvalidPropertyType("Invalid float64 value".to_string()))
        }
        PropertyValue::StringValue(s) => Ok(JsonValue::String(s.clone())),
        PropertyValue::Bytes(b) => {
            Ok(JsonValue::String(base64::engine::general_purpose::STANDARD.encode(b)))
        }
        PropertyValue::Date(date) => {
            let neo4j_date =
                serde_json::json!({
                "year": date.year,
                "month": date.month,
                "day": date.day
            });
            Ok(neo4j_date)
        }
        PropertyValue::Time(time) => {
            let neo4j_time =
                serde_json::json!({
                "hour": time.hour,
                "minute": time.minute,
                "second": time.second,
                "nanosecond": time.nanosecond
            });
            Ok(neo4j_time)
        }
        PropertyValue::Datetime(datetime) => {
            let neo4j_datetime =
                serde_json::json!({
                "year": datetime.date.year,
                "month": datetime.date.month,
                "day": datetime.date.day,
                "hour": datetime.time.hour,
                "minute": datetime.time.minute,
                "second": datetime.time.second,
                "nanosecond": datetime.time.nanosecond,
                "timezone": datetime.timezone_offset_minutes
            });
            Ok(neo4j_datetime)
        }
        PropertyValue::Duration(duration) => {
            let neo4j_duration =
                serde_json::json!({
                "seconds": duration.seconds,
                "nanoseconds": duration.nanoseconds
            });
            Ok(neo4j_duration)
        }
        // Geospatial types - convert to Neo4j Point/Geography
        PropertyValue::Point(point) => {
            let neo4j_point =
                serde_json::json!({
                "crs": "wgs-84",
                "latitude": point.latitude,
                "longitude": point.longitude,
                "altitude": point.altitude
            });
            Ok(neo4j_point)
        }
        PropertyValue::Linestring(linestring) => {
            let coordinates: Vec<JsonValue> = linestring.coordinates
                .iter()
                .map(|p| serde_json::json!([p.longitude, p.latitude, p.altitude]))
                .collect();

            let neo4j_linestring =
                serde_json::json!({
                "type": "LineString",
                "coordinates": coordinates,
            });
            Ok(neo4j_linestring)
        }
        PropertyValue::Polygon(polygon) => {
            let mut rings = vec![];

            // Exterior ring
            let exterior: Vec<JsonValue> = polygon.exterior
                .iter()
                .map(|p| serde_json::json!([p.longitude, p.latitude, p.altitude]))
                .collect();
            rings.push(exterior);

            // Interior rings (holes)
            if let Some(holes) = &polygon.holes {
                for hole in holes {
                    let hole_coords: Vec<JsonValue> = hole
                        .iter()
                        .map(|p| serde_json::json!([p.longitude, p.latitude, p.altitude]))
                        .collect();
                    rings.push(hole_coords);
                }
            }

            let neo4j_polygon =
                serde_json::json!({
                "type": "Polygon",
                "coordinates": rings,
            });
            Ok(neo4j_polygon)
        }
    }
}

/// Convert  PropertyValue to JSON for general use

/// Convert JSON to  PropertyValue
pub fn json_to_property_value(value: &JsonValue) -> Result<PropertyValue, GraphError> {
    match value {
        JsonValue::Null => Ok(PropertyValue::NullValue),
        JsonValue::Bool(b) => Ok(PropertyValue::Boolean(*b)),
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
                Err(GraphError::InvalidPropertyType("Invalid number format".to_string()))
            }
        }
        JsonValue::String(s) => Ok(PropertyValue::StringValue(s.clone())),
        JsonValue::Array(_) => {
            // Arrays not supported in current PropertyValue enum
            Err(GraphError::InvalidPropertyType("Arrays not supported".to_string()))
        }
        JsonValue::Object(_) => {
            // Objects not supported in current PropertyValue enum
            Err(GraphError::InvalidPropertyType("Objects not supported".to_string()))
        }
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
