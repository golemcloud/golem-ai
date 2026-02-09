use crate::client::{Neo4jStatement, Neo4jStatements};
use crate::conversions::{self};
use crate::helpers::{
    element_id_to_key, parse_edge_from_row, parse_vertex_from_neo4j_node, EdgeListProcessor,
    EdgeProcessor, ElementIdHelper, Neo4jResponseProcessor, VertexListProcessor, VertexProcessor,
};
use crate::Transaction;
use golem_ai_graph::model::transactions::{
    CreateEdgeOptions, FindEdgesOptions, FindShortestPathOptions, GetAdjacentVerticesOptions,
    GetNeighborhoodOptions, GetVerticesAtDistanceOptions, Path, QueryExecutionResult, Subgraph,
};
use golem_ai_graph::model::types::{
    CreateVertexOptions, ExecuteQueryOptions, FindAllPathsOptions, FindVerticesOptions,
    GetConnectedEdgesOptions, PathExistsOptions, QueryResult, UpdateEdgeOptions,
    UpdateVertexOptions,
};
use golem_ai_graph::model::{
    errors::GraphError,
    types::{Direction, Edge, ElementId, Vertex},
};
use golem_ai_graph::query_utils::{build_sort_clause, build_where_clause, QuerySyntax};
use golem_ai_graph::TransactionInterface;
use serde_json::Map;
use std::collections::HashMap;

impl Transaction {
    pub(crate) fn execute_schema_query_and_extract_string_list(
        &self,
        query: &str,
    ) -> Result<Vec<String>, GraphError> {
        let statement = Neo4jStatement::with_row_only(query.to_string(), HashMap::new());
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        let result = response.first_result()?;
        result.check_errors()?;

        let mut items = Vec::new();
        for data_item in &result.data {
            if let Some(row) = &data_item.row {
                if let Some(value) = row.first().and_then(|v| v.as_str()) {
                    items.push(value.to_string());
                }
            }
        }
        Ok(items)
    }
}

