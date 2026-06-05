#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(debug_assertions)]
use std::process::Command as StdCommand;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_shell::{
    process::{CommandChild, CommandEvent},
    ShellExt,
};
use tokio::sync::oneshot;

#[cfg(debug_assertions)]
fn engine_flags(
    strict_align: bool,
    enable_ocr: bool,
    spec_tolerance: bool,
    flatten_headers: bool,
) -> [&'static str; 4] {
    [
        if strict_align { "--strict" } else { "--normal" },
        if enable_ocr { "--ocr" } else { "--no-ocr" },
        if spec_tolerance {
            "--tolerate"
        } else {
            "--no-tolerate"
        },
        if flatten_headers {
            "--flatten"
        } else {
            "--no-flatten"
        },
    ]
}

#[cfg(debug_assertions)]
fn run_python_engine(
    file_path: &str,
    strict_align: bool,
    enable_ocr: bool,
    spec_tolerance: bool,
    flatten_headers: bool,
) -> Result<String, String> {
    let script_path = concat!(env!("CARGO_MANIFEST_DIR"), "/engine.py");
    let flags = engine_flags(strict_align, enable_ocr, spec_tolerance, flatten_headers);

    let output = StdCommand::new("python")
        .arg(script_path)
        .arg(file_path)
        .args(flags)
        .output()
        .map_err(|e| format!("Unable to start Python process: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Document parsing failed: {}", stderr))
    }
}

#[derive(Clone)]
struct EngineManager {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    app_handle: Mutex<Option<AppHandle>>,
    next_id: AtomicU64,
    process_generation: AtomicU64,
    pending: Mutex<HashMap<String, oneshot::Sender<Result<DocumentParseResult, String>>>>,
    state: Mutex<EngineState>,
    restarting: Mutex<bool>,
}

struct EngineState {
    child: Option<CommandChild>,
    active_generation: u64,
    ready: bool,
    queued: VecDeque<EngineQueuedRequest>,
    in_flight: HashMap<String, EngineQueuedRequest>,
}

#[derive(Clone)]
struct EngineQueuedRequest {
    id: String,
    payload: String,
    attempts: u8,
}

#[derive(Serialize)]
struct EngineRequest {
    id: String,
    file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    strict_align: bool,
    enable_ocr: bool,
    spec_tolerance: bool,
    flatten_headers: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    header_row_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_rows: Option<Vec<usize>>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentParseResult {
    markdown: String,
    confidence: Option<f64>,
    meta: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temp_file_path: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParseProgress {
    id: String,
    percent: u8,
    stage: String,
}

#[derive(Deserialize)]
struct EngineResponse {
    id: String,
    ok: bool,
    markdown: Option<String>,
    confidence: Option<f64>,
    meta: Option<Value>,
    error: Option<String>,
}

impl EngineManager {
    fn new() -> Self {
        Self {
            inner: Arc::new(EngineInner {
                app_handle: Mutex::new(None),
                next_id: AtomicU64::new(1),
                process_generation: AtomicU64::new(0),
                pending: Mutex::new(HashMap::new()),
                state: Mutex::new(EngineState {
                    child: None,
                    active_generation: 0,
                    ready: false,
                    queued: VecDeque::new(),
                    in_flight: HashMap::new(),
                }),
                restarting: Mutex::new(false),
            }),
        }
    }

    fn set_app_handle(&self, app_handle: AppHandle) {
        *self.inner.app_handle.lock().unwrap() = Some(app_handle);
    }

    fn start(&self, app_handle: AppHandle) {
        self.set_app_handle(app_handle);
        if let Err(error) = self.restart() {
            eprintln!("[Engine Warn]: initial start failed: {}", error);
        }
    }

    fn next_request_id(&self) -> String {
        format!(
            "{}-{}",
            std::process::id(),
            self.inner.next_id.fetch_add(1, Ordering::Relaxed)
        )
    }

    async fn request(
        &self,
        file_path: &str,
        strict_align: bool,
        enable_ocr: bool,
        spec_tolerance: bool,
        flatten_headers: bool,
    ) -> Result<DocumentParseResult, String> {
        self.request_with_id(
            None,
            file_path,
            strict_align,
            enable_ocr,
            spec_tolerance,
            flatten_headers,
        )
        .await
    }

    async fn request_with_id(
        &self,
        request_id: Option<String>,
        file_path: &str,
        strict_align: bool,
        enable_ocr: bool,
        spec_tolerance: bool,
        flatten_headers: bool,
    ) -> Result<DocumentParseResult, String> {
        let id = request_id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| self.next_request_id());
        self.request_engine(EngineRequest {
            id,
            file_path: file_path.to_string(),
            mode: None,
            strict_align,
            enable_ocr,
            spec_tolerance,
            flatten_headers,
            header_row_index: None,
            skip_rows: None,
        })
        .await
    }

    async fn request_repair(
        &self,
        request_id: Option<String>,
        file_path: &str,
        header_row_index: usize,
        skip_rows: Vec<usize>,
    ) -> Result<DocumentParseResult, String> {
        let id = request_id
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| self.next_request_id());
        self.request_engine(EngineRequest {
            id,
            file_path: file_path.to_string(),
            mode: Some("repair".to_string()),
            strict_align: true,
            enable_ocr: false,
            spec_tolerance: false,
            flatten_headers: false,
            header_row_index: Some(header_row_index),
            skip_rows: Some(skip_rows),
        })
        .await
    }

