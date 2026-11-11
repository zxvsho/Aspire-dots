use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct WhItem {
    pub id: String,
    pub full_url: String,
    pub thumb_url: String,
    pub ext: String,
}

#[derive(Clone, Debug)]
pub struct SearchPage {
    pub items: Vec<WhItem>,
    pub last_page: u32,
    pub seed: Option<String>,
}

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .new_agent()
}

pub fn search(
    cfg: &crate::BgConfig,
    query: &str,
    categories: &str,
    sorting: &str,
    purity: &str,
    seed: Option<&str>,
    page: u32,
) -> Result<SearchPage, String> {
    let agent = agent();
    let mut req = agent
        .get("https://wallhaven.cc/api/v1/search")
        .query("categories", categories)
        .query("purity", purity)
        .query("ratios", "16x9")
        .query("page", page.to_string());
    if !cfg.wallhaven.atleast.trim().is_empty() {
        req = req.query("atleast", cfg.wallhaven.atleast.trim());
    }
    let is_hot = sorting == "hot";
    let sorting = if is_hot {
        req = req.query("topRange", "1d");
        "toplist"
    } else {
        sorting
    };
    req = req.query("sorting", sorting);
    if sorting == "toplist" && !is_hot {
        req = req.query("topRange", "1M");
    }
    if let Some(s) = seed {
        if page > 1 {
            req = req.query("seed", s);
        }
    }
    if !query.trim().is_empty() {
        req = req.query("q", query.trim());
    }
    if !cfg.wallhaven.api_key.trim().is_empty() {
        req = req.header("X-API-Key", cfg.wallhaven.api_key.trim());
    }
    let mut resp = req.call().map_err(|e| {
        if let ureq::Error::StatusCode(code) = e {
            match code {
                401 => "Wallhaven: invalid or required API key (401)".to_string(),
                429 => "Wallhaven: request limit reached (429)".to_string(),
                _ => format!("Wallhaven: HTTP {code}"),
            }
        } else {
            "Wallhaven: network or connection error".to_string()
        }
    })?;
    if !resp.status().is_success() {
        return Err(format!("wallhaven: HTTP {}", resp.status()));
    }
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("wallhaven: {e}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Wallhaven: invalid JSON ({e})"))?;
    let data = json
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut items = Vec::new();
    for entry in data {
        let id = entry
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            continue;
        }
        let url = entry
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let full_url = entry
            .get("path")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&url)
            .to_string();
        let thumb_url = entry
            .get("thumbs")
            .and_then(|v| {
                v.get("original")
                    .or_else(|| v.get("large"))
                    .or_else(|| v.get("small"))
            })
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let ext = extension_of(&full_url);
        items.push(WhItem {
            id,
            full_url,
            thumb_url,
            ext,
        });
    }
    let meta = json.get("meta");
    let last_page = meta
        .and_then(|m| m.get("last_page"))
        .and_then(|v| v.as_u64())
        .unwrap_or(page as u64) as u32;
    let seed = meta.and_then(|m| m.get("seed")).and_then(|v| {
        v.as_str()
            .map(|s| s.to_string())
            .or_else(|| v.as_i64().map(|n| n.to_string()))
    });
    Ok(SearchPage {
        items,
        last_page,
        seed,
    })
}

fn extension_of(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");
    let ext = ext.to_lowercase();
    if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
        ext
    } else {
        "jpg".to_string()
    }
}

pub fn cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache/aspire-bg")
}

pub fn thumb_cache_path(url: &str) -> PathBuf {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    url.hash(&mut h);
    cache_dir().join(format!("wh-{:016x}.{}", h.finish(), extension_of(url)))
}

pub fn download_thumb(url: &str) -> Result<PathBuf, String> {
    if url.is_empty() {
        return Err("thumbnail URL is empty".to_string());
    }
    let path = thumb_cache_path(url);
    if path.is_file() {
        trim_cache();
        return Ok(path);
    }
    let agent = agent();
    let mut resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("wallhaven thumb: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("wallhaven thumb: HTTP {}", resp.status()));
    }
    let bytes = resp
        .body_mut()
        .read_to_vec()
        .map_err(|e| format!("wallhaven thumb: {e}"))?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, &bytes).map_err(|e| format!("wallhaven thumb: {e}"))?;
    trim_cache();
    Ok(path)
}

pub fn download_full(item: &WhItem, dest_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("wallhaven: {e}"))?;
    let safe_id: String = item
        .id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let ext = if item.ext.is_empty() {
        "jpg".to_string()
    } else {
        item.ext.clone()
    };
    let final_path = dest_dir.join(format!("wallhaven-{}.{}", safe_id, ext));
    if final_path.is_file() {
        return Ok(final_path);
    }
    let agent = agent();
    let mut resp = agent
        .get(&item.full_url)
        .call()
        .map_err(|e| format!("wallhaven: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("wallhaven: HTTP {}", resp.status()));
    }
    let tmp = dest_dir.join(format!(".wallhaven-{}.{}.part", safe_id, ext));
    let bytes = resp
        .body_mut()
        .read_to_vec()
        .map_err(|e| format!("wallhaven: {e}"))?;
    std::fs::write(&tmp, &bytes).map_err(|e| format!("wallhaven: {e}"))?;
    std::fs::rename(&tmp, &final_path).map_err(|e| format!("wallhaven: {e}"))?;
    Ok(final_path)
}

pub fn trim_cache() {
    let dir = cache_dir();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return;
    };
    let mut files: Vec<(PathBuf, std::time::SystemTime, u64)> = Vec::new();
    let mut total: u64 = 0;
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !name.starts_with("wh-") {
            continue;
        }
        let Ok(meta) = p.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let size = meta.len();
        total += size;
        files.push((p, mtime, size));
    }
    // límites: 200 archivos o 50 MB
    const MAX_FILES: usize = 200;
    const MAX_BYTES: u64 = 50 * 1024 * 1024;
    files.sort_by_key(|x| x.1);
    let mut i = 0;
    while (files.len().saturating_sub(i) > MAX_FILES || total > MAX_BYTES) && i < files.len() {
        let (p, _, size) = &files[i];
        let _ = std::fs::remove_file(p);
        total = total.saturating_sub(*size);
        i += 1;
    }
}

pub fn trim_wallhaven_dir(dir: &Path, applied: &str, new_file: &str) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !name.starts_with("wallhaven-") {
            continue;
        }
        let Ok(meta) = p.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let path_str = p.to_string_lossy().to_string();
        if path_str == applied || path_str == new_file {
            continue;
        }
        let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        files.push((p, mtime));
    }
    files.sort_by_key(|x| x.1);
    while files.len() + 2 > 30 && !files.is_empty() {
        // +2 = aplicado + nuevo (protegidos)
        if let Some((p, _)) = files.first() {
            let _ = std::fs::remove_file(p);
        }
        files.remove(0);
    }
}