fn cypher_syntax() -> QuerySyntax {
    QuerySyntax {
        equal: "=",
        not_equal: "<>",
        less_than: "<",
        less_than_or_equal: "<=",
        greater_than: ">",
        greater_than_or_equal: ">=",
        contains: "CONTAINS",
        starts_with: "STARTS WITH",
        ends_with: "ENDS WITH",
        regex_match: "=~",
        param_prefix: "$",
    }
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
        // TODO: not handled:
        //         timeout-seconds: option<u32>,
        //         max-results: option<u32>,
        //         explain: option<bool>,
        //         profile: option<bool>,

        let mut params = HashMap::new();
        if let Some(p) = options.parameters {
            for (key, value) in p {
                params.insert(key, conversions::to_json_value(value)?);
            }
        }

        let statement = Neo4jStatement::new(options.query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        let result = response.first_result()?;
        result.check_errors()?;

        let columns: Vec<String> = result.columns.clone().unwrap_or_default();
        let mut rows = Vec::new();

        for data_item in &result.data {
            if let Some(row_data) = &data_item.row {
                rows.push(row_data.clone());
            }
        }

        let query_result_value = if columns.len() == 1 {
            let mut values = Vec::new();
            for row in rows {
                if let Some(val) = row.first() {
                    values.push(conversions::from_json_value(val.clone())?);
                }
            }
            QueryResult::Values(values)
        } else {
            let mut maps = Vec::new();
            for row in rows {
                let mut map_row = Vec::new();
                for (i, col_name) in columns.iter().enumerate() {
                    if let Some(val) = row.get(i) {
                        map_row
                            .push((col_name.clone(), conversions::from_json_value(val.clone())?));
                    }
                }
                maps.push(map_row);
            }
            QueryResult::Maps(maps)
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
        let mut params = HashMap::new();
        params.insert(
            "from_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&options.from_vertex)),
        );
        params.insert(
            "to_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&options.to_vertex)),
        );

        let query = r#"
            MATCH (a), (b)
            WHERE
              (elementId(a) = $from_id OR id(a) = toInteger($from_id))
              AND
              (elementId(b) = $to_id   OR id(b) = toInteger($to_id))
            MATCH p = shortestPath((a)-[*]-(b))
            RETURN p
        "#
        .to_string();

        let statement = Neo4jStatement::new(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;

        let result = match response.first_result() {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        if !result.errors.is_empty() {
            return Err(GraphError::InternalError(format!(
                "Neo4j error: {:?}",
                result.errors[0]
            )));
        }

        if result.data.is_empty() {
            return Ok(None);
        }

        if let Some(row_data) = result.data.first() {
            if let Some(graph_data) = &row_data.graph {
                let path = crate::helpers::parse_path_from_graph_data(graph_data)?;
                Ok(Some(path))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn find_all_paths(&self, options: FindAllPathsOptions) -> Result<Vec<Path>, GraphError> {
        let path_spec = match options.path {
            Some(path_options) => {
                if path_options.vertex_types.is_some()
                    || path_options.vertex_filters.is_some()
                    || path_options.edge_filters.is_some()
                {
                    return Err(GraphError::UnsupportedOperation(
                        "vertex_types, vertex_filters, and edge_filters are not yet supported in find_all_paths"
                            .to_string(),
                    ));
                }
                let edge_types = path_options.edge_types.map_or("".to_string(), |types| {
                    if types.is_empty() {
                        "".to_string()
                    } else {
                        format!(":{}", types.join("|"))
                    }
                });
                let depth = path_options
                    .max_depth
                    .map_or("*".to_string(), |d| format!("*1..{d}"));
                format!("-[{}]-", format_args!("r{}{}", edge_types, depth))
            }
            None => "-[*]-".to_string(),
        };

        let limit_clause = options
            .limit
            .map_or("".to_string(), |l| format!("LIMIT {l}"));
        let query = format!(
            "MATCH p = (a){path_spec}(b) WHERE elementId(a) = $from_id AND elementId(b) = $to_id RETURN p {limit_clause}"
        );

        let mut params = HashMap::new();
        params.insert(
            "from_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&options.from_vertex)),
        );
        params.insert(
            "to_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&options.to_vertex)),
        );

        let statement = Neo4jStatement::new(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;

        let result = match response.first_result() {
            Ok(r) => r,
            Err(_) => return Ok(vec![]),
        };

        if !result.errors.is_empty() {
            return Err(GraphError::InvalidQuery(format!("{:?}", result.errors[0])));
        }

        let mut paths = Vec::new();
        for row_data in &result.data {
            if let Some(graph_data) = &row_data.graph {
                let path = crate::helpers::parse_path_from_graph_data(graph_data)?;
                paths.push(path);
            }
        }

        Ok(paths)
    }

    fn get_neighborhood(&self, options: GetNeighborhoodOptions) -> Result<Subgraph, GraphError> {
        let (left_arrow, right_arrow) = match options.direction {
            Direction::Outgoing => ("", "->"),
            Direction::Incoming => ("<-", ""),
            Direction::Both => ("-", "-"),
        };

        let edge_type_str = options.edge_types.map_or("".to_string(), |types| {
            if types.is_empty() {
                "".to_string()
            } else {
                format!(":{}", types.join("|"))
            }
        });

        let depth = options.depth;
        let limit_clause = options
            .max_vertices
            .map_or("".to_string(), |l| format!("LIMIT {l}"));

        let query = format!(
            "MATCH p = (c){left_arrow}[r{edge_type_str}*1..{depth}]{right_arrow}(n)\
              WHERE ( elementId(c) = $id OR id(c) = toInteger($id) )\
              RETURN p {limit_clause}"
        );

        let params = ElementIdHelper::to_cypher_parameter(&options.center);
        let statement = Neo4jStatement::new(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;

        let result = match response.first_result() {
            Ok(r) => r,
            Err(_) => {
                return Ok(Subgraph {
                    vertices: vec![],
                    edges: vec![],
                })
            }
        };

        if !result.errors.is_empty() {
            return Err(GraphError::InvalidQuery(format!("{:?}", result.errors[0])));
        }

        let mut all_vertices: HashMap<String, Vertex> = HashMap::new();
        let mut all_edges: HashMap<String, Edge> = HashMap::new();

        for row_data in &result.data {
            if let Some(graph_data) = &row_data.graph {
                let path = crate::helpers::parse_path_from_graph_data(graph_data)?;
                for v in path.vertices {
                    all_vertices.insert(element_id_to_key(&v.id), v);
                }
                for e in path.edges {
                    all_edges.insert(element_id_to_key(&e.id), e);
                }
            }
        }

        Ok(Subgraph {
            vertices: all_vertices.into_values().collect(),
            edges: all_edges.into_values().collect(),
        })
    }

    fn path_exists(&self, options: PathExistsOptions) -> Result<bool, GraphError> {
        self.find_all_paths(FindAllPathsOptions {
            from_vertex: options.from_vertex,
            to_vertex: options.to_vertex,
            path: options.path,
            limit: Some(1),
        })
        .map(|paths| !paths.is_empty())
    }

    fn get_vertices_at_distance(
        &self,
        options: GetVerticesAtDistanceOptions,
    ) -> Result<Vec<Vertex>, GraphError> {
        let (left_arrow, right_arrow) = match options.direction {
            Direction::Outgoing => ("", "->"),
            Direction::Incoming => ("<-", ""),
            Direction::Both => ("-", "-"),
        };

        let edge_type_str = options.edge_types.map_or("".to_string(), |types| {
            if types.is_empty() {
                "".to_string()
            } else {
                format!(":{}", types.join("|"))
            }
        });

        let distance = options.distance;
        let query = format!(
            "MATCH (a){left_arrow}[{edge_type_str}*{distance}]{right_arrow}(b) WHERE elementId(a) = $id RETURN DISTINCT b"
        );

        let params = ElementIdHelper::to_cypher_parameter(&options.source);
        let statement = Neo4jStatement::new(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        VertexListProcessor::process_response(response)
    }

    fn get_adjacent_vertices(
        &self,
        options: GetAdjacentVerticesOptions,
    ) -> Result<Vec<Vertex>, GraphError> {
        let (left_pattern, right_pattern) = match options.direction {
            Direction::Outgoing => ("-", "->"),
            Direction::Incoming => ("<-", "-"),
            Direction::Both => ("-", "-"),
        };

        let edge_type_str = options.edge_types.map_or("".to_string(), |types| {
            if types.is_empty() {
                "".to_string()
            } else {
                format!(":{}", types.join("|"))
            }
        });

        let limit_clause = options
            .limit
            .map_or("".to_string(), |l| format!("LIMIT {l}"));

        let full_query = format!(
            "MATCH (a){left_pattern}[r{edge_type_str}]{right_pattern}(b) WHERE elementId(a) = $id RETURN b {limit_clause}"
        );

        let params = ElementIdHelper::to_cypher_parameter(&options.vertex_id);
        let statement = Neo4jStatement::new(full_query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        VertexListProcessor::process_response(response)
    }

    fn get_connected_edges(
        &self,
        options: GetConnectedEdgesOptions,
    ) -> Result<Vec<Edge>, GraphError> {
        let (left_pattern, right_pattern) = match options.direction {
            Direction::Outgoing => ("-", "->"),
            Direction::Incoming => ("<-", "-"),
            Direction::Both => ("-", "-"),
        };

        let edge_type_str = options.edge_types.map_or("".to_string(), |types| {
            if types.is_empty() {
                "".to_string()
            } else {
                format!(":{}", types.join("|"))
            }
        });

        let limit_clause = options
            .limit
            .map_or("".to_string(), |l| format!("LIMIT {l}"));

        let full_query = format!(
            "MATCH (a){left_pattern}[r{edge_type_str}]{right_pattern}(b) WHERE elementId(a) = $id RETURN elementId(r), type(r), properties(r), elementId(startNode(r)), elementId(endNode(r)) {limit_clause}"
        );

        let params = ElementIdHelper::to_cypher_parameter(&options.vertex_id);
        let statement = Neo4jStatement::with_row_only(full_query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        EdgeListProcessor::process_response(response)
    }

    fn create_vertex(&self, options: CreateVertexOptions) -> Result<Vertex, GraphError> {
        let mut labels = vec![options.vertex_type];
        labels.extend(options.labels.unwrap_or_default());

        let properties_map =
            conversions::to_cypher_properties(options.properties.unwrap_or_default())?;
        let mut params = HashMap::new();
        params.insert(
            "props".to_string(),
            serde_json::Value::Object(properties_map.into_iter().collect()),
        );

        let query = format!("CREATE (n:`{}`) SET n = $props RETURN n", labels.join(":"));

        let statement = Neo4jStatement::new(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        VertexProcessor::process_response(response)
    }

    fn create_vertices(
        &self,
        vertices: Vec<CreateVertexOptions>,
    ) -> Result<Vec<Vertex>, GraphError> {
        if vertices.is_empty() {
            return Ok(vec![]);
        }

        let mut statements = Vec::new();
        for options in vertices {
            let mut labels = vec![options.vertex_type];
            if let Some(l) = options.labels {
                labels.extend(l);
            }
            let cypher_labels = labels.join(":");
            let properties_map =
                conversions::to_cypher_properties(options.properties.unwrap_or_default())?;

            let query = format!("CREATE (n:`{cypher_labels}`) SET n = $props RETURN n");
            let params = [(
                "props".to_string(),
                serde_json::Value::Object(properties_map.into_iter().collect()),
            )]
            .into_iter()
            .collect();

            statements.push(Neo4jStatement::new(query, params));
        }

        let statements_obj = Neo4jStatements::batch(statements);
        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements_obj)?;

        let mut created_vertices = Vec::new();
        for result in response.results.iter() {
            if !result.errors.is_empty() {
                return Err(GraphError::InternalError(format!(
                    "Neo4j error on create_vertices: {:?}",
                    result.errors[0]
                )));
            }

            for row_data in &result.data {
                if let Some(graph_data) = &row_data.graph {
                    for node in &graph_data.nodes {
                        let vertex = parse_vertex_from_neo4j_node(node, None)?;
                        created_vertices.push(vertex);
                    }
                }
            }
        }

        Ok(created_vertices)
    }

    fn get_vertex(&self, id: ElementId) -> Result<Option<Vertex>, GraphError> {
        if let ElementId::StringValue(s) = &id {
            if let Some((prop, value)) = s
                .strip_prefix("prop:")
                .and_then(|rest| rest.split_once(":"))
            {
                let mut params = HashMap::new();
                params.insert(
                    "value".to_string(),
                    serde_json::Value::String(value.to_string()),
                );

                let query = format!("MATCH (n) WHERE n.`{prop}` = $value RETURN n");
                let statement = Neo4jStatement::new(query, params);
                let statements = Neo4jStatements::single(statement);

                let response = self
                    .api
                    .execute_typed_transaction(&self.transaction_url, &statements)?;

                let result = match response.first_result() {
                    Ok(r) => r,
                    Err(_) => return Ok(None),
                };

                if !result.errors.is_empty() {
                    return Ok(None);
                }

                if result.data.is_empty() {
                    return Ok(None);
                }

                return match VertexProcessor::process_response(response) {
                    Ok(vertex) => Ok(Some(vertex)),
                    Err(_) => Ok(None),
                };
            }
        }

        let params = ElementIdHelper::to_cypher_parameter(&id);
        let statement = Neo4jStatement::new(
            "MATCH (n) WHERE elementId(n) = $id RETURN n".to_string(),
            params,
        );
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;

        let result = match response.first_result() {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        if !result.errors.is_empty() {
            return Ok(None);
        }

        if result.data.is_empty() {
            return Ok(None);
        }

        match VertexProcessor::process_response(response) {
            Ok(vertex) => Ok(Some(vertex)),
            Err(_) => Ok(None),
        }
    }

    fn update_vertex(&self, options: UpdateVertexOptions) -> Result<Vertex, GraphError> {
        let properties_map = conversions::to_cypher_properties(options.properties)?;

        let mut params = ElementIdHelper::to_cypher_parameter(&options.id);
        params.insert(
            "props".to_string(),
            serde_json::Value::Object(properties_map.into_iter().collect()),
        );

        let statement = {
            if options.partial.unwrap_or_default() {
                Neo4jStatement::new(
                    "MATCH (n) WHERE elementId(n) = $id SET n += $props RETURN n".to_string(),
                    params,
                )
            } else {
                Neo4jStatement::new(
                    "MATCH (n) WHERE elementId(n) = $id SET n = $props RETURN n".to_string(),
                    params,
                )
            }
        };
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;

        // TODO: handle if there was no matching vertex, and fallback to create
        //       if options.create_missing is set to true
        VertexProcessor::process_response(response)
    }

    fn delete_vertex(&self, id: ElementId, delete_edges: bool) -> Result<(), GraphError> {
        let params = ElementIdHelper::to_cypher_parameter(&id);
        let detach_str = if delete_edges { "DETACH" } else { "" };

        let query = format!("MATCH (n) WHERE elementId(n) = $id {detach_str} DELETE n");
        let statement = Neo4jStatement::with_row_only(query, params);
        let statements = Neo4jStatements::single(statement);

        self.api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        Ok(())
    }

    fn find_vertices(&self, options: FindVerticesOptions) -> Result<Vec<Vertex>, GraphError> {
        let mut params = Map::new();
        let syntax = cypher_syntax();

        let match_clause = match &options.vertex_type {
            Some(vt) => format!("MATCH (n:`{vt}`)"),
            None => "MATCH (n)".to_string(),
        };

        let where_clause = build_where_clause(&options.filters, "n", &mut params, &syntax, |v| {
            conversions::to_json_value(v)
        })?;
        let sort_clause = build_sort_clause(&options.sort, "n");

        let limit_clause = options
            .limit
            .map_or("".to_string(), |l| format!("LIMIT {l}"));
        let offset_clause = options
            .offset
            .map_or("".to_string(), |o| format!("SKIP {o}"));

        let full_query = format!(
            "{match_clause} {where_clause} RETURN n {sort_clause} {offset_clause} {limit_clause}"
        );

        let statement = Neo4jStatement::new(full_query, params.into_iter().collect());
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        VertexListProcessor::process_response(response)
    }

    fn create_edge(&self, options: CreateEdgeOptions) -> Result<Edge, GraphError> {
        let properties_map =
            conversions::to_cypher_properties(options.properties.unwrap_or_default())?;

        let mut params = HashMap::new();
        params.insert(
            "from_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&options.from_vertex)),
        );
        params.insert(
            "to_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&options.to_vertex)),
        );
        params.insert(
            "props".to_string(),
            serde_json::Value::Object(properties_map.into_iter().collect()),
        );

        // TODO: escape? also review other places for possible missing escaping
        let query = format!(
            "MATCH (a) WHERE elementId(a) = $from_id \
             MATCH (b) WHERE elementId(b) = $to_id \
             CREATE (a)-[r:`{}`]->(b) SET r = $props \
             RETURN elementId(r), type(r), properties(r), \
                    elementId(startNode(r)), elementId(endNode(r))",
            options.edge_type
        );

        let statement = Neo4jStatement::with_row_only(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        EdgeProcessor::process_response(response)
    }

    fn create_edges(&self, edges: Vec<CreateEdgeOptions>) -> Result<Vec<Edge>, GraphError> {
        if edges.is_empty() {
            return Ok(vec![]);
        }

        let mut statements = Vec::new();
        for options in edges {
            let properties_map =
                conversions::to_cypher_properties(options.properties.unwrap_or_default())?;

            let mut params = HashMap::new();
            params.insert(
                "from_id".to_string(),
                serde_json::Value::String(ElementIdHelper::to_cypher_value(&options.from_vertex)),
            );
            params.insert(
                "to_id".to_string(),
                serde_json::Value::String(ElementIdHelper::to_cypher_value(&options.to_vertex)),
            );
            params.insert(
                "props".to_string(),
                serde_json::Value::Object(properties_map.into_iter().collect()),
            );

            let query = format!(
                "MATCH (a), (b) WHERE elementId(a) = $from_id AND elementId(b) = $to_id \
                 CREATE (a)-[r:`{}`]->(b) SET r = $props \
                 RETURN elementId(r), type(r), properties(r), elementId(a), elementId(b)",
                options.edge_type
            );

            statements.push(Neo4jStatement::with_row_only(query, params));
        }

        let statements_obj = Neo4jStatements::batch(statements);
        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements_obj)?;

        let mut created_edges = Vec::new();
        for result in response.results.iter() {
            if !result.errors.is_empty() {
                return Err(GraphError::InternalError(format!(
                    "Neo4j error on create_edges: {:?}",
                    result.errors[0]
                )));
            }

            for row_data in &result.data {
                if let Some(row) = &row_data.row {
                    let edge = parse_edge_from_row(row)?;
                    created_edges.push(edge);
                }
            }
        }

        Ok(created_edges)
    }

    fn get_edge(&self, id: ElementId) -> Result<Option<Edge>, GraphError> {
        let params = ElementIdHelper::to_cypher_parameter(&id);

        let query = "MATCH ()-[r]-() WHERE elementId(r) = $id \
                     RETURN elementId(r), type(r), properties(r), \
                            elementId(startNode(r)), elementId(endNode(r))"
            .to_string();

        let statement = Neo4jStatement::with_row_only(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;

        let result = match response.first_result() {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        if !result.errors.is_empty() {
            return Ok(None);
        }

        if result.data.is_empty() {
            return Ok(None);
        }

        match EdgeProcessor::process_response(response) {
            Ok(edge) => Ok(Some(edge)),
            Err(_) => Ok(None),
        }
    }

    fn update_edge(&self, options: UpdateEdgeOptions) -> Result<Edge, GraphError> {
        let properties_map = conversions::to_cypher_properties(options.properties)?;

        let mut params = ElementIdHelper::to_cypher_parameter(&options.id);
        params.insert(
            "props".to_string(),
            serde_json::Value::Object(properties_map.into_iter().collect()),
        );

        let query = {
            if options.partial.unwrap_or_default() {
                "MATCH ()-[r]-() WHERE elementId(r) = $id SET r += $props \
                     RETURN elementId(r), type(r), properties(r), \
                            elementId(startNode(r)), elementId(endNode(r))"
                    .to_string()
            } else {
                "MATCH ()-[r]-() WHERE elementId(r) = $id SET r = $props \
                     RETURN elementId(r), type(r), properties(r), \
                            elementId(startNode(r)), elementId(endNode(r))"
                    .to_string()
            }
        };

        let statement = Neo4jStatement::with_row_only(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        // TODO: handle if there was no matching edge, and fallback to create
        //       if options.create_missing_with is set to true
        EdgeProcessor::process_response(response)
    }

    fn delete_edge(&self, id: ElementId) -> Result<(), GraphError> {
        let params = ElementIdHelper::to_cypher_parameter(&id);

        let query = "MATCH ()-[r]-() WHERE elementId(r) = $id DELETE r".to_string();
        let statement = Neo4jStatement::with_row_only(query, params);
        let statements = Neo4jStatements::single(statement);

        self.api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        Ok(())
    }

    fn find_edges(&self, options: FindEdgesOptions) -> Result<Vec<Edge>, GraphError> {
        let mut params = Map::new();
        let syntax = cypher_syntax();

        let edge_type_str = options.edge_types.map_or("".to_string(), |types| {
            if types.is_empty() {
                "".to_string()
            } else {
                format!(":{}", types.join("|"))
            }
        });

        let match_clause = format!("MATCH ()-[r{}]-()", &edge_type_str);

        let where_clause = build_where_clause(&options.filters, "r", &mut params, &syntax, |v| {
            conversions::to_json_value(v)
        })?;
        let sort_clause = build_sort_clause(&options.sort, "r");

        let limit_clause = options
            .limit
            .map_or("".to_string(), |l| format!("LIMIT {l}"));
        let offset_clause = options
            .offset
            .map_or("".to_string(), |o| format!("SKIP {o}"));

        let full_query = format!(
            "{match_clause} {where_clause} RETURN elementId(r), type(r), properties(r), elementId(startNode(r)), elementId(endNode(r)) {sort_clause} {offset_clause} {limit_clause}"
        );

        let statement = Neo4jStatement::with_row_only(full_query, params.into_iter().collect());
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        EdgeListProcessor::process_response(response)
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

        let result = self.api.commit_transaction(&self.transaction_url);

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

        let result = self.api.rollback_transaction(&self.transaction_url);

        if result.is_ok() {
            let mut state = self.state.write().unwrap();
            *state = crate::TransactionState::RolledBack;
        }

        result
    }

    fn is_active(&self) -> bool {
        let state = self.state.read().unwrap();
        match *state {
            crate::TransactionState::Active => true,
            crate::TransactionState::Committed | crate::TransactionState::RolledBack => false,
        }
    }
}