    async fn request_engine(&self, request: EngineRequest) -> Result<DocumentParseResult, String> {
        if !self.is_running_or_starting() {
            return Err("Engine sidecar is not running".to_string());
        }

        let id = request.id.clone();
        let payload = serde_json::to_string(&request)
            .map_err(|e| format!("Unable to encode engine request: {}", e))?;

        let (tx, rx) = oneshot::channel();
        self.inner.pending.lock().unwrap().insert(id.clone(), tx);

        self.dispatch_or_queue(EngineQueuedRequest {
            id: id.clone(),
            payload,
            attempts: 0,
        });

        match tokio::time::timeout(Duration::from_secs(300), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Engine response channel closed".to_string()),
            Err(_) => {
                self.inner.pending.lock().unwrap().remove(&id);
                Err("Engine request timed out".to_string())
            }
        }
    }

    fn is_running_or_starting(&self) -> bool {
        let has_child = self.inner.state.lock().unwrap().child.is_some();
        let restarting = *self.inner.restarting.lock().unwrap();
        has_child || restarting
    }

    fn restart(&self) -> Result<(), String> {
        {
            let mut restarting = self.inner.restarting.lock().unwrap();
            if *restarting {
                return Ok(());
            }
            *restarting = true;
        }

        let result = self.start_process();
        *self.inner.restarting.lock().unwrap() = false;
        result
    }

    fn start_process(&self) -> Result<(), String> {
        let generation = self
            .inner
            .process_generation
            .fetch_add(1, Ordering::Relaxed)
            + 1;
        let mut failed_ids = Vec::new();
        let old_child = {
            let mut state = self.inner.state.lock().unwrap();
            state.ready = false;
            state.active_generation = generation;

            let in_flight = std::mem::take(&mut state.in_flight);
            for (_, mut request) in in_flight {
                request.attempts += 1;
                if request.attempts <= 1 {
                    state.queued.push_front(request);
                } else {
                    failed_ids.push(request.id);
                }
            }

            state.child.take()
        };

        if let Some(child) = old_child {
            let _ = child.kill();
        }

        for id in failed_ids {
            if let Some(sender) = self.inner.pending.lock().unwrap().remove(&id) {
                let _ = sender.send(Err(
                    "Engine process restarted while handling request".to_string()
                ));
            }
        }

        let app_handle = self
            .inner
            .app_handle
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "App handle is not available for engine startup".to_string())?;
        let spawn_result = app_handle
            .shell()
            .sidecar("engine-server")
            .and_then(|command| command.spawn())
            .map_err(|error| format!("Unable to start bundled engine sidecar: {}", error));

