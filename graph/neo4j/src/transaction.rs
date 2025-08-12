use crate::client::{Neo4jStatement, Neo4jStatements};
use crate::conversions::{self};
use crate::helpers::{
    parse_edge_from_row, parse_vertex_from_neo4j_node, EdgeListProcessor, EdgeProcessor,
    ElementIdHelper, Neo4jResponseProcessor, VertexListProcessor, VertexProcessor,
};
use crate::Transaction;
use golem_graph::golem::graph::{
    errors::GraphError,
    transactions::{EdgeSpec, GuestTransaction, VertexSpec},
    types::{Direction, Edge, ElementId, FilterCondition, PropertyMap, SortSpec, Vertex},
};
use golem_graph::query_utils::{build_sort_clause, build_where_clause, QuerySyntax};
use serde_json::{json, Map};
use std::collections::HashMap;

impl Transaction {
    pub(crate) fn commit(&self) -> Result<(), GraphError> {
        self.api.commit_transaction(&self.transaction_url)
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

impl GuestTransaction for Transaction {
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

    fn create_vertex(
        &self,
        vertex_type: String,
        properties: PropertyMap,
    ) -> Result<Vertex, GraphError> {
        self.create_vertex_with_labels(vertex_type, vec![], properties)
    }

    fn create_vertex_with_labels(
        &self,
        vertex_type: String,
        additional_labels: Vec<String>,
        properties: PropertyMap,
    ) -> Result<Vertex, GraphError> {
        let mut labels = vec![vertex_type];
        labels.extend(additional_labels);

        let properties_map = conversions::to_cypher_properties(properties)?;
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

    fn update_vertex(&self, id: ElementId, properties: PropertyMap) -> Result<Vertex, GraphError> {
        let properties_map = conversions::to_cypher_properties(properties)?;

        let mut params = ElementIdHelper::to_cypher_parameter(&id);
        params.insert(
            "props".to_string(),
            serde_json::Value::Object(properties_map.into_iter().collect()),
        );

        let statement = Neo4jStatement::new(
            "MATCH (n) WHERE elementId(n) = $id SET n = $props RETURN n".to_string(),
            params,
        );
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        VertexProcessor::process_response(response)
    }

    fn update_vertex_properties(
        &self,
        id: ElementId,
        updates: PropertyMap,
    ) -> Result<Vertex, GraphError> {
        let properties_map = conversions::to_cypher_properties(updates)?;

        let mut params = ElementIdHelper::to_cypher_parameter(&id);
        params.insert(
            "props".to_string(),
            serde_json::Value::Object(properties_map.into_iter().collect()),
        );

        let statement = Neo4jStatement::new(
            "MATCH (n) WHERE elementId(n) = $id SET n += $props RETURN n".to_string(),
            params,
        );
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
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

    fn find_vertices(
        &self,
        vertex_type: Option<String>,
        filters: Option<Vec<FilterCondition>>,
        sort: Option<Vec<SortSpec>>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Vertex>, GraphError> {
        let mut params = Map::new();
        let syntax = cypher_syntax();

        let match_clause = match &vertex_type {
            Some(vt) => format!("MATCH (n:`{vt}`)"),
            None => "MATCH (n)".to_string(),
        };

        let where_clause = build_where_clause(&filters, "n", &mut params, &syntax, |v| {
            conversions::to_json_value(v)
        })?;
        let sort_clause = build_sort_clause(&sort, "n");

        let limit_clause = limit.map_or("".to_string(), |l| format!("LIMIT {l}"));
        let offset_clause = offset.map_or("".to_string(), |o| format!("SKIP {o}"));

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

    fn create_edge(
        &self,
        edge_type: String,
        from_vertex: ElementId,
        to_vertex: ElementId,
        properties: PropertyMap,
    ) -> Result<Edge, GraphError> {
        let properties_map = conversions::to_cypher_properties(properties)?;

        let mut params = HashMap::new();
        params.insert(
            "from_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&from_vertex)),
        );
        params.insert(
            "to_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&to_vertex)),
        );
        params.insert(
            "props".to_string(),
            serde_json::Value::Object(properties_map.into_iter().collect()),
        );

        let query = format!(
            "MATCH (a) WHERE elementId(a) = $from_id \
             MATCH (b) WHERE elementId(b) = $to_id \
             CREATE (a)-[r:`{edge_type}`]->(b) SET r = $props \
             RETURN elementId(r), type(r), properties(r), \
                    elementId(startNode(r)), elementId(endNode(r))"
        );

        let statement = Neo4jStatement::with_row_only(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        EdgeProcessor::process_response(response)
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

    fn update_edge(&self, id: ElementId, properties: PropertyMap) -> Result<Edge, GraphError> {
        let properties_map = conversions::to_cypher_properties(properties)?;

        let mut params = ElementIdHelper::to_cypher_parameter(&id);
        params.insert(
            "props".to_string(),
            serde_json::Value::Object(properties_map.into_iter().collect()),
        );

        let query = "MATCH ()-[r]-() WHERE elementId(r) = $id SET r = $props \
                     RETURN elementId(r), type(r), properties(r), \
                            elementId(startNode(r)), elementId(endNode(r))"
            .to_string();

        let statement = Neo4jStatement::with_row_only(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        EdgeProcessor::process_response(response)
    }

    fn update_edge_properties(
        &self,
        id: ElementId,
        updates: PropertyMap,
    ) -> Result<Edge, GraphError> {
        let properties_map = conversions::to_cypher_properties(updates)?;

        let mut params = ElementIdHelper::to_cypher_parameter(&id);
        params.insert(
            "props".to_string(),
            serde_json::Value::Object(properties_map.into_iter().collect()),
        );

        let query = "MATCH ()-[r]-() WHERE elementId(r) = $id SET r += $props \
                     RETURN elementId(r), type(r), properties(r), \
                            elementId(startNode(r)), elementId(endNode(r))"
            .to_string();

        let statement = Neo4jStatement::with_row_only(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
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

    fn find_edges(
        &self,
        edge_types: Option<Vec<String>>,
        filters: Option<Vec<FilterCondition>>,
        sort: Option<Vec<SortSpec>>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Edge>, GraphError> {
        let mut params = Map::new();
        let syntax = cypher_syntax();

        let edge_type_str = edge_types.map_or("".to_string(), |types| {
            if types.is_empty() {
                "".to_string()
            } else {
                format!(":{}", types.join("|"))
            }
        });

        let match_clause = format!("MATCH ()-[r{}]-()", &edge_type_str);

        let where_clause = build_where_clause(&filters, "r", &mut params, &syntax, |v| {
            conversions::to_json_value(v)
        })?;
        let sort_clause = build_sort_clause(&sort, "r");

        let limit_clause = limit.map_or("".to_string(), |l| format!("LIMIT {l}"));
        let offset_clause = offset.map_or("".to_string(), |o| format!("SKIP {o}"));

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

    fn get_adjacent_vertices(
        &self,
        vertex_id: ElementId,
        direction: Direction,
        edge_types: Option<Vec<String>>,
        limit: Option<u32>,
    ) -> Result<Vec<Vertex>, GraphError> {
        let (left_pattern, right_pattern) = match direction {
            Direction::Outgoing => ("-", "->"),
            Direction::Incoming => ("<-", "-"),
            Direction::Both => ("-", "-"),
        };

        let edge_type_str = edge_types.map_or("".to_string(), |types| {
            if types.is_empty() {
                "".to_string()
            } else {
                format!(":{}", types.join("|"))
            }
        });

        let limit_clause = limit.map_or("".to_string(), |l| format!("LIMIT {l}"));

        let full_query = format!(
            "MATCH (a){left_pattern}[r{edge_type_str}]{right_pattern}(b) WHERE elementId(a) = $id RETURN b {limit_clause}"
        );

        let params = ElementIdHelper::to_cypher_parameter(&vertex_id);
        let statement = Neo4jStatement::new(full_query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        VertexListProcessor::process_response(response)
    }

    fn get_connected_edges(
        &self,
        vertex_id: ElementId,
        direction: Direction,
        edge_types: Option<Vec<String>>,
        limit: Option<u32>,
    ) -> Result<Vec<Edge>, GraphError> {
        let (left_pattern, right_pattern) = match direction {
            Direction::Outgoing => ("-", "->"),
            Direction::Incoming => ("<-", "-"),
            Direction::Both => ("-", "-"),
        };

        let edge_type_str = edge_types.map_or("".to_string(), |types| {
            if types.is_empty() {
                "".to_string()
            } else {
                format!(":{}", types.join("|"))
            }
        });

        let limit_clause = limit.map_or("".to_string(), |l| format!("LIMIT {l}"));

        let full_query = format!(
            "MATCH (a){left_pattern}[r{edge_type_str}]{right_pattern}(b) WHERE elementId(a) = $id RETURN elementId(r), type(r), properties(r), elementId(startNode(r)), elementId(endNode(r)) {limit_clause}"
        );

        let params = ElementIdHelper::to_cypher_parameter(&vertex_id);
        let statement = Neo4jStatement::with_row_only(full_query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        EdgeListProcessor::process_response(response)
    }

    fn create_vertices(&self, vertices: Vec<VertexSpec>) -> Result<Vec<Vertex>, GraphError> {
        if vertices.is_empty() {
            return Ok(vec![]);
        }

        let mut statements = Vec::new();
        for spec in vertices {
            let mut labels = vec![spec.vertex_type];
            if let Some(additional) = spec.additional_labels {
                labels.extend(additional);
            }
            let cypher_labels = labels.join(":");
            let properties_map = conversions::to_cypher_properties(spec.properties)?;

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

    fn create_edges(&self, edges: Vec<EdgeSpec>) -> Result<Vec<Edge>, GraphError> {
        if edges.is_empty() {
            return Ok(vec![]);
        }

        let mut statements = Vec::new();
        for spec in edges {
            let properties_map = conversions::to_cypher_properties(spec.properties)?;

            let mut params = HashMap::new();
            params.insert(
                "from_id".to_string(),
                serde_json::Value::String(ElementIdHelper::to_cypher_value(&spec.from_vertex)),
            );
            params.insert(
                "to_id".to_string(),
                serde_json::Value::String(ElementIdHelper::to_cypher_value(&spec.to_vertex)),
            );
            params.insert(
                "props".to_string(),
                serde_json::Value::Object(properties_map.into_iter().collect()),
            );

            let query = format!(
                "MATCH (a), (b) WHERE elementId(a) = $from_id AND elementId(b) = $to_id \
                 CREATE (a)-[r:`{}`]->(b) SET r = $props \
                 RETURN elementId(r), type(r), properties(r), elementId(a), elementId(b)",
                spec.edge_type
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

    fn upsert_vertex(
        &self,
        id: Option<ElementId>,
        vertex_type: String,
        properties: PropertyMap,
    ) -> Result<Vertex, GraphError> {
        if id.is_some() {
            return Err(GraphError::UnsupportedOperation(
                "upsert_vertex with a specific element ID is not yet supported. \
                Please provide matching properties and a null ID."
                    .to_string(),
            ));
        }
        if properties.is_empty() {
            return Err(GraphError::InvalidQuery(
                "upsert_vertex requires at least one property to match on for the MERGE operation."
                    .to_string(),
            ));
        }

        let set_props = conversions::to_cypher_properties(properties)?;

        let mut params = HashMap::new();
        let merge_prop_clauses: Vec<String> = set_props
            .keys()
            .map(|k| {
                let param_name = format!("match_{k}");
                params.insert(param_name.clone(), set_props[k].clone());
                format!("{k}: ${param_name}")
            })
            .collect();
        let merge_clause = format!("{{ {} }}", merge_prop_clauses.join(", "));

        params.insert("set_props".to_string(), json!(set_props));

        let query = format!("MERGE (n:`{vertex_type}` {merge_clause}) SET n = $set_props RETURN n");

        let statement = Neo4jStatement::new(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        VertexProcessor::process_response(response)
    }

    fn upsert_edge(
        &self,
        id: Option<ElementId>,
        edge_type: String,
        from_vertex: ElementId,
        to_vertex: ElementId,
        properties: PropertyMap,
    ) -> Result<Edge, GraphError> {
        if id.is_some() {
            return Err(GraphError::UnsupportedOperation(
                "upsert_edge with a specific element ID is not yet supported. \
                Please provide matching properties and a null ID."
                    .to_string(),
            ));
        }

        let set_props = conversions::to_cypher_properties(properties)?;

        let mut params = HashMap::new();
        let merge_prop_clauses: Vec<String> = set_props
            .keys()
            .map(|k| {
                let param_name = format!("match_{k}");
                params.insert(param_name.clone(), set_props[k].clone());
                format!("{k}: ${param_name}")
            })
            .collect();

        let merge_clause = if merge_prop_clauses.is_empty() {
            "".to_string()
        } else {
            format!("{{ {} }}", merge_prop_clauses.join(", "))
        };

        params.insert(
            "from_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&from_vertex)),
        );
        params.insert(
            "to_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&to_vertex)),
        );
        params.insert("set_props".to_string(), json!(set_props));

        let query = format!(
            "MATCH (a), (b) WHERE elementId(a) = $from_id AND elementId(b) = $to_id \
            MERGE (a)-[r:`{edge_type}` {merge_clause}]->(b) \
            SET r = $set_props \
            RETURN elementId(r), type(r), properties(r), elementId(startNode(r)), elementId(endNode(r))"
        );

        let statement = Neo4jStatement::with_row_only(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        EdgeProcessor::process_response(response)
    }

    fn is_active(&self) -> bool {
        let state = self.state.read().unwrap();
        match *state {
            crate::TransactionState::Active => true,
            crate::TransactionState::Committed | crate::TransactionState::RolledBack => false,
        }
    }
}
