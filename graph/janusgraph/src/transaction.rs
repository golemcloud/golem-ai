use crate::conversions;
use crate::helpers;
use crate::helpers::{element_id_to_key, parse_path_from_gremlin, parse_vertex_from_gremlin};
use crate::query_utils;
use crate::Transaction;
use golem_ai_graph::model::transactions::{
    CreateVertexOptions, ExecuteQueryOptions, FindAllPathsOptions, FindEdgesOptions,
    FindShortestPathOptions, FindVerticesOptions, GetAdjacentVerticesOptions,
    GetConnectedEdgesOptions, GetNeighborhoodOptions, GetVerticesAtDistanceOptions, Path,
    PathExistsOptions, PropertyValue, QueryExecutionResult, Subgraph,
};
use golem_ai_graph::model::types::{
    CreateEdgeOptions, QueryParameters, QueryResult, UpdateEdgeOptions, UpdateVertexOptions,
};
use golem_ai_graph::model::{
    errors::GraphError,
    types::{Direction, Edge, ElementId, Vertex},
};
use golem_ai_graph::TransactionInterface;
use log::trace;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

fn graphson_map_to_object(data: &Value) -> Result<Value, GraphError> {
    let arr = data
        .get("@value")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            GraphError::InternalError("Expected GraphSON Map with @value array".into())
        })?;

    let mut obj = serde_json::Map::new();
    let mut iter = arr.iter();
    while let (Some(k), Some(v)) = (iter.next(), iter.next()) {
        let key = if let Some(s) = k.as_str() {
            s.to_string()
        } else if let Some(inner) = k.get("@value").and_then(Value::as_str) {
            inner.to_string()
        } else {
            return Err(GraphError::InternalError(format!(
                "Expected string key in GraphSON Map, got {k}"
            )));
        };

        let val = if let Some(inner) = v.get("@value") {
            inner.clone()
        } else {
            v.clone()
        };

        obj.insert(key, val);
    }

    Ok(Value::Object(obj))
}

impl TransactionInterface for Transaction {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn execute_query(
        &self,
        options: ExecuteQueryOptions,
    ) -> Result<QueryExecutionResult, GraphError> {
        let params = options.parameters.unwrap_or_default();
        let (final_query, bindings_map) = if params.is_empty() {
            (options.query, serde_json::Map::new())
        } else {
            match to_bindings(params.clone()) {
                Ok(bindings) => (options.query, bindings),
                Err(_e) => {
                    let mut inline_query = options.query;
                    for (key, value) in &params {
                        let replacement = match value {
                            PropertyValue::Float32Value(f) => f.to_string(),
                            PropertyValue::Float64Value(f) => f.to_string(),
                            PropertyValue::Int32(i) => i.to_string(),
                            PropertyValue::Int64(i) => i.to_string(),
                            PropertyValue::StringValue(s) => format!("'{s}'"),
                            PropertyValue::Boolean(b) => b.to_string(),
                            _ => {
                                continue;
                            }
                        };
                        inline_query = inline_query.replace(key, &replacement);
                    }
                    (inline_query, serde_json::Map::new())
                }
            }
        };

        let response = self.api.execute(&final_query, Some(json!(bindings_map)))?;
        let query_result_value = parse_gremlin_response(response)?;

        Ok(QueryExecutionResult {
            query_result_value,
            execution_time_ms: None,
            rows_affected: None,
            explanation: None,
            profile_data: None,
        })
    }

    fn find_shortest_path(
        &self,
        options: FindShortestPathOptions,
    ) -> Result<Option<Path>, GraphError> {
        let mut bindings = serde_json::Map::new();
        bindings.insert("from_id".to_string(), id_to_json(options.from_vertex));
        bindings.insert("to_id".to_string(), id_to_json(options.to_vertex));

        let gremlin =
            "g.V(from_id).repeat(outE().inV().simplePath()).until(hasId(to_id)).path().limit(1)";

        let resp = self.api.execute(gremlin, Some(Value::Object(bindings)))?;

        let data_array = if let Some(data) = resp.as_object() {
            if data.get("@type") == Some(&Value::String("g:List".to_string())) {
                data.get("@value").and_then(|v| v.as_array())
            } else {
                None
            }
        } else {
            resp.as_array()
        };

        if let Some(arr) = data_array {
            if let Some(val) = arr.first() {
                return Ok(Some(parse_path_from_gremlin(val)?));
            } else {
                trace!("[DEBUG][find_shortest_path] Data array is empty");
            }
        } else {
            trace!("[DEBUG][find_shortest_path] No data array in response");
        }

        Ok(None)
    }

    fn find_all_paths(&self, options: FindAllPathsOptions) -> Result<Vec<Path>, GraphError> {
        if let Some(opts) = &options.path {
            if opts.vertex_types.is_some()
                || opts.vertex_filters.is_some()
                || opts.edge_filters.is_some()
            {
                return Err(GraphError::UnsupportedOperation(
                    "vertex_types, vertex_filters, and edge_filters are not yet supported in find_all_paths"
                        .to_string(),
                ));
            }
        }

        let mut bindings = serde_json::Map::new();
        let edge_types = options.path.and_then(|o| o.edge_types);
        let step = build_traversal_step(&Direction::Both, &edge_types, &mut bindings);
        bindings.insert("from_id".to_string(), id_to_json(options.from_vertex));
        bindings.insert("to_id".to_string(), id_to_json(options.to_vertex));

        let mut gremlin =
            format!("g.V(from_id).repeat({step}.simplePath()).until(hasId(to_id)).path()");
        if let Some(lim) = options.limit {
            gremlin.push_str(&format!(".limit({lim})"));
        }

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let data_array = if let Some(data) = response.as_object() {
            if data.get("@type") == Some(&Value::String("g:List".to_string())) {
                data.get("@value").and_then(|v| v.as_array())
            } else {
                None
            }
        } else {
            response.as_array()
        };

        if let Some(arr) = data_array {
            arr.iter().map(parse_path_from_gremlin).collect()
        } else {
            Ok(Vec::new())
        }
    }

