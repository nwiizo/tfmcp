//! TFE API path and query encoding.

use crate::tfe::types::PageParams;

pub(super) fn encode_path_segment(value: &str) -> String {
    value.replace('/', "%2F")
}

pub(super) fn registry_collection_path(
    organization: &str,
    collection: &str,
    query: Option<&str>,
    registry_name: Option<&str>,
    provider: Option<&str>,
    page: PageParams,
) -> String {
    let mut params = vec![
        ("page[number]", page.number.to_string()),
        ("page[size]", page.size.to_string()),
    ];
    if let Some(query) = query.filter(|value| !value.is_empty()) {
        params.push(("q", query.to_string()));
    }
    if let Some(registry_name) = registry_name.filter(|value| !value.is_empty()) {
        params.push(("filter[registry_name]", registry_name.to_string()));
    }
    if let Some(provider) = provider.filter(|value| !value.is_empty()) {
        params.push(("filter[provider]", provider.to_string()));
    }

    format!(
        "/organizations/{}/{}?{}",
        encode_path_segment(organization),
        collection,
        query_string(params)
    )
}

fn query_string(params: Vec<(&str, String)>) -> String {
    params
        .into_iter()
        .map(|(key, value)| format!("{}={}", encode_query(key), encode_query(&value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn encode_query(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            other => format!("%{other:02X}").chars().collect(),
        })
        .collect()
}
