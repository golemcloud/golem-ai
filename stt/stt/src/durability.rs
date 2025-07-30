

#[cfg(feature = "durability")]
pub mod durable_impl {
    // TODO: Implement durability helpers similar to llm::durability.
}

#[cfg(not(feature = "durability"))]
pub mod passthrough_impl {
    // No-op implementations when durability feature is disabled.
} 