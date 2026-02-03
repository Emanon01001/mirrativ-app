use libloading::Library;
use serde::{Deserialize, Serialize};
use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::ThreadId;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

// libmpv FFI types
type MpvHandle = *mut c_void;

// libmpv FFI function types
type MpvCreateFn = unsafe extern "C" fn() -> MpvHandle;
type MpvInitializeFn = unsafe extern "C" fn(MpvHandle) -> c_int;
type MpvCommandFn = unsafe extern "C" fn(MpvHandle, *const *const c_char) -> c_int;
type MpvSetOptionStringFn = unsafe extern "C" fn(MpvHandle, *const c_char, *const c_char) -> c_int;
type MpvTerminateDestroyFn = unsafe extern "C" fn(MpvHandle);

struct MpvLib {
    _lib: Library,
    create: MpvCreateFn,
    initialize: MpvInitializeFn,
    command: MpvCommandFn,
    set_option_string: MpvSetOptionStringFn,
    terminate_destroy: MpvTerminateDestroyFn,
}

// libmpvはスレッドセーフなのでSend/Syncを実装
unsafe impl Send for MpvLib {}
unsafe impl Sync for MpvLib {}

impl MpvLib {
    unsafe fn load_sym<T: Copy>(lib: &Library, name: &'static [u8]) -> Result<T, String> {
        lib.get::<T>(name)
            .map(|sym| *sym)
            .map_err(|e| {
                let name_str = std::str::from_utf8(name).unwrap_or("symbol");
                format!("Failed to load {}: {}", name_str, e)
            })
    }

    fn load() -> Result<Self, String> {
        unsafe {
            // libmpv-2.dllをロード
            let lib = Library::new("libmpv-2.dll")
                .map_err(|e| format!("Failed to load libmpv-2.dll: {}. Please ensure libmpv-2.dll is in the same directory as the executable or in the system PATH.", e))?;

            let create = Self::load_sym::<MpvCreateFn>(&lib, b"mpv_create")?;
            let initialize = Self::load_sym::<MpvInitializeFn>(&lib, b"mpv_initialize")?;
            let command = Self::load_sym::<MpvCommandFn>(&lib, b"mpv_command")?;
            let set_option_string =
                Self::load_sym::<MpvSetOptionStringFn>(&lib, b"mpv_set_option_string")?;
            let terminate_destroy =
                Self::load_sym::<MpvTerminateDestroyFn>(&lib, b"mpv_terminate_destroy")?;

            Ok(Self {
                _lib: lib,
                create,
                initialize,
                command,
                set_option_string,
                terminate_destroy,
            })
        }
    }
}

struct MpvPlayer {
    lib: Arc<MpvLib>,
    handle: MpvHandle,
}

// libmpvハンドルはスレッドセーフ
unsafe impl Send for MpvPlayer {}

impl MpvPlayer {
    fn new(lib: Arc<MpvLib>) -> Result<Self, String> {
        unsafe {
            let handle = (lib.create)();
            if handle.is_null() {
                return Err("Failed to create mpv handle".to_string());
            }

            Ok(Self { lib, handle })
        }
    }

    fn initialize(&self) -> Result<(), String> {
        unsafe {
            let ret = (self.lib.initialize)(self.handle);
            if ret < 0 {
                return Err(format!("Failed to initialize mpv: error code {}", ret));
            }
            Ok(())
        }
    }

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
                        return BOOL(0);
                    }

                    BOOL(1)
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

    fn command(&self, args: &[&str]) -> Result<(), String> {
        unsafe {
            let c_args: Result<Vec<CString>, _> = args.iter().map(|s| CString::new(*s)).collect();
            let c_args = c_args.map_err(|e| e.to_string())?;

            let mut ptrs: Vec<*const c_char> = c_args.iter().map(|s| s.as_ptr()).collect();
            ptrs.push(ptr::null());

            let ret = (self.lib.command)(self.handle, ptrs.as_ptr());
            if ret < 0 {
                return Err(format!("Failed to execute command: error code {}", ret));
            }
            Ok(())
        }
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        unsafe {
            (self.lib.terminate_destroy)(self.handle);
        }
    }
}

#[derive(Default)]
pub struct MpvPlayerManager {
    state: Arc<Mutex<MpvPlayerState>>,
    lib: Arc<Mutex<Option<Arc<MpvLib>>>>,
}

struct MpvPlayerState {
    player: Option<MpvPlayer>,
    current_url: Option<String>,
    is_playing: bool,
    is_paused: bool,
    autoplay_blocked: bool,
    window_handle: Option<isize>,
    embedded: bool,
    window_label: Option<String>,
}

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
                return BOOL(0);
            }

            BOOL(1)
        }

        let _ = EnumChildWindows(Some(main), Some(cb), LPARAM(0));

        if FOUND == 0 {
            // Fallback to the main window if we can't locate the webview child.
            return Ok(main);
        }

        Ok(HWND(FOUND as *mut _))
    }
}

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

#[cfg(windows)]
fn create_embed_host_window(app: &AppHandle, window_label: &str) -> Result<isize, String> {
    let app = app.clone();
    let label = window_label.to_string();
    run_on_main_thread_with_result(&app.clone(), move || create_embed_host_window_inner(&app, &label))
}

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

