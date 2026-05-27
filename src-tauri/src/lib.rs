use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Local, Utc};
use git2::{
    Cred, FetchOptions, IndexAddOption, PushOptions, RemoteCallbacks, Repository, Signature,
};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    sync::{mpsc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State, WindowEvent,
};
use walkdir::WalkDir;

const APP_NAME: &str = "ConfigPilot";
const DEFAULT_REPO: &str = "configpilot-dotfiles";
const KEYCHAIN_SERVICE: &str = "com.configpilot.github";
const KEYCHAIN_USER: &str = "github-token";
const DEVICE_FLOW_URL: &str = "https://github.com/login/device/code";
const AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const API_BASE: &str = "https://api.github.com";
const OAUTH_CALLBACK_PORT: u16 = 39119;

#[derive(Debug, thiserror::Error)]
enum CommandError {
    #[error("{0}")]
    Message(String),
}

impl serde::Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<anyhow::Error> for CommandError {
    fn from(value: anyhow::Error) -> Self {
        Self::Message(value.to_string())
    }
}

type CommandResult<T> = std::result::Result<T, CommandError>;

#[derive(Default)]
struct WatcherState {
    watcher: Option<RecommendedWatcher>,
}

#[derive(Clone)]
struct AppPaths {
    data_dir: PathBuf,
    repo_dir: PathBuf,
    state_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    github_username: Option<String>,
    repo_owner: Option<String>,
    repo_name: String,
    repo_private: bool,
    last_sync_at: Option<String>,
    last_commit: Option<String>,
    auto_watch: bool,
    sync_status: String,
    logs: Vec<String>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            github_username: None,
            repo_owner: None,
            repo_name: DEFAULT_REPO.to_string(),
            repo_private: true,
            last_sync_at: None,
            last_commit: None,
            auto_watch: true,
            sync_status: "idle".to_string(),
            logs: vec!["ConfigPilot 已准备就绪。".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppStateDto {
    github_username: Option<String>,
    repo_owner: Option<String>,
    repo_name: String,
    repo_private: bool,
    last_sync_at: Option<String>,
    last_commit: Option<String>,
    auto_watch: bool,
    sync_status: String,
    has_token: bool,
    config_files: Vec<ConfigFile>,
    conflicts: Vec<ConflictRecord>,
    logs: Vec<String>,
    app_data_dir: String,
    repo_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigFile {
    id: String,
    label: String,
    source_path: String,
    target_path: String,
    exists: bool,
    is_dir: bool,
    hash: Option<String>,
    last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    app: String,
    version: String,
    generated_at: String,
    files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestFile {
    id: String,
    source_path: String,
    target_path: String,
    hash: String,
    synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceFlowStart {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceTokenStatus {
    status: String,
    message: String,
    username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncReport {
    status: String,
    message: String,
    synced_files: Vec<String>,
    conflicts: Vec<ConflictRecord>,
    commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConflictRecord {
    file_id: String,
    source_path: String,
    target_path: String,
    local_conflict_path: String,
    remote_conflict_path: String,
    local_hash: String,
    remote_hash: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RepoResponse {
    full_name: String,
    clone_url: String,
    private: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct UserResponse {
    login: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct DeviceFlowResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct SyncNowRequest {
    mode: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ResolveConflictRequest {
    file_id: String,
    strategy: String,
}

pub fn run() {
    load_dotenv();
    tauri::Builder::default()
        .manage(Mutex::new(WatcherState::default()))
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            start_github_browser_login,
            start_github_device_flow,
            poll_github_device_flow,
            ensure_private_repo,
            scan_config_files,
            sync_now,
            start_watcher,
            stop_watcher,
            resolve_conflict,
            open_path,
            clear_local_cache
        ])
        .setup(|app| {
            setup_tray(app.handle())?;
            let paths = app_paths(app.handle())?;
            fs::create_dir_all(&paths.data_dir)?;
            fs::create_dir_all(&paths.repo_dir)?;
            if !paths.state_file.exists() {
                save_state(&paths, &PersistedState::default())?;
            }
            let state = load_state(&paths)?;
            if state.auto_watch {
                let watcher_state = app.state::<Mutex<WatcherState>>();
                start_watcher_internal(app.handle().clone(), &watcher_state)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Resized(size) if size.width == 0 && size.height == 0 => {
                let _ = window.hide();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running ConfigPilot");
}

fn setup_tray(app: &AppHandle) -> Result<()> {
    let show_item = MenuItem::with_id(app, "show", "显示 ConfigPilot", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
    TrayIconBuilder::with_id("configpilot-tray")
        .tooltip("ConfigPilot")
        .icon(
            app.default_window_icon()
                .context("缺少应用托盘图标")?
                .clone(),
        )
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn get_app_state(app: AppHandle) -> CommandResult<AppStateDto> {
    let paths = app_paths(&app)?;
    let state = load_state(&paths)?;
    let config_files = scan_configs()?;
    let conflicts = load_conflicts(&paths)?;
    Ok(AppStateDto {
        github_username: state.github_username,
        repo_owner: state.repo_owner,
        repo_name: state.repo_name,
        repo_private: state.repo_private,
        last_sync_at: state.last_sync_at,
        last_commit: state.last_commit,
        auto_watch: state.auto_watch,
        sync_status: state.sync_status,
        has_token: load_token_with_fallback(&app).is_ok(),
        config_files,
        conflicts,
        logs: state.logs,
        app_data_dir: paths.data_dir.display().to_string(),
        repo_dir: paths.repo_dir.display().to_string(),
    })
}

#[tauri::command]
async fn start_github_browser_login(app: AppHandle) -> CommandResult<DeviceTokenStatus> {
    let result = tauri::async_runtime::spawn_blocking(move || browser_login_blocking(app))
        .await
        .map_err(|error| anyhow!("浏览器登录任务失败：{error}"))??;
    Ok(result)
}

#[tauri::command]
async fn start_github_device_flow() -> CommandResult<DeviceFlowStart> {
    let client_id = github_client_id()?;
    let client = github_client();
    let response = client
        .post(DEVICE_FLOW_URL)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("scope", "repo read:user"),
        ])
        .send()
        .await
        .context("无法启动 GitHub Device Flow")?;
    let status = response.status();
    let body = response
        .text()
        .await
        .context("无法读取 GitHub Device Flow 响应")?;
    if !status.is_success() {
        return Err(anyhow!("GitHub Device Flow 返回错误状态 {status}: {body}").into());
    }
    let response = serde_json::from_str::<DeviceFlowResponse>(&body)
        .context("无法解析 GitHub Device Flow 响应")?;

    Ok(DeviceFlowStart {
        device_code: response.device_code,
        user_code: response.user_code,
        verification_uri: response.verification_uri,
        expires_in: response.expires_in,
        interval: response.interval,
    })
}

fn browser_login_blocking(app: AppHandle) -> Result<DeviceTokenStatus> {
    let client_id = github_client_id()?;
    let client_secret = github_client_secret()?;
    let listener = TcpListener::bind(("127.0.0.1", OAUTH_CALLBACK_PORT)).with_context(|| {
        format!("无法启动本机 OAuth 回调服务，请确认 127.0.0.1:{OAUTH_CALLBACK_PORT} 未被占用")
    })?;
    let redirect_uri = format!("http://127.0.0.1:{OAUTH_CALLBACK_PORT}/callback");
    let state = random_url_token(24);
    let verifier = random_url_token(64);
    let challenge = pkce_challenge(&verifier);
    let auth_url = format!(
        "{AUTHORIZE_URL}?client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("repo read:user"),
        urlencoding::encode(&state),
        urlencoding::encode(&challenge)
    );

    Command::new("open")
        .arg(&auth_url)
        .status()
        .context("无法打开 GitHub 登录页面")?;

    let (mut stream, _) = listener.accept().context("等待 GitHub OAuth 回调失败")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .context("无法设置 OAuth 回调读取超时")?;
    let mut first_line = String::new();
    {
        let mut reader = BufReader::new(&mut stream);
        reader
            .read_line(&mut first_line)
            .context("无法读取 GitHub OAuth 回调请求")?;
    }
    let path = first_line.split_whitespace().nth(1).unwrap_or_default();
    let params = parse_query(path);
    let returned_state = params.get("state").cloned().unwrap_or_default();
    if returned_state != state {
        write_oauth_response(
            &mut stream,
            false,
            "state 校验失败，请回到 ConfigPilot 重试。",
        )?;
        return Err(anyhow!("GitHub OAuth state 校验失败"));
    }
    let Some(code) = params.get("code").cloned() else {
        write_oauth_response(&mut stream, false, "GitHub 没有返回授权 code。")?;
        return Err(anyhow!("GitHub 没有返回授权 code"));
    };
    write_oauth_response(&mut stream, true, "授权完成，可以回到 ConfigPilot。")?;

    let token = tauri::async_runtime::block_on(exchange_web_code(
        &client_id,
        &client_secret,
        &code,
        &redirect_uri,
        &verifier,
    ))?;
    let user = tauri::async_runtime::block_on(github_user(&token))?;
    let paths = app_paths(&app)?;
    store_token(&paths, &token)?;
    let mut state = load_state(&paths)?;
    state.github_username = Some(user.login.clone());
    state.repo_owner = Some(user.login.clone());
    state.sync_status = "authorized".to_string();
    add_state_log(&mut state, "GitHub 浏览器登录授权成功。");
    save_state(&paths, &state)?;

    Ok(DeviceTokenStatus {
        status: "authorized".to_string(),
        message: "GitHub 登录成功。".to_string(),
        username: Some(user.login),
    })
}

async fn exchange_web_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<String> {
    let response = github_client()
        .post(TOKEN_URL)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .context("无法交换 GitHub OAuth token")?;
    let status = response.status();
    let body = response
        .text()
        .await
        .context("无法读取 GitHub token 响应")?;
    if !status.is_success() {
        return Err(anyhow!("GitHub token 交换失败 {status}: {body}"));
    }
    let token_response: TokenResponse =
        serde_json::from_str(&body).context("无法解析 GitHub token 响应")?;
    if let Some(error) = token_response.error {
        return Err(anyhow!(
            "GitHub token 交换失败：{} {}",
            error,
            token_response.error_description.unwrap_or_default()
        ));
    }
    token_response
        .access_token
        .ok_or_else(|| anyhow!("GitHub token 响应中没有 access_token"))
}

#[tauri::command]
async fn poll_github_device_flow(
    app: AppHandle,
    device_code: String,
) -> CommandResult<DeviceTokenStatus> {
    let client_id = github_client_id()?;
    let client = github_client();
    let response = client
        .post(TOKEN_URL)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("device_code", device_code.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .context("无法轮询 GitHub 授权状态")?
        .json::<TokenResponse>()
        .await
        .context("无法解析 GitHub 授权响应")?;

    if let Some(error) = response.error {
        return Ok(DeviceTokenStatus {
            status: error.clone(),
            message: response.error_description.unwrap_or(error),
            username: None,
        });
    }

    let token = response
        .access_token
        .ok_or_else(|| anyhow!("GitHub 未返回 access_token"))?;
    let paths = app_paths(&app)?;
    store_token(&paths, &token)?;
    let user = github_user(&token).await?;
    let mut state = load_state(&paths)?;
    state.github_username = Some(user.login.clone());
    state.repo_owner = Some(user.login.clone());
    state.sync_status = "authorized".to_string();
    add_state_log(&mut state, "GitHub 授权成功。");
    save_state(&paths, &state)?;

    Ok(DeviceTokenStatus {
        status: "authorized".to_string(),
        message: "GitHub 授权成功。".to_string(),
        username: Some(user.login),
    })
}

#[tauri::command]
async fn ensure_private_repo(app: AppHandle) -> CommandResult<RepoResponse> {
    let token = load_token_with_fallback(&app)?;
    let paths = app_paths(&app)?;
    let mut state = load_state(&paths)?;
    let user = github_user(&token).await?;
    let repo = ensure_repo(&token, DEFAULT_REPO).await?;
    state.github_username = Some(user.login.clone());
    state.repo_owner = Some(user.login);
    state.repo_name = DEFAULT_REPO.to_string();
    state.repo_private = repo.private;
    add_state_log(
        &mut state,
        &format!("GitHub 仓库已就绪：{}。", repo.full_name),
    );
    save_state(&paths, &state)?;
    Ok(repo)
}

#[tauri::command]
fn scan_config_files() -> CommandResult<Vec<ConfigFile>> {
    Ok(scan_configs()?)
}

#[tauri::command]
async fn sync_now(app: AppHandle, request: SyncNowRequest) -> CommandResult<SyncReport> {
    let mode = request.mode.as_str();
    let report = match mode {
        "backup" => backup_sync(&app).await?,
        "restore" => restore_sync(&app).await?,
        "bidirectional" => bidirectional_sync(&app).await?,
        other => return Err(anyhow!("未知同步模式：{other}").into()),
    };
    Ok(report)
}

#[tauri::command]
fn start_watcher(app: AppHandle, state: State<Mutex<WatcherState>>) -> CommandResult<String> {
    start_watcher_internal(app, &state)?;
    Ok("自动监听已开启。".to_string())
}

fn start_watcher_internal(app: AppHandle, watcher_state: &Mutex<WatcherState>) -> Result<()> {
    let paths = app_paths(&app)?;
    let mut persisted = load_state(&paths)?;
    let app_for_thread = app.clone();
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx).context("无法创建文件监听器")?;

    for config in scan_configs()? {
        if config.exists {
            watcher
                .watch(Path::new(&config.source_path), RecursiveMode::Recursive)
                .with_context(|| format!("无法监听 {}", config.source_path))?;
        }
    }

    thread::spawn(move || {
        let mut last = Instant::now();
        while let Ok(event) = rx.recv() {
            if event.is_err() {
                continue;
            }
            if last.elapsed() < Duration::from_secs(2) {
                continue;
            }
            last = Instant::now();
            let app_clone = app_for_thread.clone();
            tauri::async_runtime::spawn(async move {
                let _ = backup_sync(&app_clone).await;
            });
        }
    });

    watcher_state
        .lock()
        .map_err(|_| anyhow!("监听器状态锁定失败"))?
        .watcher = Some(watcher);
    persisted.auto_watch = true;
    persisted.sync_status = "watching".to_string();
    add_state_log(&mut persisted, "自动监听已开启。");
    save_state(&paths, &persisted)?;
    Ok(())
}

#[tauri::command]
fn stop_watcher(app: AppHandle, state: State<Mutex<WatcherState>>) -> CommandResult<String> {
    let paths = app_paths(&app)?;
    let mut persisted = load_state(&paths)?;
    state
        .lock()
        .map_err(|_| anyhow!("监听器状态锁定失败"))?
        .watcher = None;
    persisted.auto_watch = false;
    persisted.sync_status = "idle".to_string();
    add_state_log(&mut persisted, "自动监听已停止。");
    save_state(&paths, &persisted)?;
    Ok("自动监听已停止。".to_string())
}

#[tauri::command]
fn resolve_conflict(app: AppHandle, request: ResolveConflictRequest) -> CommandResult<String> {
    let paths = app_paths(&app)?;
    let conflicts = load_conflicts(&paths)?;
    let conflict = conflicts
        .iter()
        .find(|item| item.file_id == request.file_id)
        .ok_or_else(|| anyhow!("未找到冲突记录：{}", request.file_id))?;

    match request.strategy.as_str() {
        "local" => {
            copy_path(
                Path::new(&conflict.local_conflict_path),
                &paths.repo_dir.join(&conflict.target_path),
            )?;
        }
        "remote" => {
            copy_path(
                Path::new(&conflict.remote_conflict_path),
                Path::new(&conflict.source_path),
            )?;
        }
        "open" => {
            open_path(conflict.source_path.clone())?;
        }
        other => return Err(anyhow!("未知冲突处理策略：{other}").into()),
    }

    let remaining: Vec<ConflictRecord> = conflicts
        .into_iter()
        .filter(|item| item.file_id != request.file_id)
        .collect();
    save_conflicts(&paths, &remaining)?;
    Ok("冲突已处理。".to_string())
}

#[tauri::command]
fn open_path(path: String) -> CommandResult<()> {
    Command::new("open")
        .arg(path)
        .status()
        .context("无法打开路径")?;
    Ok(())
}

#[tauri::command]
fn clear_local_cache(app: AppHandle) -> CommandResult<String> {
    let paths = app_paths(&app)?;
    if paths.repo_dir.exists() {
        fs::remove_dir_all(&paths.repo_dir).context("无法清理本地同步缓存")?;
    }
    fs::create_dir_all(&paths.repo_dir).context("无法重新创建本地同步缓存目录")?;
    let mut state = load_state(&paths)?;
    state.last_commit = None;
    state.last_sync_at = None;
    state.sync_status = "idle".to_string();
    add_state_log(&mut state, "本地同步缓存已清理。");
    save_state(&paths, &state)?;
    Ok("本地同步缓存已清理。".to_string())
}

async fn backup_sync(app: &AppHandle) -> Result<SyncReport> {
    let token = load_token_with_fallback(app)?;
    let paths = app_paths(app)?;
    let mut state = load_state(&paths)?;
    state.sync_status = "syncing".to_string();
    save_state(&paths, &state)?;

    let repo = ensure_repo(&token, DEFAULT_REPO).await?;
    ensure_repo_checkout(&paths, &repo.clone_url, &token)?;
    let conflicts = detect_conflicts(&paths)?;
    if !conflicts.is_empty() {
        save_conflicts(&paths, &conflicts)?;
        state.sync_status = "conflict".to_string();
        add_state_log(&mut state, "发现同步冲突，已保留本地和远端副本。");
        save_state(&paths, &state)?;
        return Ok(SyncReport {
            status: "conflict".to_string(),
            message: "发现同步冲突。".to_string(),
            synced_files: vec![],
            conflicts,
            commit: None,
        });
    }

    let config_files = scan_configs()?;
    let synced = copy_configs_to_repo(&paths, &config_files)?;
    write_manifest(&paths, &config_files)?;
    let commit = commit_and_push(&paths, &token)?;
    state.last_sync_at = Some(now_string());
    state.last_commit = commit.clone();
    state.sync_status = "success".to_string();
    add_state_log(
        &mut state,
        &format!("备份完成：{} 个配置项。", synced.len()),
    );
    save_state(&paths, &state)?;
    Ok(SyncReport {
        status: "success".to_string(),
        message: "备份同步完成。".to_string(),
        synced_files: synced,
        conflicts: vec![],
        commit,
    })
}

async fn restore_sync(app: &AppHandle) -> Result<SyncReport> {
    let token = load_token_with_fallback(app)?;
    let paths = app_paths(app)?;
    let mut state = load_state(&paths)?;
    let repo = ensure_repo(&token, DEFAULT_REPO).await?;
    ensure_repo_checkout(&paths, &repo.clone_url, &token)?;
    pull_repo(&paths, &token)?;
    let config_files = scan_configs()?;
    let mut restored = vec![];
    for config in config_files {
        let source = paths.repo_dir.join(&config.target_path);
        if source.exists() {
            let destination = PathBuf::from(&config.source_path);
            if destination.exists() {
                let backup = destination
                    .with_extension(format!("configpilot-backup-{}", Utc::now().timestamp()));
                copy_path(&destination, &backup)?;
            }
            copy_path(&source, &destination)?;
            restored.push(config.source_path);
        }
    }
    state.sync_status = "success".to_string();
    state.last_sync_at = Some(now_string());
    add_state_log(
        &mut state,
        &format!("云端恢复完成：{} 个配置项。", restored.len()),
    );
    save_state(&paths, &state)?;
    Ok(SyncReport {
        status: "success".to_string(),
        message: "云端恢复完成。".to_string(),
        synced_files: restored,
        conflicts: vec![],
        commit: state.last_commit,
    })
}

async fn bidirectional_sync(app: &AppHandle) -> Result<SyncReport> {
    backup_sync(app).await
}

fn scan_configs() -> Result<Vec<ConfigFile>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("无法定位用户主目录"))?;
    let candidates = vec![
        ("zshrc", "Zsh 主配置", home.join(".zshrc"), "zsh/.zshrc"),
        (
            "zprofile",
            "Zsh 登录配置",
            home.join(".zprofile"),
            "zsh/.zprofile",
        ),
        (
            "zshenv",
            "Zsh 环境配置",
            home.join(".zshenv"),
            "zsh/.zshenv",
        ),
        (
            "zsh_config",
            "Zsh 配置目录",
            home.join(".config/zsh"),
            "zsh/config-zsh",
        ),
        (
            "ghostty_config",
            "Ghostty 主配置",
            home.join(".config/ghostty/config"),
            "ghostty/config",
        ),
        (
            "ghostty_dir",
            "Ghostty 配置目录",
            home.join(".config/ghostty"),
            "ghostty/ghostty",
        ),
    ];

    candidates
        .into_iter()
        .map(|(id, label, path, target)| {
            let exists = path.exists();
            let is_dir = exists && path.is_dir();
            let hash = if exists {
                Some(hash_path(&path)?)
            } else {
                None
            };
            let last_modified = if exists {
                Some(
                    fs::metadata(&path)?
                        .modified()
                        .ok()
                        .map(DateTime::<Local>::from)
                        .map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "-".to_string()),
                )
            } else {
                None
            };
            Ok(ConfigFile {
                id: id.to_string(),
                label: label.to_string(),
                source_path: path.display().to_string(),
                target_path: target.to_string(),
                exists,
                is_dir,
                hash,
                last_modified,
            })
        })
        .collect()
}

fn copy_configs_to_repo(paths: &AppPaths, config_files: &[ConfigFile]) -> Result<Vec<String>> {
    let mut synced = vec![];
    for config in config_files.iter().filter(|item| item.exists) {
        let source = PathBuf::from(&config.source_path);
        let target = paths.repo_dir.join(&config.target_path);
        if target.exists() {
            if target.is_dir() {
                fs::remove_dir_all(&target)?;
            } else {
                fs::remove_file(&target)?;
            }
        }
        copy_path(&source, &target)?;
        synced.push(config.source_path.clone());
    }
    Ok(synced)
}

fn write_manifest(paths: &AppPaths, config_files: &[ConfigFile]) -> Result<()> {
    let synced_at = now_string();
    let files = config_files
        .iter()
        .filter(|item| item.exists)
        .map(|item| ManifestFile {
            id: item.id.clone(),
            source_path: item.source_path.clone(),
            target_path: item.target_path.clone(),
            hash: item.hash.clone().unwrap_or_default(),
            synced_at: synced_at.clone(),
        })
        .collect();
    let manifest = Manifest {
        app: APP_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: synced_at,
        files,
    };
    fs::write(
        paths.repo_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(())
}

fn detect_conflicts(paths: &AppPaths) -> Result<Vec<ConflictRecord>> {
    let manifest_path = paths.repo_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Ok(vec![]);
    }
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(manifest_path)?)?;
    let remote_by_id: HashMap<String, ManifestFile> = manifest
        .files
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect();
    let mut conflicts = vec![];
    for config in scan_configs()?.into_iter().filter(|item| item.exists) {
        let Some(remote) = remote_by_id.get(&config.id) else {
            continue;
        };
        let Some(local_hash) = config.hash.clone() else {
            continue;
        };
        let remote_path = paths.repo_dir.join(&config.target_path);
        if !remote_path.exists() {
            continue;
        }
        let remote_hash = hash_path(&remote_path)?;
        if local_hash != remote.hash && remote_hash != remote.hash && local_hash != remote_hash {
            let conflict_dir = paths.data_dir.join("conflicts");
            fs::create_dir_all(&conflict_dir)?;
            let local_conflict = conflict_dir.join(format!("{}.local.conflict", config.id));
            let remote_conflict = conflict_dir.join(format!("{}.remote.conflict", config.id));
            copy_path(Path::new(&config.source_path), &local_conflict)?;
            copy_path(&remote_path, &remote_conflict)?;
            conflicts.push(ConflictRecord {
                file_id: config.id,
                source_path: config.source_path,
                target_path: config.target_path,
                local_conflict_path: local_conflict.display().to_string(),
                remote_conflict_path: remote_conflict.display().to_string(),
                local_hash,
                remote_hash,
                created_at: now_string(),
            });
        }
    }
    Ok(conflicts)
}

fn ensure_repo_checkout(paths: &AppPaths, clone_url: &str, token: &str) -> Result<()> {
    if paths.repo_dir.join(".git").exists() {
        pull_repo(paths, token)?;
        return Ok(());
    }
    if paths.repo_dir.exists() {
        fs::remove_dir_all(&paths.repo_dir)?;
    }
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_, _, _| Cred::userpass_plaintext("x-access-token", token));
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options);
    builder.clone(clone_url, &paths.repo_dir)?;
    Ok(())
}

fn pull_repo(paths: &AppPaths, token: &str) -> Result<()> {
    let repo = Repository::open(&paths.repo_dir)?;
    let mut remote = repo.find_remote("origin")?;
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_, _, _| Cred::userpass_plaintext("x-access-token", token));
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);
    remote.fetch(&["main"], Some(&mut fetch_options), None)?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;
    if analysis.is_fast_forward() {
        let refname = "refs/heads/main";
        match repo.find_reference(refname) {
            Ok(mut reference) => {
                reference.set_target(fetch_commit.id(), "Fast-forward")?;
                repo.set_head(refname)?;
                repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            }
            Err(_) => {
                repo.reference(refname, fetch_commit.id(), true, "Setting main")?;
                repo.set_head(refname)?;
                repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            }
        }
    }
    Ok(())
}

fn commit_and_push(paths: &AppPaths, token: &str) -> Result<Option<String>> {
    let repo = Repository::open(&paths.repo_dir)?;
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;
    if index.is_empty() {
        return Ok(None);
    }
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = Signature::now("ConfigPilot", "configpilot@local")?;
    let parent = repo
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repo.find_commit(oid).ok());
    let message = format!(
        "sync: update configs {}",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    let oid = if let Some(parent) = parent {
        repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent])?
    } else {
        repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[])?
    };
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_, _, _| Cred::userpass_plaintext("x-access-token", token));
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);
    let mut remote = repo.find_remote("origin")?;
    remote.push(
        &["refs/heads/main:refs/heads/main"],
        Some(&mut push_options),
    )?;
    Ok(Some(oid.to_string()))
}

async fn ensure_repo(token: &str, name: &str) -> Result<RepoResponse> {
    let client = github_client();
    let user = github_user(token).await?;
    let get_url = format!("{API_BASE}/repos/{}/{}", user.login, name);
    let get_response = client
        .get(&get_url)
        .bearer_auth(token)
        .send()
        .await
        .context("无法检查 GitHub 仓库")?;
    if get_response.status().is_success() {
        return Ok(get_response.json::<RepoResponse>().await?);
    }
    let create_response = client
        .post(format!("{API_BASE}/user/repos"))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "name": name,
            "private": true,
            "auto_init": true,
            "description": "ConfigPilot personal zsh and Ghostty configuration sync repository"
        }))
        .send()
        .await
        .context("无法创建 GitHub 私有仓库")?
        .error_for_status()
        .context("GitHub 创建仓库失败")?
        .json::<RepoResponse>()
        .await?;
    Ok(create_response)
}

