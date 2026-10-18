mod cache;
mod catalog;
mod model;
mod proxy;
mod sources;

use cache::BoardCache;
use catalog::BoardConfig;
use model::BoardResult;
use proxy::{ProxyMode, ProxySetting, ProxySettings};
use sources::Source;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Manager;

pub struct AppState {
    /// 直连客户端。
    pub http: reqwest::Client,
    /// 系统代理客户端（跟随 Windows 系统代理）。启动时读一次；用户改系统代理需要重启才生效。
    pub proxy_http: Arc<Mutex<Option<reqwest::Client>>>,
    /// 用户手动填的代理客户端。运行时可热重建。
    pub manual_http: Arc<Mutex<Option<reqwest::Client>>>,
    pub manual_http_url: Arc<Mutex<Option<String>>>,
    pub cache: Arc<BoardCache>,
    pub catalog: Arc<Mutex<Vec<BoardConfig>>>,
    pub catalog_path: PathBuf,
    pub proxy_state: Arc<Mutex<ProxySettings>>,
    pub settings_path: PathBuf,
}

impl Clone for AppState {
    /// 浅拷贝：reqwest::Client 本身用 Arc 共享；所有 mutex 都是 Arc，也是浅拷贝。
    fn clone(&self) -> Self {
        Self {
            http: self.http.clone(),
            proxy_http: Arc::clone(&self.proxy_http),
            manual_http: Arc::clone(&self.manual_http),
            manual_http_url: Arc::clone(&self.manual_http_url),
            cache: Arc::clone(&self.cache),
            catalog: Arc::clone(&self.catalog),
            catalog_path: self.catalog_path.clone(),
            proxy_state: Arc::clone(&self.proxy_state),
            settings_path: self.settings_path.clone(),
        }
    }
}

impl AppState {
    fn new(
        http: reqwest::Client,
        proxy_http: Option<reqwest::Client>,
        cache: Arc<BoardCache>,
        catalog: Arc<Mutex<Vec<BoardConfig>>>,
        catalog_path: PathBuf,
        proxy_state: Arc<Mutex<ProxySettings>>,
        settings_path: PathBuf,
    ) -> Self {
        Self {
            http,
            proxy_http: Arc::new(Mutex::new(proxy_http)),
            manual_http: Arc::new(Mutex::new(None)),
            manual_http_url: Arc::new(Mutex::new(None)),
            cache,
            catalog,
            catalog_path,
            proxy_state,
            settings_path,
        }
    }
}

fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                  (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 HotBoard/0.1";

fn build_client(proxy: &reqwest::Proxy) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(12))
        .proxy(proxy.clone())
        .build()
        .map_err(|e| format!("构建 HTTP 客户端失败: {e}"))
}

fn build_direct_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| format!("构建直连 HTTP 客户端失败: {e}"))
}

async fn fetch_one(
    source: &dyn Source,
    client: &reqwest::Client,
    cache: &BoardCache,
) -> BoardResult {
    let id = source.id().to_string();
    let name = source.name().to_string();
    let timeout = Duration::from_secs(20);
    let result = tokio::time::timeout(timeout, source.fetch(client)).await;

    match result {
        Ok(Ok(items)) => {
            let board = BoardResult {
                id: id.clone(),
                name,
                items,
                updated_at: now_unix(),
                error: None,
                from_cache: false,
            };
            let _ = cache.put(&board);
            board
        }
        Ok(Err(e)) => fallback(id, name, cache, e.to_string()),
        Err(_) => fallback(id, name, cache, "请求超时".into()),
    }
}

fn fallback(id: String, name: String, cache: &BoardCache, err: String) -> BoardResult {
    if let Ok(Some(mut cached)) = cache.get(&id) {
        cached.error = Some(err);
        return cached;
    }
    BoardResult {
        id,
        name,
        items: vec![],
        updated_at: now_unix(),
        error: Some(err),
        from_cache: false,
    }
}

// ---------------------------------------------------------------------------
// 代理分派
// ---------------------------------------------------------------------------