        let (mut rx, child) = match spawn_result {
            Ok(process) => process,
            Err(sidecar_error) => {
                #[cfg(not(debug_assertions))]
                {
                    self.fail_queued_requests(&sidecar_error);
                    return Err(sidecar_error);
                }

                #[cfg(debug_assertions)]
                {
                    let script_path = concat!(env!("CARGO_MANIFEST_DIR"), "/engine_server.py");
                    eprintln!("[Engine Warn]: {}", sidecar_error);
                    app_handle
                        .shell()
                        .command("python")
                        .args([script_path])
                        .spawn()
                        .map_err(|error| {
                            let message = format!(
                                "Unable to start development Python engine server: {}",
                                error
                            );
                            self.fail_queued_requests(&message);
                            message
                        })?
                }
            }
        };

        {
            let mut state = self.inner.state.lock().unwrap();
            state.child = Some(child);
            state.active_generation = generation;
            state.ready = false;
        }

        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        manager
                            .handle_stdout(generation, String::from_utf8_lossy(&line).to_string());
                    }
                    CommandEvent::Stderr(line) => {
                        eprintln!("[Engine stderr]: {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Terminated(payload) => {
                        eprintln!("[Engine Warn]: process terminated: {:?}", payload.code);
                        manager.handle_exit(generation);
                        return;
                    }
                    CommandEvent::Error(error) => {
                        eprintln!("[Engine Warn]: process error: {}", error);
                        manager.handle_exit(generation);
                        return;
                    }
                    _ => {}
                }
            }

            manager.handle_exit(generation);
        });

        Ok(())
    }

    fn dispatch_or_queue(&self, request: EngineQueuedRequest) {
        let mut needs_restart = false;

        {
            let mut state = self.inner.state.lock().unwrap();
            if state.ready {
                if let Some(child) = state.child.as_mut() {
                    let line = format!("{}\n", request.payload);
                    match child.write(line.as_bytes()) {
                        Ok(()) => {
                            state.in_flight.insert(request.id.clone(), request);
                        }
                        Err(error) => {
                            eprintln!("[Engine Warn]: stdin write failed: {}", error);
                            state.ready = false;
                            state.queued.push_front(request);
                            needs_restart = true;
                        }
                    }
                } else {
                    state.ready = false;
                    state.queued.push_back(request);
                    needs_restart = true;
                }
            } else {
                state.queued.push_back(request);
            }
        }

        if needs_restart {
            if let Err(error) = self.restart() {
                eprintln!(
                    "[Engine Warn]: restart after write failure failed: {}",
                    error
                );
            }
        }
    }

    fn handle_stdout(&self, generation: u64, raw_line: String) {
        if !self.is_current_generation(generation) {
            return;
        }

        let line = raw_line.trim_end_matches(['\r', '\n']).trim();
        if line.is_empty() {
            return;
        }

        let value = match serde_json::from_str::<Value>(line) {
            Ok(value) => value,
            Err(error) => {
                eprintln!(
                    "[Engine Warn]: invalid stdout JSON: {}; line={}",
                    error, line
                );
                return;
            }
        };

        if value.get("type").and_then(Value::as_str) == Some("ready") {
            {
                let mut state = self.inner.state.lock().unwrap();
                if state.active_generation != generation {
                    return;
                }
                state.ready = true;
            }
            self.flush_queue();
            return;
        }

        if value.get("type").and_then(Value::as_str) == Some("progress") {
            let progress = match serde_json::from_value::<ParseProgress>(value) {
                Ok(progress) => progress,
                Err(error) => {
                    eprintln!("[Engine Warn]: invalid progress payload: {}", error);
                    return;
                }
            };
            self.emit_progress(progress);
            return;
        }

        let response = match serde_json::from_value::<EngineResponse>(value) {
            Ok(response) => response,
            Err(error) => {
                eprintln!("[Engine Warn]: invalid response payload: {}", error);
                return;
            }
        };

        {
            let mut state = self.inner.state.lock().unwrap();
            if state.active_generation != generation {
                return;
            }
            state.in_flight.remove(&response.id);
        }

        if let Some(sender) = self.inner.pending.lock().unwrap().remove(&response.id) {
            let result = if response.ok {
                self.emit_progress(ParseProgress {
                    id: response.id.clone(),
                    percent: 100,
                    stage: "生成 Markdown".to_string(),
                });
                Ok(DocumentParseResult {
                    markdown: response.markdown.unwrap_or_default(),
                    confidence: response.confidence,
                    meta: response.meta,
                    temp_file_path: None,
                })
            } else {
                Err(response
                    .error
                    .unwrap_or_else(|| "Engine returned an unknown error".to_string()))
            };
            let _ = sender.send(result);
        }
    }

    fn flush_queue(&self) {
        loop {
            let mut needs_restart = false;
            let has_more = {
                let mut state = self.inner.state.lock().unwrap();
                if !state.ready {
                    return;
                }

                let Some(request) = state.queued.pop_front() else {
                    return;
                };

                if let Some(child) = state.child.as_mut() {
                    let line = format!("{}\n", request.payload);
                    match child.write(line.as_bytes()) {
                        Ok(()) => {
                            state.in_flight.insert(request.id.clone(), request);
                            !state.queued.is_empty()
                        }
                        Err(error) => {
                            eprintln!(
                                "[Engine Warn]: stdin write failed while flushing: {}",
                                error
                            );
                            state.ready = false;
                            state.queued.push_front(request);
                            needs_restart = true;
                            false
                        }
                    }
                } else {
                    state.ready = false;
                    state.queued.push_front(request);
                    needs_restart = true;
                    false
                }
            };

            if needs_restart {
                if let Err(error) = self.restart() {
                    eprintln!("[Engine Warn]: restart while flushing failed: {}", error);
                }
                return;
            }

            if !has_more {
                return;
            }
        }
    }

    fn handle_exit(&self, generation: u64) {
        let mut failed_ids = Vec::new();

        {
            let mut state = self.inner.state.lock().unwrap();
            if state.active_generation != generation {
                return;
            }

            state.ready = false;
            state.child = None;

            let in_flight = std::mem::take(&mut state.in_flight);
            for (_, mut request) in in_flight {
                request.attempts += 1;
                if request.attempts <= 1 {
                    state.queued.push_front(request);
                } else {
                    failed_ids.push(request.id);
                }
            }
        }

        for id in failed_ids {
            if let Some(sender) = self.inner.pending.lock().unwrap().remove(&id) {
                let _ = sender.send(Err(
                    "Engine process exited while handling request".to_string()
                ));
            }
        }

        if let Err(error) = self.restart() {
            eprintln!("[Engine Warn]: automatic restart failed: {}", error);
        }
    }

    fn is_current_generation(&self, generation: u64) -> bool {
        self.inner.state.lock().unwrap().active_generation == generation
    }

    fn emit_progress(&self, progress: ParseProgress) {
        let app_handle = self.inner.app_handle.lock().unwrap().clone();
        if let Some(app_handle) = app_handle {
            if let Err(error) = app_handle.emit("parse-progress", progress) {
                eprintln!("[Engine Warn]: failed to emit parse progress: {}", error);
            }
        }
    }

    fn fail_queued_requests(&self, message: &str) {
        let failed_ids = {
            let mut state = self.inner.state.lock().unwrap();
            state.ready = false;
            state.child = None;

            let mut ids = Vec::new();
            ids.extend(state.queued.drain(..).map(|request| request.id));
            ids.extend(state.in_flight.drain().map(|(id, _)| id));
            ids
        };

        for id in failed_ids {
            if let Some(sender) = self.inner.pending.lock().unwrap().remove(&id) {
                let _ = sender.send(Err(message.to_string()));
            }
        }
    }
}

