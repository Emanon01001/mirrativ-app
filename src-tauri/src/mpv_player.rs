// ─────────────────────────────────────────────────────────────────────────────
// mpv_player.rs
//
// libmpv を動的リンクして動画を再生する Tauri 統合モジュール。
// Windows 向けに MPV を WebView ウィンドウへ埋め込む機能を提供する。
//
// 主な責務:
//   - libmpv-2.dll の動的ロードと FFI バインディング
//   - MpvPlayerManager による再生状態の管理
//   - Win32 API を使った埋め込みホストウィンドウの作成・削除
//   - Tauri コマンド経由でフロントエンドと連携
// ─────────────────────────────────────────────────────────────────────────────

use libloading::Library;
use serde::{Deserialize, Serialize};
use std::ffi::{c_char, c_int, c_void, CString};
#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::ThreadId;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

// ─────────────────────────────────────────────────────────────────────────────
// libmpv FFI 型定義
// ─────────────────────────────────────────────────────────────────────────────

/// libmpv の不透明ハンドル型
type MpvHandle = *mut c_void;

/// mpv_create: 新しい MPV インスタンスを生成する
type MpvCreateFn = unsafe extern "C" fn() -> MpvHandle;
/// mpv_initialize: MPV インスタンスを初期化する（オプション設定後に呼ぶ）
type MpvInitializeFn = unsafe extern "C" fn(MpvHandle) -> c_int;
/// mpv_command: MPV にコマンドを送信する（loadfile, stop など）
type MpvCommandFn = unsafe extern "C" fn(MpvHandle, *const *const c_char) -> c_int;
/// mpv_set_option_string: 文字列オプションを設定する
type MpvSetOptionStringFn = unsafe extern "C" fn(MpvHandle, *const c_char, *const c_char) -> c_int;
/// mpv_terminate_destroy: MPV インスタンスを終了して破棄する
type MpvTerminateDestroyFn = unsafe extern "C" fn(MpvHandle);

// ─────────────────────────────────────────────────────────────────────────────
// libmpv ライブラリラッパー
// ─────────────────────────────────────────────────────────────────────────────

/// 動的ロードした libmpv ライブラリと関数ポインタを保持する構造体
struct MpvLib {
    _lib: Library,
    create: MpvCreateFn,
    initialize: MpvInitializeFn,
    command: MpvCommandFn,
    set_option_string: MpvSetOptionStringFn,
    terminate_destroy: MpvTerminateDestroyFn,
}

// libmpv はスレッドセーフなので Send/Sync を実装
unsafe impl Send for MpvLib {}
unsafe impl Sync for MpvLib {}

impl MpvLib {
    /// ライブラリから指定シンボルを安全にロードするヘルパー
    unsafe fn load_sym<T: Copy>(lib: &Library, name: &'static [u8]) -> Result<T, String> {
        lib.get::<T>(name).map(|sym| *sym).map_err(|e| {
            let name_str = std::str::from_utf8(name).unwrap_or("symbol");
            format!("Failed to load {}: {}", name_str, e)
        })
    }

    /// DLL のロード先ディレクトリを Windows に通知する（依存 DLL の検索パスとして使用）
    #[cfg(windows)]
    fn set_dll_directory(dir: &Path) {
        use std::os::windows::ffi::OsStrExt;

        let wide: Vec<u16> = dir
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let _ = unsafe {
            windows::Win32::System::LibraryLoader::SetDllDirectoryW(windows::core::PCWSTR(
                wide.as_ptr(),
            ))
        };
    }

