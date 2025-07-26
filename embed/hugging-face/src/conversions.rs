use golem_embed::error::unsupported;
use golem_embed::golem::embed::embed::{
    Config, ContentPart, EmbeddingResponse as GolemEmbeddingResponse, Error,
};

use crate::client::{EmbeddingRequest, EmbeddingResponse};

pub fn create_embedding_request(
    inputs: Vec<ContentPart>,
    config: Config,
) -> Result<(EmbeddingRequest, String), Error> {
    let mut input_texts = Vec::new();
    for content in inputs {
        match content {
            ContentPart::Text(text) => input_texts.push(text),
            ContentPart::Image(_) => {
                return Err(unsupported(
                    "Image embeddings are not supported by Hugging Face.",
                ))
            }
        }
    }

    let model = config
        .model
        .unwrap_or_else(|| "sentence-transformers/all-MiniLM-L6-v2".to_string());

    let provider_params = config
        .provider_options
        .into_iter()
        .map(|kv| {
            let value =
                serde_json::from_str(&kv.value).unwrap_or(serde_json::Value::String(kv.value));
            (kv.key, value)
        })
        .collect();

    let request = EmbeddingRequest {
        inputs: input_texts,
        truncate: config.truncation,
        provider_params,
    };

    Ok((request, model))
}

pub fn process_embedding_response(
    response: EmbeddingResponse,
    model: String,
) -> Result<GolemEmbeddingResponse, Error> {
    let mut embeddings = Vec::new();
    for (index, embedding_vec) in response.iter().enumerate() {
        embeddings.push(golem_embed::golem::embed::embed::Embedding {
            index: index as u32,
            vector: golem_embed::golem::embed::embed::VectorData::FloatArray(embedding_vec.clone()),
        });
    }

    Ok(GolemEmbeddingResponse {
        embeddings,
        usage: None,
        model,
        provider_metadata_json: None,
    })
}
