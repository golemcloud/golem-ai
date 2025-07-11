use std::{collections::HashMap, fmt::Debug};

use golem_search::{
    error::{from_reqwest_error, from_status_code},
    golem::search::types::SearchError,
};
use log::trace;
use reqwest::{header::HeaderMap, Client, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub struct AlgoliaClient {
    client: Client,
    app_id: String,
    api_key: String,
}

impl AlgoliaClient {
    pub fn new(app_id: &str, api_key: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("X-Algolia-API-Key", api_key.parse().unwrap());
        headers.insert("X-Algolia-Application-Id", app_id.parse().unwrap());
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to initialize HTTP client");
        Self {
            client,
            app_id: app_id.to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub fn delete_index(&self, index_name: &str) -> Result<(), SearchError> {
        trace!("Deleting index : {}", index_name);

        let url = format!(
            "https://{}.algolia.net/1/indexes/{}",
            self.app_id, index_name
        );
        let response: Response = self
            .client
            .delete(url)
            .send()
            .map_err(|error| from_reqwest_error("Request Failed", error))?;

        match parse_response::<DeleteIndexResponse>(response) {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub fn list_indexes(
        &self,
        query_params: ListIndexesQueryParams,
    ) -> Result<Vec<IndexItem>, SearchError> {
        trace!("Listing indexes");

        let url = format!("https://{}.algolia.net/1/indexes", self.app_id);
        let response: Response = self
            .client
            .get(url)
            .query(&query_params)
            .send()
            .map_err(|error| from_reqwest_error("Request Failed", error))?;

        match parse_response::<ListIndexesResponse>(response) {
            Ok(response) => Ok(response.items),
            Err(error) => Err(error),
        }
    }

    fn create_index(
        &self,
        index_name: &str,
        request_body: CreateIndexRequest,
    ) -> Result<(), SearchError> {
        trace!("Creating index : {}", index_name);

        let url = format!(
            "https://{}.algolia.net/1/indexes/{}/settings",
            self.app_id, index_name
        );
        let response: Response = self
            .client
            .post(url)
            .json(&request_body)
            .send()
            .map_err(|error| from_reqwest_error("Request Failed", error))?;

        match parse_response::<CreateIndexResponse>(response) {
            Ok(_) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CreateIndexRequest {
    #[serde(rename = "indexName")]
    index_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CreateIndexResponse {
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "taskID")]
    task_id: u128,
}



#[derive(Deserialize, Debug)]
struct DeleteIndexResponse {
    #[serde(rename = "taskId")]
    task_id: u128,
    #[serde(rename = "deletedAt")]
    deleted_at: String,
}

#[derive(Serialize, Debug)]
pub struct ListIndexesQueryParams {
    #[serde(rename = "hitsPerPage")]
    pub hits_per_page: Option<u16>,
    pub page: Option<u16>,
}

#[derive(Deserialize, Debug)]
struct ListIndexesResponse {
    items: Vec<IndexItem>,
    #[serde(rename = "nbPages")]
    nb_pages: u64,
}

#[derive(Deserialize, Debug)]
pub struct IndexItem {
    pub name: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    entries: u64,
    #[serde(rename = "dataSize")]
    data_size: u64,
    #[serde(rename = "fileSize")]
    file_size: u64,
    #[serde(rename = "lastBuildTimeS")]
    last_build_time_s: u64,
    #[serde(rename = "numberOfPendingTasks")]
    number_of_pending_tasks: u64,
    #[serde(rename = "pendingTask")]
    pending_task: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    primary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replicas: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "virtual")]
    virtual_index: Option<bool>,
}

fn parse_response<T: DeserializeOwned + Debug>(response: Response) -> Result<T, SearchError> {
    let status = response.status();
    match status {
        StatusCode::OK => {
            let body = response.json::<T>().map_err(|err: reqwest::Error| {
                from_reqwest_error("Failed to decode response body", err)
            })?;
            trace!("Received response from Algolia API: {body:?}");
            Ok(body)
        }
        _ => Err(from_status_code(response)),
    }
}
