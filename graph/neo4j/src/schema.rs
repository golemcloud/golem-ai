use crate::client::{Neo4jStatement, Neo4jStatements};
use crate::helpers::{config_from_env, map_neo4j_type_to_wit};
use crate::{GraphNeo4jComponent, SchemaManager};
use golem_graph::durability::ExtendedGuest;
use golem_graph::golem::graph::{
    connection::ConnectionConfig,
    errors::GraphError,
    schema::{
        EdgeLabelSchema, EdgeTypeDefinition, Guest as SchemaGuest, GuestSchemaManager,
        IndexDefinition, IndexType, PropertyDefinition, PropertyType,
        SchemaManager as SchemaManagerResource, VertexLabelSchema,
    },
};
use log::trace;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

impl SchemaGuest for GraphNeo4jComponent {
    type SchemaManager = SchemaManager;

    fn get_schema_manager(
        config: Option<ConnectionConfig>,
    ) -> Result<SchemaManagerResource, GraphError> {
        let final_config = match config {
            Some(provided_config) => provided_config,
            None => config_from_env()?,
        };
        let graph = GraphNeo4jComponent::connect_internal(&final_config)?;
        let manager = SchemaManager {
            graph: Arc::new(graph),
        };
        Ok(SchemaManagerResource::new(manager))
    }
}

impl GuestSchemaManager for SchemaManager {
    fn define_vertex_label(&self, schema: VertexLabelSchema) -> Result<(), GraphError> {
        for prop in schema.properties {
            if prop.required {
                let q = format!(
                    "CREATE CONSTRAINT constraint_required_{label}_{name} \
                     IF NOT EXISTS FOR (n:{label}) REQUIRE n.{name} IS NOT NULL",
                    label = schema.label,
                    name = prop.name
                );
                let tx = self.graph.begin_transaction()?;
                let statement = Neo4jStatement::with_row_only(q, HashMap::new());
                let statements = Neo4jStatements::single(statement);

                match tx
                    .api
                    .execute_typed_transaction(&tx.transaction_url, &statements)
                {
                    Err(e) => {
                        let is_enterprise_error = matches!(
                            &e,
                            GraphError::SchemaViolation(_) | GraphError::UnsupportedOperation(_)
                        );
                        if is_enterprise_error {
                            trace!("[WARN] Skipping property existence constraint: requires Neo4j Enterprise Edition. Error: {e}");
                            tx.commit()?;
                        } else {
                            return Err(e);
                        }
                    }
                    Ok(_) => tx.commit()?,
                }
            }

            if prop.unique {
                let q = format!(
                    "CREATE CONSTRAINT constraint_unique_{label}_{name} \
                     IF NOT EXISTS FOR (n:{label}) REQUIRE n.{name} IS UNIQUE",
                    label = schema.label,
                    name = prop.name
                );
                let tx = self.graph.begin_transaction()?;
                let statement = Neo4jStatement::with_row_only(q, HashMap::new());
                let statements = Neo4jStatements::single(statement);
                tx.api
                    .execute_typed_transaction(&tx.transaction_url, &statements)?;
                tx.commit()?;
            }
        }

        Ok(())
    }

    fn define_edge_label(&self, schema: EdgeLabelSchema) -> Result<(), GraphError> {
        let tx = self.graph.begin_transaction()?;
        let mut statements = Vec::new();

        for prop in schema.properties {
            if prop.required {
                let constraint_name =
                    format!("constraint_rel_required_{}_{}", &schema.label, &prop.name);
                let query = format!(
                    "CREATE CONSTRAINT {} IF NOT EXISTS FOR ()-[r:{}]-() REQUIRE r.{} IS NOT NULL",
                    constraint_name, &schema.label, &prop.name
                );
                statements.push(Neo4jStatement::with_row_only(query, HashMap::new()));
            }
            if prop.unique {}
        }

        if statements.is_empty() {
            return tx.commit();
        }

        let statements_batch = Neo4jStatements::batch(statements);
        tx.api
            .execute_typed_transaction(&tx.transaction_url, &statements_batch)?;

        tx.commit()
    }

