use golem_search::error::{from_reqwest_error, internal_error, search_error_from_status};
use golem_search::golem::search::types::SearchError;
use log::trace;
use reqwest::{Client, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::fmt::Debug;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MeilisearchApi {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchIndex {
    #[serde(rename = "taskUid")]
    pub task_uid: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "primaryKey")]
    pub primary_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchIndexListResponse {
    pub results: Vec<MeilisearchIndex>,
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchCreateIndexRequest {
    pub uid: String,
    #[serde(rename = "primaryKey", skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchTaskError {
    pub message: String,
    pub code: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchTask {
    #[serde(rename = "taskUid", alias = "uid")]
    pub task_uid: u64,
    #[serde(rename = "indexUid", skip_serializing_if = "Option::is_none")]
    pub index_uid: Option<String>,
    #[serde(rename = "batchUid", skip_serializing_if = "Option::is_none")]
    pub batch_uid: Option<u64>,
    pub status: String,
    #[serde(rename = "type")]
    pub task_type: String,
    #[serde(rename = "enqueuedAt")]
    pub enqueued_at: String,
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(rename = "finishedAt", skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(rename = "canceledBy", skip_serializing_if = "Option::is_none")]
    pub canceled_by: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MeilisearchTaskError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
}

// Meilisearch Document
pub type MeilisearchDocument = JsonMap<String, JsonValue>;

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchDocumentsResponse {
    pub results: Vec<MeilisearchDocument>,
    pub offset: u32,
    pub limit: u32,
    pub total: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchDocumentFetchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchSearchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub q: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<String>>,
    #[serde(
        rename = "attributesToRetrieve",
        skip_serializing_if = "Option::is_none"
    )]
    pub attributes_to_retrieve: Option<Vec<String>>,
    #[serde(
        rename = "attributesToHighlight",
        skip_serializing_if = "Option::is_none"
    )]
    pub attributes_to_highlight: Option<Vec<String>>,
    #[serde(rename = "attributesToCrop", skip_serializing_if = "Option::is_none")]
    pub attributes_to_crop: Option<Vec<String>>,
    #[serde(rename = "cropLength", skip_serializing_if = "Option::is_none")]
    pub crop_length: Option<u32>,
    #[serde(
        rename = "showMatchesPosition",
        skip_serializing_if = "Option::is_none"
    )]
    pub show_matches_position: Option<bool>,
    #[serde(rename = "matchingStrategy", skip_serializing_if = "Option::is_none")]
    pub matching_strategy: Option<String>,
    #[serde(rename = "showRankingScore", skip_serializing_if = "Option::is_none")]
    pub show_ranking_score: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeilisearchSearchResponse {
    pub hits: Vec<MeilisearchDocument>,
    pub offset: u32,
    pub limit: u32,
    #[serde(rename = "estimatedTotalHits")]
    pub estimated_total_hits: u32,
    #[serde(rename = "processingTimeMs")]
    pub processing_time_ms: u32,
    pub query: String,
    #[serde(rename = "facetDistribution", skip_serializing_if = "Option::is_none")]
    pub facet_distribution: Option<JsonMap<String, JsonValue>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MeilisearchSettings {
    #[serde(
        rename = "displayedAttributes",
        skip_serializing_if = "Option::is_none"
    )]
    pub displayed_attributes: Option<Vec<String>>,
    #[serde(
        rename = "searchableAttributes",
        skip_serializing_if = "Option::is_none"
    )]
    pub searchable_attributes: Option<Vec<String>>,
    #[serde(
        rename = "filterableAttributes",
        skip_serializing_if = "Option::is_none"
    )]
    pub filterable_attributes: Option<Vec<String>>,
    #[serde(rename = "sortableAttributes", skip_serializing_if = "Option::is_none")]
    pub sortable_attributes: Option<Vec<String>>,
    #[serde(rename = "rankingRules", skip_serializing_if = "Option::is_none")]
    pub ranking_rules: Option<Vec<String>>,
    #[serde(rename = "stopWords", skip_serializing_if = "Option::is_none")]
    pub stop_words: Option<Vec<String>>,
    #[serde(rename = "synonyms", skip_serializing_if = "Option::is_none")]
    pub synonyms: Option<JsonMap<String, JsonValue>>,
    #[serde(rename = "distinctAttribute", skip_serializing_if = "Option::is_none")]
    pub distinct_attribute: Option<String>,
    #[serde(rename = "typoTolerance", skip_serializing_if = "Option::is_none")]
    pub typo_tolerance: Option<JsonValue>,
    #[serde(rename = "faceting", skip_serializing_if = "Option::is_none")]
    pub faceting: Option<JsonValue>,
    #[serde(rename = "pagination", skip_serializing_if = "Option::is_none")]
    pub pagination: Option<JsonValue>,
}

impl MeilisearchApi {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to initialize HTTP client");

