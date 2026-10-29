//! Projects on disk: `<root>/<id>/project.json`, `source.txt`, `characters/*.png`, `shots/*.png`, `render/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::{AppError, AppResult};
use crate::model::{NewProject, Project, ProjectSummary, now_ms};

const PROJECT_FILE: &str = "project.json";
const SOURCE_FILE: &str = "source.txt";

pub struct ProjectStore {
    root: PathBuf,
    /// One lock per project: every read-modify-write of project.json holds it.
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl ProjectStore {
    pub fn new(root: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&root)?;
        Ok(Self { root, locks: Mutex::default() })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Folder of a project. The id comes from the UI, so reject anything that could leave `root`.
    pub fn dir(&self, id: &str) -> AppResult<PathBuf> {
        let valid = !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !valid {
            return Err(AppError::invalid(format!("Mã dự án không hợp lệ: '{id}'")));
        }
        Ok(self.root.join(id))
    }

    pub async fn list(&self) -> AppResult<Vec<ProjectSummary>> {
        let mut out = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.root).await?;
        while let Some(entry) = entries.next_entry().await? {
            let Ok(id) = entry.file_name().into_string() else { continue };
            // A folder without a readable project.json is skipped, not fatal: one broken project must not hide the rest.
            if let Ok(p) = self.get(&id).await {
                out.push(ProjectSummary::from(&p));
            }
        }
        out.sort_by_key(|p| std::cmp::Reverse(p.updated_at));
        Ok(out)
    }

    pub async fn get(&self, id: &str) -> AppResult<Project> {
        let path = self.dir(id)?.join(PROJECT_FILE);
        let bytes = tokio::fs::read(&path).await.map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => AppError::not_found(format!("Không tìm thấy dự án '{id}'")),
            _ => e.into(),
        })?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn create(&self, input: &NewProject) -> AppResult<Project> {
        let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_owned();
        let dir = self.dir(&id)?;
        tokio::fs::create_dir_all(&dir).await?;
        tokio::fs::write(dir.join(SOURCE_FILE), &input.text).await?;
        let now = now_ms();
        let project = Project {
            id,
            title: input.source_label.chars().take(80).collect(),
            source_kind: input.source_kind,
            source_label: input.source_label.clone(),
            language: input.language.clone(),
            format: input.format,
            style: String::new(),
            characters: Vec::new(),
            shots: Vec::new(),
            video: None,
            music: None,
            created_at: now,
            updated_at: now,
        };
        write_atomic(&dir.join(PROJECT_FILE), &serde_json::to_vec_pretty(&project)?).await?;
        Ok(project)
    }

    /// Read-modify-write under the project lock. `f` failing leaves the file untouched.
    pub async fn update<R>(&self, id: &str, f: impl FnOnce(&mut Project) -> AppResult<R>) -> AppResult<(Project, R)> {
        let lock = self.lock_for(id);
        let _guard = lock.lock().await;
        let mut project = self.get(id).await?;
        let out = f(&mut project)?;
        project.updated_at = now_ms();
        write_atomic(&self.dir(id)?.join(PROJECT_FILE), &serde_json::to_vec_pretty(&project)?).await?;
        Ok((project, out))
    }

    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let lock = self.lock_for(id);
        let _guard = lock.lock().await;
        tokio::fs::remove_dir_all(self.dir(id)?).await?;
        Ok(())
    }

    pub async fn source(&self, id: &str) -> AppResult<String> {
        Ok(tokio::fs::read_to_string(self.dir(id)?.join(SOURCE_FILE)).await?)
    }

    /// Writes a generated file at a project-relative path built by the app (never by the UI).
    pub async fn write_file(&self, id: &str, rel: &str, bytes: &[u8]) -> AppResult<()> {
        let path = self.dir(id)?.join(rel);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        write_atomic(&path, bytes).await
    }

    fn lock_for(&self, id: &str) -> Arc<tokio::sync::Mutex<()>> {
        // shortcut: locks are never removed, one tiny entry per project opened; fine for hundreds of projects.
        let mut locks = self.locks.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(locks.entry(id.to_owned()).or_default())
    }
}

/// Write to a temp file, then rename over the target: a crash mid-write never leaves a half-written file.
async fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Character, Format, ProjectPatch, SourceKind, Take, TakeTarget};

    fn store() -> (tempfile::TempDir, ProjectStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = ProjectStore::new(dir.path().to_path_buf()).unwrap();
        (dir, store)
    }

    fn input() -> NewProject {
        NewProject {
            source_kind: SourceKind::Prompt,
            source_label: "Rừng Na Uy".into(),
            text: "Toru và Naoko đi dạo".into(),
            language: "vi".into(),
            format: Format::Landscape,
            shot_count: 4,
        }
    }

    #[test]
    fn rejects_ids_that_escape_the_root() {
        let (_d, s) = store();
        for bad in ["", "..", "../x", "a/b", "a\\b", "C:", "ABC"] {
            assert!(s.dir(bad).is_err(), "{bad} should be rejected");
        }
        assert!(s.dir("3f2a9c1b7d4e").is_ok());
    }

    #[tokio::test]
    async fn create_update_list_roundtrip() {
        let (_d, s) = store();
        let p = s.create(&input()).await.unwrap();
        assert_eq!(s.source(&p.id).await.unwrap(), "Toru và Naoko đi dạo");

        s.update(&p.id, |p| {
            p.characters.push(Character {
                id: "toru".into(),
                name: "Toru".into(),
                description: String::new(),
                takes: vec![],
                selected_take: None,
            });
            Ok(())
        })
        .await
        .unwrap();
        let take = Take { id: "toru-1".into(), file: "characters/toru-1.png".into(), seed: 1, created_at: 0, review: None };
        let (p2, ()) = s.update(&p.id, |p| p.push_take(&TakeTarget::Character("toru".into()), take)).await.unwrap();
        assert_eq!(p2.characters[0].selected_take.as_deref(), Some("toru-1"));

        let list = s.list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Rừng Na Uy");
    }

    #[tokio::test]
    async fn failed_update_leaves_file_unchanged() {
        let (_d, s) = store();
        let p = s.create(&input()).await.unwrap();
        let err =
            s.update(&p.id, |p| p.apply(ProjectPatch::Meta { title: "  ".into(), style: "x".into(), format: Format::Portrait })).await;
        assert!(err.is_err());
        assert_eq!(s.get(&p.id).await.unwrap().format, Format::Landscape);
    }

    #[tokio::test]
    async fn concurrent_updates_are_not_lost() {
        let (_d, s) = store();
        let s = Arc::new(s);
        let p = s.create(&input()).await.unwrap();
        s.update(&p.id, |p| {
            p.characters.push(Character {
                id: "a".into(),
                name: "A".into(),
                description: String::new(),
                takes: vec![],
                selected_take: None,
            });
            Ok(())
        })
        .await
        .unwrap();
        let tasks: Vec<_> = (0..20)
            .map(|i| {
                let s = Arc::clone(&s);
                let id = p.id.clone();
                tokio::spawn(async move {
                    let take = Take { id: format!("t{i}"), file: format!("characters/t{i}.png"), seed: i, created_at: 0, review: None };
                    s.update(&id, |p| p.push_take(&TakeTarget::Character("a".into()), take)).await.unwrap();
                })
            })
            .collect();
        for t in tasks {
            t.await.unwrap();
        }
        assert_eq!(s.get(&p.id).await.unwrap().characters[0].takes.len(), 20);
    }
}
