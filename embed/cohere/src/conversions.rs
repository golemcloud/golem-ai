use std::{fs, path::Path};

use base64::{engine::general_purpose, Engine};
use golem_embed::{
    error::unsupported,
    golem::embed::embed::{
        Config, ContentPart, Embedding, EmbeddingResponse as GolemEmbeddingResponse, Error,
        OutputDtype, RerankResponse as GolemRerankResponse, RerankResult, TaskType, Usage,
        VectorData,
    },
};
use log::trace;
use golem_wasi_http::{Client, Url};

use crate::client::{
    EmbeddingRequest, EmbeddingResponse, EmbeddingType, InputType, Meta, RerankRequest,
    RerankResponse,
};

fn output_dtype_to_cohere_embedding_type(dtype: OutputDtype) -> EmbeddingType {
    match dtype {
        OutputDtype::FloatArray => EmbeddingType::Float,
        OutputDtype::Int8 => EmbeddingType::Int8,
        OutputDtype::Uint8 => EmbeddingType::Uint8,
        OutputDtype::Binary => EmbeddingType::Binary,
        OutputDtype::Ubinary => EmbeddingType::Ubinary,
    }
}

pub fn create_embed_request(
    inputs: Vec<ContentPart>,
    config: Config,
) -> Result<EmbeddingRequest, Error> {
    let mut text_inputs = Vec::new();
    let mut image_inputs = Vec::new();
    for input in inputs {
        match input {
            ContentPart::Text(text) => text_inputs.push(text),
            ContentPart::Image(image) => match image_to_base64(&image.url) {
                Ok(base64_data) => image_inputs.push(base64_data),
                Err(err) => {
                    trace!("Failed to encode image: {}\nError: {}\n", image.url, err);
                }
            },
        }
    }

    if !text_inputs.is_empty() && !image_inputs.is_empty() {
        return Err(unsupported(
            "To use images and text together. Use Cohere's 'inputs' param using 'provider_options'. You can provider key 'inputs' and its value as string.",
        ));
    }

    let input_type = if image_inputs.is_empty() && !text_inputs.is_empty() {
        match config.task_type {
            Some(TaskType::RetrievalQuery) => InputType::SearchQuery,
            Some(TaskType::RetrievalDocument) => InputType::SearchDocument,
            Some(TaskType::Classification) => InputType::Classification,
            Some(TaskType::Clustering) => InputType::Clustering,
            None => InputType::SearchQuery,
            _ => return Err(unsupported("task_type")),
        }
    } else {
        InputType::Image
    };

    let model = config
        .model
        .unwrap_or_else(|| "embed-english-v3.0".to_string());

    let embedding_types = config
        .output_dtype
        .map(|dtype| vec![output_dtype_to_cohere_embedding_type(dtype)]);

    let provider_params = config
        .provider_options
        .into_iter()
        .map(|kv| {
            let value =
                serde_json::from_str(&kv.value).unwrap_or(serde_json::Value::String(kv.value));
            (kv.key, value)
        })
        .collect();

    Ok(EmbeddingRequest {
        model,
        input_type,
        embedding_types,
        images: Some(image_inputs),
        texts: Some(text_inputs),
        output_dimension: config.dimensions,
        provider_params,
    })
}

pub fn create_rerank_request(
    query: String,
    documents: Vec<String>,
    config: Config,
) -> Result<RerankRequest, Error> {
    let model = config.model.unwrap_or_else(|| "rerank-2-lite".to_string());

    let provider_params = config
        .provider_options
        .into_iter()
        .map(|kv| {
            let value =
                serde_json::from_str(&kv.value).unwrap_or(serde_json::Value::String(kv.value));
            (kv.key, value)
        })
        .collect();

    Ok(RerankRequest {
        model,
        query,
        documents,
        provider_params,
    })
}

