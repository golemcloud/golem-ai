use crate::{helpers, ArangoDb, SchemaManager};
use async_trait::async_trait;
use golem_ai_graph::{
    durability::ExtendedGuest,
    model::{
        connection::ConnectionConfig,
        errors::GraphError,
        schema::{
            ContainerInfo, ContainerType, EdgeLabelSchema, EdgeTypeDefinition, IndexDefinition,
            SchemaManager as SchemaManagerResource, VertexLabelSchema,
        },
    },
    SchemaManagerInterface, SchemaManagerProvider,
};
use std::sync::Arc;

impl SchemaManagerProvider for ArangoDb {
    type SchemaManager = SchemaManager;

    async fn get_schema_manager(
        config: Option<ConnectionConfig>,
    ) -> Result<golem_ai_graph::model::schema::SchemaManager, GraphError> {
        let final_config = match config {
            Some(provided_config) => provided_config,
            None => helpers::config_from_env()?,
        };

        let graph = ArangoDb::connect_internal(&final_config).await?;

        let manager = SchemaManager {
            graph: Arc::new(graph),
        };

        Ok(SchemaManagerResource::new(manager))
    }
}

#[async_trait(?Send)]
impl SchemaManagerInterface for SchemaManager {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn define_vertex_label(&self, schema: VertexLabelSchema) -> Result<(), GraphError> {
        self.create_container(schema.label, ContainerType::VertexContainer)
            .await
    }

    async fn define_edge_label(&self, schema: EdgeLabelSchema) -> Result<(), GraphError> {
        self.create_container(schema.label, ContainerType::EdgeContainer)
            .await
    }

    async fn get_vertex_label_schema(
        &self,
        _label: String,
    ) -> Result<Option<VertexLabelSchema>, GraphError> {
        Err(GraphError::UnsupportedOperation(
            "get_vertex_label_schema is not yet supported".to_string(),
        ))
    }

    async fn get_edge_label_schema(
        &self,
        _label: String,
    ) -> Result<Option<EdgeLabelSchema>, GraphError> {
        Err(GraphError::UnsupportedOperation(
            "get_edge_label_schema is not yet supported".to_string(),
        ))
    }

    async fn list_vertex_labels(&self) -> Result<Vec<String>, GraphError> {
        let all_containers = self.list_containers().await?;
        Ok(all_containers
            .into_iter()
            .filter(|c| matches!(c.container_type, ContainerType::VertexContainer))
            .map(|c| c.name)
            .collect())
    }

    async fn list_edge_labels(&self) -> Result<Vec<String>, GraphError> {
        let all_containers = self.list_containers().await?;
        Ok(all_containers
            .into_iter()
            .filter(|c| matches!(c.container_type, ContainerType::EdgeContainer))
            .map(|c| c.name)
            .collect())
    }

    async fn create_index(&self, index: IndexDefinition) -> Result<(), GraphError> {
        self.graph
            .api
            .create_index(
                index.label,
                index.properties,
                index.unique,
                index.index_type,
                Some(index.name),
            )
            .await
    }

    async fn drop_index(&self, name: String) -> Result<(), GraphError> {
        self.graph.api.drop_index(&name).await
    }

    async fn list_indexes(&self) -> Result<Vec<IndexDefinition>, GraphError> {
        self.graph.api.list_indexes().await
    }

    async fn get_index(&self, name: String) -> Result<Option<IndexDefinition>, GraphError> {
        self.graph.api.get_index(&name).await
    }

    async fn define_edge_type(&self, definition: EdgeTypeDefinition) -> Result<(), GraphError> {
        self.graph.api.define_edge_type(definition).await
    }

    async fn list_edge_types(&self) -> Result<Vec<EdgeTypeDefinition>, GraphError> {
        self.graph.api.list_edge_types().await
    }

    async fn create_container(
        &self,
        name: String,
        container_type: ContainerType,
    ) -> Result<(), GraphError> {
        self.graph
            .api
            .create_collection(&name, container_type)
            .await
    }

    async fn list_containers(&self) -> Result<Vec<ContainerInfo>, GraphError> {
        self.graph.api.list_collections().await
    }
}
