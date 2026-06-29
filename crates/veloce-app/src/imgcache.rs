use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use egui::load::{BytesPoll, LoadError};

#[derive(Clone)]
enum Entry {
    Pending,
    Ready(egui::load::Bytes),
    Failed,
}

pub struct DiskCacheLoader {
    dir: PathBuf,
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

impl DiskCacheLoader {
    pub fn new() -> Self {
        let dir = directories::ProjectDirs::from("", "", "veloce")
            .map(|d| d.cache_dir().join("images"))
            .unwrap_or_else(|| std::env::temp_dir().join("veloce-images"));
        let _ = std::fs::create_dir_all(&dir);
        Self {
            dir,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn cache_path(&self, uri: &str) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        uri.hash(&mut hasher);
        let hex = format!("{:016x}", hasher.finish());
        self.dir.join(hex)
    }
}

impl egui::load::BytesLoader for DiskCacheLoader {
    fn id(&self) -> &str {
        "veloce_disk_cache"
    }

    fn load(&self, ctx: &egui::Context, uri: &str) -> egui::load::BytesLoadResult {
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            return Err(LoadError::NotSupported);
        }

        let mut map = self.cache.lock().unwrap();
        match map.get(uri) {
            Some(Entry::Ready(bytes)) => {
                return Ok(BytesPoll::Ready {
                    size: None,
                    bytes: bytes.clone(),
                    mime: None,
                });
            }
            Some(Entry::Pending) => return Ok(BytesPoll::Pending { size: None }),
            Some(Entry::Failed) => {
                return Err(LoadError::Loading("fetch failed".into()));
            }
            None => {}
        }

        let cache_path = self.cache_path(uri);

        // Disk hit
        if let Ok(data) = std::fs::read(&cache_path) {
            if !data.is_empty() {
                let bytes = egui::load::Bytes::Shared(Arc::from(data));
                map.insert(uri.to_string(), Entry::Ready(bytes.clone()));
                return Ok(BytesPoll::Ready {
                    size: None,
                    bytes,
                    mime: None,
                });
            }
        }

        // Network fetch
        map.insert(uri.to_string(), Entry::Pending);
        drop(map);

        let cache_arc = Arc::clone(&self.cache);
        let ctx2 = ctx.clone();
        let uri_owned = uri.to_string();

        ehttp::fetch(ehttp::Request::get(uri), move |result| {
            let mut map = cache_arc.lock().unwrap();
            match result {
                Ok(resp) if resp.ok => {
                    let _ = std::fs::write(&cache_path, &resp.bytes);
                    let bytes = egui::load::Bytes::Shared(Arc::from(resp.bytes));
                    map.insert(uri_owned, Entry::Ready(bytes));
                }
                _ => {
                    map.insert(uri_owned, Entry::Failed);
                }
            }
            drop(map);
            ctx2.request_repaint();
        });

        Ok(BytesPoll::Pending { size: None })
    }

    fn forget(&self, uri: &str) {
        self.cache.lock().unwrap().remove(uri);
    }

    fn forget_all(&self) {
        self.cache.lock().unwrap().clear();
    }

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .unwrap()
            .values()
            .map(|e| match e {
                Entry::Ready(b) => b.len(),
                _ => 0,
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_is_deterministic() {
        let loader = DiskCacheLoader::new();
        let uri = "https://example.com/img.png";
        let p1 = loader.cache_path(uri);
        let p2 = loader.cache_path(uri);
        assert_eq!(p1, p2, "same URI must produce same path");
    }

    #[test]
    fn cache_path_differs_for_different_uris() {
        let loader = DiskCacheLoader::new();
        let p1 = loader.cache_path("https://a.com/1.png");
        let p2 = loader.cache_path("https://a.com/2.png");
        assert_ne!(p1, p2, "different URIs must produce different paths");
    }

    #[test]
    fn cache_path_is_under_cache_dir() {
        let loader = DiskCacheLoader::new();
        let p = loader.cache_path("https://a.com/1.png");
        assert!(p.starts_with(&loader.dir));
    }
}