        Self {
            client,
            base_url,
            api_key,
        }
    }

    fn create_request(&self, method: &str, url: &str) -> RequestBuilder {
        trace!("[Meilisearch] HTTP {method} {url}");

        let mut req = match method {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "DELETE" => self.client.delete(url),
            "PATCH" => self.client.patch(url),
            _ => self
                .client
                .request(reqwest::Method::from_bytes(method.as_bytes()).unwrap(), url),
        };

        if let Some(api_key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {api_key}"));
        }
        req = req.header("Content-Type", "application/json");

        req
    }
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, SearchError> {
    let status = response.status();

    trace!("Received response from Meilisearch API: {response:?}");

    if status.is_success() {
        let body = response
            .json::<T>()
            .map_err(|err| from_reqwest_error("Failed to decode response body", err))?;

        trace!("Received response from Meilisearch API: {body:?}");

        Ok(body)
    } else {
        let error_body = response
            .text()
            .map_err(|err| from_reqwest_error("Failed to receive error response body", err))?;

        trace!("Received {status} response from Meilisearch API: {error_body:?}");

        Err(search_error_from_status(status))
    }
}

impl MeilisearchApi {
    pub fn list_indexes(&self) -> Result<MeilisearchIndexListResponse, SearchError> {
        trace!("Listing indexes");

        let url = format!("{}/indexes", self.base_url);

        let response = self
            .create_request("GET", &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to list indexes: {e}")))?;

        parse_response(response)
    }

    pub fn _get_index(&self, index_uid: &str) -> Result<MeilisearchIndex, SearchError> {
        trace!("Getting index: {index_uid}");

        let url = format!("{}/indexes/{}", self.base_url, index_uid);

        let response = self
            .create_request("GET", &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to get index: {e}")))?;

        parse_response(response)
    }

    pub fn create_index(
        &self,
        request: &MeilisearchCreateIndexRequest,
    ) -> Result<MeilisearchTask, SearchError> {
        trace!("Creating index: {}", request.uid);

        let url = format!("{}/indexes", self.base_url);

        let response = self
            .create_request("POST", &url)
            .json(request)
            .send()
            .map_err(|e| internal_error(format!("Failed to create index: {e}")))?;

        parse_response(response)
    }

    pub fn delete_index(&self, index_uid: &str) -> Result<MeilisearchTask, SearchError> {
        trace!("Deleting index: {index_uid}");

        let url = format!("{}/indexes/{}", self.base_url, index_uid);

        let response = self
            .create_request("DELETE", &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to delete index: {e}")))?;

        parse_response(response)
    }

    pub fn _get_documents(
        &self,
        index_uid: &str,
        request: &MeilisearchDocumentFetchRequest,
    ) -> Result<MeilisearchDocumentsResponse, SearchError> {
        trace!("Getting documents from index: {index_uid}");

        let url = format!("{}/indexes/{}/documents/fetch", self.base_url, index_uid);

        let response = self
            .create_request("POST", &url)
            .json(request)
            .send()
            .map_err(|e| internal_error(format!("Failed to get documents: {e}")))?;

        parse_response(response)
    }

    pub fn get_document(
        &self,
        index_uid: &str,
        document_id: &str,
    ) -> Result<Option<MeilisearchDocument>, SearchError> {
        trace!("Getting document {document_id} from index: {index_uid}");

        let url = format!(
            "{}/indexes/{}/documents/{}",
            self.base_url, index_uid, document_id
        );

        let response = self
            .create_request("GET", &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to get document: {e}")))?;

        if response.status() == 404 {
            Ok(None)
        } else {
            Ok(Some(parse_response(response)?))
        }
    }

    pub fn add_documents(
        &self,
        index_uid: &str,
        documents: &[MeilisearchDocument],
    ) -> Result<MeilisearchTask, SearchError> {
        trace!(
            "Adding {} documents to index: {}",
            documents.len(),
            index_uid
        );

        let url = format!("{}/indexes/{}/documents", self.base_url, index_uid);

        let response = self
            .create_request("POST", &url)
            .json(documents)
            .send()
            .map_err(|e| internal_error(format!("Failed to add documents: {e}")))?;

        parse_response(response)
    }

    pub fn _update_documents(
        &self,
        index_uid: &str,
        documents: &[MeilisearchDocument],
    ) -> Result<MeilisearchTask, SearchError> {
        trace!(
            "Updating {} documents in index: {}",
            documents.len(),
            index_uid
        );

        let url = format!("{}/indexes/{}/documents", self.base_url, index_uid);

        let response = self
            .create_request("PUT", &url)
            .json(documents)
            .send()
            .map_err(|e| internal_error(format!("Failed to update documents: {e}")))?;

        parse_response(response)
    }

    pub fn delete_document(
        &self,
        index_uid: &str,
        document_id: &str,
    ) -> Result<MeilisearchTask, SearchError> {
        trace!("Deleting document {document_id} from index: {index_uid}");

        let url = format!(
            "{}/indexes/{}/documents/{}",
            self.base_url, index_uid, document_id
        );

        let response = self
            .create_request("DELETE", &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to delete document: {e}")))?;

        parse_response(response)
    }

    pub fn delete_documents(
        &self,
        index_uid: &str,
        document_ids: &[String],
    ) -> Result<MeilisearchTask, SearchError> {
        trace!(
            "Deleting {} documents from index: {}",
            document_ids.len(),
            index_uid
        );

        let url = format!(
            "{}/indexes/{}/documents/delete-batch",
            self.base_url, index_uid
        );

        let response = self
            .create_request("POST", &url)
            .json(document_ids)
            .send()
            .map_err(|e| internal_error(format!("Failed to delete documents: {e}")))?;

        parse_response(response)
    }

    pub fn _delete_all_documents(&self, index_uid: &str) -> Result<MeilisearchTask, SearchError> {
        trace!("Deleting all documents from index: {index_uid}");

        let url = format!("{}/indexes/{}/documents", self.base_url, index_uid);

        let response = self
            .create_request("DELETE", &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to delete all documents: {e}")))?;

        parse_response(response)
    }

    pub fn search(
        &self,
        index_uid: &str,
        request: &MeilisearchSearchRequest,
    ) -> Result<MeilisearchSearchResponse, SearchError> {
        trace!("Searching in index: {index_uid}");

        let url = format!("{}/indexes/{}/search", self.base_url, index_uid);

        let response = self
            .create_request("POST", &url)
            .json(request)
            .send()
            .map_err(|e| internal_error(format!("Failed to search: {e}")))?;

        parse_response(response)
    }

    pub fn get_settings(&self, index_uid: &str) -> Result<MeilisearchSettings, SearchError> {
        trace!("Getting settings for index: {index_uid}");

        let url = format!("{}/indexes/{}/settings", self.base_url, index_uid);

        let response = self
            .create_request("GET", &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to get settings: {e}")))?;

        parse_response(response)
    }

    pub fn update_settings(
        &self,
        index_uid: &str,
        settings: &MeilisearchSettings,
    ) -> Result<MeilisearchTask, SearchError> {
        trace!("Updating settings for index: {index_uid}");

        let url = format!("{}/indexes/{}/settings", self.base_url, index_uid);

        let response = self
            .create_request("PATCH", &url)
            .json(settings)
            .send()
            .map_err(|e| internal_error(format!("Failed to update settings: {e}")))?;

        parse_response(response)
    }

    pub fn _reset_settings(&self, index_uid: &str) -> Result<MeilisearchTask, SearchError> {
        trace!("Resetting settings for index: {index_uid}");

        let url = format!("{}/indexes/{}/settings", self.base_url, index_uid);

        let response = self
            .create_request("DELETE", &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to reset settings: {e}")))?;

        parse_response(response)
    }

    // Task Management (for checking async operation status)
    pub fn get_task(&self, task_uid: u64) -> Result<MeilisearchTask, SearchError> {
        trace!("Getting task: {task_uid}");

        let url = format!("{}/tasks/{}", self.base_url, task_uid);

        let response = self
            .create_request("GET", &url)
            .send()
            .map_err(|e| internal_error(format!("Failed to get task: {e}")))?;

        parse_response(response)
    }

    /// Production-level wait_for_task with exponential backoff
    pub fn wait_for_task(&self, task_uid: u64) -> Result<(), SearchError> {
        self.wait_for_task_with_config(
            task_uid,
            30,
            Duration::from_millis(100),
            Duration::from_secs(5),
        )
    }

    pub fn wait_for_task_with_config(
        &self,
        task_uid: u64,
        max_attempts: u32,
        initial_delay: Duration,
        max_delay: Duration,
    ) -> Result<(), SearchError> {
        trace!("Waiting for task {task_uid} with exponential backoff (max_attempts: {max_attempts}, initial_delay: {initial_delay:?}, max_delay: {max_delay:?})");

        let mut delay = initial_delay;

        for attempt in 1..=max_attempts {
            let task = self.get_task(task_uid)?;
            trace!(
                "Task {} attempt {}/{}: status = {}",
                task_uid,
                attempt,
                max_attempts,
                task.status
            );

            match task.status.as_str() {
                "succeeded" => {
                    trace!("Task {task_uid} completed successfully after {attempt} attempts");
                    return Ok(());
                }
                "failed" => {
                    let error_msg = format!("Task {task_uid} failed after {attempt} attempts");
                    trace!("{error_msg}");
                    return Err(SearchError::Internal(error_msg));
                }
                "canceled" => {
                    let error_msg =
                        format!("Task {task_uid} was canceled after {attempt} attempts");
                    trace!("{error_msg}");
                    return Err(SearchError::Internal(error_msg));
                }
                status => {
                    trace!(
                        "Task {task_uid} is still {status}, waiting {delay:?} before retry {attempt}/{max_attempts}"
                    );

                    std::thread::sleep(delay);

                    let next_delay = std::cmp::min(delay * 2, max_delay);

                    let jitter_range = next_delay.as_millis() / 10; // 10% jitter
                    let jitter = Duration::from_millis(
                        (task_uid % (jitter_range as u64 * 2)).saturating_sub(jitter_range as u64),
                    );
                    delay = next_delay.saturating_add(jitter);
                }
            }
        }

        let error_msg = format!(
            "Task {task_uid} timed out after {max_attempts} attempts (max delay: {max_delay:?})"
        );
        trace!("{error_msg}");
        Err(SearchError::Internal(error_msg))
    }
}
