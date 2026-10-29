mod commands;
mod error;
mod health;
mod jobs;
mod model;
mod pipeline;
mod services;
mod settings;
mod store;

use std::sync::{Arc, RwLock};

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::AppState;
use crate::jobs::{Context, EventSink, JobInfo, Jobs};
use crate::settings::{Settings, settings_path};
use crate::store::ProjectStore;

/// Event names shared with the UI (`src/lib/ipc.ts`).
const EVENT_JOB: &str = "job";
const EVENT_PROJECT_CHANGED: &str = "project-changed";

struct WindowSink(AppHandle);

impl EventSink for WindowSink {
    // Emitting only fails when the window is gone; there is nobody left to tell.
    fn job(&self, info: &JobInfo) {
        let _ = self.0.emit(EVENT_JOB, info);
    }

    fn project_changed(&self, project_id: &str) {
        let _ = self.0.emit(EVENT_PROJECT_CHANGED, project_id);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let settings_path = settings_path(&app.path().app_config_dir()?);
            let settings = Arc::new(RwLock::new(Settings::load(&settings_path)));
            let store = Arc::new(ProjectStore::new(app.path().app_data_dir()?.join("projects"))?);
            // The UI shows generated images straight from disk via the asset protocol, scoped to this folder only.
            app.asset_protocol_scope().allow_directory(store.root(), true)?;
            // All services are local; skipping the system proxy keeps a corporate proxy from intercepting localhost.
            let http = reqwest::Client::builder().no_proxy().build()?;
            let jobs = Jobs::start(Context {
                store: Arc::clone(&store),
                settings: Arc::clone(&settings),
                http: http.clone(),
                sink: Box::new(WindowSink(app.handle().clone())),
            });
            app.manage(AppState { store, settings, settings_path, http, jobs });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects_root,
            commands::projects_list,
            commands::project_get,
            commands::project_create,
            commands::project_patch,
            commands::project_delete,
            commands::project_music_set,
            commands::project_music_remove,
            commands::job_enqueue,
            commands::jobs_list,
            commands::job_cancel,
            commands::settings_get,
            commands::settings_save,
            commands::health_check,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AI Video Studio");
}
