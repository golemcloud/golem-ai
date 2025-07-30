use base64::{engine::general_purpose, Engine as _};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use golem_graph::golem::graph::{
    errors::GraphError,
    types::{Date, Datetime, Point, PropertyValue, Time},
};
use serde_json::{json, Value};

pub(crate) fn to_json_value(value: PropertyValue) -> Result<Value, GraphError> {
    Ok(match value {
        PropertyValue::NullValue => Value::Null,
        PropertyValue::Boolean(b) => Value::Bool(b),
        PropertyValue::Int8(i) => json!(i),
        PropertyValue::Int16(i) => json!(i),
        PropertyValue::Int32(i) => json!(i),
        PropertyValue::Int64(i) => json!(i),
        PropertyValue::Uint8(i) => json!(i),
        PropertyValue::Uint16(i) => json!(i),
        PropertyValue::Uint32(i) => json!(i),
        PropertyValue::Uint64(i) => json!(i),
        PropertyValue::Float32Value(f) => json!(f),
        PropertyValue::Float64Value(f) => json!(f),
        PropertyValue::StringValue(s) => Value::String(s),
        PropertyValue::Bytes(b) => Value::String(general_purpose::STANDARD.encode(b)),
        PropertyValue::Date(d) => {
            Value::String(format!("{:04}-{:02}-{:02}", d.year, d.month, d.day))
        }
        PropertyValue::Datetime(dt) => {
            let date_str = format!(
                "{:04}-{:02}-{:02}",
                dt.date.year, dt.date.month, dt.date.day
            );
            let time_str = format!(
                "{:02}:{:02}:{:02}.{}",
                dt.time.hour,
                dt.time.minute,
                dt.time.second,
                format_args!("{:09}", dt.time.nanosecond)
            );
            Value::String(format!("{date_str}T{time_str}Z"))
        }
        PropertyValue::Point(p) => {
            if let Some(alt) = p.altitude {
                Value::String(format!("POINT ({} {} {})", p.longitude, p.latitude, alt))
            } else {
                Value::String(format!("POINT ({} {})", p.longitude, p.latitude))
            }
        }
        _ => {
            return Err(GraphError::UnsupportedOperation(
                "This property type is not supported as a Gremlin binding".to_string(),
            ))
        }
    })
}

pub(crate) fn from_gremlin_value(value: &Value) -> Result<PropertyValue, GraphError> {
    match value {
        Value::Null => Ok(PropertyValue::NullValue),
        Value::Bool(b) => Ok(PropertyValue::Boolean(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(PropertyValue::Int64(i))
            } else if let Some(f) = n.as_f64() {
                Ok(PropertyValue::Float64Value(f))
            } else {
                Err(GraphError::InvalidPropertyType(
                    "Unsupported number type from Gremlin".to_string(),
                ))
            }
        }
        Value::String(s) => {
            if let Ok(dt) = parse_iso_datetime(s) {
                return Ok(PropertyValue::Datetime(dt));
            }
            if let Ok(d) = parse_iso_date(s) {
                return Ok(PropertyValue::Date(d));
            }
            if let Ok(p) = parse_wkt_point(s) {
                return Ok(PropertyValue::Point(p));
            }
            Ok(PropertyValue::StringValue(s.clone()))
        }
        Value::Object(obj) => {
            // Handle GraphSON wrapped values like {"@type": "g:Int64", "@value": 29}
            if let (Some(Value::String(gtype)), Some(gvalue)) =
                (obj.get("@type"), obj.get("@value"))
            {
                match gtype.as_str() {
                    "g:Int64" | "g:Int32" | "g:Int16" | "g:Int8" => {
                        if let Some(i) = gvalue.as_i64() {
                            Ok(PropertyValue::Int64(i))
                        } else {
                            Err(GraphError::InvalidPropertyType(
                                "Invalid GraphSON integer value".to_string(),
                            ))
                        }
                    }
                    "g:Float" | "g:Double" => {
                        if let Some(f) = gvalue.as_f64() {
                            Ok(PropertyValue::Float64Value(f))
                        } else {
                            Err(GraphError::InvalidPropertyType(
                                "Invalid GraphSON float value".to_string(),
                            ))
                        }
                    }
                    _ => {
                        // For other GraphSON types, try to parse the @value recursively
                        from_gremlin_value(gvalue)
                    }
                }
            } else {
                Err(GraphError::InvalidPropertyType(
                    "Gremlin objects without GraphSON @type/@value cannot be converted to a WIT property type.".to_string(),
                ))
            }
        }
        Value::Array(_) => Err(GraphError::InvalidPropertyType(
            "Gremlin arrays cannot be converted to a WIT property type.".to_string(),
        )),
    }
}

fn parse_wkt_point(s: &str) -> Result<Point, ()> {
    if !s.starts_with("POINT") {
        return Err(());
    }
    let content = s.trim_start_matches("POINT").trim();
    let content = content.strip_prefix('(').unwrap_or(content);
    let content = content.strip_suffix(')').unwrap_or(content);

    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(());
    }

    let lon = parts[0].parse::<f64>().map_err(|_| ())?;
    let lat = parts[1].parse::<f64>().map_err(|_| ())?;
    let alt = if parts.len() > 2 {
        parts[2].parse::<f64>().ok()
    } else {
        None
    };

    Ok(Point {
        longitude: lon,
        latitude: lat,
        altitude: alt,
    })
}

fn parse_iso_date(s: &str) -> Result<Date, ()> {
    match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        Ok(date) => Ok(Date {
            year: date.year() as u32,
            month: date.month() as u8,
            day: date.day() as u8,
        }),
        Err(_) => Err(()),
    }
}

fn parse_iso_datetime(s: &str) -> Result<Datetime, ()> {
    // Try multiple datetime formats commonly used by Gremlin/JanusGraph
    let formats = [
        "%Y-%m-%dT%H:%M:%S%.fZ", // ISO format with milliseconds and Z
        "%Y-%m-%dT%H:%M:%SZ",    // ISO format without milliseconds and Z
        "%Y-%m-%dT%H:%M:%S%.f",  // ISO format with milliseconds, no Z
        "%Y-%m-%dT%H:%M:%S",     // ISO format without milliseconds, no Z
    ];

    for format in &formats {
        if let Ok(datetime) = NaiveDateTime::parse_from_str(s, format) {
            let date = Date {
                year: datetime.year() as u32,
                month: datetime.month() as u8,
                day: datetime.day() as u8,
            };
            let time = Time {
                hour: datetime.hour() as u8,
                minute: datetime.minute() as u8,
                second: datetime.second() as u8,
                nanosecond: datetime.nanosecond(),
            };
            return Ok(Datetime {
                date,
                time,
                timezone_offset_minutes: Some(0), // Gremlin dates are timezone-aware
            });
        }
    }

    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use golem_graph::golem::graph::types::{Duration, PropertyValue};

    #[test]
    fn test_unsupported_duration_conversion() {
        let original = PropertyValue::Duration(Duration {
            seconds: 10,
            nanoseconds: 0,
        });

        let result = to_json_value(original);
        assert!(matches!(result, Err(GraphError::UnsupportedOperation(_))));
    }
}
