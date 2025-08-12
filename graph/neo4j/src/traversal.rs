use crate::client::{Neo4jStatement, Neo4jStatements};
use crate::helpers::{
    element_id_to_key, ElementIdHelper, Neo4jResponseProcessor, VertexListProcessor,
};
use crate::{GraphNeo4jComponent, Transaction};

use golem_graph::golem::graph::{
    errors::GraphError,
    traversal::{
        Direction, Guest as TraversalGuest, NeighborhoodOptions, Path, PathOptions, Subgraph,
    },
    types::{Edge, ElementId, Vertex},
};
use std::collections::HashMap;

impl Transaction {
    pub fn find_shortest_path(
        &self,
        from_vertex: ElementId,
        to_vertex: ElementId,
        _options: Option<PathOptions>,
    ) -> Result<Option<Path>, GraphError> {
        let mut params = std::collections::HashMap::new();
        params.insert(
            "from_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&from_vertex)),
        );
        params.insert(
            "to_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&to_vertex)),
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

    pub fn find_all_paths(
        &self,
        from_vertex: ElementId,
        to_vertex: ElementId,
        options: Option<PathOptions>,
        limit: Option<u32>,
    ) -> Result<Vec<Path>, GraphError> {
        let path_spec = match options {
            Some(opts) => {
                if opts.vertex_types.is_some()
                    || opts.vertex_filters.is_some()
                    || opts.edge_filters.is_some()
                {
                    return Err(GraphError::UnsupportedOperation(
                        "vertex_types, vertex_filters, and edge_filters are not yet supported in find_all_paths"
                            .to_string(),
                    ));
                }
                let edge_types = opts.edge_types.map_or("".to_string(), |types| {
                    if types.is_empty() {
                        "".to_string()
                    } else {
                        format!(":{}", types.join("|"))
                    }
                });
                let depth = opts
                    .max_depth
                    .map_or("*".to_string(), |d| format!("*1..{d}"));
                format!("-[{}]-", format_args!("r{}{}", edge_types, depth))
            }
            None => "-[*]-".to_string(),
        };

        let limit_clause = limit.map_or("".to_string(), |l| format!("LIMIT {l}"));
        let query = format!(
            "MATCH p = (a){path_spec}(b) WHERE elementId(a) = $from_id AND elementId(b) = $to_id RETURN p {limit_clause}"
        );

        let mut params = std::collections::HashMap::new();
        params.insert(
            "from_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&from_vertex)),
        );
        params.insert(
            "to_id".to_string(),
            serde_json::Value::String(ElementIdHelper::to_cypher_value(&to_vertex)),
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

    pub fn get_neighborhood(
        &self,
        center: ElementId,
        options: NeighborhoodOptions,
    ) -> Result<Subgraph, GraphError> {
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

        let params = ElementIdHelper::to_cypher_parameter(&center);
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

    pub fn path_exists(
        &self,
        from_vertex: ElementId,
        to_vertex: ElementId,
        options: Option<PathOptions>,
    ) -> Result<bool, GraphError> {
        self.find_all_paths(from_vertex, to_vertex, options, Some(1))
            .map(|paths| !paths.is_empty())
    }

    pub fn get_vertices_at_distance(
        &self,
        source: ElementId,
        distance: u32,
        direction: Direction,
        edge_types: Option<Vec<String>>,
    ) -> Result<Vec<Vertex>, GraphError> {
        let (left_arrow, right_arrow) = match direction {
            Direction::Outgoing => ("", "->"),
            Direction::Incoming => ("<-", ""),
            Direction::Both => ("-", "-"),
        };

        let edge_type_str = edge_types.map_or("".to_string(), |types| {
            if types.is_empty() {
                "".to_string()
            } else {
                format!(":{}", types.join("|"))
            }
        });

        let query = format!(
            "MATCH (a){left_arrow}[{edge_type_str}*{distance}]{right_arrow}(b) WHERE elementId(a) = $id RETURN DISTINCT b"
        );

        let params = ElementIdHelper::to_cypher_parameter(&source);
        let statement = Neo4jStatement::new(query, params);
        let statements = Neo4jStatements::single(statement);

        let response = self
            .api
            .execute_typed_transaction(&self.transaction_url, &statements)?;
        VertexListProcessor::process_response(response)
    }
}

impl TraversalGuest for GraphNeo4jComponent {
    fn find_shortest_path(
        transaction: golem_graph::golem::graph::transactions::TransactionBorrow<'_>,
        from_vertex: ElementId,
        to_vertex: ElementId,
        _options: Option<PathOptions>,
    ) -> Result<Option<Path>, GraphError> {
        let tx: &Transaction = transaction.get();
        tx.find_shortest_path(from_vertex, to_vertex, _options)
    }

    fn find_all_paths(
        transaction: golem_graph::golem::graph::transactions::TransactionBorrow<'_>,
        from_vertex: ElementId,
        to_vertex: ElementId,
        options: Option<PathOptions>,
        limit: Option<u32>,
    ) -> Result<Vec<Path>, GraphError> {
        let tx: &Transaction = transaction.get();
        tx.find_all_paths(from_vertex, to_vertex, options, limit)
    }

    fn get_neighborhood(
        transaction: golem_graph::golem::graph::transactions::TransactionBorrow<'_>,
        center: ElementId,
        options: NeighborhoodOptions,
    ) -> Result<Subgraph, GraphError> {
        let tx: &Transaction = transaction.get();
        tx.get_neighborhood(center, options)
    }

    fn path_exists(
        transaction: golem_graph::golem::graph::transactions::TransactionBorrow<'_>,
        from_vertex: ElementId,
        to_vertex: ElementId,
        options: Option<PathOptions>,
    ) -> Result<bool, GraphError> {
        let tx: &Transaction = transaction.get();
        tx.path_exists(from_vertex, to_vertex, options)
    }

    fn get_vertices_at_distance(
        transaction: golem_graph::golem::graph::transactions::TransactionBorrow<'_>,
        source: ElementId,
        distance: u32,
        direction: Direction,
        edge_types: Option<Vec<String>>,
    ) -> Result<Vec<Vertex>, GraphError> {
        let tx: &Transaction = transaction.get();
        tx.get_vertices_at_distance(source, distance, direction, edge_types)
    }
}
