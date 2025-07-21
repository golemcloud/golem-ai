use golem_search::error::{from_reqwest_error, internal_error, search_error_from_status};
use golem_search::golem::search::types::SearchError;
use log::trace;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

// Custom deserializer to handle null values as empty vectors
fn deserialize_nullable_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<Vec<String>>::deserialize(deserializer).map(|opt| opt.unwrap_or_default())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IndexSettings {
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        rename = "searchableAttributes",
        deserialize_with = "deserialize_nullable_vec",
        default
    )]
    pub searchable_attributes: Vec<String>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        rename = "attributesForFaceting",
        deserialize_with = "deserialize_nullable_vec",
        default
    )]
    pub attributes_for_faceting: Vec<String>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        rename = "unretrievableAttributes",
        deserialize_with = "deserialize_nullable_vec",
        default
    )]
    pub unretrievable_attributes: Vec<String>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        rename = "attributesToRetrieve",
        deserialize_with = "deserialize_nullable_vec",
        default
    )]
    pub attributes_to_retrieve: Vec<String>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        rename = "ranking",
        deserialize_with = "deserialize_nullable_vec",
        default
    )]
    pub ranking: Vec<String>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        rename = "customRanking",
        deserialize_with = "deserialize_nullable_vec",
        default
    )]
    pub custom_ranking: Vec<String>,
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        rename = "replicas",
        deserialize_with = "deserialize_nullable_vec",
        default
    )]
    pub replicas: Vec<String>,
}

/// The Algolia Search API client for managing indices and performing search
/// Based on https://www.algolia.com/doc/api-reference/search-api/
#[derive(Clone)]
pub struct AlgoliaSearchApi {
    client: Client,
    application_id: String,
    api_key: String,
    search_url: String,
    write_url: String,
}

impl AlgoliaSearchApi {
    pub fn new(application_id: String, api_key: String) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");

        let search_url = format!("https://{}.algolia.net", application_id.to_lowercase());
        let write_url = format!("https://{}.algolia.net", application_id.to_lowercase());

