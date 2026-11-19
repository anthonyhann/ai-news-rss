use sha2::{Digest, Sha256};
use std::collections::HashSet;
use url::Url;

use crate::models::{RawItem, Story, StorySource};

const TRACKING_PARAMS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "fbclid",
    "gclid",
    "dclid",
    "msclkid",
    "mc_cid",
    "mc_eid",
    "igshid",
    "ref",
    "ref_src",
];

pub fn normalize_url(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return String::new();
    }
    let Ok(mut url) = Url::parse(raw) else {
        return raw.trim_end_matches('/').to_string();
    };
    url.set_fragment(None);
    if let Some(host) = url.host_str() {
        let lower = host.to_ascii_lowercase();
        let _ = url.set_host(Some(&lower));
    }
    let filtered: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| {
            let k = k.to_ascii_lowercase();
            !TRACKING_PARAMS.contains(&k.as_str()) && !k.starts_with("utm_")
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if filtered.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(filtered);
    }
    let mut path = url.path().to_string();
    if path.ends_with("/amp") || path.ends_with("/amp/") {
        path = path
            .trim_end_matches('/')
            .trim_end_matches("/amp")
            .to_string();
        if path.is_empty() {
            path = "/".into();
        }
        url.set_path(&path);
    }
    if url.path() != "/" {
        let path = url.path().trim_end_matches('/').to_string();
        url.set_path(&path);
    }
    url.to_string()
}

pub fn content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut latin = String::new();
    let mut cjk: Vec<char> = Vec::new();
    let flush_latin = |latin: &mut String, out: &mut Vec<String>| {
        if !latin.is_empty() {
            let token = latin.to_ascii_lowercase();
            if token.len() > 1
                && !matches!(
                    token.as_str(),
                    "the" | "and" | "for" | "with" | "from" | "this" | "that" | "are" | "was"
                )
            {
                out.push(token);
            }
            latin.clear();
        }
    };
    let flush_cjk = |cjk: &mut Vec<char>, out: &mut Vec<String>| {
        if cjk.len() == 1 {
            out.push(cjk[0].to_string());
        } else {
            for pair in cjk.windows(2) {
                out.push(format!("{}{}", pair[0], pair[1]));
            }
        }
        cjk.clear();
    };
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            latin.push(ch);
        } else if is_cjk(ch) {
            flush_latin(&mut latin, &mut out);
            cjk.push(ch);
        } else {
            flush_latin(&mut latin, &mut out);
            flush_cjk(&mut cjk, &mut out);
        }
    }
    flush_latin(&mut latin, &mut out);
    flush_cjk(&mut cjk, &mut out);
    out
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

pub fn simhash64(text: &str) -> u64 {
    let tokens = tokenize(text);
    let refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    simhash::simhash_stream(refs.into_iter())
}

pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

pub fn jaccard(a: &str, b: &str) -> f64 {
    let aa: HashSet<String> = tokenize(a).into_iter().collect();
    let bb: HashSet<String> = tokenize(b).into_iter().collect();
    if aa.is_empty() && bb.is_empty() {
        return 1.0;
    }
    let inter = aa.intersection(&bb).count() as f64;
    let union = aa.union(&bb).count() as f64;
    if union == 0.0 { 0.0 } else { inter / union }
}

pub fn near_duplicate(
    title_a: &str,
    hash_a: u64,
    title_b: &str,
    hash_b: u64,
    max_hamming: u32,
    min_jaccard: f64,
) -> bool {
    hamming(hash_a, hash_b) <= max_hamming || jaccard(title_a, title_b) >= min_jaccard
}

pub fn story_source_from_item(item: &RawItem, source_name: &str, tier: u8) -> StorySource {
    StorySource {
        item_id: item.id.clone(),
        source_id: item.source_id.clone(),
        source_name: source_name.to_string(),
        kind: item.kind,
        tier,
        title: item.title.clone(),
        url: item.url.clone(),
        author: item.author.clone(),
        published_at: item.published_at,
        excerpt: item.excerpt.clone(),
        score: item.score,
    }
}

pub fn new_story(item: &RawItem, source_name: &str, tier: u8) -> Story {
    Story {
        id: uuid::Uuid::new_v4().to_string(),
        primary_item_id: item.id.clone(),
        title_zh: item.title_zh.clone().unwrap_or_else(|| item.title.clone()),
        title_en: item.title_en.clone().unwrap_or_else(|| item.title.clone()),
        summary_zh: item.summary_zh.clone().unwrap_or_default(),
        summary_en: item.summary_en.clone().unwrap_or_default(),
        first_seen_at: item.published_at,
        last_seen_at: item.fetched_at,
        sources: vec![story_source_from_item(item, source_name, tier)],
        category: item
            .metadata
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        ..Story::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_tracking_and_trailing_slash() {
        assert_eq!(
            normalize_url("https://Example.com/a/?utm_source=x&b=1#frag"),
            "https://example.com/a?b=1"
        );
    }

    #[test]
    fn same_event_titles_are_similar() {
        assert!(
            jaccard(
                "OpenAI releases GPT-5 with multimodal reasoning",
                "OpenAI launches GPT-5 multimodal reasoning model"
            ) > 0.3
        );
    }

    #[test]
    fn different_events_are_not_identical() {
        assert!(jaccard("Gemma 4 released", "Gemma 4 jailbroken") < 0.9);
    }

    #[test]
    fn cjk_tokens_work() {
        let tokens = tokenize("OpenAI 发布 GPT-5 多模态推理");
        assert!(tokens.iter().any(|t| t == "多模"));
        assert!(tokens.iter().any(|t| t == "gpt-5" || t == "gpt"));
    }
}