#[tauri::command]
async fn convert_document(
    file_path: String,
    engine: State<'_, EngineManager>,
) -> Result<String, String> {
    engine
        .request(&file_path, true, false, false, false)
        .await
        .map(|result| result.markdown)
        .or_else(|error| {
            #[cfg(not(debug_assertions))]
            {
                Err(error)
            }

            #[cfg(debug_assertions)]
            {
                eprintln!(
                    "[Engine Warn]: persistent engine failed, using one-shot fallback: {}",
                    error
                );
                run_python_engine(&file_path, true, false, false, false)
            }
        })
}

#[cfg(debug_assertions)]
fn fallback_parse_result(markdown: String) -> DocumentParseResult {
    DocumentParseResult {
        markdown,
        confidence: None,
        meta: None,
        temp_file_path: None,
    }
}

fn unique_temp_path(file_name: &str) -> std::path::PathBuf {
    let extension = std::path::Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.trim().is_empty())
        .unwrap_or("tmp");

    std::env::temp_dir().join(format!(
        "markdowngui-{}.{}",
        uuid::Uuid::new_v4(),
        extension
    ))
}

fn cleanup_temp_path(file_path: &str) -> Result<(), String> {
    let path = std::path::PathBuf::from(file_path);
    if !path.exists() {
        return Ok(());
    }

    let canonical_path = path
        .canonicalize()
        .map_err(|e| format!("Unable to resolve temp file path: {}", e))?;
    let canonical_temp_dir = std::env::temp_dir()
        .canonicalize()
        .map_err(|e| format!("Unable to resolve temp dir: {}", e))?;

    if !canonical_path.starts_with(canonical_temp_dir) {
        return Err("Refusing to remove file outside temp dir".to_string());
    }

    std::fs::remove_file(canonical_path).map_err(|e| format!("Unable to remove temp file: {}", e))
}

