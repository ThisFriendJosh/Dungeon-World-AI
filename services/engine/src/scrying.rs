use super::*;
use serde_json::json;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScryingBookmark {
    pub id: String,
    pub title: String,
    pub url: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BookmarkRequest {
    pub title: String,
    pub url: String,
}
pub fn validated_browser_url(value: &str) -> Result<String, EngineError> {
    let value = value.trim();
    let bad = || {
        EngineError::Invalid("Enter an HTTP or HTTPS address without embedded credentials.".into())
    };
    if value.len() > 2048 || value.chars().any(char::is_control) {
        return Err(bad());
    }
    let url = url::Url::parse(value).map_err(|_| bad())?;
    if !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url
            .host_str()
            .is_some_and(|h| h == "tauri.localhost" || h.ends_with(".tauri.localhost"))
    {
        return Err(bad());
    }
    Ok(url.into())
}
impl CampaignStore {
    pub fn list_scrying_bookmarks(
        &self,
        campaign_id: &str,
    ) -> Result<Vec<ScryingBookmark>, EngineError> {
        let conn = connect(&self.locate(campaign_id)?, false)?;
        let mut stmt=conn.prepare("SELECT id,title,url FROM scrying_bookmarks WHERE campaign_id=?1 ORDER BY title COLLATE NOCASE,id")?;
        let result = stmt
            .query_map([campaign_id], |r| {
                Ok(ScryingBookmark {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    url: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(result)
    }
    pub fn save_scrying_bookmark(
        &self,
        campaign_id: &str,
        request: BookmarkRequest,
    ) -> Result<Saved<ScryingBookmark>, EngineError> {
        let title = validate_name(&request.title, "Bookmark title")?;
        if title.chars().any(char::is_control) {
            return Err(EngineError::Invalid(
                "Use a bookmark title without control characters.".into(),
            ));
        }
        let url = validated_browser_url(&request.url)?;
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM scrying_bookmarks WHERE campaign_id=?1 AND url=?2",
                params![campaign_id, url],
                |r| r.get(0),
            )
            .optional()?;
        if existing.is_none() {
            let count: i64 = tx.query_row(
                "SELECT COUNT(*) FROM scrying_bookmarks WHERE campaign_id=?1",
                [campaign_id],
                |r| r.get(0),
            )?;
            if count >= 100 {
                return Err(EngineError::Invalid(
                    "This campaign supports up to 100 browser bookmarks.".into(),
                ));
            }
        }
        let id = existing.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = now();
        tx.execute("INSERT INTO scrying_bookmarks(id,campaign_id,title,url,created_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(campaign_id,url) DO UPDATE SET title=excluded.title",params![id,campaign_id,title,url,timestamp])?;
        record_event(
            &tx,
            campaign_id,
            "scrying.bookmark.saved",
            &json!({"id":id,"title":title}),
            &timestamp,
        )?;
        tx.commit()?;
        Ok(saved(
            ScryingBookmark { id, title, url },
            &dir,
            &conn,
            campaign_id,
        ))
    }
    pub fn remove_scrying_bookmark(
        &self,
        campaign_id: &str,
        id: &str,
        confirmed: bool,
    ) -> Result<Saved<()>, EngineError> {
        if !confirmed {
            return Err(EngineError::Invalid(
                "Confirm removal of this bookmark.".into(),
            ));
        }
        let _guard = WRITE_LOCK
            .lock()
            .map_err(|_| EngineError::Invalid("Save lock unavailable.".into()))?;
        let dir = self.locate(campaign_id)?;
        let mut conn = connect(&dir, false)?;
        let tx = conn.transaction()?;
        if tx.execute(
            "DELETE FROM scrying_bookmarks WHERE campaign_id=?1 AND id=?2",
            params![campaign_id, id],
        )? != 1
        {
            return Err(EngineError::Invalid(
                "Bookmark not found in this campaign.".into(),
            ));
        }
        record_event(
            &tx,
            campaign_id,
            "scrying.bookmark.removed",
            &json!({"id":id}),
            &now(),
        )?;
        tx.commit()?;
        Ok(saved((), &dir, &conn, campaign_id))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_urls_reject_commands_files_and_credentials() {
        for value in [
            "javascript:alert(1)",
            "file:///secret",
            "data:text/html,test",
            "about:config",
            "//example.org",
            concat!("https", "://user:password@example.org"),
            concat!("http", "://tauri.localhost/"),
        ] {
            assert!(validated_browser_url(value).is_err(), "{value}");
        }
        assert_eq!(
            validated_browser_url(concat!("https", "://EXAMPLE.org/book#page=2")).unwrap(),
            concat!("https", "://example.org/book#page=2")
        );
    }
    #[test]
    fn bookmarks_upsert_without_duplicates_and_remain_campaign_scoped() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("scrying-test-{}", Uuid::new_v4()));
        let store = CampaignStore::new(path.clone()).unwrap();
        let a = store.create_campaign("A").unwrap().value;
        let b = store.create_campaign("B").unwrap().value;
        let one = store
            .save_scrying_bookmark(
                &a.id,
                BookmarkRequest {
                    title: "Reference".into(),
                    url: concat!("https", "://example.org/book").into(),
                },
            )
            .unwrap()
            .value;
        store
            .save_scrying_bookmark(
                &a.id,
                BookmarkRequest {
                    title: "Renamed".into(),
                    url: one.url.clone(),
                },
            )
            .unwrap();
        assert_eq!(store.list_scrying_bookmarks(&a.id).unwrap().len(), 1);
        assert_eq!(
            store.list_scrying_bookmarks(&a.id).unwrap()[0].title,
            "Renamed"
        );
        assert!(store.remove_scrying_bookmark(&b.id, &one.id, true).is_err());
        assert!(store
            .remove_scrying_bookmark(&a.id, &one.id, false)
            .is_err());
        store.remove_scrying_bookmark(&a.id, &one.id, true).unwrap();
        assert!(store.list_scrying_bookmarks(&a.id).unwrap().is_empty());
        drop(store);
        let _ = fs::remove_dir_all(path);
    }
}
