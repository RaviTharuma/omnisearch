//! Query-intent routing used only when mode=auto.

use crate::health::HealthBoard;
use crate::types::{ProviderId, SearchMode, SearchRequest, SearchType};

pub fn select_providers(
    request: &SearchRequest,
    configured: &[ProviderId],
    health: &HealthBoard,
    cooldown_skip: bool,
) -> Vec<ProviderId> {
    let mut chosen = if let Some(explicit) = &request.providers {
        explicit
            .iter()
            .copied()
            .filter(|id| configured.contains(id))
            .collect::<Vec<_>>()
    } else if request.mode == SearchMode::Auto {
        intent_subset(&request.query, request.search_type, configured)
    } else {
        configured.to_vec()
    };

    if cooldown_skip && request.providers.is_none() {
        chosen.retain(|id| !health.is_cooling(*id));
    }

    if request.mode == SearchMode::Auto {
        chosen.sort_by(|a, b| {
            health
                .score(*b)
                .partial_cmp(&health.score(*a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    chosen
}

/// Free providers first, then rising estimated USD.
pub fn sort_ladder(ids: &mut [ProviderId], cost: impl Fn(ProviderId) -> f64) {
    ids.sort_by(|a, b| {
        cost(*a)
            .partial_cmp(&cost(*b))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.as_str().cmp(b.as_str()))
    });
}

fn intent_subset(query: &str, vertical: SearchType, configured: &[ProviderId]) -> Vec<ProviderId> {
    let q = query.to_ascii_lowercase();
    let mut want: Vec<ProviderId> = Vec::new();

    let inferred = if matches!(vertical, SearchType::News)
        || q.contains("news")
        || q.contains("headline")
    {
        SearchType::News
    } else if matches!(vertical, SearchType::Users)
        || q.contains("github user")
        || q.contains("github org")
    {
        SearchType::Users
    } else if matches!(vertical, SearchType::Code)
        || q.contains("github")
        || q.contains("npm ")
        || q.contains("crate")
    {
        SearchType::Code
    } else if matches!(vertical, SearchType::Video) || q.contains("youtube") || q.contains("video")
    {
        SearchType::Video
    } else if matches!(vertical, SearchType::Scholarly)
        || q.contains("arxiv")
        || q.contains("doi:")
        || q.contains("paper")
    {
        SearchType::Scholarly
    } else if matches!(vertical, SearchType::Social)
        || q.contains("reddit")
        || q.contains("tweet")
        || q.contains("instagram")
        || q.contains("mastodon")
        || q.contains("bluesky")
    {
        SearchType::Social
    } else {
        vertical
    };

    match inferred {
        SearchType::News => want.extend([
            ProviderId::Tavily,
            ProviderId::Brave,
            ProviderId::Kagi,
            ProviderId::Youcom,
            ProviderId::Exa,
        ]),
        SearchType::Code => want.extend([ProviderId::Github, ProviderId::Exa, ProviderId::Tavily]),
        SearchType::Users => want.extend([ProviderId::Github]),
        SearchType::Video => want.extend([ProviderId::Youtube, ProviderId::Exa]),
        SearchType::Scholarly => {
            want.extend([ProviderId::Scholar, ProviderId::Wikipedia, ProviderId::Exa])
        }
        SearchType::Social => want.extend([
            ProviderId::Reddit,
            ProviderId::X,
            ProviderId::Youtube,
            ProviderId::Instagram,
            ProviderId::Facebook,
            ProviderId::Mastodon,
            ProviderId::Bluesky,
        ]),
        SearchType::Web => want.extend(ProviderId::all().iter().copied()),
    }

    if !want.contains(&ProviderId::Omniroute) {
        want.push(ProviderId::Omniroute);
    }
    want.into_iter()
        .filter(|id| configured.contains(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_routes_github_intent() {
        let mut req = SearchRequest::new("github rust mcp server");
        req.mode = SearchMode::Auto;
        let configured = vec![ProviderId::Github, ProviderId::Tavily, ProviderId::Exa];
        let health = HealthBoard::new();
        let selected = select_providers(&req, &configured, &health, false);
        assert!(selected.contains(&ProviderId::Github));
    }

    #[test]
    fn auto_routes_github_users() {
        let mut req = SearchRequest::new("github user octocat");
        req.mode = SearchMode::Auto;
        let configured = vec![
            ProviderId::Github,
            ProviderId::Brave,
            ProviderId::Tavily,
            ProviderId::Exa,
        ];
        let health = HealthBoard::new();
        let selected = select_providers(&req, &configured, &health, false);
        assert_eq!(selected, vec![ProviderId::Github]);
    }

    #[test]
    fn default_all_includes_must_have_when_configured() {
        let req = SearchRequest::new("swiss ai regulation");
        let configured = ProviderId::must_have().to_vec();
        let health = HealthBoard::new();
        let selected = select_providers(&req, &configured, &health, false);
        assert!(selected.contains(&ProviderId::Brave));
        assert!(selected.contains(&ProviderId::Github));
    }

    #[test]
    fn must_have_is_brave_and_github() {
        assert_eq!(
            ProviderId::must_have(),
            &[ProviderId::Brave, ProviderId::Github]
        );
        for id in ProviderId::must_have() {
            assert!(ProviderId::all().contains(id));
        }
    }

    #[test]
    fn ladder_orders_free_before_paid() {
        let mut ids = vec![ProviderId::Tavily, ProviderId::Wikipedia, ProviderId::Exa];
        sort_ladder(&mut ids, |id| match id {
            ProviderId::Wikipedia => 0.0,
            ProviderId::Exa => 0.006,
            ProviderId::Tavily => 0.008,
            _ => 1.0,
        });
        assert_eq!(ids[0], ProviderId::Wikipedia);
        assert_eq!(ids[1], ProviderId::Exa);
        assert_eq!(ids[2], ProviderId::Tavily);
    }
}
