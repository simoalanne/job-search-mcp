mod duunitori;

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    tool, tool_handler, tool_router, ServerHandler,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
};
use serde::Deserialize;
use std::sync::Arc;

const SNIPPET_CHARS: usize = 400;
const MAX_PAGE_SIZE: u32 = 50;

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchArgs {
    /// Free-text search, e.g. "rust developer", "Helsinki", or a field like
    /// "Tieto- ja tietoliikennetekniikka (ala)". Finnish terms usually match best.
    query: String,
    /// 1-based page number. Defaults to 1.
    page: Option<u32>,
    /// Results per page, 1-50. Defaults to 10.
    page_size: Option<u32>,
    /// Return full job descriptions instead of short snippets. Defaults to false.
    full_description: Option<bool>,
}

#[derive(Clone)]
struct JobSearch {
    duunitori: duunitori::Client,
}

#[tool_router]
impl JobSearch {
    #[tool(
        description = "Search Finnish job listings on Duunitori. Each result includes a url to the listing on duunitori.fi.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn search_duunitori(
        &self,
        Parameters(args): Parameters<SearchArgs>,
    ) -> Result<Json<duunitori::SearchResult>, String> {
        let page = args.page.unwrap_or(1).max(1);
        let page_size = args.page_size.unwrap_or(10).clamp(1, MAX_PAGE_SIZE);
        let chars = (!args.full_description.unwrap_or(false)).then_some(SNIPPET_CHARS);

        self.duunitori
            .search(&args.query, page, page_size, chars)
            .await
            .map(Json)
            .map_err(|e| format!("Duunitori request failed: {e:#}"))
    }
}

#[tool_handler(
    name = "job-search",
    instructions = "Search Finnish job boards. Share the job's url when the user asks where to view or apply."
)]
impl ServerHandler for JobSearch {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let addr = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8000".into());
    let duunitori = duunitori::Client::default();

    let mcp = StreamableHttpService::new(
        move || {
            Ok(JobSearch {
                duunitori: duunitori.clone(),
            })
        },
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    let app = axum::Router::new().nest_service("/mcp", mcp);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("MCP server listening on http://{addr}/mcp");
    axum::serve(listener, app)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.ok(); })
        .await?;
    Ok(())
}