    fn get_neighborhood(&self, options: GetNeighborhoodOptions) -> Result<Subgraph, GraphError> {
        let mut bindings = serde_json::Map::new();
        bindings.insert("center_id".to_string(), id_to_json(options.center.clone()));

        let edge_step = match options.direction {
            Direction::Outgoing => "outE",
            Direction::Incoming => "inE",
            Direction::Both => "bothE",
        };
        let mut gremlin = format!(
            "g.V(center_id).repeat({}().otherV().simplePath()).times({}).path()",
            edge_step, options.depth
        );
        if let Some(lim) = options.max_vertices {
            gremlin.push_str(&format!(".limit({lim})"));
        }

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let data_array = if let Some(data) = response.as_object() {
            if data.get("@type") == Some(&Value::String("g:List".to_string())) {
                data.get("@value").and_then(|v| v.as_array())
            } else {
                None
            }
        } else {
            response.as_array()
        };

        if let Some(arr) = data_array {
            let mut verts = std::collections::HashMap::new();
            let mut edges = std::collections::HashMap::new();
            for val in arr {
                let path = parse_path_from_gremlin(val)?;
                for v in path.vertices {
                    verts.insert(element_id_to_key(&v.id), v);
                }
                for e in path.edges {
                    edges.insert(element_id_to_key(&e.id), e);
                }
            }

            Ok(Subgraph {
                vertices: verts.into_values().collect(),
                edges: edges.into_values().collect(),
            })
        } else {
            Ok(Subgraph {
                vertices: Vec::new(),
                edges: Vec::new(),
            })
        }
    }

    fn path_exists(&self, options: PathExistsOptions) -> Result<bool, GraphError> {
        self.find_all_paths(FindAllPathsOptions {
            from_vertex: options.from_vertex,
            to_vertex: options.to_vertex,
            path: options.path,
            limit: Some(1),
        })
        .map(|p| !p.is_empty())
    }

    fn get_vertices_at_distance(
        &self,
        options: GetVerticesAtDistanceOptions,
    ) -> Result<Vec<Vertex>, GraphError> {
        let mut bindings = serde_json::Map::new();
        bindings.insert("source_id".to_string(), id_to_json(options.source));

        let step = match options.direction {
            Direction::Outgoing => "out",
            Direction::Incoming => "in",
            Direction::Both => "both",
        }
        .to_string();

        let gremlin = if let Some(labels) = &options.edge_types {
            if !labels.is_empty() {
                let label_bindings: Vec<String> = labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| {
                        let key = format!("edge_label_{i}");
                        bindings.insert(key.clone(), json!(label));
                        key
                    })
                    .collect();
                let labels_str = label_bindings.join(", ");
                format!(
                    "g.V(source_id).repeat({step}({labels_str})).times({distance}).dedup().elementMap()",
                    distance = options.distance,
                )
            } else {
                format!(
                    "g.V(source_id).repeat({step}()).times({distance}).dedup().elementMap()",
                    distance = options.distance,
                )
            }
        } else {
            format!(
                "g.V(source_id).repeat({step}()).times({distance}).dedup().elementMap()",
                distance = options.distance
            )
        };

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let data_array = if let Some(data) = response.as_object() {
            if data.get("@type") == Some(&Value::String("g:List".to_string())) {
                data.get("@value").and_then(|v| v.as_array())
            } else {
                None
            }
        } else {
            response.as_array()
        };