async fn github_user(token: &str) -> Result<UserResponse> {
    github_client()
        .get(format!("{API_BASE}/user"))
        .bearer_auth(token)
        .send()
        .await
        .context("无法请求 GitHub 用户信息")?
        .error_for_status()
        .context("GitHub 用户信息请求失败")?
        .json::<UserResponse>()
        .await
        .context("无法解析 GitHub 用户信息")
}

fn github_client() -> Client {
    Client::builder()
        .user_agent("ConfigPilot/0.1.0")
        .build()
        .expect("reqwest client")
}

fn github_client_id() -> Result<String> {
    load_dotenv();
    std::env::var("CONFIGPILOT_GITHUB_CLIENT_ID")
        .or_else(|_| std::env::var("VITE_CONFIGPILOT_GITHUB_CLIENT_ID"))
        .map_err(|_| {
            anyhow!("缺少 GitHub OAuth Client ID。请设置 CONFIGPILOT_GITHUB_CLIENT_ID 环境变量。")
        })
}

fn github_client_secret() -> Result<String> {
    load_dotenv();
    std::env::var("CONFIGPILOT_GITHUB_CLIENT_SECRET")
        .or_else(|_| std::env::var("VITE_CONFIGPILOT_GITHUB_CLIENT_SECRET"))
        .map_err(|_| anyhow!("缺少 GitHub OAuth Client Secret。浏览器登录需要设置 CONFIGPILOT_GITHUB_CLIENT_SECRET 环境变量。"))
}

