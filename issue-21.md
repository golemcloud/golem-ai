Implement Durable Vector Database Provider Components for golem:vector WIT Interface #21
Open
@jdegoes
Description
jdegoes
opened on May 26 · edited by jdegoes
Contributor
I have attached to this ticket a WIT file that describes a generic interface for vector database operations. This interface can be implemented by various providers, either by emulating features not present in a given provider, utilizing the provider's native support for a feature, or indicating an error if a particular combination is not natively supported by a provider.

The intent of this WIT specification is to allow developers of WASM components (on wasmCloud, Spin, or Golem) to leverage vector database capabilities to build agents and services in a portable and provider-agnostic fashion.

This ticket involves constructing implementations of this WIT interface for the following providers:

Qdrant: Offers advanced vector similarity search with support for sparse vectors, recommendations, and discovery operations.​
Pinecone: Provides managed vector database services with hybrid vector support and namespace organization.​
Milvus: Supplies enterprise-grade vector database with comprehensive data types and clustering capabilities.​
pgvector: Extends PostgreSQL with vector operations, binary vectors, and mathematical functions.​
These implementations must be written in Rust and compilable to WASM Components (WASI 0.23 only, since Golem does not yet support WASI 0.3). The standard Rust toolchain for WASM component development can be employed (see cargo component and the Rust examples of components in this and other Golem repositories).

Additionally, these implementations should incorporate custom durability semantics using the Golem durability API and the Golem host API. This approach ensures that durability is managed at the level of individual vector operations (upsert, search, delete), providing a higher-level and clearer operation log, which aids in debugging and monitoring. See golem:llm and golem:embed for more details and durable implementations in this same repository.

The final deliverables associated with this ticket are:

Qdrant implementation: A WASM Component (WASI 0.2), named vector-qdrant.wasm, with a full test suite and custom durability implementation at the level of vector operations.​
Pinecone implementation: A WASM Component (WASI 0.2), named vector-pinecone.wasm, with a full test suite and custom durability implementation at the level of vector operations.​
Milvus implementation: A WASM Component (WASI 0.2), named vector-milvus.wasm, with a full test suite and custom durability implementation at the level of vector operations.​
pgvector implementation: A WASM Component (WASI 0.2), named vector-pgvector.wasm, with a full test suite and custom durability implementation at the level of vector operations.​
Note: If you have a strong recommendation to swap out one or two of these with other popular / common vector databases, then as long as you get permission beforehand, that's okay with me. However, we definitely need Pinecone, pgvector, and Qdrant.

These components will require runtime configuration, notably API keys, connection strings, and database credentials. For configuring this information, the components can use environment variables for now (in the future, they will use wasi-runtime-config, but Golem does not support this yet, whereas Golem has good support for environment variables).

Moreover, the Rust components need to be tested within Golem to ensure compatibility with Golem 1.2.x.

This WIT has been designed by examining and comparing the APIs of Qdrant, Pinecone, Chroma, Weaviate, Milvus, and pgvector. However, given there are no implementations, it is possible the provided WIT is not the optimal abstraction across all these providers. Therefore, deviations from the proposed design can be made. However, to be accepted, any deviation must be fully justified and deemed by Golem core contributors to be an improvement from the original specification.
package golem:vector@1.0.0;

/// Core types and fundamental data structures for vector operations
interface types {
    /// Unique identifier for vectors and collections
    type id = string;
    
    /// Standard dense vector representation
    type dense-vector = list<f32>;
    
    /// Sparse vector with explicit indices
    record sparse-vector {
        /// Zero-based indices of non-zero elements
        indices: list<u32>,
        /// Values corresponding to the indices
        values: list<f32>,
        /// Total dimensionality of the vector space
        total-dimensions: u32,
    }
    
    /// Binary vector representation
    record binary-vector {
        /// Packed binary data
        data: list<u8>,
        /// Number of bits/dimensions
        dimensions: u32,
    }
    