/// 根据源的 effective mode 选一个 HTTP 客户端。
fn pick_client_with_settings(
    state: &AppState,
    settings: &ProxySettings,
    source_id: &str,
    server_default: ProxyMode,
) -> Arc<reqwest::Client> {
    if settings.setting == ProxySetting::Off {
        return Arc::new(state.http.clone());
    }
    let mode = settings
        .source_modes
        .get(source_id)
        .copied()
        .unwrap_or(server_default);

    match mode {
        ProxyMode::Direct => Arc::new(state.http.clone()),
        ProxyMode::Proxy => pick_proxy_client(state, settings.setting),
        ProxyMode::Auto => match settings.setting {
            ProxySetting::Off => Arc::new(state.http.clone()),
            ProxySetting::Manual => pick_proxy_client(state, settings.setting),
            ProxySetting::Auto => {
                if state.proxy_http.lock().unwrap().is_some() {
                    pick_proxy_client(state, settings.setting)
                } else {
                    Arc::new(state.http.clone())
                }
            }
        },
    }
}

fn pick_proxy_client(state: &AppState, setting: ProxySetting) -> Arc<reqwest::Client> {
    match setting {
        ProxySetting::Manual => state
            .manual_http
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| Arc::new(c.clone()))
            .unwrap_or_else(|| Arc::new(state.http.clone())),
        ProxySetting::Auto => state
            .proxy_http
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| Arc::new(c.clone()))
            .unwrap_or_else(|| Arc::new(state.http.clone())),
        ProxySetting::Off => Arc::new(state.http.clone()),
    }
}

// ---------------------------------------------------------------------------
// Tauri 命令
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_cached(state: tauri::State<'_, AppState>) -> Result<Vec<BoardResult>, String> {
    let boards = state.catalog.lock().unwrap().clone();
    let ids: Vec<String> = boards.into_iter().map(|board| board.id).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    state.cache.get_all(&refs).map_err(|e| e.to_string())
}

#[tauri::command]
async fn refresh_all(state: tauri::State<'_, AppState>) -> Result<Vec<BoardResult>, String> {
    // 1) 刷新云端榜单清单
    let current = state.catalog.lock().unwrap().clone();
    let boards = catalog::resolve(&state.http, &state.catalog_path, &current).await;
    *state.catalog.lock().unwrap() = boards.clone();

    // 2) 快照当前代理设置（避免 async 期间持锁）
    let settings = state.proxy_state.lock().unwrap().clone();
    let cache = state.cache.clone();

    // 3) 先同步构造好所有 future，再 join_all。避免在 map 里持锁/移动。
    let futs: Vec<_> = boards
        .iter()
        .map(|board| {
            let client_arc =
                pick_client_with_settings(&state, &settings, &board.id, board.source_mode);
            let source = catalog::build_source(board);
            let cache = cache.clone();
            async move {
                fetch_one(source.as_ref(), client_arc.as_ref(), cache.as_ref()).await
            }
        })
        .collect();

    Ok(futures::future::join_all(futs).await)
}

// ---------------------------------------------------------------------------
// 设置面板 wire 类型
// ---------------------------------------------------------------------------

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
enum ProxySettingWire {
    Auto,
    Manual,
    Off,
}

impl From<ProxySetting> for ProxySettingWire {
    fn from(v: ProxySetting) -> Self {
        match v {
            ProxySetting::Auto => Self::Auto,
            ProxySetting::Manual => Self::Manual,
            ProxySetting::Off => Self::Off,
        }
    }
}

