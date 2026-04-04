#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    if let Err(error) = ai_debug_agent::run_from_env().await {
        tracing::error!(error = ?error, "Application exited with an error");
        std::process::exit(1);
    }
}