        Self {
            application_id,
            api_key,
            client,
            search_url,
            write_url,
        }
    }

    fn create_request(&self, method: Method, url: &str) -> RequestBuilder {
        self.client
            .request(method, url)
            .header("X-Algolia-Application-Id", &self.application_id)
            .header("X-Algolia-API-Key", &self.api_key)
            .header("Content-Type", "application/json")
    }

    pub fn delete_index(&self, index_name: &str) -> Result<DeleteIndexResponse, SearchError> {
        trace!("Deleting index: {index_name}");

        let url = format!("{}/1/indexes/{}", self.write_url, index_name);

        let response = self
            .create_request(Method::DELETE, &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to delete index: {e}")))?;

        parse_response(response)
    }

    pub fn list_indexes(&self) -> Result<ListIndexesResponse, SearchError> {
        trace!("Listing indexes");

        let url = format!("{}/1/indexes", self.write_url);

        let response = self
            .create_request(Method::GET, &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to list indexes: {e}")))?;

        parse_response(response)
    }

    pub fn save_object(
        &self,
        index_name: &str,
        object: &AlgoliaObject,
    ) -> Result<SaveObjectResponse, SearchError> {
        trace!("Saving object to index: {index_name}");

        let url = format!("{}/1/indexes/{}", self.write_url, index_name);

        let response = self
            .create_request(Method::POST, &url)
            .json(object)
            .send()
            .map_err(|e| internal_error(format!("Failed to save object: {e}")))?;

        parse_response(response)
    }

    pub fn save_objects(
        &self,
        index_name: &str,
        objects: &[AlgoliaObject],
    ) -> Result<SaveObjectsResponse, SearchError> {
        trace!("Saving {} objects to index: {index_name}", objects.len());

        let url = format!("{}/1/indexes/{}/batch", self.write_url, index_name);
        let batch_request = BatchRequest {
            requests: objects
                .iter()
                .map(|obj| BatchOperation {
                    action: "addObject".to_string(),
                    body: obj.clone(),
                })
                .collect(),
        };

        let response = self
            .create_request(Method::POST, &url)
            .json(&batch_request)
            .send()
            .map_err(|e| internal_error(format!("Failed to save objects: {e}")))?;

        parse_response(response)
    }

    pub fn delete_object(
        &self,
        index_name: &str,
        object_id: &str,
    ) -> Result<DeleteObjectResponse, SearchError> {
        trace!("Deleting object {object_id} from index: {index_name}");

        let url = format!("{}/1/indexes/{}/{}", self.write_url, index_name, object_id);

        let response = self
            .create_request(Method::DELETE, &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to delete object: {e}")))?;

        parse_response(response)
    }

    pub fn delete_objects(
        &self,
        index_name: &str,
        object_ids: &[String],
    ) -> Result<DeleteObjectsResponse, SearchError> {
        trace!(
            "Deleting {} objects from index: {index_name}",
            object_ids.len()
        );

        let url = format!("{}/1/indexes/{}/batch", self.write_url, index_name);
        let batch_request = BatchRequest {
            requests: object_ids
                .iter()
                .map(|id| BatchOperation {
                    action: "deleteObject".to_string(),
                    body: AlgoliaObject {
                        object_id: Some(id.clone()),
                        content: serde_json::Value::Null,
                    },
                })
                .collect(),
        };

        let response = self
            .create_request(Method::POST, &url)
            .json(&batch_request)
            .send()
            .map_err(|e| internal_error(format!("Failed to delete objects: {e}")))?;

        parse_response(response)
    }

    pub fn get_object(
        &self,
        index_name: &str,
        object_id: &str,
    ) -> Result<Option<AlgoliaObject>, SearchError> {
        trace!("Getting object {object_id} from index: {index_name}");

        let url = format!("{}/1/indexes/{}/{}", self.search_url, index_name, object_id);

        let response = self.create_request(Method::GET, &url).send();

        match response {
            Ok(resp) => {
                if resp.status() == 404 {
                    Ok(None)
                } else {
                    let object: AlgoliaObject = parse_response(resp)?;
                    Ok(Some(object))
                }
            }
            Err(e) => Err(internal_error(format!("Failed to get object: {e}"))),
        }
    }

    pub fn search(
        &self,
        index_name: &str,
        query: &SearchQuery,
    ) -> Result<SearchResponse, SearchError> {
        trace!("Searching index {index_name} with query: {query:?}");

        let url = format!("{}/1/indexes/{}/query", self.search_url, index_name);

        let response = self.create_request(Method::POST, &url).json(query).send();

        match response {
            Ok(resp) => parse_response(resp),
            Err(e) => {
                let error_msg = format!("Failed to search: {url}: {e}");
                Err(internal_error(error_msg))
            }
        }
    }

    pub fn get_settings(&self, index_name: &str) -> Result<IndexSettings, SearchError> {
        trace!("Getting settings for index: {index_name}");

        let url = format!("{}/1/indexes/{}/settings", self.write_url, index_name);

        let response = self
            .create_request(Method::GET, &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to get settings: {e}")))?;

        parse_response(response)
    }

    pub fn set_settings(
        &self,
        index_name: &str,
        settings: &IndexSettings,
    ) -> Result<SetSettingsResponse, SearchError> {
        trace!("Setting settings for index: {index_name}");

        let url = format!("{}/1/indexes/{}/settings", self.write_url, index_name);

        let response = self
            .create_request(Method::PUT, &url)
            .json(settings)
            .send()
            .map_err(|e| internal_error(format!("Failed to set settings: {e}")))?;

        parse_response(response)
    }

    pub fn _wait_for_task(&self, index_name: &str, task_id: u64) -> Result<(), SearchError> {
        trace!("Waiting for task {task_id} on index {index_name}");
        let url = format!(
            "{}/1/indexes/{}/task/{}",
            self.write_url, index_name, task_id
        );

        for _ in 0..20 {
            // Poll for up to 10 seconds
            let response = self.create_request(Method::GET, &url).send();
            match response {
                Ok(resp) => {
                    let body_str = match resp.text() {
                        Ok(s) => s,
                        Err(_e) => {
                            continue;
                        }
                    };
                    let body: serde_json::Value = match serde_json::from_str(&body_str) {
                        Ok(b) => b,
                        Err(_e) => {
                            continue;
                        }
                    };
                    if body.get("status").and_then(|s| s.as_str()) == Some("published") {
                        return Ok(());
                    }
                }
                Err(e) => {
                    eprintln!("[Algolia] Error waiting for task: {e:?}");
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        Err(internal_error(format!(
            "Task {task_id} did not complete in time."
        )))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgoliaObject {
    #[serde(rename = "objectID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    #[serde(flatten)]
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numeric_filters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(rename = "hitsPerPage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hits_per_page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub facets: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attributes_to_retrieve: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typo_tolerance: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analytics: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub page: u32,
    #[serde(rename = "nbHits")]
    pub nb_hits: u32,
    #[serde(rename = "nbPages")]
    pub nb_pages: u32,
    #[serde(rename = "hitsPerPage")]
    pub hits_per_page: u32,
    #[serde(rename = "processingTimeMS")]
    pub processing_time_ms: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets_stats: Option<serde_json::Value>,
    #[serde(rename = "exhaustiveNbHits", default)]
    pub exhaustive_nb_hits: bool,
    #[serde(rename = "exhaustiveFacetsCount", default)]
    pub exhaustive_facets_count: bool,
    pub query: String,
    pub params: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    #[serde(rename = "objectID")]
    pub object_id: String,
    #[serde(rename = "_highlightResult")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_result: Option<serde_json::Value>,
    #[serde(rename = "_snippetResult")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet_result: Option<serde_json::Value>,
    #[serde(rename = "_rankingInfo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_info: Option<RankingInfo>,
    #[serde(flatten)]
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingInfo {
    #[serde(rename = "nbTypos")]
    pub nb_typos: u32,
    #[serde(rename = "firstMatchedWord")]
    pub first_matched_word: u32,
    #[serde(rename = "proximityDistance")]
    pub proximity_distance: u32,
    #[serde(rename = "userScore")]
    pub user_score: u32,
    #[serde(rename = "geoDistance")]
    pub geo_distance: u32,
    #[serde(rename = "geoPrecision")]
    pub geo_precision: u32,
    #[serde(rename = "nbExactWords")]
    pub nb_exact_words: u32,
    pub words: u32,
    pub filters: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteIndexResponse {
    #[serde(rename = "taskID")]
    pub task_id: u64,
    #[serde(rename = "deletedAt")]
    pub deleted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListIndexesResponse {
    pub items: Vec<IndexInfo>,
    #[serde(rename = "nbPages")]
    pub nb_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub entries: u64,
    #[serde(rename = "dataSize")]
    pub data_size: u64,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    #[serde(rename = "lastBuildTimeS")]
    pub last_build_time_s: u64,
    #[serde(rename = "numberOfPendingTasks")]
    pub number_of_pending_tasks: u64,
    #[serde(rename = "pendingTask")]
    pub pending_task: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveObjectResponse {
    #[serde(rename = "taskID")]
    pub task_id: u64,
    #[serde(rename = "objectID")]
    pub object_id: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveObjectsResponse {
    #[serde(rename = "taskID")]
    pub task_id: u64,
    #[serde(rename = "objectIDs")]
    pub object_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteObjectResponse {
    #[serde(rename = "taskID")]
    pub task_id: u64,
    #[serde(rename = "deletedAt")]
    pub deleted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteObjectsResponse {
    #[serde(rename = "taskID")]
    pub task_id: u64,
    #[serde(rename = "objectIDs")]
    pub object_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSettingsResponse {
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "taskID")]
    pub task_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub requests: Vec<BatchOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperation {
    pub action: String,
    pub body: AlgoliaObject,
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, SearchError> {
    let status = response.status();

    trace!("Received response from Algolia API: {response:?}");

    if status.is_success() {
        let body = response
            .json::<T>()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))?;

        trace!("Received response from xAI API: {body:?}");

        Ok(body)
    } else {
        let error_body = response
            .text()
            .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;

        trace!("Received {status} response from xAI API: {error_body:?}");

        Err(search_error_from_status(status))
    }
}