    /// Half-precision vector (16-bit floats)
    record half-vector {
        /// Half-precision values (represented as f32 for compatibility)
        data: list<f32>,
        /// Number of dimensions
        dimensions: u32,
    }
    
    /// Vector data supporting multiple representations
    variant vector-data {
        /// Standard 32-bit floating point vector
        dense(dense-vector),
        /// Sparse vector representation
        sparse(sparse-vector),
        /// Binary/bit vector
        binary(binary-vector),
        /// Half-precision vector
        half(half-vector),
        /// Named vectors for multi-vector collections
        named(list<tuple<string, dense-vector>>),
        /// Hybrid dense + sparse combination
        hybrid(tuple<dense-vector, sparse-vector>),
    }
    
    /// Supported distance metrics
    enum distance-metric {
        /// Cosine similarity (1 - cosine distance)
        cosine,
        /// Euclidean (L2) distance
        euclidean,
        /// Dot product / inner product
        dot-product,
        /// Manhattan (L1) distance
        manhattan,
        /// Hamming distance (for binary vectors)
        hamming,
        /// Jaccard distance (for binary/sparse vectors)
        jaccard,
    }
    
    /// Metadata value types
    variant metadata-value {
        string-val(string),
        number-val(f64),
        integer-val(s64),
        boolean-val(bool),
        array-val(list<metadata-value>),
        object-val(list<tuple<string, metadata-value>>),
        null-val,
        /// Geographic coordinates
        geo-val(geo-coordinates),
        /// ISO 8601 datetime string
        datetime-val(string),
        /// Binary data
        blob-val(list<u8>),
    }
    
    /// Geographic coordinates
    record geo-coordinates {
        latitude: f64,
        longitude: f64,
    }
    
    /// Key-value metadata
    type metadata = list<tuple<string, metadata-value>>;
    
    /// Filter operators for metadata queries
    enum filter-operator {
        /// Equal to
        eq,
        /// Not equal to
        ne,
        /// Greater than
        gt,
        /// Greater than or equal
        gte,
        /// Less than
        lt,
        /// Less than or equal
        lte,
        /// Value is in list
        %in,
        /// Value is not in list
        nin,
        /// Text contains substring (case insensitive)
        contains,
        /// Text doesn't contain substring
        not-contains,
        /// Regular expression match
        regex,
        /// Geographic distance within radius
        geo-within,
        /// Geographic bounding box
        geo-bbox,
    }
    
    /// Basic filter condition
    record filter-condition {
        /// Field path (supports nested fields with dot notation)
        field: string,
        /// Filter operator
        operator: filter-operator,
        /// Value to compare against
        value: metadata-value,
    }
    
    /// Complex filter expressions with boolean logic
    variant filter-expression {
        /// Simple condition
        condition(filter-condition),
        /// Logical AND of multiple expressions
        and(list<filter-expression>),
        /// Logical OR of multiple expressions
        or(list<filter-expression>),
        /// Logical NOT of expression
        not(filter-expression),
    }
    
    /// Vector record for storage operations
    record vector-record {
        /// Unique identifier
        id: id,
        /// Vector data
        vector: vector-data,
        /// Associated metadata
        metadata: option<metadata>,
    }
    
    /// Search result with similarity score
    record search-result {
        /// Vector identifier
        id: id,
        /// Similarity score (higher = more similar)
        score: f32,
        /// Distance from query vector (lower = more similar)
        distance: f32,
        /// Vector data (if requested)
        vector: option<vector-data>,
        /// Associated metadata (if requested)
        metadata: option<metadata>,
    }
    
