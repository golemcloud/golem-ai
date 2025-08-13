declare module 'graph-library' {
  import * as golemGraph100Connection from 'golem:graph/connection@1.0.0';
  import * as golemGraph100Errors from 'golem:graph/errors@1.0.0';
  import * as golemGraph100Transactions from 'golem:graph/transactions@1.0.0';
  import * as golemGraph100Types from 'golem:graph/types@1.0.0';
  export namespace types {
    export type Date = {
      year: number;
      month: number;
      day: number;
    };
    export type Time = {
      hour: number;
      minute: number;
      second: number;
      nanosecond: number;
    };
    export type Datetime = {
      date: Date;
      time: Time;
      timezoneOffsetMinutes: number | undefined;
    };
    export type Duration = {
      seconds: bigint;
      nanoseconds: number;
    };
    export type Point = {
      longitude: number;
      latitude: number;
      altitude: number | undefined;
    };
    export type Linestring = {
      coordinates: Point[];
    };
    export type Polygon = {
      exterior: Point[];
      holes: Point[][] | undefined;
    };
    export type PropertyValue = {
      tag: 'null-value'
    } |
    {
      tag: 'boolean'
      val: boolean
    } |
    {
      tag: 'int8'
      val: number
    } |
    {
      tag: 'int16'
      val: number
    } |
    {
      tag: 'int32'
      val: number
    } |
    {
      tag: 'int64'
      val: bigint
    } |
    {
      tag: 'uint8'
      val: number
    } |
    {
      tag: 'uint16'
      val: number
    } |
    {
      tag: 'uint32'
      val: number
    } |
    {
      tag: 'uint64'
      val: bigint
    } |
    {
      tag: 'float32-value'
      val: number
    } |
    {
      tag: 'float64-value'
      val: number
    } |
    {
      tag: 'string-value'
      val: string
    } |
    {
      tag: 'bytes'
      val: number[]
    } |
    {
      tag: 'date'
      val: Date
    } |
    {
      tag: 'time'
      val: Time
    } |
    {
      tag: 'datetime'
      val: Datetime
    } |
    {
      tag: 'duration'
      val: Duration
    } |
    {
      tag: 'point'
      val: Point
    } |
    {
      tag: 'linestring'
      val: Linestring
    } |
    {
      tag: 'polygon'
      val: Polygon
    };
    export type ElementId = {
      tag: 'string-value'
      val: string
    } |
    {
      tag: 'int64'
      val: bigint
    } |
    {
      tag: 'uuid'
      val: string
    };
    export type PropertyMap = [string, PropertyValue][];
    export type Vertex = {
      id: ElementId;
      vertexType: string;
      additionalLabels: string[];
      properties: PropertyMap;
    };
    export type Edge = {
      id: ElementId;
      edgeType: string;
      fromVertex: ElementId;
      toVertex: ElementId;
      properties: PropertyMap;
    };
    export type Path = {
      vertices: Vertex[];
      edges: Edge[];
      length: number;
    };
    export type Direction = "outgoing" | "incoming" | "both";
    export type ComparisonOperator = "equal" | "not-equal" | "less-than" | "less-than-or-equal" | "greater-than" | "greater-than-or-equal" | "contains" | "starts-with" | "ends-with" | "regex-match" | "in-list" | "not-in-list";
    export type FilterCondition = {
      property: string;
      operator: ComparisonOperator;
      value: PropertyValue;
    };
    export type SortSpec = {
      property: string;
      ascending: boolean;
    };
  }
  export namespace errors {
    export type ElementId = golemGraph100Types.ElementId;
    export type GraphError = {
      tag: 'unsupported-operation'
      val: string
    } |
    {
      tag: 'connection-failed'
      val: string
    } |
    {
      tag: 'authentication-failed'
      val: string
    } |
    {
      tag: 'authorization-failed'
      val: string
    } |
    {
      tag: 'element-not-found'
      val: ElementId
    } |
    {
      tag: 'duplicate-element'
      val: ElementId
    } |
    {
      tag: 'schema-violation'
      val: string
    } |
    {
      tag: 'constraint-violation'
      val: string
    } |
    {
      tag: 'invalid-property-type'
      val: string
    } |
    {
      tag: 'invalid-query'
      val: string
    } |
    {
      tag: 'transaction-failed'
      val: string
    } |
    {
      tag: 'transaction-conflict'
    } |
    {
      tag: 'transaction-timeout'
    } |
    {
      tag: 'deadlock-detected'
    } |
    {
      tag: 'timeout'
    } |
    {
      tag: 'resource-exhausted'
      val: string
    } |
    {
      tag: 'internal-error'
      val: string
    } |
    {
      tag: 'service-unavailable'
      val: string
    };
  }
  export namespace transactions {
    export class Transaction {
      async createVertex(vertexType: string, properties: PropertyMap): Promise<Result<Vertex, GraphError>>;
      async createVertexWithLabels(vertexType: string, additionalLabels: string[], properties: PropertyMap): Promise<Result<Vertex, GraphError>>;
      async getVertex(id: ElementId): Promise<Result<Vertex | undefined, GraphError>>;
      async updateVertex(id: ElementId, properties: PropertyMap): Promise<Result<Vertex, GraphError>>;
      async updateVertexProperties(id: ElementId, updates: PropertyMap): Promise<Result<Vertex, GraphError>>;
      async deleteVertex(id: ElementId, deleteEdges: boolean): Promise<Result<void, GraphError>>;
      async findVertices(vertexType: string | undefined, filters: FilterCondition[] | undefined, sort: SortSpec[] | undefined, limit: number | undefined, offset: number | undefined): Promise<Result<Vertex[], GraphError>>;
      async createEdge(edgeType: string, fromVertex: ElementId, toVertex: ElementId, properties: PropertyMap): Promise<Result<Edge, GraphError>>;
      async getEdge(id: ElementId): Promise<Result<Edge | undefined, GraphError>>;
      async updateEdge(id: ElementId, properties: PropertyMap): Promise<Result<Edge, GraphError>>;
      async updateEdgeProperties(id: ElementId, updates: PropertyMap): Promise<Result<Edge, GraphError>>;
      async deleteEdge(id: ElementId): Promise<Result<void, GraphError>>;
      async findEdges(edgeTypes: string[] | undefined, filters: FilterCondition[] | undefined, sort: SortSpec[] | undefined, limit: number | undefined, offset: number | undefined): Promise<Result<Edge[], GraphError>>;
      async getAdjacentVertices(vertexId: ElementId, direction: Direction, edgeTypes: string[] | undefined, limit: number | undefined): Promise<Result<Vertex[], GraphError>>;
      async getConnectedEdges(vertexId: ElementId, direction: Direction, edgeTypes: string[] | undefined, limit: number | undefined): Promise<Result<Edge[], GraphError>>;
      async createVertices(vertices: VertexSpec[]): Promise<Result<Vertex[], GraphError>>;
      async createEdges(edges: EdgeSpec[]): Promise<Result<Edge[], GraphError>>;
      async upsertVertex(id: ElementId | undefined, vertexType: string, properties: PropertyMap): Promise<Result<Vertex, GraphError>>;
      async upsertEdge(id: ElementId | undefined, edgeType: string, fromVertex: ElementId, toVertex: ElementId, properties: PropertyMap): Promise<Result<Edge, GraphError>>;
      async commit(): Promise<Result<void, GraphError>>;
      async rollback(): Promise<Result<void, GraphError>>;
      async isActive(): Promise<boolean>;
    }
    export type Vertex = golemGraph100Types.Vertex;
    export type Edge = golemGraph100Types.Edge;
    export type Path = golemGraph100Types.Path;
    export type ElementId = golemGraph100Types.ElementId;
    export type PropertyMap = golemGraph100Types.PropertyMap;
    export type PropertyValue = golemGraph100Types.PropertyValue;
    export type FilterCondition = golemGraph100Types.FilterCondition;
    export type SortSpec = golemGraph100Types.SortSpec;
    export type Direction = golemGraph100Types.Direction;
    export type GraphError = golemGraph100Errors.GraphError;
    export type VertexSpec = {
      vertexType: string;
      additionalLabels: string[] | undefined;
      properties: PropertyMap;
    };
    export type EdgeSpec = {
      edgeType: string;
      fromVertex: ElementId;
      toVertex: ElementId;
      properties: PropertyMap;
    };
    export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
  }
  export namespace connection {
    export function connect(config: ConnectionConfig): Promise<Result<Graph, GraphError>>;
    export class Graph {
      async beginTransaction(): Promise<Result<Transaction, GraphError>>;
      async beginReadTransaction(): Promise<Result<Transaction, GraphError>>;
      async ping(): Promise<Result<void, GraphError>>;
      async close(): Promise<Result<void, GraphError>>;
      async getStatistics(): Promise<Result<GraphStatistics, GraphError>>;
    }
    export type GraphError = golemGraph100Errors.GraphError;
    export type Transaction = golemGraph100Transactions.Transaction;
    export type ConnectionConfig = {
      hosts: string[];
      port: number | undefined;
      databaseName: string | undefined;
      username: string | undefined;
      password: string | undefined;
      timeoutSeconds: number | undefined;
      maxConnections: number | undefined;
      providerConfig: [string, string][];
    };
    export type GraphStatistics = {
      vertexCount: bigint | undefined;
      edgeCount: bigint | undefined;
      labelCount: number | undefined;
      propertyCount: bigint | undefined;
    };
    export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
  }
  export namespace schema {
    export function getSchemaManager(config: ConnectionConfig | undefined): Promise<Result<SchemaManager, GraphError>>;
    export class SchemaManager {
      async defineVertexLabel(schema: VertexLabelSchema): Promise<Result<void, GraphError>>;
      async defineEdgeLabel(schema: EdgeLabelSchema): Promise<Result<void, GraphError>>;
      async getVertexLabelSchema(label: string): Promise<Result<VertexLabelSchema | undefined, GraphError>>;
      async getEdgeLabelSchema(label: string): Promise<Result<EdgeLabelSchema | undefined, GraphError>>;
      async listVertexLabels(): Promise<Result<string[], GraphError>>;
      async listEdgeLabels(): Promise<Result<string[], GraphError>>;
      async createIndex(index: IndexDefinition): Promise<Result<void, GraphError>>;
      async dropIndex(name: string): Promise<Result<void, GraphError>>;
      async listIndexes(): Promise<Result<IndexDefinition[], GraphError>>;
      async getIndex(name: string): Promise<Result<IndexDefinition | undefined, GraphError>>;
      async defineEdgeType(definition: EdgeTypeDefinition): Promise<Result<void, GraphError>>;
      async listEdgeTypes(): Promise<Result<EdgeTypeDefinition[], GraphError>>;
      async createContainer(name: string, containerType: ContainerType): Promise<Result<void, GraphError>>;
      async listContainers(): Promise<Result<ContainerInfo[], GraphError>>;
    }
    export type PropertyValue = golemGraph100Types.PropertyValue;
    export type GraphError = golemGraph100Errors.GraphError;
    export type ConnectionConfig = golemGraph100Connection.ConnectionConfig;
    export type PropertyType = "boolean" | "int32" | "int64" | "float32-type" | "float64-type" | "string-type" | "bytes" | "date" | "datetime" | "point" | "list-type" | "map-type";
    export type IndexType = "exact" | "range" | "text" | "geospatial";
    export type PropertyDefinition = {
      name: string;
      propertyType: PropertyType;
      required: boolean;
      unique: boolean;
      defaultValue: PropertyValue | undefined;
    };
    export type VertexLabelSchema = {
      label: string;
      properties: PropertyDefinition[];
      container: string | undefined;
    };
    export type EdgeLabelSchema = {
      label: string;
      properties: PropertyDefinition[];
      fromLabels: string[] | undefined;
      toLabels: string[] | undefined;
      container: string | undefined;
    };
    export type IndexDefinition = {
      name: string;
      label: string;
      properties: string[];
      indexType: IndexType;
      unique: boolean;
      container: string | undefined;
    };
    export type EdgeTypeDefinition = {
      collection: string;
      fromCollections: string[];
      toCollections: string[];
    };
    export type ContainerType = "vertex-container" | "edge-container";
    export type ContainerInfo = {
      name: string;
      containerType: ContainerType;
      elementCount: bigint | undefined;
    };
    export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
  }
  export namespace query {
    export function executeQuery(transaction: Transaction, query: string, parameters: QueryParameters | undefined, options: QueryOptions | undefined): Promise<Result<QueryExecutionResult, GraphError>>;
    export type Vertex = golemGraph100Types.Vertex;
    export type Edge = golemGraph100Types.Edge;
    export type Path = golemGraph100Types.Path;
    export type PropertyValue = golemGraph100Types.PropertyValue;
    export type GraphError = golemGraph100Errors.GraphError;
    export type Transaction = golemGraph100Transactions.Transaction;
    export type QueryResult = {
      tag: 'vertices'
      val: Vertex[]
    } |
    {
      tag: 'edges'
      val: Edge[]
    } |
    {
      tag: 'paths'
      val: Path[]
    } |
    {
      tag: 'values'
      val: PropertyValue[]
    } |
    {
      tag: 'maps'
      val: [string, PropertyValue][][]
    };
    export type QueryParameters = [string, PropertyValue][];
    export type QueryOptions = {
      timeoutSeconds: number | undefined;
      maxResults: number | undefined;
      explain: boolean;
      profile: boolean;
    };
    export type QueryExecutionResult = {
      queryResultValue: QueryResult;
      executionTimeMs: number | undefined;
      rowsAffected: number | undefined;
      explanation: string | undefined;
      profileData: string | undefined;
    };
    export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
  }
  export namespace traversal {
    export function findShortestPath(transaction: Transaction, fromVertex: ElementId, toVertex: ElementId, options: PathOptions | undefined): Promise<Result<Path | undefined, GraphError>>;
    export function findAllPaths(transaction: Transaction, fromVertex: ElementId, toVertex: ElementId, options: PathOptions | undefined, limit: number | undefined): Promise<Result<Path[], GraphError>>;
    export function getNeighborhood(transaction: Transaction, center: ElementId, options: NeighborhoodOptions): Promise<Result<Subgraph, GraphError>>;
    export function pathExists(transaction: Transaction, fromVertex: ElementId, toVertex: ElementId, options: PathOptions | undefined): Promise<Result<boolean, GraphError>>;
    export function getVerticesAtDistance(transaction: Transaction, source: ElementId, distance: number, direction: Direction, edgeTypes: string[] | undefined): Promise<Result<Vertex[], GraphError>>;
    export type Vertex = golemGraph100Types.Vertex;
    export type Edge = golemGraph100Types.Edge;
    export type Path = golemGraph100Types.Path;
    export type ElementId = golemGraph100Types.ElementId;
    export type Direction = golemGraph100Types.Direction;
    export type FilterCondition = golemGraph100Types.FilterCondition;
    export type GraphError = golemGraph100Errors.GraphError;
    export type Transaction = golemGraph100Transactions.Transaction;
    export type PathOptions = {
      maxDepth: number | undefined;
      edgeTypes: string[] | undefined;
      vertexTypes: string[] | undefined;
      vertexFilters: FilterCondition[] | undefined;
      edgeFilters: FilterCondition[] | undefined;
    };
    export type NeighborhoodOptions = {
      depth: number;
      direction: Direction;
      edgeTypes: string[] | undefined;
      maxVertices: number | undefined;
    };
    export type Subgraph = {
      vertices: Vertex[];
      edges: Edge[];
    };
    export type Result<T, E> = { tag: 'ok', val: T } | { tag: 'err', val: E };
  }
}