impl From<ProxySettingWire> for ProxySetting {
    fn from(v: ProxySettingWire) -> Self {
        match v {
            ProxySettingWire::Auto => Self::Auto,
            ProxySettingWire::Manual => Self::Manual,
            ProxySettingWire::Off => Self::Off,
        }
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPayload {
    setting: ProxySettingWire,
    proxy_url: Option<String>,
    /// 用户手动改过的源。缺省 = 跟随云端默认。
    source_modes: std::collections::BTreeMap<String, ProxyMode>,
    /// 云端 boards.json 里配置的默认模式（只读展示）。
    source_defaults: std::collections::BTreeMap<String, ProxyMode>,
    /// 榜单 id → 显示名称（用于设置面板展示）。
    source_names: std::collections::BTreeMap<String, String>,
    /// 当前 Windows 系统检测到的代理（前端提示用）。
    system_proxy: Option<String>,
    /// 手动代理当前是否已构建成功（setting=Manual 时）。
    manual_client_active: bool,
}

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsUpdate {
    setting: ProxySettingWire,
    proxy_url: Option<String>,
    source_modes: std::collections::BTreeMap<String, ProxyMode>,
}

fn build_payload(state: &AppState, settings: &ProxySettings) -> SettingsPayload {
    let boards = state.catalog.lock().unwrap().clone();
    let source_defaults = boards
        .iter()
        .map(|b| (b.id.clone(), b.source_mode))
        .collect();
    let source_names = boards
        .iter()
        .map(|b| (b.id.clone(), b.name.clone()))
        .collect();
    let manual_client_active = state.manual_http.lock().unwrap().is_some();
    SettingsPayload {
        setting: settings.setting.into(),
        proxy_url: settings.proxy_url.clone(),
        source_modes: settings.source_modes.clone(),
        source_defaults,
        source_names,
        system_proxy: proxy::detect_system_proxy(),
        manual_client_active,
    }
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> SettingsPayload {
    let settings = state.proxy_state.lock().unwrap().clone();
    build_payload(&state, &settings)
}

#[tauri::command]
fn set_settings(
    state: tauri::State<'_, AppState>,
    payload: SettingsUpdate,
) -> Result<SettingsPayload, String> {
    let new_setting = payload.setting.into();
    let new_url = payload.proxy_url.filter(|s| !s.trim().is_empty());

    // 判断是否需要重建手动代理客户端
    let need_rebuild_manual = {
        let s = state.proxy_state.lock().unwrap();
        (new_setting == ProxySetting::Manual
            && s.proxy_url.as_deref() != new_url.as_deref())
            || (s.setting == ProxySetting::Manual && new_setting != ProxySetting::Manual)
    };

    // 写入新设置
    {
        let mut settings_guard = state.proxy_state.lock().unwrap();
        settings_guard.setting = new_setting;
        settings_guard.proxy_url = new_url.clone();
        settings_guard.source_modes = payload.source_modes.clone();
    }

    // 需要时重建手动代理客户端
    if need_rebuild_manual {
        let settings = state.proxy_state.lock().unwrap().clone();
        rebuild_manual(&state, &settings)?;
    }

    // 持久化（写失败不阻塞返回）
    let settings = state.proxy_state.lock().unwrap().clone();
    if let Err(e) = proxy::write_settings(&settings, &state.settings_path) {
        eprintln!("写入 proxy_config.json 失败: {e}");
    }

    Ok(build_payload(&state, &settings))
}

/// 根据当前 settings 重建手动代理客户端。
fn rebuild_manual(state: &AppState, settings: &ProxySettings) -> Result<(), String> {
    if settings.setting != ProxySetting::Manual {
        *state.manual_http.lock().unwrap() = None;
        *state.manual_http_url.lock().unwrap() = None;
        return Ok(());
    }
    let url = match settings.proxy_url.as_deref() {
        Some(u) if !u.trim().is_empty() => u,
        _ => {
            *state.manual_http.lock().unwrap() = None;
            *state.manual_http_url.lock().unwrap() = None;
            return Ok(());
        }
    };
    let proxy =
        reqwest::Proxy::all(url).map_err(|e| format!("无效代理地址 '{url}': {e}"))?;
    let client = build_client(&proxy)?;
    *state.manual_http.lock().unwrap() = Some(client);
    *state.manual_http_url.lock().unwrap() = Some(url.to_string());
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("app data dir");

            let http = build_direct_client().expect("build direct client");

            // 系统代理客户端（跟随 Windows 系统代理，仅启动时读一次）
            let system_proxy_url = proxy::detect_system_proxy();
            let proxy_http = system_proxy_url.as_deref().and_then(|url| {
                reqwest::Proxy::all(url)
                    .ok()
                    .and_then(|p| build_client(&p).ok())
            });

            let settings_path = data_dir.join(proxy::SETTINGS_FILE);
            let settings = proxy::load_settings(&settings_path);

            let catalog_path = data_dir.join("board_catalog.json");
            let boards = catalog::load_local(&catalog_path);

            let state = AppState::new(
                http,
                proxy_http,
                Arc::new(
                    BoardCache::open(data_dir.clone()).expect("open cache"),
                ),
                Arc::new(Mutex::new(boards)),
                catalog_path,
                Arc::new(Mutex::new(settings.clone())),
                settings_path,
            );
            app.manage(state.clone());

            if settings.setting == ProxySetting::Manual {
                if let Err(e) = rebuild_manual(&state, &settings) {
                    eprintln!("初始化手动代理客户端失败: {e}");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_cached,
            refresh_all,
            get_settings,
            set_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