    /// Standard error types
    variant vector-error {
        /// Resource not found
        not-found(string),
        /// Resource already exists
        already-exists(string),
        /// Invalid parameters or configuration
        invalid-params(string),
        /// Feature not supported by this provider
        unsupported-feature(string),
        /// Vector dimension mismatch
        dimension-mismatch(string),
        /// Invalid vector format or data
        invalid-vector(string),
        /// Authentication/authorization failure
        unauthorized(string),
        /// Rate limit exceeded
        rate-limited(string),
        /// Internal provider error
        provider-error(string),
        /// Network/connection issues
        connection-error(string),
    }
    

}

/// Collection/index management and configuration
interface collections {
    use types.{id, distance-metric, vector-error};
    
    /// Index configuration parameters
    record index-config {
        index-type: option<string>,
        parameters: list<tuple<string, string>>,
    }
    
    /// Collection information and statistics
    record collection-info {
        name: string,
        description: option<string>,
        dimension: u32,
        metric: distance-metric,
        vector-count: u64,
        size-bytes: option<u64>,
        index-ready: bool,
        created-at: option<u64>,
        updated-at: option<u64>,
        provider-stats: option<types.metadata>,
    }
    
    /// Create or update collection (upsert)
    upsert-collection: func(
        name: string,
        description: option<string>,
        dimension: u32,
        metric: distance-metric,
        index-config: option<index-config>,
        metadata: option<types.metadata>
    ) -> result<collection-info, vector-error>;
    
    /// List all collections
    list-collections: func() -> result<list<collection-info>, vector-error>;
    
    /// Get collection information
    get-collection: func(name: string) -> result<collection-info, vector-error>;
    
    /// Update collection metadata only
    update-collection: func(
        name: string,
        description: option<string>,
        metadata: option<types.metadata>
    ) -> result<collection-info, vector-error>;
    
    /// Delete collection and all vectors
    delete-collection: func(name: string) -> result<_, vector-error>;
    
    /// Check if collection exists
    collection-exists: func(name: string) -> result<bool, vector-error>;
}

/// Core vector operations (CRUD)
interface vectors {
    use types.{id, vector-record, vector-data, metadata, filter-expression, vector-error};
    
    /// Batch operation result
    record batch-result {
        success-count: u32,
        failure-count: u32,
        errors: list<tuple<u32, vector-error>>,
    }
    
    /// List response with pagination
    record list-response {
        vectors: list<vector-record>,
        next-cursor: option<string>,
        total-count: option<u64>,
    }
    
    /// Upsert vectors into collection
    upsert-vectors: func(
        collection: string,
        vectors: list<vector-record>,
        namespace: option<string>
    ) -> result<batch-result, vector-error>;
    
    /// Upsert single vector (convenience)
    upsert-vector: func(
        collection: string,
        id: id,
        vector: vector-data,
        metadata: option<metadata>,
        namespace: option<string>
    ) -> result<_, vector-error>;
    
    /// Get vectors by IDs
    get-vectors: func(
        collection: string,
        ids: list<id>,
        namespace: option<string>,
        include-vectors: option<bool>,
        include-metadata: option<bool>
    ) -> result<list<vector-record>, vector-error>;
    
    /// Get single vector by ID (convenience)
    get-vector: func(
        collection: string,
        id: id,
        namespace: option<string>
    ) -> result<option<vector-record>, vector-error>;
    
    /// Update vector in place
    update-vector: func(
        collection: string,
        id: id,
        vector: option<vector-data>,
        metadata: option<metadata>,
        namespace: option<string>,
        merge-metadata: option<bool>
    ) -> result<_, vector-error>;
    
    /// Delete vectors by IDs
    delete-vectors: func(
        collection: string,
        ids: list<id>,
        namespace: option<string>
    ) -> result<u32, vector-error>;
    
    /// Delete vectors by filter
    delete-by-filter: func(
        collection: string,
        filter: filter-expression,
        namespace: option<string>
    ) -> result<u32, vector-error>;
    
    /// Delete all vectors in namespace
    delete-namespace: func(
        collection: string,
        namespace: string
    ) -> result<u32, vector-error>;
    