    /// libmpv-2.dll を複数のパスから順番に探してロードする。
    ///
    /// 検索順序:
    ///   1. `<resource_dir>/bin/` (リリースバンドル)
    ///   2. `<resource_dir>/`
    ///   3. `<exe_dir>/bin/`
    ///   4. `<exe_dir>/resources/bin/`
    ///   5. カレントディレクトリ（システムパス）
    fn load(app: &AppHandle) -> Result<Self, String> {
        const MPV_DLL_NAME: &str = "libmpv-2.dll";

        // release bundle では resources/bin に配置されるため、resource_dir を優先する
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(resource_dir) = app.path().resource_dir() {
            candidates.push(resource_dir.join("bin").join(MPV_DLL_NAME));
            candidates.push(resource_dir.join(MPV_DLL_NAME));
        }

        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                candidates.push(exe_dir.join("bin").join(MPV_DLL_NAME));
                candidates.push(exe_dir.join("resources").join("bin").join(MPV_DLL_NAME));
            }
        }

        candidates.push(PathBuf::from(MPV_DLL_NAME));

        let mut tried: Vec<String> = Vec::new();
        let mut last_err: Option<String> = None;

        for dll_path in candidates {
            let path_str = dll_path.display().to_string();
            if tried.iter().any(|p| p == &path_str) {
                continue;
            }
            tried.push(path_str);

            #[cfg(windows)]
            if let Some(dir) = dll_path.parent() {
                Self::set_dll_directory(dir);
            }

            let lib = match unsafe { Library::new(&dll_path) } {
                Ok(lib) => lib,
                Err(err) => {
                    last_err = Some(err.to_string());
                    continue;
                }
            };

            let create = unsafe { Self::load_sym::<MpvCreateFn>(&lib, b"mpv_create")? };
            let initialize = unsafe { Self::load_sym::<MpvInitializeFn>(&lib, b"mpv_initialize")? };
            let command = unsafe { Self::load_sym::<MpvCommandFn>(&lib, b"mpv_command")? };
            let set_option_string =
                unsafe { Self::load_sym::<MpvSetOptionStringFn>(&lib, b"mpv_set_option_string")? };
            let terminate_destroy =
                unsafe { Self::load_sym::<MpvTerminateDestroyFn>(&lib, b"mpv_terminate_destroy")? };

            return Ok(Self {
                _lib: lib,
                create,
                initialize,
                command,
                set_option_string,
                terminate_destroy,
            });
        }

        let last_err = last_err.unwrap_or_else(|| "Unknown error".to_string());
        Err(format!(
            "Failed to load {MPV_DLL_NAME}. Tried: {}. Last error: {}. Please ensure mpv DLLs are in src-tauri/bin (dev) or bundled resources/bin (release).",
            tried.join(", "),
            last_err
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MPV プレイヤーインスタンス
// ─────────────────────────────────────────────────────────────────────────────

/// 単一の MPV プレイヤーインスタンス（ハンドルと共有ライブラリ参照を保持）
struct MpvPlayer {
    lib: Arc<MpvLib>,
    handle: MpvHandle,
}

// libmpv ハンドルはスレッドセーフ
unsafe impl Send for MpvPlayer {}

impl MpvPlayer {
    /// 新しい MPV インスタンスを生成する
    fn new(lib: Arc<MpvLib>) -> Result<Self, String> {
        unsafe {
            let handle = (lib.create)();
            if handle.is_null() {
                return Err("Failed to create mpv handle".to_string());
            }

            Ok(Self { lib, handle })
        }
    }

    /// MPV を初期化する（オプション設定後、loadfile 前に呼ぶ必要がある）
    fn initialize(&self) -> Result<(), String> {
        unsafe {
            let ret = (self.lib.initialize)(self.handle);
            if ret < 0 {
                return Err(format!("Failed to initialize mpv: error code {}", ret));
            }
            Ok(())
        }
    }

    /// 文字列オプションを設定する（initialize 前に呼ぶ必要があるものが多い）
    fn set_option(&self, name: &str, value: &str) -> Result<(), String> {
        unsafe {
            let name_c = CString::new(name).map_err(|e| e.to_string())?;
            let value_c = CString::new(value).map_err(|e| e.to_string())?;

            let ret = (self.lib.set_option_string)(self.handle, name_c.as_ptr(), value_c.as_ptr());
            if ret < 0 {
                return Err(format!("Failed to set option {}: error code {}", name, ret));
            }
            Ok(())
        }
    }

    /// MPV が作成したウィンドウの HWND を取得する（Windows のみ）。
    /// ウィンドウのクラス名またはタイトルに "mpv" が含まれるものを探す。
    fn get_window_handle(&self) -> Option<isize> {
        #[cfg(windows)]
        {
            use windows::core::BOOL;
            use windows::Win32::Foundation::{HWND, LPARAM};
            use windows::Win32::UI::WindowsAndMessaging::{
                EnumWindows, GetClassNameW, GetWindowTextW,
            };

            unsafe {
                static mut FOUND_HWND: Option<isize> = None;
                FOUND_HWND = None;

                unsafe extern "system" fn enum_callback(hwnd: HWND, _lparam: LPARAM) -> BOOL {
                    let mut title: [u16; 512] = [0; 512];
                    let mut class: [u16; 512] = [0; 512];

                    let _ = GetWindowTextW(hwnd, &mut title);
                    let _ = GetClassNameW(hwnd, &mut class);

                    let title_str = String::from_utf16_lossy(&title);
                    let class_str = String::from_utf16_lossy(&class);

                    if class_str.contains("mpv") || title_str.contains("MPV-Tauri-Player") {
                        FOUND_HWND = Some(hwnd.0 as isize);
                        return BOOL(0); // 列挙を停止
                    }

                    BOOL(1) // 列挙を継続
                }

                let _ = EnumWindows(Some(enum_callback), LPARAM(0));
                FOUND_HWND
            }
        }

        #[cfg(not(windows))]
        {
            None
        }
    }

    /// MPV にコマンドを送信する（"loadfile", "stop", "cycle pause" など）
    fn command(&self, args: &[&str]) -> Result<(), String> {
        unsafe {
            let c_args: Result<Vec<CString>, _> = args.iter().map(|s| CString::new(*s)).collect();
            let c_args = c_args.map_err(|e| e.to_string())?;

            let mut ptrs: Vec<*const c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
            ptrs.push(ptr::null()); // NULL 終端

            let ret = (self.lib.command)(self.handle, ptrs.as_ptr());
            if ret < 0 {
                return Err(format!("Failed to execute command: error code {}", ret));
            }
            Ok(())
        }
    }
}

impl Drop for MpvPlayer {
    /// ドロップ時に MPV インスタンスを正常終了させる
    fn drop(&mut self) {
        unsafe {
            (self.lib.terminate_destroy)(self.handle);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// プレイヤー状態
// ─────────────────────────────────────────────────────────────────────────────

/// MPV プレイヤーマネージャー（Tauri の管理状態として登録される）
#[derive(Default)]
pub struct MpvPlayerManager {
    state: Arc<Mutex<MpvPlayerState>>,
    /// ライブラリは遅延ロード（初回 start_mpv 時にロード）
    lib: Arc<Mutex<Option<Arc<MpvLib>>>>,
}

/// MPV プレイヤーの内部状態
struct MpvPlayerState {
    player: Option<MpvPlayer>,
    current_url: Option<String>,
    is_playing: bool,
    is_paused: bool,
    autoplay_blocked: bool,
    /// 埋め込みホストウィンドウの HWND
    window_handle: Option<isize>,
    /// WebView への埋め込みモードかどうか
    embedded: bool,
    /// 埋め込み先の Tauri ウィンドウラベル
    window_label: Option<String>,
}

/// 埋め込みモード時にのみ HWND を取り出す（通常モードでは None を返す）
fn take_embed_hwnd(state: &mut MpvPlayerState) -> Option<isize> {
    if state.embedded {
        return state.window_handle.take();
    }
    state.window_handle = None;
    None
}

impl Default for MpvPlayerState {
    fn default() -> Self {
        Self {
            player: None,
            current_url: None,
            is_playing: false,
            is_paused: false,
            autoplay_blocked: false,
            window_handle: None,
            embedded: false,
            window_label: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Win32 ウィンドウ操作（Windows 専用）
// ─────────────────────────────────────────────────────────────────────────────

/// WebView ウィンドウ内の Chrome レンダラー子ウィンドウの HWND を取得する。
/// 見つからない場合はメインウィンドウの HWND にフォールバックする。
#[cfg(windows)]
fn get_webview_hwnd(
    app: &AppHandle,
    window_label: &str,
) -> Result<windows::Win32::Foundation::HWND, String> {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetClassNameW};

    let main = get_window_hwnd(app, window_label)?;

    unsafe {
        use windows::core::BOOL;

        static mut FOUND: isize = 0;

        FOUND = 0;

        unsafe extern "system" fn cb(hwnd: HWND, _lparam: LPARAM) -> BOOL {
            let mut class: [u16; 256] = [0; 256];
            let len = GetClassNameW(hwnd, &mut class);
            if len == 0 {
                return BOOL(1);
            }

            let class_str = String::from_utf16_lossy(&class[..len as usize]);

            // ここは必要ならログ出して自分の環境のクラス名に合わせて調整してな
            // eprintln!("child class={}", class_str);

            if class_str.contains("Chrome_WidgetWin_0")
                || class_str.contains("Chrome_RenderWidgetHostHWND")
            {
                FOUND = hwnd.0 as isize;
                return BOOL(0); // 列挙停止
            }

            BOOL(1) // 列挙継続
        }

        let _ = EnumChildWindows(Some(main), Some(cb), LPARAM(0));

        if FOUND == 0 {
            // WebView 子ウィンドウが見つからない場合はメインウィンドウを使用
            return Ok(main);
        }

        Ok(HWND(FOUND as *mut _))
    }
}

/// Tauri ウィンドウラベルから Win32 HWND を取得する
#[cfg(windows)]
fn get_window_hwnd(
    app: &AppHandle,
    window_label: &str,
) -> Result<windows::Win32::Foundation::HWND, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;

    let main_window = app
        .get_webview_window(window_label)
        .ok_or_else(|| format!("Window not found: {}", window_label))?;

    let handle = main_window
        .window_handle()
        .map_err(|e| format!("Failed to get window handle: {}", e))?;

    match handle.as_ref() {
        RawWindowHandle::Win32(win32_handle) => Ok(HWND(win32_handle.hwnd.get() as *mut _)),
        _ => Err("Not a Win32 window".to_string()),
    }
}

/// MPV 埋め込み用のホストウィンドウ（子ウィンドウ）を作成する。
/// メインスレッドで実行する必要があるため、run_on_main_thread_with_result を使用する。
#[cfg(windows)]
fn create_embed_host_window(app: &AppHandle, window_label: &str) -> Result<isize, String> {
    let app = app.clone();
    let label = window_label.to_string();
    run_on_main_thread_with_result(&app.clone(), move || {
        create_embed_host_window_inner(&app, &label)
    })
}

/// 埋め込みホストウィンドウを実際に作成する内部関数（メインスレッド上で実行）
#[cfg(windows)]
fn create_embed_host_window_inner(app: &AppHandle, window_label: &str) -> Result<isize, String> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_NOACTIVATE,
        WS_EX_NOPARENTNOTIFY, WS_VISIBLE,
    };

    let parent_hwnd = get_webview_hwnd(app, window_label)?;
    let hinstance = unsafe { GetModuleHandleW(PCWSTR::null()) }.unwrap_or_default();

    // WS_CHILD: 親ウィンドウの子として作成（親が閉じると自動で閉じる）
    // WS_CLIPSIBLINGS/WS_CLIPCHILDREN: 描画の重なりを防ぐ
    // WS_EX_NOACTIVATE: フォーカスを奪わない
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_NOPARENTNOTIFY | WS_EX_NOACTIVATE,
            w!("STATIC"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
            0,
            0,
            1,
            1,
            Some(parent_hwnd),
            None,
            Some(HINSTANCE(hinstance.0)),
            None,
        )
    }
    .map_err(|e| format!("Failed to create mpv host window: {}", e))?;

    Ok(hwnd.0 as isize)
}

/// 埋め込みホストウィンドウを破棄する
#[cfg(windows)]
fn destroy_embed_host_window(app: &AppHandle, hwnd: isize) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;

    let app = app.clone();
    run_on_main_thread_with_result(&app, move || unsafe {
        DestroyWindow(HWND(hwnd as *mut _))
            .map_err(|e| format!("Failed to destroy mpv host window: {}", e))
    })
    .map(|_| ())
}

/// WebView にフォーカスを戻す（MPV 起動後に呼ぶと UI 操作性が向上する）
#[cfg(windows)]
fn focus_webview(app: &AppHandle, window_label: &str) {
    use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;

    if let Ok(hwnd) = get_webview_hwnd(app, window_label) {
        unsafe {
            let _ = SetFocus(Some(hwnd));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// プレイヤー情報・状態ユーティリティ
// ─────────────────────────────────────────────────────────────────────────────

/// フロントエンドに送信するプレイヤー状態（Tauri イベント payload）
#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerInfo {
    pub is_playing: bool,
    pub is_paused: bool,
    pub autoplay_blocked: bool,
    pub current_url: Option<String>,
}

impl PlayerInfo {
    fn from_state(state: &MpvPlayerState) -> Self {
        Self {
            is_playing: state.is_playing,
            is_paused: state.is_paused,
            autoplay_blocked: state.autoplay_blocked,
            current_url: state.current_url.clone(),
        }
    }
}

/// フロントエンドへプレイヤー状態を emit する（mpv://state イベント）
fn emit_player_state(app: &AppHandle, info: PlayerInfo) {
    if let Err(err) = app.emit("mpv://state", info) {
        eprintln!("Failed to emit mpv state: {}", err);
    }
}

/// MPV の yes/no/true/false/0/1 形式の文字列を bool に変換する
fn parse_mpv_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" | "on" => Some(true),
        "no" | "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

/// stop 理由が「ユーザーまたはウィンドウクローズ」の場合、自動再生をブロックすべきか判定する
fn should_block_autoplay(reason: Option<&str>) -> bool {
    matches!(reason, Some("user") | Some("window-close") | Some("close"))
}

/// MPV コマンドの実行内容を元にプレイヤー状態を更新する。
/// 状態が変化した場合は true を返す（イベント emit の判断に使用）。
fn apply_command_to_state(state: &mut MpvPlayerState, args: &[String]) -> bool {
    let cmd = args.get(0).map(|v| v.as_str()).unwrap_or_default();
    let mut changed = false;
    match cmd {
        "stop" | "quit" => {
            if state.is_playing || state.current_url.is_some() {
                state.is_playing = false;
                state.is_paused = false;
                state.current_url = None;
                changed = true;
            }
        }
        "loadfile" => {
            if let Some(url) = args.get(1) {
                if state.current_url.as_deref() != Some(url) || !state.is_playing {
                    state.current_url = Some(url.clone());
                    state.is_playing = true;
                    state.is_paused = false;
                    changed = true;
                }
            }
        }
        "cycle" => {
            // cycle pause: 一時停止の切り替え
            if matches!(args.get(1).map(|v| v.as_str()), Some("pause")) {
                state.is_paused = !state.is_paused;
                changed = true;
            }
        }
        "set" => {
            // set pause yes/no: 一時停止の明示的な設定
            if matches!(args.get(1).map(|v| v.as_str()), Some("pause")) {
                if let Some(value) = args.get(2).and_then(|v| parse_mpv_bool(v)) {
                    if state.is_paused != value {
                        state.is_paused = value;
                        changed = true;
                    }
                }
            }
        }
        _ => {}
    }
    changed
}

// ─────────────────────────────────────────────────────────────────────────────
// MpvPlayerManager 実装
// ─────────────────────────────────────────────────────────────────────────────

impl MpvPlayerManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// libmpv ライブラリを遅延ロードして返す。
    /// 既にロード済みの場合はキャッシュを返す。
    fn ensure_lib(&self, app: &AppHandle) -> Result<Arc<MpvLib>, String> {
        let mut lib_guard = self
            .lib
            .lock()
            .map_err(|_| "Failed to lock mpv library state".to_string())?;

        if lib_guard.is_none() {
            *lib_guard = Some(Arc::new(MpvLib::load(app)?));
        }

        lib_guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "Failed to initialize mpv library".to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// メインスレッド ディスパッチ
// Win32 の多くの API はメインスレッドでのみ呼び出せるため、
// 非同期タスクから呼ぶ際はこのユーティリティを経由する。
// ─────────────────────────────────────────────────────────────────────────────

/// アプリ起動時にメインスレッドの ID を記録する（is_main_thread の判定に使用）
static MAIN_THREAD_ID: OnceLock<ThreadId> = OnceLock::new();

pub fn init_main_thread_id() {
    let _ = MAIN_THREAD_ID.set(std::thread::current().id());
}

/// 現在のスレッドがメインスレッドかどうかを判定する
fn is_main_thread() -> bool {
    MAIN_THREAD_ID
        .get()
        .map(|id| *id == std::thread::current().id())
        .unwrap_or(false)
}

/// クロージャをメインスレッドで実行し、Result を返す。
/// 既にメインスレッドにいる場合は直接実行する。
fn run_on_main_thread_with_result<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    if is_main_thread() {
        return f();
    }
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(f());
    })
    .map_err(|e| format!("Failed to run on main thread: {}", e))?;
    rx.recv()
        .map_err(|_| "Failed to receive main thread result".to_string())?
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri コマンド
// ─────────────────────────────────────────────────────────────────────────────

/// MPV プレイヤーを起動して指定 URL の動画を再生する。
///
/// - embedded=true の場合は WebView ウィンドウに埋め込む（Windows 専用）
/// - embedded=false の場合はスタンドアロンウィンドウで再生する
/// - 既存のプレイヤーがある場合は停止してから新たに起動する
#[tauri::command]
pub async fn start_mpv(
    app: AppHandle,
    manager: tauri::State<'_, MpvPlayerManager>,
    url: String,
    embedded: Option<bool>,
    window_label: Option<String>,
    demuxer_format: Option<String>,
) -> Result<(), String> {
    let lib = manager.ensure_lib(&app)?;

    // 既存プレイヤーを停止して埋め込みウィンドウを破棄
    let old_hwnd = {
        let mut state = manager.state.lock().map_err(|e| e.to_string())?;
        let hwnd = take_embed_hwnd(&mut state);
        state.player = None;
        state.current_url = None;
        state.is_playing = false;
        state.embedded = false;
        state.window_label = None;
        hwnd
    };
    if let Some(hwnd) = old_hwnd {
        #[cfg(windows)]
        {
            let _ = destroy_embed_host_window(&app, hwnd);
        }
    }

    let embedded = embedded.unwrap_or(true);
    let target_window = window_label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .unwrap_or("main");

    // 新しいプレイヤーを生成
    let player = MpvPlayer::new(lib)?;
    let mut created_embed_hwnd: Option<isize> = None;

    // エラー時にプレイヤーと埋め込みウィンドウを確実にクリーンアップするマクロ
    macro_rules! cleanup_and_return {
        ($err:expr) => {{
            let err_msg = $err;
            drop(player);
            if let Some(hwnd) = created_embed_hwnd.take() {
                #[cfg(windows)]
                {
                    let _ = destroy_embed_host_window(&app, hwnd);
                }
            }
            return Err(err_msg);
        }};
    }

    // 映像出力バックエンドを選択する。
    // video-rotate が必要なため gpu を優先し、非対応の場合のみ direct3d にフォールバック。
    let mut selected_vo: Option<&str> = None;
    for candidate in ["gpu", "direct3d"] {
        if player.set_option("vo", candidate).is_ok() {
            selected_vo = Some(candidate);
            break;
        }
    }
    let selected_vo = selected_vo.ok_or_else(|| "Failed to configure mpv video output".to_string())?;
    eprintln!("mpv selected vo: {}", selected_vo);

    // 共通オプションを設定
    for (key, value) in [
        ("hwdec", "no"),         // ハードウェアデコード無効（互換性のため）
        ("keep-open", "yes"),    // 再生終了後もウィンドウを維持
        ("border", "no"),        // ウィンドウ枠を非表示
        ("msg-level", "all=v"),  // 詳細ログを有効化
    ] {
        player.set_option(key, value)?;
    }

    // ログファイルのパスを設定（exe と同階層、なければ一時ディレクトリ）
    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("mirrativ-mpv.log")))
        .unwrap_or_else(|| std::env::temp_dir().join("mirrativ-mpv.log"));
    let log_path_str = log_path.to_string_lossy().to_string();
    eprintln!("mpv log file: {}", log_path_str);
    player.set_option("log-file", &log_path_str)?;

    let demuxer_format = demuxer_format
        .as_deref()
        .map(str::trim)
        .filter(|fmt| !fmt.is_empty());

    if let Some(fmt) = demuxer_format {
        // Raw AnnexB over named pipe is extension-less, so force the demuxer format.
        player.set_option("demuxer-lavf-format", fmt)?;
        let _ = player.set_option("cache", "no");
        let _ = player.set_option("demuxer-readahead-secs", "0");
        let _ = player.set_option("untimed", "yes");
        eprintln!("mpv forced demuxer format: {}", fmt);
    }

    let mut window_handle = None;
    if embedded {
        #[cfg(windows)]
        {
            // WebView 内に埋め込むためのホストウィンドウを作成
            let host_hwnd = create_embed_host_window(&app, &target_window)?;
            created_embed_hwnd = Some(host_hwnd);
            if let Err(err) = player.set_option("wid", &format!("{}", host_hwnd)) {
                cleanup_and_return!(err);
            }
            // 埋め込み時はキーボード・マウス入力を MPV に渡さない
            for (key, value) in [
                ("input-default-bindings", "no"),
                ("input-vo-keyboard", "no"),
                ("input-cursor", "no"),
                ("ontop", "no"),
            ] {
                if let Err(err) = player.set_option(key, value) {
                    cleanup_and_return!(err);
                }
            }
            window_handle = Some(host_hwnd);
        }
        #[cfg(not(windows))]
        {
            return Err("Embedded mode is only supported on Windows".to_string());
        }
    } else {
        // スタンドアロンモード: 独立ウィンドウで常に最前面表示
        player.set_option("title", "MPV-Tauri-Player")?;
        player.set_option("ontop", "yes")?;
    }

    // プレイヤーを初期化して再生開始
    if let Err(err) = player.initialize() {
        cleanup_and_return!(err);
    }

    if let Err(err) = player.command(&["loadfile", &url]) {
        cleanup_and_return!(err);
    }

    if !embedded {
        // スタンドアロンモードではウィンドウ作成を待ってから HWND を取得
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let mut mpv_window_handle = player.get_window_handle();

        eprintln!("MPV window handle: {:?}", mpv_window_handle);
        if mpv_window_handle.is_none() {
            eprintln!("Warning: Failed to find MPV window, retrying...");
            // ウィンドウ作成に時間がかかる場合があるので数回リトライ
            for i in 1..=3 {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                mpv_window_handle = player.get_window_handle();
                eprintln!("Retry {} - MPV window handle: {:?}", i, mpv_window_handle);
                if mpv_window_handle.is_some() {
                    break;
                }
            }
        }
        window_handle = mpv_window_handle;
    }

    // 状態を更新してフロントエンドに通知
    let info = {
        let mut state = manager.state.lock().map_err(|e| e.to_string())?;
        state.player = Some(player);
        state.current_url = Some(url);
        state.is_playing = true;
        state.is_paused = false;
        state.autoplay_blocked = false;
        state.window_handle = window_handle;
        state.embedded = embedded;
        state.window_label = Some(target_window.to_string());
        PlayerInfo::from_state(&state)
    };
    emit_player_state(&app, info);

    #[cfg(windows)]
    {
        if embedded {
            // MPV 起動後に WebView にフォーカスを戻してキー操作を維持する
            focus_webview(&app, target_window);
        }
    }

    Ok(())
}

/// MPV プレイヤーを停止して状態をリセットする。
/// reason が "user" または "window-close" の場合は自動再生をブロックする。
#[tauri::command]
pub async fn stop_mpv(
    app: AppHandle,
    manager: tauri::State<'_, MpvPlayerManager>,
    reason: Option<String>,
) -> Result<(), String> {
    let (player, old_hwnd, info) = {
        let mut state = manager.state.lock().map_err(|e| e.to_string())?;
        let player = state.player.take();
        let hwnd = take_embed_hwnd(&mut state);
        state.current_url = None;
        state.is_playing = false;
        state.is_paused = false;
        if should_block_autoplay(reason.as_deref()) {
            state.autoplay_blocked = true;
        }
        state.embedded = false;
        state.window_label = None;
        let info = PlayerInfo::from_state(&state);
        (player, hwnd, info)
    };

    if let Some(player) = player {
        let _ = player.command(&["stop"]);
    }

    if let Some(hwnd) = old_hwnd {
        #[cfg(windows)]
        {
            let _ = destroy_embed_host_window(&app, hwnd);
        }
    }

    emit_player_state(&app, info);
    Ok(())
}

/// MPV にコマンドを送信する（volume, pause, video-rotate などの制御）。
/// 状態が変化した場合のみ mpv://state イベントを emit する。
#[tauri::command]
pub async fn mpv_command(
    app: AppHandle,
    manager: tauri::State<'_, MpvPlayerManager>,
    args: Vec<String>,
) -> Result<(), String> {
    if args.is_empty() {
        return Err("No command arguments provided".to_string());
    }

    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let (info, changed) = {
        let mut state = manager.state.lock().map_err(|e| e.to_string())?;
        let player = state
            .player
            .as_ref()
            .ok_or_else(|| "MPV player not initialized".to_string())?;

        player.command(&refs)?;

        let changed = apply_command_to_state(&mut state, &args);
        let info = PlayerInfo::from_state(&state);
        (info, changed)
    };

    if changed {
        emit_player_state(&app, info);
    }
    Ok(())
}

/// 現在のプレイヤー状態を取得する（フロントエンドの定期ポーリングで使用）
#[tauri::command]
pub async fn get_player_info(
    manager: tauri::State<'_, MpvPlayerManager>,
) -> Result<PlayerInfo, String> {
    let state = manager.state.lock().map_err(|e| e.to_string())?;

    Ok(PlayerInfo::from_state(&state))
}

/// プレイヤー用ウィンドウを作成する。既に存在する場合は前面に出す。
#[tauri::command]
pub async fn create_player_window(app: AppHandle) -> Result<String, String> {
    let window = if let Some(window) = app.get_webview_window("player") {
        window
    } else {
        WebviewWindowBuilder::new(&app, "player", WebviewUrl::App("player".into()))
            .title("Player")
            .inner_size(960.0, 540.0)
            .min_inner_size(480.0, 270.0)
            .resizable(true)
            .build()
            .map_err(|e| format!("Failed to create player window: {}", e))?
    };

    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;

    Ok("player".to_string())
}

/// プレイヤーを停止してウィンドウを閉じる
#[tauri::command]
pub async fn close_player_window(
    app: AppHandle,
    manager: tauri::State<'_, MpvPlayerManager>,
) -> Result<(), String> {
    let _ = stop_mpv(app.clone(), manager, Some("window-close".to_string())).await;
    if let Some(window) = app.get_webview_window("player") {
        let _ = window.close();
    }
    Ok(())
}

/// MPV ウィンドウの位置とサイズを変更する（埋め込みモード専用）。
/// 埋め込みモードの場合は親ウィンドウ相対座標、スタンドアロンの場合は画面絶対座標で指定する。
#[tauri::command]
pub async fn position_mpv_window(
    app: AppHandle,
    manager: tauri::State<'_, MpvPlayerManager>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    let (mpv_hwnd, embedded, anchor_label) = {
        let state = manager.state.lock().map_err(|e| e.to_string())?;
        let hwnd = state
            .window_handle
            .ok_or_else(|| "MPV window not found".to_string())?;
        (hwnd, state.embedded, state.window_label.clone())
    };

    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{HWND, POINT};
        use windows::Win32::Graphics::Gdi::ClientToScreen;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE};

        run_on_main_thread_with_result(&app.clone(), move || unsafe {
            let mpv_hwnd = HWND(mpv_hwnd as *mut _);

            if embedded {
                // 埋め込みモード: 親ウィンドウ内のクライアント座標で配置
                if SetWindowPos(mpv_hwnd, None, x, y, width, height, SWP_NOACTIVATE).is_err() {
                    return Err("Failed to position embedded mpv window".to_string());
                }
            } else {
                // スタンドアロンモード: クライアント座標をスクリーン座標に変換して配置
                let anchor_label = anchor_label.as_deref().unwrap_or("main");
                let main_hwnd = get_window_hwnd(&app, anchor_label)?;

                let mut origin = POINT { x: 0, y: 0 };
                if !ClientToScreen(main_hwnd, &mut origin).as_bool() {
                    return Err("Failed to convert client coords to screen".to_string());
                }

                let abs_x = origin.x + x;
                let abs_y = origin.y + y;

                // mpv ウィンドウを最前面に配置
                if SetWindowPos(
                    mpv_hwnd,
                    Some(HWND_TOPMOST),
                    abs_x,
                    abs_y,
                    width,
                    height,
                    SWP_NOACTIVATE,
                )
                .is_err()
                {
                    return Err("Failed to position mpv window".to_string());
                }
            }
            Ok(())
        })?;
    }

    #[cfg(not(windows))]
    {
        return Err("Only supported on Windows".to_string());
    }

    Ok(())
}
