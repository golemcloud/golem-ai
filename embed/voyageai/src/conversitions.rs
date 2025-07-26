use std::collections::HashMap;

use golem_embed::{
    error::unsupported,
    golem::embed::embed::{
        Config, ContentPart, Embedding, EmbeddingResponse as GolemEmbeddingResponse, Error,
        OutputDtype as GolemOutputDtype, OutputFormat as GolemOutputFormat,
        RerankResponse as GolemRerankResponse, RerankResult as GolemRerankResult, TaskType, Usage,
    },
};

use crate::client::{
    Embedding as VoyageEmbedding, EmbeddingRequest, EmbeddingResponse, EncodingFormat, InputType,
    OutputDtype, RerankRequest, RerankResponse,
};

pub fn create_embedding_request(
    inputs: Vec<ContentPart>,
    config: Config,
) -> Result<EmbeddingRequest, Error> {
    let mut text_inputs = Vec::new();

    for input in inputs {
        match input {
            ContentPart::Text(text) => text_inputs.push(text),
            ContentPart::Image(_) => {
                return Err(unsupported("VoyageAI do not support image embedding."));
            }
        }
    }

    let model = config
        .model
        .unwrap_or_else(|| "voyage-3.5-lite".to_string());

    let input_type = match config.task_type {
        Some(TaskType::RetrievalQuery) => Some(InputType::Query),
        Some(TaskType::RetrievalDocument) => Some(InputType::Document),
        None => Some(InputType::Document),
        _ => return Err(unsupported("task_type")),
    };

    let output_dtype = match config.output_dtype {
        Some(GolemOutputDtype::FloatArray) => Some(OutputDtype::Float),
        Some(GolemOutputDtype::Int8) => Some(OutputDtype::Int8),
        Some(GolemOutputDtype::Uint8) => Some(OutputDtype::Uint8),
        Some(GolemOutputDtype::Binary) => Some(OutputDtype::Binary),
        Some(GolemOutputDtype::Ubinary) => Some(OutputDtype::Ubinary),
        _ => None,
    };

    let encoding_format = match config.output_format {
        Some(GolemOutputFormat::Base64) => Some(EncodingFormat::Base64),
        _ => None,
    };

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
        input: text_inputs,
        model,
        input_type,
        truncation: config.truncation,
        output_dimension: config.dimensions,
        output_dtype,
        encoding_format,
        provider_params,
    })
}

pub fn process_embedding_response(
    output_dtype: Option<GolemOutputDtype>,
    response: EmbeddingResponse,
) -> Result<GolemEmbeddingResponse, Error> {
    let mut embeddings = Vec::new();

    for embedding_data in response.data {
        match embedding_data.embedding {
            VoyageEmbedding::Base64(data) => {
                embeddings.push(Embedding {
                    index: embedding_data.index,
                    vector: golem_embed::golem::embed::embed::VectorData::Base64(data),
                });
            }
            VoyageEmbedding::Float(data) => {
                embeddings.push(Embedding {
                    index: embedding_data.index,
                    vector: golem_embed::golem::embed::embed::VectorData::FloatArray(data),
                });
            }
            VoyageEmbedding::Integer(data) => match output_dtype.unwrap() {
                GolemOutputDtype::Int8 => {
                    embeddings.push(Embedding {
                        index: embedding_data.index,
                        vector: golem_embed::golem::embed::embed::VectorData::Int8(data),
                    });
                }
                GolemOutputDtype::Uint8 => {
                    let uint8_data: Vec<u8> = data.into_iter().map(|x| x as u8).collect();
                    embeddings.push(Embedding {
                        index: embedding_data.index,
                        vector: golem_embed::golem::embed::embed::VectorData::Uint8(uint8_data),
                    });
                }
                GolemOutputDtype::Binary => {
                    embeddings.push(Embedding {
                        index: embedding_data.index,
                        vector: golem_embed::golem::embed::embed::VectorData::Binary(data),
                    });
                }
                GolemOutputDtype::Ubinary => {
                    let ubinary_data: Vec<u8> = data.into_iter().map(|x| x as u8).collect();
                    embeddings.push(Embedding {
                        index: embedding_data.index,
                        vector: golem_embed::golem::embed::embed::VectorData::Ubinary(ubinary_data),
                    });
                }

                _ => {
                    return Err(unsupported(
                        "Unsupported output dtype for integer embeddings",
                    ));
                }
            },
        }
    }

    let usage = Usage {
        input_tokens: None,
        total_tokens: Some(response.usage.total_tokens),
    };

    Ok(GolemEmbeddingResponse {
        embeddings,
        usage: Some(usage),
        model: response.model,
        provider_metadata_json: None,
    })
}

pub fn create_rerank_request(
    query: String,
    documents: Vec<String>,
    config: Config,
) -> Result<RerankRequest, Error> {
    let model = config.model.unwrap_or_else(|| "rerank-2-lite".to_string());
    let provider_params: HashMap<String, serde_json::Value> = config
        .provider_options
        .into_iter()
        .map(|kv| {
            let value =
                serde_json::from_str(&kv.value).unwrap_or(serde_json::Value::String(kv.value));
            (kv.key, value)
        })
        .collect();

    Ok(RerankRequest {
        query,
        documents,
        model,
        truncation: config.truncation,
        provider_params,
    })
}

pub fn process_rerank_response(response: RerankResponse) -> Result<GolemRerankResponse, Error> {
    let mut results = Vec::new();
    for result in response.data {
        results.push(GolemRerankResult {
            index: result.index,
            relevance_score: result.relevance_score,
            document: result.document,
        });
    }

    let usage = Usage {
        input_tokens: Some(response.usage.total_tokens),
        total_tokens: Some(response.usage.total_tokens),
    };

    Ok(GolemRerankResponse {
        results,
        usage: Some(usage),
        model: response.model,
        provider_metadata_json: None,
    })
}