#[tauri::command]
async fn cleanup_temp_document(file_path: String) -> Result<(), String> {
    cleanup_temp_path(&file_path)
}

#[tauri::command]
async fn repair_document(
    file_path: String,
    header_row_index: usize,
    skip_rows: Vec<usize>,
    request_id: Option<String>,
    engine: State<'_, EngineManager>,
) -> Result<DocumentParseResult, String> {
    engine
        .request_repair(request_id, &file_path, header_row_index, skip_rows)
        .await
}

#[tauri::command]
async fn convert_document_bytes(
    file_name: String,
    file_data_base64: String,
    strict_align: bool,
    enable_ocr: bool,
    spec_tolerance: bool,
    flatten_headers: bool,
    request_id: Option<String>,
    engine: State<'_, EngineManager>,
) -> Result<DocumentParseResult, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&file_data_base64)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;

    let temp_path = unique_temp_path(&file_name);
    std::fs::write(&temp_path, &bytes).map_err(|e| format!("Unable to create temp file: {}", e))?;

    let temp_path_string = temp_path.to_string_lossy().to_string();
    let mut result = engine
        .request_with_id(
            request_id,
            &temp_path_string,
            strict_align,
            enable_ocr,
            spec_tolerance,
            flatten_headers,
        )
        .await
        .or_else(|error| {
            #[cfg(not(debug_assertions))]
            {
                Err(error)
            }

            #[cfg(debug_assertions)]
            {
                eprintln!(
                    "[Engine Warn]: persistent engine failed, using one-shot fallback: {}",
                    error
                );
                run_python_engine(
                    &temp_path_string,
                    strict_align,
                    enable_ocr,
                    spec_tolerance,
                    flatten_headers,
                )
                .map(fallback_parse_result)
            }
        });

    if let Ok(parse_result) = result.as_mut() {
        if parse_result.confidence.unwrap_or(1.0) < 0.7 && parse_result.meta.is_some() {
            parse_result.temp_file_path = Some(temp_path_string);
            return result;
        }
    }

    let _ = std::fs::remove_file(&temp_path);
    result
}

#[tauri::command]
async fn process_document(
    file_path: String,
    strict_align: bool,
    enable_ocr: bool,
    spec_tolerance: bool,
    flatten_headers: bool,
    engine: State<'_, EngineManager>,
) -> Result<String, String> {
    engine
        .request(
            &file_path,
            strict_align,
            enable_ocr,
            spec_tolerance,
            flatten_headers,
        )
        .await
        .map(|result| result.markdown)
        .or_else(|error| {
            #[cfg(not(debug_assertions))]
            {
                Err(error)
            }

            #[cfg(debug_assertions)]
            {
                eprintln!(
                    "[Engine Warn]: persistent engine failed, using one-shot fallback: {}",
                    error
                );
                run_python_engine(
                    &file_path,
                    strict_align,
                    enable_ocr,
                    spec_tolerance,
                    flatten_headers,
                )
            }
        })
}

fn main() {
    let engine_manager = EngineManager::new();
    let startup_engine = engine_manager.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(engine_manager)
        .setup(move |app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(2)).await;
                startup_engine.start(app_handle);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            convert_document,
            convert_document_bytes,
            process_document,
            repair_document,
            cleanup_temp_document
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
