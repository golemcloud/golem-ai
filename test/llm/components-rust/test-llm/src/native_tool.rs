#![allow(clippy::too_many_arguments)]

use crate::TestHelperClient;
use golem_rust::agentic::{InputStream, OutputStream};
use golem_rust::{
    tool_definition, tool_implementation, FromSchema, FromWire, IntoSchema, IntoWire, ToolError,
    WireSchema,
};

#[derive(Debug, Clone, IntoSchema, FromSchema, IntoWire, FromWire, WireSchema)]
pub struct NativeToolOutput {
    pub message: String,
    pub stdin_bytes: u64,
    pub side_effect_count: u64,
}

/// A deterministic registered tool used to verify the LLM adapter.
#[tool_definition(version = "1.0.0")]
pub trait NativeLlmFixture {
    /// Read stdin, emit independent output streams, and return a structured value.
    #[arg(prefix = "option", default = "hello")]
    #[arg(repeat = "option", default = 2, min = 1, max = 4)]
    #[arg(diagnostics, channel = "stderr")]
    async fn run(
        &self,
        counter_name: String,
        mode: String,
        prefix: String,
        repeat: u32,
        stdin: InputStream,
        stdout: OutputStream,
        diagnostics: OutputStream,
    ) -> Result<NativeToolOutput, NativeToolError>;
}

/// Declared failures from the native LLM fixture.
#[derive(ToolError)]
pub enum NativeToolError {
    /// The caller requested the deterministic rejection path.
    #[tool_error(kind = "usage-error", exit_code = 2)]
    Rejected { reason: String },
}

struct NativeLlmFixtureImpl;

#[tool_implementation]
impl NativeLlmFixture for NativeLlmFixtureImpl {
    async fn run(
        &self,
        counter_name: String,
        mode: String,
        prefix: String,
        repeat: u32,
        mut stdin: InputStream,
        mut stdout: OutputStream,
        mut diagnostics: OutputStream,
    ) -> Result<NativeToolOutput, NativeToolError> {
        let mut input = Vec::new();
        while let Some(chunk) = stdin.next().await {
            input.extend(chunk.expect("fixture stdin must remain readable"));
        }

        let side_effect_count = TestHelperClient::get(counter_name).inc_and_get().await;
        let text = String::from_utf8(input.clone()).expect("fixture expects UTF-8 stdin");
        let message = (0..repeat)
            .map(|_| format!("{prefix}:{text}"))
            .collect::<Vec<_>>()
            .join("|");

        stdout
            .write_all(format!("stdout:{message}").into_bytes())
            .await
            .expect("fixture stdout must remain writable");
        diagnostics
            .write_all(format!("stderr:{}", input.len()).into_bytes())
            .await
            .expect("fixture stderr must remain writable");
        stdout.finish().await.expect("finish fixture stdout");
        diagnostics.finish().await.expect("finish fixture stderr");

        if mode == "reject" {
            Err(NativeToolError::Rejected {
                reason: format!("rejected:{text}"),
            })
        } else {
            Ok(NativeToolOutput {
                message,
                stdin_bytes: input.len() as u64,
                side_effect_count,
            })
        }
    }
}