fn load_dotenv() {
    let _ = dotenvy::dotenv();
    if let Ok(current_dir) = std::env::current_dir() {
        let _ = dotenvy::from_path(current_dir.join(".env"));
        let _ = dotenvy::from_path(current_dir.join("../.env"));
    }
}

fn random_url_token(byte_len: usize) -> String {
    let mut bytes = vec![0_u8; byte_len];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

fn parse_query(path: &str) -> HashMap<String, String> {
    let query = path
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or_default();
    query
        .split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((
                key.to_string(),
                urlencoding::decode(value).ok()?.to_string(),
            ))
        })
        .collect()
}

fn write_oauth_response(stream: &mut std::net::TcpStream, ok: bool, message: &str) -> Result<()> {
    let title = if ok {
        "ConfigPilot 授权完成"
    } else {
        "ConfigPilot 授权失败"
    };
    let body = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body style=\"font-family:-apple-system,BlinkMacSystemFont,sans-serif;padding:40px\"><h1>{}</h1><p>{}</p></body></html>",
        title, title, message
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn app_paths(app: &AppHandle) -> Result<AppPaths> {
    let data_dir = app.path().app_data_dir().context("无法定位应用数据目录")?;
    Ok(AppPaths {
        repo_dir: data_dir.join("repo"),
        state_file: data_dir.join("state.json"),
        data_dir,
    })
}

fn load_state(paths: &AppPaths) -> Result<PersistedState> {
    if !paths.state_file.exists() {
        return Ok(PersistedState::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(
        &paths.state_file,
    )?)?)
}

fn save_state(paths: &AppPaths, state: &PersistedState) -> Result<()> {
    fs::create_dir_all(&paths.data_dir)?;
    fs::write(&paths.state_file, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn add_state_log(state: &mut PersistedState, message: &str) {
    state.logs.insert(
        0,
        format!("{}  {}", Local::now().format("%H:%M:%S"), message),
    );
    state.logs.truncate(80);
}

fn conflicts_file(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join("conflicts.json")
}

fn load_conflicts(paths: &AppPaths) -> Result<Vec<ConflictRecord>> {
    let file = conflicts_file(paths);
    if !file.exists() {
        return Ok(vec![]);
    }
    Ok(serde_json::from_str(&fs::read_to_string(file)?)?)
}

fn save_conflicts(paths: &AppPaths, conflicts: &[ConflictRecord]) -> Result<()> {
    fs::write(
        conflicts_file(paths),
        serde_json::to_string_pretty(conflicts)?,
    )?;
    Ok(())
}

fn save_token(token: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER)?;
    entry.set_password(token)?;
    Ok(())
}

fn store_token(paths: &AppPaths, token: &str) -> Result<()> {
    let keychain_result = save_token(token);
    save_token_fallback(paths, token)?;
    if let Err(error) = keychain_result {
        eprintln!(
            "ConfigPilot: Keychain token save failed, fallback token file was written: {error}"
        );
    }
    Ok(())
}

fn load_token() -> Result<String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER)?;
    entry
        .get_password()
        .context("未找到 GitHub token，请先授权")
}

