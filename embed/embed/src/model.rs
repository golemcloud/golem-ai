
#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    golem_rust::FromValueAndType,
    golem_rust::IntoValue,
)]
pub enum TaskType {
    RetrievalQuery,
    RetrievalDocument,
    SemanticSimilarity,
    Classification,
    Clustering,
    QuestionAnswering,
    FactVerification,
    CodeRetrieval,
}

#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    golem_rust::FromValueAndType,
    golem_rust::IntoValue,
)]
pub enum OutputFormat {
    FloatArray,
    Binary,
    Base64,
}

#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    golem_rust::FromValueAndType,
    golem_rust::IntoValue,
)]
pub enum OutputDtype {
    FloatArray,
    Int8,
    Uint8,
    Binary,
    Ubinary,
}

#[repr(u8)]
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    golem_rust::FromValueAndType,
    golem_rust::IntoValue,
)]
pub enum ErrorCode {
    InvalidRequest,
    ModelNotFound,
    Unsupported,
    AuthenticationFailed,
    ProviderError,
    RateLimitExceeded,
    InternalError,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub enum ContentPart {
    Text(String),
    Image(ImageUrl),
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct Kv {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct Config {
    pub model: Option<String>,
    pub task_type: Option<TaskType>,
    pub dimensions: Option<u32>,
    pub truncation: Option<bool>,
    pub output_format: Option<OutputFormat>,
    pub output_dtype: Option<OutputDtype>,
    pub user: Option<String>,
    pub provider_options: Vec<Kv>,
}

#[derive(Clone, Copy, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct Usage {
    pub input_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub enum VectorData {
    Float(Vec<f32>),
    Int8(Vec<i8>),
    Uint8(Vec<u8>),
    Binary(Vec<i8>),
    Ubinary(Vec<u8>),
    Base64(String),
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct Embedding {
    pub index: u32,
    pub vector: VectorData,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Embedding>,
    pub usage: Option<Usage>,
    pub model: String,
    pub provider_metadata_json: Option<String>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct RerankResult {
    pub index: u32,
    pub relevance_score: f32,
    pub document: Option<String>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct RerankResponse {
    pub results: Vec<RerankResult>,
    pub usage: Option<Usage>,
    pub model: String,
    pub provider_metadata_json: Option<String>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    pub provider_error_json: Option<String>,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}
