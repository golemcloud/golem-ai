use crate::SearchStreamInterface;

pub type IndexName = String;
pub type DocumentId = String;
pub type Json = String;

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub enum SearchError {
    IndexNotFound,
    InvalidQuery(String),
    Unsupported,
    Internal(String),
    Timeout,
    RateLimited,
}

impl core::fmt::Display for SearchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for SearchError {}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct Doc {
    pub id: DocumentId,
    pub content: Json,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct HighlightConfig {
    pub fields: Vec<String>,
    pub pre_tag: Option<String>,
    pub post_tag: Option<String>,
    pub max_length: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct SearchConfig {
    pub timeout_ms: Option<u32>,
    pub boost_fields: Vec<(String, f32)>,
    pub attributes_to_retrieve: Vec<String>,
    pub language: Option<String>,
    pub typo_tolerance: Option<bool>,
    pub exact_match_boost: Option<f32>,
    pub provider_params: Option<Json>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub filters: Vec<String>,
    pub sort: Vec<String>,
    pub facets: Vec<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub offset: Option<u32>,
    pub highlight: Option<HighlightConfig>,
    pub config: Option<SearchConfig>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct SearchHit {
    pub id: DocumentId,
    pub score: Option<f64>,
    pub content: Option<Json>,
    pub highlights: Option<Json>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct SearchResults {
    pub total: Option<u32>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub hits: Vec<SearchHit>,
    pub facets: Option<Json>,
    pub took_ms: Option<u32>,
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
pub enum FieldType {
    Text,
    Keyword,
    Integer,
    Float,
    Boolean,
    Date,
    GeoPoint,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct SchemaField {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub facet: bool,
    pub sort: bool,
    pub index: bool,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct Schema {
    pub fields: Vec<SchemaField>,
    pub primary_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, golem_rust::FromValueAndType, golem_rust::IntoValue)]
pub struct CreateIndexOptions {
    pub index_name: String,
    pub schema: Option<Schema>,
}

pub struct SearchStream {
    inner: Box<dyn SearchStreamInterface>,
}

impl SearchStream {
    pub fn new<T: SearchStreamInterface>(val: T) -> Self {
        Self {
            inner: Box::new(val),
        }
    }

    pub fn get<T: SearchStreamInterface>(&self) -> &T {
        self.inner
            .as_any()
            .downcast_ref::<T>()
            .expect("SearchStream type mismatch")
    }

    pub fn get_mut<T: SearchStreamInterface>(&mut self) -> &mut T {
        self.inner
            .as_any_mut()
            .downcast_mut::<T>()
            .expect("SearchStream type mismatch")
    }
}

impl std::ops::Deref for SearchStream {
    type Target = dyn SearchStreamInterface;

    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl std::ops::DerefMut for SearchStream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}