#[cfg(windows)]
fn destroy_embed_host_window(app: &AppHandle, hwnd: isize) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;

    let app = app.clone();
    run_on_main_thread_with_result(&app, move || {
        unsafe {
            DestroyWindow(HWND(hwnd as *mut _))
                .map_err(|e| format!("Failed to destroy mpv host window: {}", e))
        }
    })
    .map(|_| ())
}

#[cfg(windows)]
fn focus_webview(app: &AppHandle, window_label: &str) {
    use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;

    if let Ok(hwnd) = get_webview_hwnd(app, window_label) {
        unsafe {
            let _ = SetFocus(Some(hwnd));
        }
    }
}

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

fn emit_player_state(app: &AppHandle, info: PlayerInfo) {
    if let Err(err) = app.emit("mpv://state", info) {
        eprintln!("Failed to emit mpv state: {}", err);
    }
}

fn parse_mpv_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" | "on" => Some(true),
        "no" | "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

fn should_block_autoplay(reason: Option<&str>) -> bool {
    matches!(reason, Some("user") | Some("window-close") | Some("close"))
}

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
            if matches!(args.get(1).map(|v| v.as_str()), Some("pause")) {
                state.is_paused = !state.is_paused;
                changed = true;
            }
        }
        "set" => {
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

impl MpvPlayerManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_lib(&self) -> Result<Arc<MpvLib>, String> {
        let mut lib_guard = self.lib.lock().unwrap();

        if lib_guard.is_none() {
            *lib_guard = Some(Arc::new(MpvLib::load()?));
        }

        Ok(lib_guard.as_ref().unwrap().clone())
    }
}

static MAIN_THREAD_ID: OnceLock<ThreadId> = OnceLock::new();

pub fn init_main_thread_id() {
    let _ = MAIN_THREAD_ID.set(std::thread::current().id());
}

fn is_main_thread() -> bool {
    MAIN_THREAD_ID
        .get()
        .map(|id| *id == std::thread::current().id())
        .unwrap_or(false)
}

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

#[tauri::command]
pub async fn start_mpv(
    app: AppHandle,
    manager: tauri::State<'_, MpvPlayerManager>,
    url: String,
    embedded: Option<bool>,
    window_label: Option<String>,
) -> Result<(), String> {
    let lib = manager.ensure_lib()?;
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

    // 新しいプレイヤーを作成
    let player = MpvPlayer::new(lib)?;

    // mpv設定
    for (key, value) in [
        ("vo", "direct3d"),
        ("hwdec", "no"),
        ("keep-open", "yes"),
        ("border", "no"),
        ("msg-level", "all=v"),
    ] {
        player.set_option(key, value)?;
    }

    let log_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("mirrativ-mpv.log")))
        .unwrap_or_else(|| std::env::temp_dir().join("mirrativ-mpv.log"));
    let log_path_str = log_path.to_string_lossy().to_string();
    eprintln!("mpv log file: {}", log_path_str);
    player.set_option("log-file", &log_path_str)?;

    let mut window_handle = None;
    if embedded {
        #[cfg(windows)]
        {
            let host_hwnd = create_embed_host_window(&app, &target_window)?;
            player.set_option("wid", &format!("{}", host_hwnd))?;
            for (key, value) in [
                ("input-default-bindings", "no"),
                ("input-vo-keyboard", "no"),
                ("input-cursor", "no"),
                ("ontop", "no"),
            ] {
                player.set_option(key, value)?;
            }
            window_handle = Some(host_hwnd);
        }
        #[cfg(not(windows))]
        {
            return Err("Embedded mode is only supported on Windows".to_string());
        }
    } else {
        player.set_option("title", "MPV-Tauri-Player")?;
        player.set_option("ontop", "yes")?;
    }

    // 初期化
    player.initialize()?;

    // URLを設定して再生開始
    player.command(&["loadfile", &url])?;

    if !embedded {
        // ウィンドウが作成されるまで少し待つ
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // ウィンドウハンドルを取得（リトライ付き）
        let mut mpv_window_handle = player.get_window_handle();

        eprintln!("MPV window handle: {:?}", mpv_window_handle);
        if mpv_window_handle.is_none() {
            eprintln!("Warning: Failed to find MPV window, retrying...");
            // もう少し待ってリトライ
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
            focus_webview(&app, target_window);
        }
    }

    Ok(())
}

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

#[tauri::command]
pub async fn get_player_info(
    manager: tauri::State<'_, MpvPlayerManager>,
) -> Result<PlayerInfo, String> {
    let state = manager.state.lock().map_err(|e| e.to_string())?;

    Ok(PlayerInfo::from_state(&state))
}

// 互換性のために残す
#[tauri::command]
pub async fn create_player_window(
    app: AppHandle,
) -> Result<String, String> {
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
                if SetWindowPos(mpv_hwnd, None, x, y, width, height, SWP_NOACTIVATE).is_err() {
                    return Err("Failed to position embedded mpv window".to_string());
                }
            } else {
                let anchor_label = anchor_label.as_deref().unwrap_or("main");
                let main_hwnd = get_window_hwnd(&app, anchor_label)?;

                let mut origin = POINT { x: 0, y: 0 };
                if !ClientToScreen(main_hwnd, &mut origin).as_bool() {
                    return Err("Failed to convert client coords to screen".to_string());
                }

                let abs_x = origin.x + x;
                let abs_y = origin.y + y;

                // mpvウィンドウを配置（常に最前面）
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