pub fn image_to_base64(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = if Url::parse(source).is_ok() {
        let client = Client::new();
        let response = client.get(source).send()?;
        response.bytes()?.to_vec()
    } else {
        let path = Path::new(source);
        fs::read(path)?
    };

    let kind = infer::get(&bytes).ok_or("Could not determine MIME type")?;
    let mime_type = kind.mime_type();

    let base64_data = general_purpose::STANDARD.encode(&bytes);
    let data_uri = format!("data:{mime_type};base64,{base64_data}");

    Ok(data_uri)
}

pub fn process_embedding_response(
    response: EmbeddingResponse,
    config: Config,
) -> Result<GolemEmbeddingResponse, Error> {
    let mut embeddings: Vec<Embedding> = Vec::new();
    if let Some(emdeddings_array) = response.embeddings.int8 {
        for (index, embedding) in emdeddings_array.iter().enumerate() {
            embeddings.push(Embedding {
                index: index as u32,
                vector: VectorData::Int8(embedding.clone()),
            });
        }
    };
    if let Some(emdeddings_array) = response.embeddings.uint8 {
        for (index, embedding) in emdeddings_array.iter().enumerate() {
            embeddings.push(Embedding {
                index: index as u32,
                vector: VectorData::Uint8(embedding.clone()),
            });
        }
    };
    if let Some(emdeddings_array) = response.embeddings.binary {
        for (index, embedding) in emdeddings_array.iter().enumerate() {
            embeddings.push(Embedding {
                index: index as u32,
                vector: VectorData::Binary(embedding.clone()),
            });
        }
    };
    if let Some(emdeddings_array) = response.embeddings.ubinary {
        for (index, embedding) in emdeddings_array.iter().enumerate() {
            embeddings.push(Embedding {
                index: index as u32,
                vector: VectorData::Ubinary(embedding.clone()),
            });
        }
    };
    if let Some(emdeddings_array) = response.embeddings.float {
        for (index, embedding) in emdeddings_array.iter().enumerate() {
            embeddings.push(Embedding {
                index: index as u32,
                vector: VectorData::Float(embedding.clone()),
            });
        }
    };

    let provider_metadata_json =
        get_embed_provider_metadata(&response.texts, &response.meta, &response.id);

    Ok(GolemEmbeddingResponse {
        embeddings,
        model: config
            .model
            .unwrap_or_else(|| "embed-english-v3.0".to_string()),
        usage: Some(Usage {
            input_tokens: response.meta.unwrap().billed_units.unwrap().input_tokens,
            total_tokens: None,
        }),
        provider_metadata_json,
    })
}

pub fn get_embed_provider_metadata(
    texts: &Option<Vec<String>>,
    meta: &Option<Meta>,
    id: &String,
) -> Option<String> {
    let meta = meta
        .as_ref()
        .map(|m| serde_json::to_string(m).unwrap_or_default())
        .unwrap_or_default();
    let texts = texts
        .as_ref()
        .map(|t| serde_json::to_string(t).unwrap_or_default())
        .unwrap_or_default();
    let meta_data = format!(r#"{{"id":"{id}","meta":"{meta}","text":{texts}}}"#);
    Some(meta_data)
}

pub fn process_rerank_response(
    response: RerankResponse,
    config: Config,
) -> Result<GolemRerankResponse, Error> {
    let results = response
        .clone()
        .results
        .iter()
        .map(|result| RerankResult {
            index: result.index,
            relevance_score: result.relevance_score,
            document: None,
        })
        .collect();

    let usage = response.clone().meta.and_then(|meta| {
        meta.billed_units.map(|billed_units| Usage {
            input_tokens: billed_units.input_tokens,
            total_tokens: billed_units.output_tokens,
        })
    });

    Ok(GolemRerankResponse {
        results,
        usage,
        model: config.model.unwrap_or_else(|| "rerank-2-lite".to_string()),
        provider_metadata_json: Some(get_rerank_provider_metadata(response)),
    })
}

fn get_rerank_provider_metadata(response: RerankResponse) -> String {
    let meta = serde_json::to_string(&response.meta.unwrap()).unwrap_or_default();
    format!(
        r#"{{"id":"{}","meta":"{}", "warnings":"{}"}}"#,
        response.id.unwrap_or_default(),
        meta,
        response.warnings.unwrap_or_default(),
    )
}