fn token_fallback_file(paths: &AppPaths) -> PathBuf {
    paths.data_dir.join("github-token.local")
}

fn save_token_fallback(paths: &AppPaths, token: &str) -> Result<()> {
    fs::create_dir_all(&paths.data_dir)?;
    fs::write(token_fallback_file(paths), token)?;
    Ok(())
}

fn load_token_with_fallback(app: &AppHandle) -> Result<String> {
    match load_token() {
        Ok(token) => Ok(token),
        Err(keychain_error) => {
            let paths = app_paths(app)?;
            let fallback = token_fallback_file(&paths);
            if fallback.exists() {
                let token = fs::read_to_string(fallback)?.trim().to_string();
                if !token.is_empty() {
                    return Ok(token);
                }
            }
            Err(anyhow!(
                "未找到 GitHub token，请先点击“GitHub 登录”完成授权。Keychain 读取结果：{}",
                keychain_error
            ))
        }
    }
}

fn hash_path(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    if path.is_file() {
        hash_file(path, &mut hasher)?;
    } else {
        let mut files = WalkDir::new(path)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path().is_file())
            .map(|entry| entry.path().to_path_buf())
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            hasher.update(
                file.strip_prefix(path)
                    .unwrap_or(&file)
                    .display()
                    .to_string(),
            );
            hash_file(&file, &mut hasher)?;
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_file(path: &Path, hasher: &mut Sha256) -> Result<()> {
    hasher.update(fs::read(path)?);
    Ok(())
}

fn copy_path(source: &Path, destination: &Path) -> Result<()> {
    if source.is_dir() {
        fs::create_dir_all(destination)?;
        for entry in WalkDir::new(source) {
            let entry = entry?;
            let relative = entry.path().strip_prefix(source)?;
            let target = destination.join(relative);
            if entry.path().is_dir() {
                fs::create_dir_all(&target)?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(entry.path(), target)?;
            }
        }
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination)?;
    }
    Ok(())
}

fn now_string() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
