use crate::helpers::{
    element_id_to_string, parse_edge_from_document, parse_path_from_document,
    parse_vertex_from_document,
};
use crate::{conversions, helpers, Transaction};
use golem_graph::model::transactions::{
    CreateEdgeOptions, CreateVertexOptions, ExecuteQueryOptions, FindAllPathsOptions,
    FindShortestPathOptions, FindVerticesOptions, GetVerticesAtDistanceOptions, Path,
    PathExistsOptions, QueryExecutionResult, Subgraph,
};
use golem_graph::model::types::{
    FindEdgesOptions, GetAdjacentVerticesOptions, GetConnectedEdgesOptions, GetNeighborhoodOptions,
    QueryResult, UpdateEdgeOptions, UpdateVertexOptions,
};
use golem_graph::model::{
    errors::GraphError,
    types::{Direction, Edge, ElementId, Vertex},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use golem_graph::TransactionInterface;

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
        let mut bind_vars = serde_json::Map::new();
        if let Some(p) = options.parameters {
            for (key, value) in p {
                bind_vars.insert(key, conversions::to_arango_value(value)?);
            }
        }

        let query_json = json!({
            "query": options.query,
            "bindVars": bind_vars,
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query_json)?;

        let result_array = if let Some(array) = response.as_array() {
            array.clone()
        } else {
            let structured_response: Result<ArangoQueryResponse, _> =
                serde_json::from_value(response.clone());
            match structured_response {
                Ok(resp) => resp.result.into_iter().collect(),
                Err(_) => {
                    return Err(GraphError::InternalError(
                        "Unexpected AQL query response format".to_string(),
                    ));
                }
            }
        };

        let query_result_value = if result_array.is_empty() {
            QueryResult::Values(vec![])
        } else {
            self.parse_query_results(result_array)?
        };

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
        let from_id = id_to_aql(&options.from_vertex);
        let to_id = id_to_aql(&options.to_vertex);
        let edge_collections = options.path.and_then(|o| o.edge_types).unwrap_or_default();

        let edge_collections_str = if edge_collections.is_empty() {
            "knows, created".to_string()
        } else {
            edge_collections.join(", ")
        };

        let query_str = format!(
            "FOR vertex, edge IN ANY SHORTEST_PATH @from_id TO @to_id {edge_collections_str} RETURN {{vertex: vertex, edge: edge}}"
        );
        let mut bind_vars = serde_json::Map::new();
        bind_vars.insert("from_id".to_string(), json!(from_id));
        bind_vars.insert("to_id".to_string(), json!(to_id));

        let request = json!({
            "query": query_str,
            "bindVars": Value::Object(bind_vars.clone()),
        });
        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, request)?;
        let arr = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Invalid response for shortest path".to_string())
        })?;

        if arr.is_empty() {
            return Ok(None);
        }

        let mut vertices = vec![];
        let mut edges = vec![];

        for item in arr {
            if let Some(obj) = item.as_object() {
                if let Some(v_doc) = obj.get("vertex").and_then(|v| v.as_object()) {
                    let coll = v_doc
                        .get("_id")
                        .and_then(|id| id.as_str())
                        .and_then(|s| s.split('/').next())
                        .unwrap_or_default();
                    let vertex = parse_vertex_from_document(v_doc, coll)?;
                    vertices.push(vertex);
                }
                if let Some(e_doc) = obj.get("edge").and_then(|e| e.as_object()) {
                    let coll = e_doc
                        .get("_id")
                        .and_then(|id| id.as_str())
                        .and_then(|s| s.split('/').next())
                        .unwrap_or_default();
                    let edge = parse_edge_from_document(e_doc, coll)?;
                    edges.push(edge);
                }
            }
        }

        let length = edges.len() as u32;
        Ok(Some(Path {
            vertices,
            edges,
            length,
        }))
    }

    fn find_all_paths(&self, options: FindAllPathsOptions) -> Result<Vec<Path>, GraphError> {
        if let Some(path_opts) = &options.path {
            if path_opts.vertex_types.is_some()
                || path_opts.vertex_filters.is_some()
                || path_opts.edge_filters.is_some()
            {
                return Err(GraphError::UnsupportedOperation(
                    "vertex_types, vertex_filters, and edge_filters are not supported".to_string(),
                ));
            }
        }

        let from_id = id_to_aql(&options.from_vertex);
        let to_id = id_to_aql(&options.to_vertex);
        let (min_depth, max_depth) = options
            .path
            .as_ref()
            .and_then(|o| o.max_depth)
            .map_or((1, 10), |d| (1, d));
        let edge_collections = options.path.and_then(|o| o.edge_types).unwrap_or_default();

        let edge_collections_str = if edge_collections.is_empty() {
            "knows, created".to_string()
        } else {
            edge_collections.join(", ")
        };
        let limit_clause = options
            .limit
            .map_or(String::new(), |l| format!("LIMIT {l}"));

        let query_str = format!(
            "FOR v, e, p IN {min_depth}..{max_depth} OUTBOUND @from_id {edge_collections_str} OPTIONS {{uniqueVertices: 'path'}} FILTER v._id == @to_id {limit_clause} RETURN {{vertices: p.vertices, edges: p.edges}}"
        );
        let request = json!({
            "query": query_str,
            "bindVars": { "from_id": from_id, "to_id": to_id }
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, request)?;
        let arr = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Invalid response for all paths".to_string())
        })?;

        arr.iter()
            .filter_map(|v| v.as_object())
            .map(parse_path_from_document)
            .collect()
    }

    fn get_neighborhood(&self, options: GetNeighborhoodOptions) -> Result<Subgraph, GraphError> {
        let center_id = id_to_aql(&options.center);
        let dir_str = match options.direction {
            Direction::Outgoing => "OUTBOUND",
            Direction::Incoming => "INBOUND",
            Direction::Both => "ANY",
        };
        let edge_collections = options.edge_types.unwrap_or_default();
        let edge_collections_str = if edge_collections.is_empty() {
            "knows, created".to_string()
        } else {
            edge_collections.join(", ")
        };
        let limit_clause = options
            .max_vertices
            .map_or(String::new(), |l| format!("LIMIT {l}"));

        let query_str = format!(
            "FOR v, e IN 1..{} {} @center_id {} {} RETURN {{vertex: v, edge: e}}",
            options.depth, dir_str, edge_collections_str, limit_clause
        );
        let request = json!({
            "query": query_str,
            "bindVars": { "center_id": center_id }
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, request)?;
        let arr = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Invalid response for neighborhood".to_string())
        })?;

        let mut verts = HashMap::new();
        let mut edges = HashMap::new();
        for item in arr {
            if let Some(obj) = item.as_object() {
                if let Some(v_doc) = obj.get("vertex").and_then(|v| v.as_object()) {
                    let coll = v_doc
                        .get("_id")
                        .and_then(|id| id.as_str())
                        .and_then(|s| s.split('/').next())
                        .unwrap_or_default();
                    let vert = parse_vertex_from_document(v_doc, coll)?;
                    verts.insert(element_id_to_string(&vert.id), vert);
                }
                if let Some(e_doc) = obj.get("edge").and_then(|e| e.as_object()) {
                    let coll = e_doc
                        .get("_id")
                        .and_then(|id| id.as_str())
                        .and_then(|s| s.split('/').next())
                        .unwrap_or_default();
                    let edge = parse_edge_from_document(e_doc, coll)?;
                    edges.insert(element_id_to_string(&edge.id), edge);
                }
            }
        }

        Ok(Subgraph {
            vertices: verts.into_values().collect(),
            edges: edges.into_values().collect(),
        })
    }

    fn path_exists(&self, options: PathExistsOptions) -> Result<bool, GraphError> {
        Ok(!self
            .find_all_paths(FindAllPathsOptions {
                from_vertex: options.from_vertex,
                to_vertex: options.to_vertex,
                path: options.path,
                limit: Some(1),
            })?
            .is_empty())
    }

    fn get_vertices_at_distance(
        &self,
        options: GetVerticesAtDistanceOptions,
    ) -> Result<Vec<Vertex>, GraphError> {
        let start = id_to_aql(&options.source);
        let dir_str = match options.direction {
            Direction::Outgoing => "OUTBOUND",
            Direction::Incoming => "INBOUND",
            Direction::Both => "ANY",
        };
        let edge_collections = options.edge_types.unwrap_or_default();
        let edge_collections_str = if edge_collections.is_empty() {
            "knows, created".to_string()
        } else {
            edge_collections.join(", ")
        };

        let distance = options.distance;
        let query_str = format!(
            "FOR v IN {distance}..{distance} {dir_str} @start {edge_collections_str} RETURN v"
        );
        let request = json!({ "query": query_str, "bindVars": { "start": start } });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, request)?;
        let arr = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Invalid response for vertices at distance".to_string())
        })?;

        arr.iter()
            .filter_map(|v| v.as_object())
            .map(|doc| {
                let coll = doc
                    .get("_id")
                    .and_then(|id| id.as_str())
                    .and_then(|s| s.split('/').next())
                    .unwrap_or_default();
                parse_vertex_from_document(doc, coll)
            })
            .collect()
    }

    fn get_adjacent_vertices(
        &self,
        options: GetAdjacentVerticesOptions,
    ) -> Result<Vec<Vertex>, GraphError> {
        let start_node = element_id_to_string(&options.vertex_id);
        let dir_str = match options.direction {
            Direction::Outgoing => "OUTBOUND",
            Direction::Incoming => "INBOUND",
            Direction::Both => "ANY",
        };

        let collections = options.edge_types.unwrap_or_default().join(", ");

        let query = json!({
            "query": format!(
                "FOR v IN 1..1 {} @start_node {} RETURN v",
                dir_str,
                collections
            ),
            "bindVars": {
                "start_node": start_node,
            }
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query)?;
        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;

        let mut vertices = vec![];
        for val in result_array {
            if let Some(doc) = val.as_object() {
                let collection = doc
                    .get("_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.split('/').next())
                    .unwrap_or_default();
                let vertex = parse_vertex_from_document(doc, collection)?;
                vertices.push(vertex);
            }
        }

        Ok(vertices)
    }

    fn get_connected_edges(
        &self,
        options: GetConnectedEdgesOptions,
    ) -> Result<Vec<Edge>, GraphError> {
        let start_node = element_id_to_string(&options.vertex_id);
        let dir_str = match options.direction {
            Direction::Outgoing => "OUTBOUND",
            Direction::Incoming => "INBOUND",
            Direction::Both => "ANY",
        };

        let collections = options.edge_types.unwrap_or_default().join(", ");

        let query = json!({
            "query": format!(
                "FOR v, e IN 1..1 {} @start_node {} RETURN e",
                dir_str,
                collections
            ),
            "bindVars": {
                "start_node": start_node,
            }
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query)?;
        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;

        let mut edges = vec![];
        for val in result_array {
            if let Some(doc) = val.as_object() {
                let collection = doc
                    .get("_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.split('/').next())
                    .unwrap_or_default();
                let edge = parse_edge_from_document(doc, collection)?;
                edges.push(edge);
            }
        }

        Ok(edges)
    }

    fn create_vertex(&self, options: CreateVertexOptions) -> Result<Vertex, GraphError> {
        if !options.labels.map(|l| l.is_empty()).unwrap_or_default() {
            return Err(GraphError::UnsupportedOperation(
                "ArangoDB does not support multiple labels per vertex. Use vertex collections instead."
                    .to_string(),
            ));
        }

        let props = conversions::to_arango_properties(options.properties.unwrap_or_default())?;

        let query = json!({
            "query": "INSERT @props INTO @@collection OPTIONS { ignoreErrors: false } RETURN NEW",
            "bindVars": {
                "props": props,
                "@collection": options.vertex_type
            }
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query)?;

        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;
        let vertex_doc = result_array
            .first()
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                GraphError::InternalError("Missing vertex document in response".to_string())
            })?;

        parse_vertex_from_document(vertex_doc, &options.vertex_type)
    }

    fn create_vertices(
        &self,
        vertices: Vec<CreateVertexOptions>,
    ) -> Result<Vec<Vertex>, GraphError> {
        let mut created_vertices = vec![];
        for vertex_options in vertices {
            let vertex = self.create_vertex(vertex_options)?;
            created_vertices.push(vertex);
        }
        Ok(created_vertices)
    }

    fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, GraphError> {
        let key = helpers::element_id_to_key(&id)?;
        let collection = if let ElementId::StringValue(s) = &id {
            s.split('/').next().unwrap_or_default()
        } else {
            ""
        };

        if collection.is_empty() {
            return Err(GraphError::InvalidQuery(
                "ElementId for get_vertex must be a full _id (e.g., 'collection/key')".to_string(),
            ));
        }

        let query = json!({
            "query": "RETURN DOCUMENT(@@collection, @key)",
            "bindVars": {
                "@collection": collection,
                "key": key
            }
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query)?;

        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;

        if let Some(vertex_doc) = result_array.first().and_then(|v| v.as_object()) {
            if vertex_doc.is_empty() || result_array.first().unwrap().is_null() {
                return Ok(None);
            }
            let vertex = parse_vertex_from_document(vertex_doc, collection)?;
            Ok(Some(vertex))
        } else {
            Ok(None)
        }
    }

    fn update_vertex(&self, options: UpdateVertexOptions) -> Result<Vertex, GraphError> {
        let key = helpers::element_id_to_key(&options.id)?;
        let collection = helpers::collection_from_element_id(&options.id)?;

        let props = conversions::to_arango_properties(options.properties)?;

        let query = {
            if options.partial.unwrap_or_default() {
                json!({
                    "query": "UPDATE @key WITH @props IN @@collection OPTIONS { keepNull: false, mergeObjects: true } RETURN NEW",
                    "bindVars": {
                        "key": key,
                        "props": props,
                        "@collection": collection
                    }
                })
            } else {
                json!({
                    "query": "REPLACE @key WITH @props IN @@collection RETURN NEW",
                    "bindVars": {
                        "key": key,
                        "props": props,
                        "@collection": collection
                    }
                })
            }
        };

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query)?;
        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;
        let vertex_doc = result_array
            .first()
            .and_then(|v| v.as_object())
            .ok_or_else(|| GraphError::ElementNotFound(options.id.clone()))?;

        // TODO: handle upsert if create_missing is defined

        helpers::parse_vertex_from_document(vertex_doc, collection)
    }

    fn delete_vertex(&self, id: ElementId, delete_edges: bool) -> Result<(), GraphError> {
        let key = helpers::element_id_to_key(&id)?;
        let collection = if let ElementId::StringValue(s) = &id {
            s.split('/').next().unwrap_or_default()
        } else {
            ""
        };

        if collection.is_empty() {
            return Err(GraphError::InvalidQuery(
                "ElementId for delete_vertex must be a full _id (e.g., 'collection/key')"
                    .to_string(),
            ));
        }

        if delete_edges {
            let vertex_id = helpers::element_id_to_string(&id);

            let collections = self.api.list_collections().unwrap_or_default();
            let edge_collections: Vec<_> = collections
                .iter()
                .filter(|c| {
                    matches!(
                        c.container_type,
                        golem_graph::model::schema::ContainerType::EdgeContainer
                    )
                })
                .map(|c| c.name.clone())
                .collect();

            for edge_collection in edge_collections {
                let delete_edges_query = json!({
                    "query": "FOR e IN @@collection FILTER e._from == @vertex_id OR e._to == @vertex_id REMOVE e IN @@collection",
                    "bindVars": {
                        "vertex_id": vertex_id,
                        "@collection": edge_collection
                    }
                });
                let _ = self
                    .api
                    .execute_in_transaction(&self.transaction_id, delete_edges_query);
            }
        }

        let simple_query = json!({
            "query": "REMOVE @key IN @@collection",
            "bindVars": {
                "key": key,
                "@collection": collection
            }
        });

        self.api
            .execute_in_transaction(&self.transaction_id, simple_query)?;
        Ok(())
    }

    fn find_vertices(&self, options: FindVerticesOptions) -> Result<Vec<Vertex>, GraphError> {
        let collection = options.vertex_type.ok_or_else(|| {
            GraphError::InvalidQuery("vertex_type must be provided for find_vertices".to_string())
        })?;

        let mut query_parts = vec![format!("FOR v IN @@collection")];
        let mut bind_vars = serde_json::Map::new();
        bind_vars.insert("@collection".to_string(), json!(collection.clone()));

        let where_clause = golem_graph::query_utils::build_where_clause(
            &options.filters,
            "v",
            &mut bind_vars,
            &aql_syntax(),
            conversions::to_arango_value,
        )?;
        if !where_clause.is_empty() {
            query_parts.push(where_clause);
        }

        let sort_clause = golem_graph::query_utils::build_sort_clause(&options.sort, "v");
        if !sort_clause.is_empty() {
            query_parts.push(sort_clause);
        }

        let limit_val = options.limit.unwrap_or(100);
        let offset_val = options.offset.unwrap_or(0);
        query_parts.push(format!("LIMIT {offset_val}, {limit_val}"));
        query_parts.push("RETURN v".to_string());

        let full_query = query_parts.join(" ");

        let query_json = json!({
            "query": full_query,
            "bindVars": bind_vars
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query_json)?;

        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;

        let mut vertices = vec![];
        for val in result_array {
            if let Some(doc) = val.as_object() {
                let vertex = helpers::parse_vertex_from_document(doc, &collection)?;
                vertices.push(vertex);
            }
        }

        Ok(vertices)
    }

    fn create_edge(&self, options: CreateEdgeOptions) -> Result<Edge, GraphError> {
        let props = conversions::to_arango_properties(options.properties.unwrap_or_default())?;
        let from_id = helpers::element_id_to_string(&options.from_vertex);
        let to_id = helpers::element_id_to_string(&options.to_vertex);

        let query = json!({
            "query": "INSERT MERGE({ _from: @from, _to: @to }, @props) INTO @@collection RETURN NEW",
            "bindVars": {
                "from": from_id,
                "to": to_id,
                "props": props,
                "@collection": options.edge_type
            }
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query)?;
        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;
        let edge_doc = result_array
            .first()
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                GraphError::InternalError("Missing edge document in response".to_string())
            })?;

        helpers::parse_edge_from_document(edge_doc, &options.edge_type)
    }

    fn create_edges(&self, edges: Vec<CreateEdgeOptions>) -> Result<Vec<Edge>, GraphError> {
        let mut created_edges = vec![];
        for edge_options in edges {
            let edge = self.create_edge(edge_options)?;
            created_edges.push(edge);
        }
        Ok(created_edges)
    }

    fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, GraphError> {
        let key = helpers::element_id_to_key(&id)?;
        let collection = if let ElementId::StringValue(s) = &id {
            s.split('/').next().unwrap_or_default()
        } else {
            ""
        };

        if collection.is_empty() {
            return Err(GraphError::InvalidQuery(
                "ElementId for get_edge must be a full _id (e.g., 'collection/key')".to_string(),
            ));
        }

        let query = json!({
            "query": "RETURN DOCUMENT(@@collection, @key)",
            "bindVars": {
                "@collection": collection,
                "key": key
            }
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query)?;
        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;

        if let Some(edge_doc) = result_array.first().and_then(|v| v.as_object()) {
            if edge_doc.is_empty() || result_array.first().unwrap().is_null() {
                return Ok(None);
            }
            let edge = helpers::parse_edge_from_document(edge_doc, collection)?;
            Ok(Some(edge))
        } else {
            Ok(None)
        }
    }

    fn update_edge(&self, options: UpdateEdgeOptions) -> Result<Edge, GraphError> {
        let key = helpers::element_id_to_key(&options.id)?;
        let collection = helpers::collection_from_element_id(&options.id)?;

        let current_edge = self
            .get_edge(options.id.clone())?
            .ok_or_else(|| GraphError::ElementNotFound(options.id.clone()))?;

        let mut props = conversions::to_arango_properties(options.properties)?;
        props.insert(
            "_from".to_string(),
            json!(helpers::element_id_to_string(&current_edge.from_vertex)),
        );
        props.insert(
            "_to".to_string(),
            json!(helpers::element_id_to_string(&current_edge.to_vertex)),
        );

        let query = {
            if options.partial.unwrap_or_default() {
                json!({
                    "query": "UPDATE @key WITH @props IN @@collection OPTIONS { keepNull: false, mergeObjects: true } RETURN NEW",
                    "bindVars": {
                        "key": key,
                        "props": props,
                        "@collection": collection
                    }
                })
            } else {
                json!({
                    "query": "REPLACE @key WITH @props IN @@collection RETURN NEW",
                    "bindVars": {
                        "key": key,
                        "props": props,
                        "@collection": collection,
                    }
                })
            }
        };

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query)?;

        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;

        let edge_doc = result_array
            .first()
            .and_then(|v| v.as_object())
            .ok_or_else(|| GraphError::ElementNotFound(options.id.clone()))?;

        helpers::parse_edge_from_document(edge_doc, collection)
    }

    fn delete_edge(&self, id: ElementId) -> Result<(), GraphError> {
        let key = helpers::element_id_to_key(&id)?;
        let collection = if let ElementId::StringValue(s) = &id {
            s.split('/').next().unwrap_or_default()
        } else {
            ""
        };

        if collection.is_empty() {
            return Err(GraphError::InvalidQuery(
                "ElementId for delete_edge must be a full _id (e.g., 'collection/key')".to_string(),
            ));
        }

        let query = json!({
            "query": "REMOVE @key IN @@collection",
            "bindVars": {
                "key": key,
                "@collection": collection
            }
        });

        self.api
            .execute_in_transaction(&self.transaction_id, query)?;
        Ok(())
    }

    fn find_edges(&self, options: FindEdgesOptions) -> Result<Vec<Edge>, GraphError> {
        let collection = options
            .edge_types
            .and_then(|mut et| et.pop())
            .ok_or_else(|| {
                GraphError::InvalidQuery("An edge_type must be provided for find_edges".to_string())
            })?;

        let mut query_parts = vec![format!("FOR e IN @@collection")];
        let mut bind_vars = serde_json::Map::new();
        bind_vars.insert("@collection".to_string(), json!(collection.clone()));

        let where_clause = golem_graph::query_utils::build_where_clause(
            &options.filters,
            "e",
            &mut bind_vars,
            &aql_syntax(),
            conversions::to_arango_value,
        )?;
        if !where_clause.is_empty() {
            query_parts.push(where_clause);
        }

        let sort_clause = golem_graph::query_utils::build_sort_clause(&options.sort, "e");
        if !sort_clause.is_empty() {
            query_parts.push(sort_clause);
        }

        let limit_val = options.limit.unwrap_or(100);
        let offset_val = options.offset.unwrap_or(0);
        query_parts.push(format!("LIMIT {offset_val}, {limit_val}"));
        query_parts.push("RETURN e".to_string());

        let full_query = query_parts.join(" ");

        let query_json = json!({
            "query": full_query,
            "bindVars": bind_vars
        });

        let response = self
            .api
            .execute_in_transaction(&self.transaction_id, query_json)?;

        let result_array = response.as_array().ok_or_else(|| {
            GraphError::InternalError("Expected array in AQL response".to_string())
        })?;

        let mut edges = vec![];
        for val in result_array {
            if let Some(doc) = val.as_object() {
                let edge = helpers::parse_edge_from_document(doc, &collection)?;
                edges.push(edge);
            }
        }

        Ok(edges)
    }

    fn commit(&self) -> Result<(), GraphError> {
        self.api.commit_transaction(&self.transaction_id)
    }

    fn rollback(&self) -> Result<(), GraphError> {
        self.api.rollback_transaction(&self.transaction_id)
    }

    fn is_active(&self) -> bool {
        self.api
            .get_transaction_status(&self.transaction_id)
            .map(|status| status == "running")
            .unwrap_or(false)
    }
}

impl Transaction {
    fn parse_query_results(&self, result_array: Vec<Value>) -> Result<QueryResult, GraphError> {
        if let Some(first_value) = result_array.first() {
            if let Ok(first_doc) = serde_json::from_value::<ArangoDocument>(first_value.clone()) {
                if first_doc.is_edge() {
                    let mut edges = Vec::new();
                    for item in result_array {
                        if let Ok(doc) = serde_json::from_value::<ArangoDocument>(item) {
                            let collection = doc.extract_collection();
                            let mut doc_map = serde_json::Map::new();
                            if let Some(id) = doc.id {
                                doc_map.insert("_id".to_string(), Value::String(id));
                            }
                            if let Some(from) = doc.from {
                                doc_map.insert("_from".to_string(), Value::String(from));
                            }
                            if let Some(to) = doc.to {
                                doc_map.insert("_to".to_string(), Value::String(to));
                            }
                            for (key, value) in doc.properties {
                                doc_map.insert(key, value);
                            }
                            edges.push(crate::helpers::parse_edge_from_document(
                                &doc_map,
                                &collection,
                            )?);
                        }
                    }
                    return Ok(QueryResult::Edges(edges));
                } else if first_doc.is_vertex() {
                    let mut vertices = Vec::new();
                    for item in result_array {
                        if let Ok(doc) = serde_json::from_value::<ArangoDocument>(item) {
                            let collection = doc.extract_collection();
                            let mut doc_map = serde_json::Map::new();
                            if let Some(id) = doc.id {
                                doc_map.insert("_id".to_string(), Value::String(id));
                            }
                            if let Some(key) = doc.key {
                                doc_map.insert("_key".to_string(), Value::String(key));
                            }
                            for (key, value) in doc.properties {
                                doc_map.insert(key, value);
                            }
                            vertices.push(crate::helpers::parse_vertex_from_document(
                                &doc_map,
                                &collection,
                            )?);
                        }
                    }
                    return Ok(QueryResult::Vertices(vertices));
                }
            }

            if first_value.is_object() {
                let mut maps = Vec::new();
                for item in result_array {
                    if let Some(obj) = item.as_object() {
                        let mut map_row = Vec::new();
                        for (key, value) in obj {
                            map_row.push((
                                key.clone(),
                                conversions::from_arango_value(value.clone())?,
                            ));
                        }
                        maps.push(map_row);
                    }
                }
                return Ok(QueryResult::Maps(maps));
            }
        }

        let mut values = Vec::new();
        for item in result_array {
            values.push(conversions::from_arango_value(item)?);
        }
        Ok(QueryResult::Values(values))
    }
}

fn aql_syntax() -> golem_graph::query_utils::QuerySyntax {
    golem_graph::query_utils::QuerySyntax {
        equal: "==",
        not_equal: "!=",
        less_than: "<",
        less_than_or_equal: "<=",
        greater_than: ">",
        greater_than_or_equal: ">=",
        contains: "CONTAINS",
        starts_with: "STARTS_WITH",
        ends_with: "ENDS_WITH",
        regex_match: "=~",
        param_prefix: "@",
    }
}

#[derive(Deserialize, Debug)]
pub struct ArangoQueryResponse {
    pub result: Vec<Value>,
}

#[derive(Deserialize, Debug)]
pub struct ArangoDocument {
    #[serde(rename = "_id")]
    pub id: Option<String>,
    #[serde(rename = "_key")]
    pub key: Option<String>,
    #[serde(rename = "_from")]
    pub from: Option<String>,
    #[serde(rename = "_to")]
    pub to: Option<String>,
    #[serde(flatten)]
    pub properties: HashMap<String, Value>,
}

impl ArangoDocument {
    pub fn is_edge(&self) -> bool {
        self.from.is_some() && self.to.is_some()
    }

    pub fn is_vertex(&self) -> bool {
        self.id.is_some() && !self.is_edge()
    }

    pub fn extract_collection(&self) -> String {
        if let Some(id) = &self.id {
            id.split('/').next().unwrap_or_default().to_string()
        } else {
            String::new()
        }
    }
}

fn id_to_aql(id: &ElementId) -> String {
    element_id_to_string(id)
}