    /// List vectors with filtering and pagination
    list-vectors: func(
        collection: string,
        namespace: option<string>,
        filter: option<filter-expression>,
        limit: option<u32>,
        cursor: option<string>,
        include-vectors: option<bool>,
        include-metadata: option<bool>
    ) -> result<list-response, vector-error>;
    
    /// Count vectors matching filter
    count-vectors: func(
        collection: string,
        filter: option<filter-expression>,
        namespace: option<string>
    ) -> result<u64, vector-error>;
}

/// Core similarity search operations
interface search {
    use types.{id, vector-data, search-result, filter-expression, vector-error};
    
    /// Search query variants
    variant search-query {
        vector(vector-data),
        by-id(id),
        multi-vector(list<tuple<string, vector-data>>),
    }
    
    /// Similarity search
    search-vectors: func(
        collection: string,
        query: search-query,
        limit: u32,
        filter: option<filter-expression>,
        namespace: option<string>,
        include-vectors: option<bool>,
        include-metadata: option<bool>,
        min-score: option<f32>,
        max-distance: option<f32>,
        search-params: option<list<tuple<string, string>>>
    ) -> result<list<search-result>, vector-error>;
    
    /// Simple vector similarity search (convenience)
    find-similar: func(
        collection: string,
        vector: vector-data,
        limit: u32,
        namespace: option<string>
    ) -> result<list<search-result>, vector-error>;
    
    /// Batch similarity search
    batch-search: func(
        collection: string,
        queries: list<search-query>,
        limit: u32,
        filter: option<filter-expression>,
        namespace: option<string>,
        include-vectors: option<bool>,
        include-metadata: option<bool>,
        search-params: option<list<tuple<string, string>>>
    ) -> result<list<list<search-result>>, vector-error>;
}
}

/// Extended search capabilities (provider-dependent)
interface search-extended {
    use types.{id, vector-data, search-result, filter-expression, vector-error, metadata-value};
    
    /// Recommendation example types
    variant recommendation-example {
        vector-id(id),
        vector-data(vector-data),
    }
    
    enum recommendation-strategy {
        average-vector,
        best-score,
        centroid,
    }
    
    /// Context pair for discovery
    record context-pair {
        positive: recommendation-example,
        negative: recommendation-example,
    }
    
    /// Grouped search result
    record grouped-search-result {
        group-value: metadata-value,
        results: list<search-result>,
        group-count: u32,
    }
    
    /// Recommendation-based search
    recommend-vectors: func(
        collection: string,
        positive: list<recommendation-example>,
        negative: option<list<recommendation-example>>,
        limit: u32,
        filter: option<filter-expression>,
        namespace: option<string>,
        strategy: option<recommendation-strategy>,
        include-vectors: option<bool>,
        include-metadata: option<bool>
    ) -> result<list<search-result>, vector-error>;
    
    /// Discovery/context-based search
    discover-vectors: func(
        collection: string,
        context-pairs: list<context-pair>,
        limit: u32,
        filter: option<filter-expression>,
        namespace: option<string>,
        include-vectors: option<bool>,
        include-metadata: option<bool>
    ) -> result<list<search-result>, vector-error>;
    
    /// Grouped search for diverse results
    search-groups: func(
        collection: string,
        query: search.search-query,
        group-by: string,
        group-size: u32,
        max-groups: u32,
        filter: option<filter-expression>,
        namespace: option<string>,
        include-vectors: option<bool>,
        include-metadata: option<bool>
    ) -> result<list<grouped-search-result>, vector-error>;
    
    /// Range search within distance bounds
    search-range: func(
        collection: string,
        vector: vector-data,
        min-distance: option<f32>,
        max-distance: f32,
        filter: option<filter-expression>,
        namespace: option<string>,
        limit: option<u32>,
        include-vectors: option<bool>,
        include-metadata: option<bool>
    ) -> result<list<search-result>, vector-error>;
    
