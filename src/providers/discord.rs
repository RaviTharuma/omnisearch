//! Discord guild message search (bot or bearer token + guild IDs).

use async_trait::async_trait;
use serde_json::Value;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::http::{HttpClient, pick_str};
use crate::types::{ProviderId, ProviderSearchRequest, SearchHit, SearchPage};

use super::{Provider, offset};

pub struct Discord {
    http: HttpClient,
    token: Option<String>,
    guild_ids: Vec<String>,
    /// When true, send `Authorization: Bearer …` (user token). Default is Bot.
    bearer: bool,
    base: String,
}

impl Discord {
    pub fn new(config: &Config, http: HttpClient) -> Self {
        Self {
            http,
            token: config.keys.discord_token.clone(),
            guild_ids: config.keys.discord_guild_ids.clone(),
            bearer: config.keys.discord_bearer,
            base: config.endpoints.discord.clone(),
        }
    }

    fn auth_header(&self, token: &str) -> String {
        if self.bearer {
            format!("Bearer {token}")
        } else {
            format!("Bot {token}")
        }
    }

    async fn search_guild(
        &self,
        token: &str,
        guild_id: &str,
        request: &ProviderSearchRequest<'_>,
    ) -> Result<SearchPage> {
        let limit = request.page_size.clamp(1, 25).to_string();
        let off = offset(request.cursor).to_string();
        let builder = self
            .http
            .get(&format!(
                "{}/guilds/{guild_id}/messages/search",
                self.base.trim_end_matches('/')
            ))
            .header("Authorization", self.auth_header(token))
            .query(&[
                ("content", request.query),
                ("limit", limit.as_str()),
                ("offset", off.as_str()),
            ]);
        let value = self.http.json("discord", builder).await?;
        let mut hits = Vec::new();
        if let Some(groups) = value.get("messages").and_then(Value::as_array) {
            for group in groups {
                let Some(rows) = group.as_array() else {
                    continue;
                };
                let message = rows
                    .iter()
                    .find(|m| m.get("hit").and_then(Value::as_bool) == Some(true))
                    .or_else(|| rows.first());
                let Some(message) = message else {
                    continue;
                };
                let Some(id) = pick_str(message, &["id"]) else {
                    continue;
                };
                let Some(channel_id) = pick_str(message, &["channel_id"]) else {
                    continue;
                };
                let content = pick_str(message, &["content"]).unwrap_or_default();
                let title = content.chars().take(80).collect::<String>();
                let url = format!("https://discord.com/channels/{guild_id}/{channel_id}/{id}");
                let mut hit = SearchHit::new(ProviderId::Discord, title, url, content);
                hit.published_at = pick_str(message, &["timestamp"]);
                hits.push(hit);
            }
        }
        let total = value
            .get("total_results")
            .and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0);
        let next = {
            let next_off = offset(request.cursor) + hits.len() as u32;
            (hits.len() as u64 > 0 && u64::from(next_off) < total).then(|| next_off.to_string())
        };
        Ok(SearchPage {
            hits,
            next_cursor: next,
            answer: None,
        })
    }
}

#[async_trait]
impl Provider for Discord {
    fn id(&self) -> ProviderId {
        ProviderId::Discord
    }

    fn is_configured(&self) -> bool {
        self.token.as_ref().is_some_and(|t| !t.is_empty()) && !self.guild_ids.is_empty()
    }

    fn skip_reason(&self) -> Option<String> {
        if self.is_configured() {
            None
        } else {
            Some("DISCORD_BOT_TOKEN and DISCORD_GUILD_IDS required".into())
        }
    }

    fn estimated_search_usd(&self) -> f64 {
        0.0
    }

    fn max_page_size(&self) -> u32 {
        25
    }

    fn notes(&self) -> &'static str {
        "Guild-scoped GET /guilds/{id}/messages/search. Requires token + guild IDs. Discord often rejects Bot tokens on this route; set credentials bearer=true (or DISCORD_AUTH=bearer) for a user token. No public global Discord search API."
    }

    async fn search(&self, request: &ProviderSearchRequest<'_>) -> Result<SearchPage> {
        let token = self
            .token
            .as_deref()
            .filter(|t| !t.is_empty())
            .ok_or_else(|| Error::NotConfigured {
                provider: "discord".into(),
                reason: "DISCORD_BOT_TOKEN not set".into(),
            })?;
        if self.guild_ids.is_empty() {
            return Err(Error::NotConfigured {
                provider: "discord".into(),
                reason: "DISCORD_GUILD_IDS not set".into(),
            });
        }
        // Single guild: honor offset cursor. Multiple: fan-out without cross-guild cursor.
        if self.guild_ids.len() == 1 {
            return self.search_guild(token, &self.guild_ids[0], request).await;
        }
        if request.cursor.is_some() {
            return Err(Error::Invalid(
                "discord cursor pagination requires a single guild id".into(),
            ));
        }
        let mut hits = Vec::new();
        let mut last_err = None;
        for guild_id in &self.guild_ids {
            match self.search_guild(token, guild_id, request).await {
                Ok(page) => hits.extend(page.hits),
                Err(err) => last_err = Some(err),
            }
        }
        if hits.is_empty() {
            return Err(last_err.unwrap_or_else(|| {
                Error::provider("discord", "no results from configured guilds")
            }));
        }
        Ok(super::page(hits))
    }
}