    fn get_vertex_label_schema(
        &self,
        label: String,
    ) -> Result<Option<VertexLabelSchema>, GraphError> {
        let tx = self.graph.begin_transaction()?;

        let props_query =
            "CALL db.schema.nodeTypeProperties() YIELD nodeLabels, propertyName, propertyTypes, mandatory \
             WHERE $label IN nodeLabels \
             RETURN propertyName, propertyTypes, mandatory";
        let mut params = HashMap::new();
        params.insert("label".to_string(), json!(&label));
        let props_stmt = Neo4jStatement::with_row_only(props_query.to_string(), params);

        let cons_query = "SHOW CONSTRAINTS YIELD name, type, properties, labelsOrTypes \
             WHERE type = 'UNIQUENESS' AND $label IN labelsOrTypes \
             RETURN properties";
        let mut cons_params = HashMap::new();
        cons_params.insert("label".to_string(), json!(&label));
        let cons_stmt = Neo4jStatement::with_row_only(cons_query.to_string(), cons_params);

        let statements = Neo4jStatements::batch(vec![props_stmt, cons_stmt]);
        let response = tx
            .api
            .execute_typed_transaction(&tx.transaction_url, &statements)?;

        tx.commit()?;

        if !response.errors.is_empty() {
            return Err(GraphError::InvalidQuery(response.errors[0].message.clone()));
        }

        let props_result = response
            .results
            .first()
            .ok_or_else(|| GraphError::InternalError("Missing property schema response".into()))?;
        props_result.check_errors()?;

        if props_result.data.is_empty() {
            return Ok(None);
        }

        #[derive(serde::Deserialize)]
        struct PropertyInfo {
            property_name: String,
            property_types: Vec<String>,
            mandatory: bool,
        }

        let mut defs: HashMap<String, PropertyDefinition> = HashMap::new();
        for data_item in &props_result.data {
            if let Some(row) = &data_item.row {
                if row.len() >= 3 {
                    let info = PropertyInfo {
                        property_name: row[0].as_str().unwrap_or("").to_string(),
                        property_types: serde_json::from_value(row[1].clone()).unwrap_or_default(),
                        mandatory: row[2].as_bool().unwrap_or(false),
                    };
                    defs.insert(
                        info.property_name.clone(),
                        PropertyDefinition {
                            name: info.property_name,
                            property_type: info
                                .property_types
                                .first()
                                .map(|s| map_neo4j_type_to_wit(s))
                                .unwrap_or(PropertyType::StringType),
                            required: info.mandatory,
                            unique: false,
                            default_value: None,
                        },
                    );
                }
            }
        }

        if let Some(cons_result) = response.results.get(1) {
            cons_result.check_errors()?;
            for data_item in &cons_result.data {
                if let Some(row) = &data_item.row {
                    if let Some(list_val) = row.first() {
                        if let Ok(list) = serde_json::from_value::<Vec<String>>(list_val.clone()) {
                            for prop_name in list {
                                if let Some(def) = defs.get_mut(&prop_name) {
                                    def.unique = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        for def in defs.values_mut() {
            if def.unique {
                def.required = true;
            }
        }

        let props = defs.into_values().collect();
        Ok(Some(VertexLabelSchema {
            label,
            properties: props,
            container: None,
        }))
    }

    fn get_edge_label_schema(&self, label: String) -> Result<Option<EdgeLabelSchema>, GraphError> {
        let tx = self.graph.begin_transaction()?;

        let props_query = "CALL db.schema.relTypeProperties() YIELD relType, propertyName, propertyTypes, mandatory WHERE relType = $label RETURN propertyName, propertyTypes, mandatory";
        let mut params = HashMap::new();
        params.insert("label".to_string(), json!(&label));
        let props_statement = Neo4jStatement::with_row_only(props_query.to_string(), params);
        let statements = Neo4jStatements::single(props_statement);

        let response = tx
            .api
            .execute_typed_transaction(&tx.transaction_url, &statements)?;
        tx.commit()?;

        let props_result = response.first_result()?;
        props_result.check_errors()?;

        if props_result.data.is_empty() {
            return Ok(None);
        }

        #[derive(serde::Deserialize)]
        struct Neo4jPropertyInfo {
            property_name: String,
            property_types: Vec<String>,
            mandatory: bool,
        }

        let mut property_definitions = Vec::new();
        for data_item in &props_result.data {
            if let Some(row) = &data_item.row {
                if row.len() >= 3 {
                    let info = Neo4jPropertyInfo {
                        property_name: row[0].as_str().unwrap_or("").to_string(),
                        property_types: serde_json::from_value(row[1].clone()).unwrap_or_default(),
                        mandatory: row[2].as_bool().unwrap_or(false),
                    };

                    if !info.property_name.is_empty() {
                        property_definitions.push(PropertyDefinition {
                            name: info.property_name,
                            property_type: info
                                .property_types
                                .first()
                                .map(|s| map_neo4j_type_to_wit(s))
                                .unwrap_or(PropertyType::StringType),
                            required: info.mandatory,
                            unique: false,
                            default_value: None,
                        });
                    }
                }
            }
        }

        Ok(Some(EdgeLabelSchema {
            label,
            properties: property_definitions,
            from_labels: None,
            to_labels: None,
            container: None,
        }))
    }

    fn list_vertex_labels(&self) -> Result<Vec<String>, GraphError> {
        let tx = self.graph.begin_transaction()?;
        let result = tx.execute_schema_query_and_extract_string_list(
            "CALL db.labels() YIELD label RETURN label",
        )?;
        tx.commit()?;
        Ok(result)
    }

    fn list_edge_labels(&self) -> Result<Vec<String>, GraphError> {
        let tx = self.graph.begin_transaction()?;
        let result = tx.execute_schema_query_and_extract_string_list(
            "CALL db.relationshipTypes() YIELD relationshipType RETURN relationshipType",
        )?;
        tx.commit()?;
        Ok(result)
    }

    fn create_index(&self, index: IndexDefinition) -> Result<(), GraphError> {
        let tx = self.graph.begin_transaction()?;

        let index_type_str = match index.index_type {
            IndexType::Range => "RANGE",
            IndexType::Text => "TEXT",
            IndexType::Geospatial => "POINT",
            IndexType::Exact => {
                return Err(GraphError::UnsupportedOperation(
                    "Neo4j does not have a separate 'Exact' index type; use RANGE or TEXT."
                        .to_string(),
                ))
            }
        };

        let properties_str = index.properties.join(", ");

        let query = format!(
            "CREATE {} INDEX {} IF NOT EXISTS FOR (n:{}) ON (n.{})",
            index_type_str, index.name, index.label, properties_str
        );

        let statement = Neo4jStatement::with_row_only(query, HashMap::new());
        let statements = Neo4jStatements::single(statement);
        tx.api
            .execute_typed_transaction(&tx.transaction_url, &statements)?;
        tx.commit()
    }

    fn drop_index(&self, name: String) -> Result<(), GraphError> {
        let tx = self.graph.begin_transaction()?;
        let query = format!("DROP INDEX {name} IF EXISTS");
        let statement = Neo4jStatement::with_row_only(query, HashMap::new());
        let statements = Neo4jStatements::single(statement);
        tx.api
            .execute_typed_transaction(&tx.transaction_url, &statements)?;
        tx.commit()
    }

    fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
        let tx = self.graph.begin_transaction()?;
        let query = "SHOW INDEXES";
        let statement = Neo4jStatement::with_row_only(query.to_string(), HashMap::new());
        let statements = Neo4jStatements::single(statement);
        let response = tx
            .api
            .execute_typed_transaction(&tx.transaction_url, &statements)?;

        tx.commit()?;

        let result = response.first_result()?;
        result.check_errors()?;

        let mut indexes = Vec::new();

        for data_item in &result.data {
            if let Some(row) = &data_item.row {
                if row.len() >= 10 {
                    let name = row[1].as_str().unwrap_or_default().to_string();
                    let index_type_str = row[4].as_str().unwrap_or_default().to_lowercase();
                    let label = row[6]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let properties: Vec<String> = row[7]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .map(|v| v.as_str().unwrap_or_default().to_string())
                                .collect()
                        })
                        .unwrap_or_default();
                    let unique = row[9].is_string();

                    let index_type = match index_type_str.as_str() {
                        "range" => IndexType::Range,
                        "text" => IndexType::Text,
                        "point" => IndexType::Geospatial,
                        _ => continue,
                    };

                    indexes.push(IndexDefinition {
                        name,
                        label,
                        properties,
                        index_type,
                        unique,
                        container: None,
                    });
                }
            }
        }
        Ok(indexes)
    }

    fn get_index(&self, _name: String) -> Result<Option<IndexDefinition>, GraphError> {
        Err(GraphError::UnsupportedOperation(
            "get_index is not supported by the Neo4j provider yet.".to_string(),
        ))
    }

    fn define_edge_type(&self, _definition: EdgeTypeDefinition) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation(
            "define_edge_type is not supported by the Neo4j provider".to_string(),
        ))
    }

    fn list_edge_types(
        &self,
    ) -> Result<Vec<golem_graph::golem::graph::schema::EdgeTypeDefinition>, GraphError> {
        Err(GraphError::UnsupportedOperation(
            "list_edge_types is not supported by the Neo4j provider".to_string(),
        ))
    }

    fn create_container(
        &self,
        _name: String,
        _container_type: golem_graph::golem::graph::schema::ContainerType,
    ) -> Result<(), GraphError> {
        Err(GraphError::UnsupportedOperation(
            "create_container is not supported by the Neo4j provider".to_string(),
        ))
    }

    fn list_containers(
        &self,
    ) -> Result<Vec<golem_graph::golem::graph::schema::ContainerInfo>, GraphError> {
        Ok(vec![])
    }
}