    /// Text/document search (auto-embedding)
    search-text: func(
        collection: string,
        query-text: string,
        limit: u32,
        filter: option<filter-expression>,
        namespace: option<string>
    ) -> result<list<search-result>, vector-error>;
}

/// Analytics and statistics
interface analytics {
    use types.{vector-error, metadata-value, filter-expression};
    
    /// Collection statistics
    record collection-stats {
        vector-count: u64,
        dimension: u32,
        size-bytes: u64,
        index-size-bytes: option<u64>,
        namespace-stats: list<tuple<string, namespace-stats>>,
        distance-distribution: option<distance-stats>,
    }
    
    record namespace-stats {
        vector-count: u64,
        size-bytes: u64,
    }
    
    record distance-stats {
        min-distance: f32,
        max-distance: f32,
        avg-distance: f32,
        percentiles: list<tuple<f32, f32>>,
    }
    
    /// Field statistics for metadata
    record field-stats {
        field-name: string,
        value-count: u64,
        unique-values: u64,
        null-count: u64,
        data-type: string,
        sample-values: list<metadata-value>,
    }
    
    /// Get collection statistics
    get-collection-stats: func(
        collection: string,
        namespace: option<string>
    ) -> result<collection-stats, vector-error>;
    
    /// Get field statistics
    get-field-stats: func(
        collection: string,
        field: string,
        namespace: option<string>
    ) -> result<field-stats, vector-error>;
    
    /// Get value distribution for a field
    get-field-distribution: func(
        collection: string,
        field: string,
        limit: option<u32>,
        namespace: option<string>
    ) -> result<list<tuple<metadata-value, u64>>, vector-error>;
}

/// Namespace/partition management
interface namespaces {
    use types.{vector-error, metadata};
    
    /// Namespace information
    record namespace-info {
        name: string,
        collection: string,
        vector-count: u64,
        size-bytes: u64,
        created-at: option<u64>,
        metadata: option<metadata>,
    }
    
    /// Create or update namespace (upsert)
    upsert-namespace: func(
        collection: string,
        namespace: string,
        metadata: option<metadata>
    ) -> result<namespace-info, vector-error>;
    
    /// List namespaces in collection
    list-namespaces: func(collection: string) -> result<list<namespace-info>, vector-error>;
    
    /// Get namespace information
    get-namespace: func(
        collection: string,
        namespace: string
    ) -> result<namespace-info, vector-error>;
    
    /// Delete namespace and all vectors within it
    delete-namespace: func(
        collection: string,
        namespace: string
    ) -> result<_, vector-error>;
    
    /// Check if namespace exists
    namespace-exists: func(
        collection: string,
        namespace: string
    ) -> result<bool, vector-error>;
}

/// Connection and configuration management
interface connection {
    use types.{vector-error, metadata};
    
    variant credentials {
        api-key(string),
        username-password(tuple<string, string>),
        token(string),
        certificate(list<u8>),
        oauth(oauth-config),
    }
    
    record oauth-config {
        client-id: string,
        client-secret: option<string>,
        token-url: string,
        scope: option<string>,
    }
    
    /// Connection status
    record connection-status {
        connected: bool,
        provider: option<string>,
        endpoint: option<string>,
        last-activity: option<u64>,
        connection-id: option<string>,
    }
    
    /// Establish connection to vector database
    connect: func(
        endpoint: string,
        credentials: option<credentials>,
        timeout-ms: option<u32>,
        options: option<metadata>
    ) -> result<_, vector-error>;
    
    /// Close connection
    disconnect: func() -> result<_, vector-error>;
    
    /// Get current connection status
    get-connection-status: func() -> result<connection-status, vector-error>;
    
    /// Test connection without modifying state
    test-connection: func(
        endpoint: string,
        credentials: option<credentials>,
        timeout-ms: option<u32>,
        options: option<metadata>
    ) -> result<bool, vector-error>;
}