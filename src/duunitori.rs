//! Thin client for the Duunitori job entries API.

use serde::{Deserialize, Serialize};

const API_URL: &str = "https://duunitori.fi/api/v1/jobentries";
const JOB_PAGE_URL: &str = "https://duunitori.fi/tyopaikat/tyo/";

/// Raw API response. Only the fields we use are declared; serde ignores the rest.
#[derive(Deserialize)]
struct ApiResponse {
    count: u64,
    next: Option<String>,
    results: Vec<ApiJob>,
}

#[derive(Deserialize)]
struct ApiJob {
    heading: String,
    date_posted: String,
    slug: String,
    municipality_name: Option<String>,
    company_name: Option<String>,
    descr: Option<String>,
}

/// Agent-friendly job entry.
#[derive(Serialize, schemars::JsonSchema)]
pub struct Job {
    pub title: String,
    pub company: Option<String>,
    pub location: Option<String>,
    pub posted: String,
    /// Link to the job on duunitori.fi.
    pub url: String,
    /// Truncated unless full descriptions were requested.
    pub description: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct SearchResult {
    pub total_count: u64,
    pub page: u32,
    pub has_more: bool,
    pub jobs: Vec<Job>,
}

#[derive(Clone, Default)]
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    pub async fn search(
        &self,
        query: &str,
        page: u32,
        page_size: u32,
        description_chars: Option<usize>,
    ) -> anyhow::Result<SearchResult> {
        let resp: ApiResponse = self
            .http
            .get(API_URL)
            .query(&[
                ("search", query),
                ("format", "json"),
                ("page", &page.to_string()),
                ("page_size", &page_size.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let jobs = resp
            .results
            .into_iter()
            .map(|j| Job {
                title: j.heading,
                company: j.company_name,
                location: j.municipality_name,
                posted: j.date_posted,
                url: format!("{JOB_PAGE_URL}{}", j.slug),
                description: truncate(j.descr.unwrap_or_default().trim(), description_chars),
            })
            .collect();

        Ok(SearchResult {
            total_count: resp.count,
            page,
            has_more: resp.next.is_some(),
            jobs,
        })
    }
}

fn truncate(s: &str, max_chars: Option<usize>) -> String {
    match max_chars {
        Some(n) if s.chars().count() > n => {
            let cut: String = s.chars().take(n).collect();
            format!("{}…", cut.trim_end())
        }
        _ => s.to_string(),
    }
}
