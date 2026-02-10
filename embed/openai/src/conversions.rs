use golem_ai_embed::error::unsupported;
use golem_ai_embed::model::{
    Config, ContentPart, EmbeddingResponse as GolemEmbeddingResponse, Error, VectorData,
};

use crate::client::{Embedding, EmbeddingRequest, EmbeddingResponse, EncodingFormat};

pub fn create_request(inputs: Vec<ContentPart>, config: Config) -> Result<EmbeddingRequest, Error> {
    let mut input = String::new();
    for content in inputs {
        match content {
            ContentPart::Text(text) => input.push_str(&text),
            ContentPart::Image(_) => {
                return Err(unsupported("Image embeddings is not supported by OpenAI."))
            }
        }
    }

    let model = config
        .model
        .unwrap_or_else(|| "text-embedding-ada-002".to_string());

    let encoding_format = match config.output_format {
        Some(golem_ai_embed::model::OutputFormat::FloatArray) => Some(EncodingFormat::Float),
        Some(golem_ai_embed::model::OutputFormat::Base64) => Some(EncodingFormat::Base64),
        Some(_) => {
            return Err(unsupported(
                "OpenAI only supports float and base64 output formats.",
            ))
        }
        None => Some(EncodingFormat::Float),
    };

    let provider_params: std::collections::HashMap<String, serde_json::Value> = config
        .provider_options
        .into_iter()
        .map(|kv| {
            let value =
                serde_json::from_str(&kv.value).unwrap_or(serde_json::Value::String(kv.value));
            (kv.key, value)
        })
        .collect();

    Ok(EmbeddingRequest {
        input,
        model,
        encoding_format,
        dimension: config.dimensions,
        user: config.user,
        provider_params,
    })
}

pub fn process_embedding_response(
    response: EmbeddingResponse,
) -> Result<GolemEmbeddingResponse, Error> {
    let mut embeddings = Vec::new();

    for embeding_data in response.data {
        match embeding_data.embedding {
            Embedding::Base64(base64_data) => {
                embeddings.push(golem_ai_embed::model::Embedding {
                    index: embeding_data.index as u32,
                    vector: VectorData::Base64(base64_data),
                });
            }
            Embedding::Float32(embedding_vector) => {
                embeddings.push(golem_ai_embed::model::Embedding {
                    index: embeding_data.index as u32,
                    vector: VectorData::Float(embedding_vector),
                });
            }
        }
    }

    let usage = golem_ai_embed::model::Usage {
        input_tokens: Some(response.usage.prompt_tokens),
        total_tokens: Some(response.usage.total_tokens),
    };

    Ok(GolemEmbeddingResponse {
        embeddings,
        usage: Some(usage),
        model: response.model,
        provider_metadata_json: None,
    })
}
