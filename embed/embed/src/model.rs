#[repr(u8)]
#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutputFormat {
    FloatArray,
    Binary,
    Base64,
}

#[repr(u8)]
#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OutputDtype {
    FloatArray,
    Int8,
    Uint8,
    Binary,
    Ubinary,
}

#[repr(u8)]
#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
pub struct ImageUrl {
    pub url: String,
}

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
pub enum ContentPart {
    Text(String),
    Image(ImageUrl),
}

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
pub struct Kv {
    pub key: String,
    pub value: String,
}

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
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

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Usage {
    pub input_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
pub enum VectorData {
    Float(Vec<f32>),
    Int8(Vec<i8>),
    Uint8(Vec<u8>),
    Binary(Vec<i8>),
    Ubinary(Vec<u8>),
    Base64(String),
}

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
pub struct Embedding {
    pub index: u32,
    pub vector: VectorData,
}

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Embedding>,
    pub usage: Option<Usage>,
    pub model: String,
    pub provider_metadata_json: Option<String>,
}

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
pub struct RerankResult {
    pub index: u32,
    pub relevance_score: f32,
    pub document: Option<String>,
}

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
pub struct RerankResponse {
    pub results: Vec<RerankResult>,
    pub usage: Option<Usage>,
    pub model: String,
    pub provider_metadata_json: Option<String>,
}

#[cfg_attr(feature = "golem", derive(golem_rust::FromValueAndType, golem_rust::IntoValue))]
#[derive(Clone, Debug, PartialEq)]
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