        if let Some(arr) = data_array {
            arr.iter().map(parse_vertex_from_gremlin).collect()
        } else {
            Ok(Vec::new())
        }
    }

    fn get_adjacent_vertices(
        &self,
        options: GetAdjacentVerticesOptions,
    ) -> Result<Vec<Vertex>, GraphError> {
        let mut bindings = serde_json::Map::new();
        let id_json = match options.vertex_id {
            ElementId::StringValue(s) => json!(s),
            ElementId::Int64(i) => json!(i),
            ElementId::Uuid(u) => json!(u.to_string()),
        };
        bindings.insert("vertex_id".to_string(), id_json);

        let direction_step = match options.direction {
            Direction::Outgoing => "out",
            Direction::Incoming => "in",
            Direction::Both => "both",
        };

        let mut gremlin = if let Some(labels) = options.edge_types {
            if !labels.is_empty() {
                let label_bindings: Vec<String> = labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| {
                        let binding_key = format!("label_{i}");
                        bindings.insert(binding_key.clone(), json!(label));
                        binding_key
                    })
                    .collect();
                let labels_str = label_bindings.join(", ");
                format!("g.V(vertex_id).{direction_step}({labels_str})")
            } else {
                format!("g.V(vertex_id).{direction_step}()")
            }
        } else {
            format!("g.V(vertex_id).{direction_step}()")
        };

        if let Some(lim) = options.limit {
            gremlin.push_str(&format!(".limit({lim})"));
        }

        gremlin.push_str(".elementMap()");

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let result_data = if let Some(arr) = response.as_array() {
            arr.clone()
        } else if let Some(inner) = response.get("@value").and_then(Value::as_array) {
            inner.clone()
        } else {
            return Err(GraphError::InternalError(
                "Invalid response from Gremlin for get_adjacent_vertices".to_string(),
            ));
        };

        result_data
            .iter()
            .map(helpers::parse_vertex_from_gremlin)
            .collect()
    }

    fn get_connected_edges(
        &self,
        options: GetConnectedEdgesOptions,
    ) -> Result<Vec<Edge>, GraphError> {
        let mut bindings = serde_json::Map::new();
        let id_json = match options.vertex_id {
            ElementId::StringValue(s) => json!(s),
            ElementId::Int64(i) => json!(i),
            ElementId::Uuid(u) => json!(u.to_string()),
        };
        bindings.insert("vertex_id".to_string(), id_json);

        let direction_step = match options.direction {
            Direction::Outgoing => "outE",
            Direction::Incoming => "inE",
            Direction::Both => "bothE",
        };

        let mut gremlin = if let Some(labels) = options.edge_types {
            if !labels.is_empty() {
                let label_bindings: Vec<String> = labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| {
                        let binding_key = format!("edge_label_{i}");
                        bindings.insert(binding_key.clone(), json!(label));
                        binding_key
                    })
                    .collect();
                let labels_str = label_bindings.join(", ");
                format!("g.V(vertex_id).{direction_step}({labels_str})")
            } else {
                format!("g.V(vertex_id).{direction_step}()")
            }
        } else {
            format!("g.V(vertex_id).{direction_step}()")
        };

        if let Some(lim) = options.limit {
            gremlin.push_str(&format!(".limit({lim})"));
        }

        gremlin.push_str(".elementMap()");

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let result_data = if let Some(arr) = response.as_array() {
            arr.clone()
        } else if let Some(inner) = response.get("@value").and_then(Value::as_array) {
            inner.clone()
        } else {
            return Err(GraphError::InternalError(
                "Invalid response from Gremlin for get_connected_edges".to_string(),
            ));
        };

        result_data
            .iter()
            .map(helpers::parse_edge_from_gremlin)
            .collect()
    }

    fn create_vertex(&self, options: CreateVertexOptions) -> Result<Vertex, GraphError> {
        let mut gremlin = "g.addV(vertex_label)".to_string();
        let mut bindings = serde_json::Map::new();
        bindings.insert("vertex_label".to_string(), json!(options.vertex_type));

        for (i, (key, value)) in options
            .properties
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            let binding_key = format!("p{i}");
            gremlin.push_str(&format!(".property(k{i}, {binding_key})"));
            bindings.insert(format!("k{i}"), json!(key));
            bindings.insert(binding_key, conversions::to_json_value(value)?);
        }
        gremlin.push_str(".elementMap()");

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let element = if let Some(_graphson_obj) = response.as_object() {
            if response.get("@type") == Some(&json!("g:List")) {
                let arr = response
                    .get("@value")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| {
                        GraphError::InternalError(
                            "Expected @value array in GraphSON List".to_string(),
                        )
                    })?;
                arr.first().ok_or_else(|| {
                    GraphError::InternalError("Empty result from vertex creation".to_string())
                })?
            } else {
                &response
            }
        } else if let Some(arr) = response.as_array() {
            arr.first().ok_or_else(|| {
                GraphError::InternalError("Empty result from vertex creation".to_string())
            })?
        } else {
            return Err(GraphError::InternalError(format!(
                "Unexpected response format from vertex creation: {response:#}"
            )));
        };

        let obj = graphson_map_to_object(element)?;

        helpers::parse_vertex_from_gremlin(&obj)
    }

    fn create_vertices(
        &self,
        vertices: Vec<CreateVertexOptions>,
    ) -> Result<Vec<Vertex>, GraphError> {
        if vertices.is_empty() {
            return Ok(vec![]);
        }

        if vertices.len() == 1 {
            let mut vertices = vertices;
            let vertex = self.create_vertex(vertices.pop().unwrap())?;
            return Ok(vec![vertex]);
        }

        let mut gremlin = "g.union(".to_string();
        let mut bindings = serde_json::Map::new();
        let mut union_parts = Vec::new();

        for (i, options) in vertices.into_iter().enumerate() {
            let label_binding = format!("l{i}");
            let mut part = format!("addV({label_binding})");
            bindings.insert(label_binding, json!(options.vertex_type));

            for (j, (key, value)) in options
                .properties
                .unwrap_or_default()
                .into_iter()
                .enumerate()
            {
                let key_binding = format!("k_{i}_{j}");
                let val_binding = format!("v_{i}_{j}");
                part.push_str(&format!(".property({key_binding}, {val_binding})"));
                bindings.insert(key_binding, json!(key));
                bindings.insert(val_binding, conversions::to_json_value(value.clone())?);
            }

            union_parts.push(part);
        }

        gremlin.push_str(&union_parts.join(", "));
        gremlin.push_str(").elementMap()");

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let result_data = if let Some(arr) = response.as_array() {
            arr.clone()
        } else if let Some(inner) = response.get("@value").and_then(Value::as_array) {
            inner.clone()
        } else {
            return Err(GraphError::InternalError(
                "Invalid response from Gremlin for create_vertices".to_string(),
            ));
        };

        result_data
            .iter()
            .map(helpers::parse_vertex_from_gremlin)
            .collect()
    }

    fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, GraphError> {
        let gremlin = "g.V(vertex_id).elementMap()".to_string();

        let mut bindings = serde_json::Map::new();
        bindings.insert(
            "vertex_id".to_string(),
            match id.clone() {
                ElementId::StringValue(s) => json!(s),
                ElementId::Int64(i) => json!(i),
                ElementId::Uuid(u) => json!(u.to_string()),
            },
        );

        let resp = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let list: Vec<Value> = if let Some(arr) = resp.as_array() {
            arr.clone()
        } else if let Some(inner) = resp.get("@value").and_then(Value::as_array) {
            inner.clone()
        } else {
            vec![]
        };

        if let Some(row) = list.into_iter().next() {
            let obj = if row.get("@type") == Some(&json!("g:Map")) {
                let vals = row.get("@value").and_then(Value::as_array).unwrap();
                let mut m = serde_json::Map::new();
                let mut it = vals.iter();
                while let (Some(kv), Some(vv)) = (it.next(), it.next()) {
                    let key = if kv.is_string() {
                        kv.as_str().unwrap().to_string()
                    } else {
                        kv.get("@value")
                            .and_then(Value::as_str)
                            .unwrap()
                            .to_string()
                    };
                    let val = if vv.is_object() {
                        vv.get("@value").cloned().unwrap_or(vv.clone())
                    } else {
                        vv.clone()
                    };
                    m.insert(key, val);
                }
                Value::Object(m)
            } else {
                row.clone()
            };

            let vertex = helpers::parse_vertex_from_gremlin(&obj)?;
            Ok(Some(vertex))
        } else {
            Ok(None)
        }
    }

    fn update_vertex(&self, options: UpdateVertexOptions) -> Result<Vertex, GraphError> {
        let mut gremlin = {
            if options.partial.unwrap_or_default() {
                "g.V(vertex_id)".to_string()
            } else {
                "g.V(vertex_id).sideEffect(properties().drop())".to_string()
            }
        };
        let mut bindings = serde_json::Map::new();
        bindings.insert(
            "vertex_id".to_string(),
            match options.id.clone() {
                ElementId::StringValue(s) => json!(s),
                ElementId::Int64(i) => json!(i),
                ElementId::Uuid(u) => json!(u.to_string()),
            },
        );

        for (i, (k, v)) in options.properties.into_iter().enumerate() {
            let kb = format!("k{i}");
            let vb = format!("v{i}");
            gremlin.push_str(&format!(".property({kb}, {vb})"));
            bindings.insert(kb.clone(), json!(k));
            bindings.insert(vb.clone(), conversions::to_json_value(v)?);
        }

        gremlin.push_str(".elementMap()");

        let resp = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let maybe_row = resp
            .as_array()
            .and_then(|arr| arr.first().cloned())
            .or_else(|| {
                resp.get("@value")
                    .and_then(Value::as_array)
                    .and_then(|arr| arr.first().cloned())
            });
        let row = maybe_row.ok_or(GraphError::ElementNotFound(options.id.clone()))?;

        let mut flat = serde_json::Map::new();
        if row.get("@type") == Some(&json!("g:Map")) {
            let vals = row.get("@value").and_then(Value::as_array).unwrap();
            let mut it = vals.iter();
            while let (Some(kv), Some(vv)) = (it.next(), it.next()) {
                let key = if kv.is_string() {
                    kv.as_str().unwrap().to_string()
                } else {
                    kv.get("@value")
                        .and_then(Value::as_str)
                        .unwrap()
                        .to_string()
                };
                let val = if vv.is_object() {
                    vv.get("@value").cloned().unwrap_or(vv.clone())
                } else {
                    vv.clone()
                };
                flat.insert(key, val);
            }
        } else if let Some(obj) = row.as_object() {
            flat = obj.clone();
        } else {
            return Err(GraphError::InternalError(
                "Unexpected Gremlin row format".into(),
            ));
        }

        let mut obj = serde_json::Map::new();
        obj.insert("id".to_string(), flat["id"].clone());
        obj.insert("label".to_string(), flat["label"].clone());

        let mut props = serde_json::Map::new();
        for (k, v) in flat.into_iter() {
            if k != "id" && k != "label" {
                props.insert(k, v);
            }
        }
        obj.insert("properties".to_string(), Value::Object(props));

        helpers::parse_vertex_from_gremlin(&Value::Object(obj))
    }

    fn delete_vertex(&self, id: ElementId, _detach: bool) -> Result<(), GraphError> {
        let gremlin = "g.V(vertex_id).drop().toList()";
        let mut bindings = serde_json::Map::new();
        bindings.insert(
            "vertex_id".to_string(),
            match id.clone() {
                ElementId::StringValue(s) => json!(s),
                ElementId::Int64(i) => json!(i),
                ElementId::Uuid(u) => json!(u.to_string()),
            },
        );

        for attempt in 1..=2 {
            let resp = self
                .api
                .execute(gremlin, Some(Value::Object(bindings.clone())));
            match resp {
                Ok(_) => {
                    log::info!("[delete_vertex] dropped vertex {id:?} (attempt {attempt})");
                    return Ok(());
                }
                Err(GraphError::TransactionTimeout) if attempt == 1 => {
                    log::warn!(
                        "[delete_vertex] Transaction timeout on vertex {id:?}, retrying drop (1/2)"
                    );
                    continue;
                }
                Err(GraphError::TransactionTimeout) => {
                    log::warn!(
                        "[delete_vertex] Transaction timeout again on {id:?}, ignoring cleanup"
                    );
                    return Ok(());
                }
                Err(GraphError::DeadlockDetected) if attempt == 1 => {
                    log::warn!(
                        "[delete_vertex] Deadlock detected on vertex {id:?}, retrying drop (1/2)"
                    );
                    continue;
                }
                Err(GraphError::DeadlockDetected) => {
                    log::warn!(
                        "[delete_vertex] Deadlock detected again on {id:?}, ignoring cleanup"
                    );
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn find_vertices(&self, options: FindVerticesOptions) -> Result<Vec<Vertex>, GraphError> {
        let mut gremlin = "g.V()".to_string();
        let mut bindings = serde_json::Map::new();

        if let Some(label) = options.vertex_type {
            gremlin.push_str(".hasLabel(vertex_label)");
            bindings.insert("vertex_label".to_string(), json!(label));
        }

        if let Some(filter_conditions) = options.filters {
            for condition in &filter_conditions {
                gremlin.push_str(&query_utils::build_gremlin_filter_step(
                    condition,
                    &mut bindings,
                )?);
            }
        }

        if let Some(sort_specs) = options.sort {
            gremlin.push_str(&query_utils::build_gremlin_sort_clause(&sort_specs));
        }

        if let Some(off) = options.offset {
            gremlin.push_str(&format!(
                ".range({}, {})",
                off,
                off + options.limit.unwrap_or(10_000)
            ));
        } else if let Some(lim) = options.limit {
            gremlin.push_str(&format!(".limit({lim})"));
        }

        gremlin.push_str(".elementMap()");

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;
        trace!("[DEBUG][find_vertices] Raw Gremlin response: {response:?}");

        let result_data = if let Some(arr) = response.as_array() {
            arr.clone()
        } else if let Some(inner) = response.get("@value").and_then(Value::as_array) {
            inner.clone()
        } else {
            return Err(GraphError::InternalError(
                "Invalid response from Gremlin for find_vertices".to_string(),
            ));
        };

        result_data
            .iter()
            .map(|item| {
                let result = helpers::parse_vertex_from_gremlin(item);
                if let Err(ref e) = result {
                    trace!("[DEBUG][find_vertices] Parse error for item {item:?}: {e:?}");
                }
                result
            })
            .collect()
    }

    fn create_edge(&self, options: CreateEdgeOptions) -> Result<Edge, GraphError> {
        let mut gremlin = "g.V(from_id).addE(edge_label).to(__.V(to_id))".to_string();
        let mut bindings = serde_json::Map::new();
        let from_clone = options.from_vertex.clone();

        bindings.insert(
            "from_id".into(),
            match options.from_vertex {
                ElementId::StringValue(s) => json!(s),
                ElementId::Int64(i) => json!(i),
                ElementId::Uuid(u) => json!(u.to_string()),
            },
        );
        bindings.insert(
            "to_id".into(),
            match options.to_vertex {
                ElementId::StringValue(s) => json!(s),
                ElementId::Int64(i) => json!(i),
                ElementId::Uuid(u) => json!(u.to_string()),
            },
        );
        bindings.insert("edge_label".into(), json!(options.edge_type));

        for (i, (k, v)) in options
            .properties
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            let kb = format!("k{i}");
            let vb = format!("v{i}");
            gremlin.push_str(&format!(".property({kb}, {vb})"));
            bindings.insert(kb.clone(), json!(k));
            bindings.insert(vb.clone(), conversions::to_json_value(v)?);
            trace!("[LOG create_edge] bound {} -> {:?}", kb, bindings[&kb]);
        }

        gremlin.push_str(".elementMap()");

        let resp = self
            .api
            .execute(&gremlin, Some(Value::Object(bindings.clone())))?;

        let row = if let Some(arr) = resp.as_array() {
            arr.first().cloned()
        } else if let Some(inner) = resp.get("@value").and_then(Value::as_array) {
            inner.first().cloned()
        } else {
            None
        }
        .ok_or_else(|| GraphError::ElementNotFound(from_clone.clone()))?;

        let mut flat = serde_json::Map::new();
        if row.get("@type") == Some(&json!("g:Map")) {
            let vals = row.get("@value").and_then(Value::as_array).unwrap();
            let mut it = vals.iter();
            while let (Some(kv), Some(vv)) = (it.next(), it.next()) {
                let key = if kv.is_string() {
                    kv.as_str().unwrap().to_string()
                } else {
                    kv.get("@value")
                        .and_then(Value::as_str)
                        .unwrap()
                        .to_string()
                };
                let val = if vv.is_object() {
                    vv.get("@value").cloned().unwrap_or(vv.clone())
                } else {
                    vv.clone()
                };
                flat.insert(key.clone(), val.clone());
            }
        } else if let Some(obj) = row.as_object() {
            flat = obj.clone();
        } else {
            return Err(GraphError::InternalError("Unexpected row format".into()));
        }

        let mut edge_json = serde_json::Map::new();

        let id_field = &flat["id"];
        let real_id = if let Some(rel) = id_field.get("relationId").and_then(Value::as_str) {
            json!(rel)
        } else {
            id_field.clone()
        };
        edge_json.insert("id".into(), real_id.clone());

        let lbl = flat["label"].clone();
        edge_json.insert("label".into(), lbl.clone());

        if let Some(arr) = flat.get("OUT").and_then(Value::as_array) {
            if let Some(vv) = arr.get(1).and_then(|v| v.get("@value")).cloned() {
                edge_json.insert("outV".into(), vv.clone());
            }
        }
        if let Some(arr) = flat.get("IN").and_then(Value::as_array) {
            if let Some(vv) = arr.get(1).and_then(|v| v.get("@value")).cloned() {
                edge_json.insert("inV".into(), vv.clone());
            }
        }

        edge_json.insert("properties".into(), json!({}));

        helpers::parse_edge_from_gremlin(&Value::Object(edge_json))
    }

    fn create_edges(&self, edges: Vec<CreateEdgeOptions>) -> Result<Vec<Edge>, GraphError> {
        if edges.is_empty() {
            return Ok(vec![]);
        }

        if edges.len() == 1 {
            let mut edges = edges;
            let edge = self.create_edge(edges.pop().unwrap())?;
            return Ok(vec![edge]);
        }

        let mut gremlin = "g.union(".to_string();
        let mut bindings = serde_json::Map::new();
        let mut union_parts = Vec::new();

        for (i, edge_spec) in edges.into_iter().enumerate() {
            let from_binding = format!("from_{i}");
            let to_binding = format!("to_{i}");
            let label_binding = format!("label_{i}");

            let from_id_json = match &edge_spec.from_vertex {
                ElementId::StringValue(s) => json!(s),
                ElementId::Int64(val) => json!(val),
                ElementId::Uuid(u) => json!(u.to_string()),
            };
            bindings.insert(from_binding.clone(), from_id_json);

            let to_id_json = match &edge_spec.to_vertex {
                ElementId::StringValue(s) => json!(s),
                ElementId::Int64(val) => json!(val),
                ElementId::Uuid(u) => json!(u.to_string()),
            };
            bindings.insert(to_binding.clone(), to_id_json);
            bindings.insert(label_binding.clone(), json!(edge_spec.edge_type));

            let mut part =
                format!("V({from_binding}).addE({label_binding}).to(__.V({to_binding}))");

            for (j, (key, value)) in edge_spec
                .properties
                .unwrap_or_default()
                .into_iter()
                .enumerate()
            {
                let key_binding = format!("k_{i}_{j}");
                let val_binding = format!("v_{i}_{j}");
                part.push_str(&format!(".property({key_binding}, {val_binding})"));
                bindings.insert(key_binding, json!(key));
                bindings.insert(val_binding, conversions::to_json_value(value.clone())?);
            }

            union_parts.push(part);
        }

        gremlin.push_str(&union_parts.join(", "));
        gremlin.push_str(").elementMap()");

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let result_data = if let Some(arr) = response.as_array() {
            arr.clone()
        } else if let Some(inner) = response.get("@value").and_then(Value::as_array) {
            inner.clone()
        } else {
            return Err(GraphError::InternalError(
                "Invalid response from Gremlin for create_edges".to_string(),
            ));
        };

        result_data
            .iter()
            .map(helpers::parse_edge_from_gremlin)
            .collect()
    }

    fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, GraphError> {
        let gremlin = "g.E(edge_id).elementMap()".to_string();
        let mut bindings = serde_json::Map::new();
        bindings.insert(
            "edge_id".into(),
            match id.clone() {
                ElementId::StringValue(s) => json!(s),
                ElementId::Int64(i) => json!(i),
                ElementId::Uuid(u) => json!(u.to_string()),
            },
        );

        let resp = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let maybe_row = resp
            .as_array()
            .and_then(|arr| arr.first().cloned())
            .or_else(|| {
                resp.get("@value")
                    .and_then(Value::as_array)
                    .and_then(|arr| arr.first().cloned())
            });
        let row = if let Some(r) = maybe_row {
            r
        } else {
            return Ok(None);
        };

        let mut flat = serde_json::Map::new();
        if row.get("@type") == Some(&json!("g:Map")) {
            let vals = row.get("@value").and_then(Value::as_array).unwrap();
            let mut it = vals.iter();
            while let (Some(kv), Some(vv)) = (it.next(), it.next()) {
                let key = if kv.is_string() {
                    kv.as_str().unwrap().to_string()
                } else if kv.get("@type") == Some(&json!("g:T"))
                    || kv.get("@type") == Some(&json!("g:Direction"))
                {
                    kv.get("@value")
                        .and_then(Value::as_str)
                        .unwrap()
                        .to_string()
                } else {
                    return Err(GraphError::InternalError(
                        "Unexpected key format in Gremlin map".into(),
                    ));
                };

                let val = if vv.is_object() {
                    if vv.get("@type") == Some(&json!("g:Map")) {
                        vv.get("@value").cloned().unwrap()
                    } else {
                        vv.get("@value").cloned().unwrap_or(vv.clone())
                    }
                } else {
                    vv.clone()
                };
                flat.insert(key.clone(), val.clone());
            }
        } else if let Some(obj) = row.as_object() {
            flat = obj.clone();
        } else {
            return Err(GraphError::InternalError(
                "Unexpected Gremlin row format".into(),
            ));
        }

        let mut edge_json = serde_json::Map::new();

        let id_field = &flat["id"];
        let real_id = id_field
            .get("relationId")
            .and_then(Value::as_str)
            .map(|s| json!(s))
            .unwrap_or_else(|| id_field.clone());
        edge_json.insert("id".into(), real_id.clone());

        let lbl = flat["label"].clone();
        edge_json.insert("label".into(), lbl.clone());

        if let Some(arr) = flat.get("OUT").and_then(Value::as_array) {
            let ov = arr[1].get("@value").cloned().unwrap();
            edge_json.insert("outV".into(), ov.clone());
        }
        if let Some(arr) = flat.get("IN").and_then(Value::as_array) {
            let iv = arr[1].get("@value").cloned().unwrap();
            edge_json.insert("inV".into(), iv.clone());
        }

        let mut props = serde_json::Map::new();
        for (k, v) in flat.into_iter() {
            if k != "id" && k != "label" && k != "IN" && k != "OUT" {
                props.insert(k.clone(), v.clone());
            }
        }
        edge_json.insert("properties".into(), Value::Object(props.clone()));

        let edge = helpers::parse_edge_from_gremlin(&Value::Object(edge_json))?;
        Ok(Some(edge))
    }

    fn update_edge(&self, options: UpdateEdgeOptions) -> Result<Edge, GraphError> {
        let id_json = match &options.id {
            ElementId::StringValue(s) => json!(s),
            ElementId::Int64(i) => json!(i),
            ElementId::Uuid(u) => json!(u.to_string()),
        };

        let mut gremlin_update = {
            if options.partial.unwrap_or_default() {
                "g.E(edge_id)".to_string()
            } else {
                "g.E(edge_id).sideEffect(properties().drop())".to_string()
            }
        };
        let mut bindings = serde_json::Map::new();
        bindings.insert("edge_id".to_string(), id_json.clone());

        for (i, (k, v)) in options.properties.iter().enumerate() {
            let kb = format!("k{i}");
            let vb = format!("v{i}");
            gremlin_update.push_str(&format!(".sideEffect(property({kb}, {vb}))"));
            bindings.insert(kb.clone(), json!(k));
            bindings.insert(vb.clone(), conversions::to_json_value(v.clone())?);
        }

        self.api
            .execute(&gremlin_update, Some(Value::Object(bindings)))?;

        let gremlin_fetch = "g.E(edge_id).elementMap()";
        let fetch_bindings = json!({ "edge_id": id_json });

        let resp = self.api.execute(gremlin_fetch, Some(fetch_bindings))?;

        let row = resp
            .as_array()
            .and_then(|arr| arr.first().cloned())
            .or_else(|| {
                resp.get("@value")
                    .and_then(Value::as_array)
                    .and_then(|a| a.first().cloned())
            })
            .ok_or_else(|| GraphError::ElementNotFound(options.id.clone()))?;

        let mut flat = serde_json::Map::new();
        if row.get("@type") == Some(&json!("g:Map")) {
            let vals = row.get("@value").and_then(Value::as_array).unwrap();
            let mut it = vals.iter();
            while let (Some(kv), Some(vv)) = (it.next(), it.next()) {
                let key = if kv.is_string() {
                    kv.as_str().unwrap().to_string()
                } else {
                    kv.get("@value")
                        .and_then(Value::as_str)
                        .unwrap()
                        .to_string()
                };
                let val = if vv.is_object() {
                    vv.get("@value").cloned().unwrap_or(vv.clone())
                } else {
                    vv.clone()
                };
                flat.insert(key.clone(), val.clone());
                log::info!("[update_edge] flat[{key}] = {val:#?}");
            }
        } else if let Some(obj) = row.as_object() {
            flat = obj.clone();
        } else {
            return Err(GraphError::InternalError("Unexpected row format".into()));
        }

        let mut ej = serde_json::Map::new();

        let id_field = &flat["id"];
        let real_id = id_field
            .get("relationId")
            .and_then(Value::as_str)
            .map(|s| json!(s))
            .unwrap_or_else(|| id_field.clone());
        ej.insert("id".into(), real_id.clone());

        ej.insert("label".into(), flat["label"].clone());

        if let Some(arr) = flat.get("OUT").and_then(Value::as_array) {
            let ov = arr[1].get("@value").cloned().unwrap();
            ej.insert("outV".into(), ov.clone());
        }
        if let Some(arr) = flat.get("IN").and_then(Value::as_array) {
            let iv = arr[1].get("@value").cloned().unwrap();
            ej.insert("inV".into(), iv.clone());
        }

        let mut props = serde_json::Map::new();
        for (k, v) in flat.into_iter() {
            if k != "id" && k != "label" && k != "IN" && k != "OUT" {
                props.insert(k.clone(), v.clone());
            }
        }
        ej.insert("properties".into(), Value::Object(props.clone()));

        let edge = helpers::parse_edge_from_gremlin(&Value::Object(ej))?;
        Ok(edge)
    }

    fn delete_edge(&self, id: ElementId) -> Result<(), GraphError> {
        let gremlin = "g.E(edge_id).drop().toList()".to_string();

        let id_json = match id {
            ElementId::StringValue(s) => json!(s),
            ElementId::Int64(i) => json!(i),
            ElementId::Uuid(u) => json!(u.to_string()),
        };
        let mut bindings = serde_json::Map::new();
        bindings.insert("edge_id".to_string(), id_json);

        self.api.execute(&gremlin, Some(Value::Object(bindings)))?;
        Ok(())
    }

    fn find_edges(&self, options: FindEdgesOptions) -> Result<Vec<Edge>, GraphError> {
        let mut gremlin = "g.E()".to_string();
        let mut bindings = serde_json::Map::new();

        if let Some(labels) = options.edge_types {
            if !labels.is_empty() {
                gremlin.push_str(".hasLabel(edge_labels)");
                bindings.insert("edge_labels".to_string(), json!(labels));
            }
        }

        if let Some(filter_conditions) = options.filters {
            for condition in &filter_conditions {
                gremlin.push_str(&query_utils::build_gremlin_filter_step(
                    condition,
                    &mut bindings,
                )?);
            }
        }

        if let Some(sort_specs) = options.sort {
            gremlin.push_str(&query_utils::build_gremlin_sort_clause(&sort_specs));
        }

        if let Some(off) = options.offset {
            gremlin.push_str(&format!(
                ".range({}, {})",
                off,
                off + options.limit.unwrap_or(10_000)
            ));
        } else if let Some(lim) = options.limit {
            gremlin.push_str(&format!(".limit({lim})"));
        }

        gremlin.push_str(".elementMap()");

        let response = self.api.execute(&gremlin, Some(Value::Object(bindings)))?;

        let result_data = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Invalid response from Gremlin for find_edges".to_string())
        })?;

        result_data
            .iter()
            .map(helpers::parse_edge_from_gremlin)
            .collect()
    }

    fn commit(&self) -> Result<(), GraphError> {
        {
            let state = self.state.read().unwrap();
            match *state {
                crate::TransactionState::Committed => return Ok(()),
                crate::TransactionState::RolledBack => {
                    return Err(GraphError::TransactionFailed(
                        "Cannot commit a transaction that has been rolled back".to_string(),
                    ));
                }
                crate::TransactionState::Active => {}
            }
        }

        let result = self.api.commit();

        if result.is_ok() {
            let mut state = self.state.write().unwrap();
            *state = crate::TransactionState::Committed;
        }
        result
    }

    fn rollback(&self) -> Result<(), GraphError> {
        {
            let state = self.state.read().unwrap();
            match *state {
                crate::TransactionState::RolledBack => return Ok(()),
                crate::TransactionState::Committed => {
                    return Err(GraphError::TransactionFailed(
                        "Cannot rollback a transaction that has been committed".to_string(),
                    ));
                }
                crate::TransactionState::Active => {}
            }
        }

        let result = self.api.rollback();

        if result.is_ok() {
            let mut state = self.state.write().unwrap();
            *state = crate::TransactionState::RolledBack;
        }
        result
    }

    fn is_active(&self) -> bool {
        let state = self.state.read().unwrap();
        match *state {
            crate::TransactionState::Active => self.api.is_session_active(),
            crate::TransactionState::Committed | crate::TransactionState::RolledBack => false,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum GraphSONValue {
    Object(GraphSONObject),
    Array(Vec<Value>),
    Primitive(Value),
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct GraphSONObject {
    #[serde(rename = "@type")]
    pub object_type: Option<String>,
    #[serde(rename = "@value")]
    pub value: Option<Value>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONVertex {
    pub id: Option<Value>,
    pub label: Option<String>,
    pub properties: Option<HashMap<String, Vec<GraphSONProperty>>>,
    #[serde(rename = "outV")]
    pub out_v: Option<Value>,
    #[serde(rename = "inV")]
    pub in_v: Option<Value>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONProperty {
    #[allow(dead_code)]
    pub id: Option<String>,
    pub value: Option<Value>,
    #[serde(rename = "@value")]
    pub at_value: Option<GraphSONPropertyValue>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONPropertyValue {
    pub value: Option<Value>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONMap {
    #[serde(rename = "@type")]
    #[allow(dead_code)]
    pub map_type: Option<String>,
    #[serde(rename = "@value")]
    pub value: Option<Vec<Value>>,
}

#[derive(Deserialize, Debug)]
pub struct GraphSONList {
    #[serde(rename = "@type")]
    #[allow(dead_code)]
    pub list_type: Option<String>,
    #[serde(rename = "@value")]
    pub value: Option<Vec<Value>>,
}

fn to_bindings(parameters: QueryParameters) -> Result<serde_json::Map<String, Value>, GraphError> {
    let mut bindings = serde_json::Map::new();
    for (key, value) in parameters {
        let json_value = match value {
            PropertyValue::Float32Value(f) => json!(f),
            PropertyValue::Float64Value(f) => json!(f),
            PropertyValue::Int32(i) => json!(i),
            PropertyValue::Int64(i) => json!(i),
            PropertyValue::Boolean(b) => json!(b),
            PropertyValue::StringValue(s) => json!(s),
            _ => conversions::to_json_value(value)?,
        };

        bindings.insert(key, json_value);
    }
    Ok(bindings)
}

fn extract_result_data(response: &Value) -> Result<Option<&Value>, GraphError> {
    if response.is_array() || response.is_object() {
        Ok(Some(response))
    } else {
        Ok(None)
    }
}

fn parse_graphson_vertex(item: &Value) -> Result<Vec<(String, PropertyValue)>, GraphError> {
    if let Some(value_obj) = item.get("@value") {
        if let Ok(vertex) = serde_json::from_value::<GraphSONVertex>(value_obj.clone()) {
            let mut row = Vec::new();

            if let Some(id_val) = vertex.id {
                if let Ok(id_value) = conversions::from_gremlin_value(&id_val) {
                    row.push(("id".to_string(), id_value));
                }
            }

            if let Some(label) = vertex.label {
                row.push(("label".to_string(), PropertyValue::StringValue(label)));
            }

            if let Some(properties) = vertex.properties {
                for (prop_key, prop_array) in properties {
                    if let Some(first_prop) = prop_array.first() {
                        if let Some(prop_value) = &first_prop.value {
                            if let Ok(converted_value) = conversions::from_gremlin_value(prop_value)
                            {
                                row.push((prop_key, converted_value));
                                continue;
                            }
                        }

                        if let Some(at_value) = &first_prop.at_value {
                            if let Some(actual_value) = &at_value.value {
                                if let Ok(converted_value) =
                                    conversions::from_gremlin_value(actual_value)
                                {
                                    row.push((prop_key, converted_value));
                                }
                            }
                        }
                    }
                }
            }

            if let Some(from_vertex) = vertex.out_v {
                if let Ok(from_value) = conversions::from_gremlin_value(&from_vertex) {
                    row.push(("from".to_string(), from_value));
                }
            }
            if let Some(to_vertex) = vertex.in_v {
                if let Ok(to_value) = conversions::from_gremlin_value(&to_vertex) {
                    row.push(("to".to_string(), to_value));
                }
            }

            return Ok(row);
        }
    }

    Err(GraphError::InternalError(
        "Failed to parse GraphSON vertex/edge".to_string(),
    ))
}

fn parse_graphson_map(item: &Value) -> Result<Vec<(String, PropertyValue)>, GraphError> {
    if let Ok(graphson_map) = serde_json::from_value::<GraphSONMap>(item.clone()) {
        if let Some(map_array) = graphson_map.value {
            let mut row = Vec::new();
            let mut i = 0;

            while i + 1 < map_array.len() {
                if let (Some(key_val), Some(value_val)) = (map_array.get(i), map_array.get(i + 1)) {
                    if let Some(key_str) = key_val.as_str() {
                        let converted_value = if let Ok(graphson_list) =
                            serde_json::from_value::<GraphSONList>(value_val.clone())
                        {
                            if let Some(list_values) = graphson_list.value {
                                if let Some(first_value) = list_values.first() {
                                    conversions::from_gremlin_value(first_value)?
                                } else {
                                    i += 2;
                                    continue;
                                }
                            } else {
                                i += 2;
                                continue;
                            }
                        } else {
                            conversions::from_gremlin_value(value_val)?
                        };

                        row.push((key_str.to_string(), converted_value));
                    }
                }
                i += 2;
            }

            return Ok(row);
        }
    }

    Err(GraphError::InternalError(
        "Failed to parse GraphSON map".to_string(),
    ))
}

fn parse_plain_object(item: &Value) -> Result<Vec<(String, PropertyValue)>, GraphError> {
    if let Some(object_map) = item.as_object() {
        let mut row = Vec::new();

        for (key, gremlin_value) in object_map {
            let converted_value = if let Ok(graphson_list) =
                serde_json::from_value::<GraphSONList>(gremlin_value.clone())
            {
                if let Some(list_values) = graphson_list.value {
                    if let Some(first_value) = list_values.first() {
                        conversions::from_gremlin_value(first_value)?
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            } else if let Some(inner_array) = gremlin_value.as_array() {
                if let Some(actual_value) = inner_array.first() {
                    conversions::from_gremlin_value(actual_value)?
                } else {
                    continue;
                }
            } else {
                conversions::from_gremlin_value(gremlin_value)?
            };

            row.push((key.clone(), converted_value));
        }

        return Ok(row);
    }

    Err(GraphError::InternalError(
        "Expected object for plain map".to_string(),
    ))
}

fn parse_gremlin_response(response: Value) -> Result<QueryResult, GraphError> {
    let result_data = extract_result_data(&response)?.ok_or_else(|| {
        GraphError::InternalError("Invalid response structure from Gremlin".to_string())
    })?;

    let arr = if let Some(graphson_obj) = result_data.as_object() {
        if let Some(value_array) = graphson_obj.get("@value").and_then(|v| v.as_array()) {
            value_array
        } else {
            return Ok(QueryResult::Values(vec![]));
        }
    } else if let Some(direct_array) = result_data.as_array() {
        direct_array
    } else {
        return Ok(QueryResult::Values(vec![]));
    };

    if arr.is_empty() {
        return Ok(QueryResult::Values(vec![]));
    }

    let first_item = arr
        .first()
        .ok_or_else(|| GraphError::InternalError("Empty result array".to_string()))?;

    if !first_item.is_object() {
        let values = arr
            .iter()
            .map(conversions::from_gremlin_value)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(QueryResult::Values(values));
    }

    let obj = first_item
        .as_object()
        .ok_or_else(|| GraphError::InternalError("Expected object in result array".to_string()))?;

    if obj.get("@type") == Some(&Value::String("g:Vertex".to_string()))
        || obj.get("@type") == Some(&Value::String("g:Edge".to_string()))
    {
        let mut maps = Vec::new();
        for item in arr {
            if let Ok(row) = parse_graphson_vertex(item) {
                if !row.is_empty() {
                    maps.push(row);
                }
            }
        }
        Ok(QueryResult::Maps(maps))
    } else if obj.get("@type") == Some(&Value::String("g:Map".to_string())) {
        let mut maps = Vec::new();
        for item in arr {
            if let Ok(row) = parse_graphson_map(item) {
                maps.push(row);
            }
        }
        Ok(QueryResult::Maps(maps))
    } else if obj.contains_key("@type") && obj.contains_key("@value") {
        let values = arr
            .iter()
            .map(conversions::from_gremlin_value)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(QueryResult::Values(values))
    } else {
        let mut maps = Vec::new();
        for item in arr {
            if let Ok(row) = parse_plain_object(item) {
                maps.push(row);
            }
        }
        Ok(QueryResult::Maps(maps))
    }
}

fn id_to_json(id: ElementId) -> Value {
    match id {
        ElementId::StringValue(s) => json!(s),
        ElementId::Int64(i) => json!(i),
        ElementId::Uuid(u) => json!(u.to_string()),
    }
}

fn build_traversal_step(
    dir: &Direction,
    edge_types: &Option<Vec<String>>,
    bindings: &mut serde_json::Map<String, Value>,
) -> String {
    let base = match dir {
        Direction::Outgoing => "outE",
        Direction::Incoming => "inE",
        Direction::Both => "bothE",
    };
    if let Some(labels) = edge_types {
        if !labels.is_empty() {
            let label_bindings: Vec<String> = labels
                .iter()
                .enumerate()
                .map(|(i, label)| {
                    let key = format!("edge_label_{}_{}", bindings.len(), i);
                    bindings.insert(key.clone(), json!(label));
                    key
                })
                .collect();
            let labels_str = label_bindings.join(", ");
            return format!("{base}({labels_str}).otherV()");
        }
    }
    format!("{base}().otherV()")
}
