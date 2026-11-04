#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod gfx;
mod selftest;
mod metrics;
mod store;
mod vol;
mod island;
mod pages;
mod pages2;
mod tasks;
mod settings;
mod settings_ui;
mod overlay; mod netstat; mod layout; mod mascot; mod slots;
mod tray;
#[cfg(feature = "devtools")]
mod bench;
mod droptarget;
mod media;
mod lyrics;
mod timer;
mod shelf;
use windows::Win32::Foundation::HMODULE;

/// %APPDATA%\Eavely (cai no temp se APPDATA nao existir)
pub(crate) fn data_dir() -> std::path::PathBuf {
    // EAVELY_DATA_DIR: so para testes automatizados (nunca gravar lixo de teste no APPDATA real)
    if let Some(d) = std::env::var_os("EAVELY_DATA_DIR") { return d.into(); }
    std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Eavely")
}

/// Log PERSISTENTE (sempre ligado, pequeno, gira em 200 KB) de tudo que liga/desliga/esconde a ilha e de crashes: %APPDATA%\Eavely\state.log. Sem dados do usuario.
pub(crate) fn state_log(s: &str) {
    use std::io::Write;
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    let d = data_dir(); let _ = std::fs::create_dir_all(&d); let p = d.join("state.log");
    if std::fs::metadata(&p).is_ok_and(|m| m.len() > 200 * 1024) { let _ = std::fs::rename(&p, d.join("state.log.1")); }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) { let _ = writeln!(f, "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {s}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond); }
}
/// Marca a sessao em running.txt na pasta de dados (removida em toda saida registrada). Se ela ainda existe no proximo inicio, o processo anterior foi morto de fora
/// (taskkill /F, instalador, desligamento forcado, queda de energia) ou caiu sem poder escrever: fica registrado com a hora do inicio anterior.
fn start_marker() {
    let p = data_dir().join("running.txt");
    if let Ok(s) = std::fs::read_to_string(&p) { state_log(&format!("encerramento anterior SEM registro de saida ({}): processo morto de fora (taskkill/instalador/queda de energia) ou queda sem log", s.trim())); }
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    let _ = std::fs::write(&p, format!("v{} pid {} iniciado {:04}-{:02}-{:02} {:02}:{:02}:{:02} em {}", env!("CARGO_PKG_VERSION"), std::process::id(), t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, std::env::current_exe().ok().and_then(|e| e.parent().map(|d| if d.to_string_lossy().contains("Programs") { "instalado" } else { "pasta solta" }.to_string())).unwrap_or_default()));
    state_log(&format!("inicio v{} pid {}", env!("CARGO_PKG_VERSION"), std::process::id()));
}
/// Reinicia o app depois de um crash (no maximo 3 vezes em 2 min, para nunca virar laco). O novo processo espera o antigo sair (--restart).
fn relaunch(why: &str) {
    let d = data_dir(); let p = d.join("restart.stamp");
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|x| x.as_secs()).unwrap_or(0);
    let mut v: Vec<u64> = std::fs::read_to_string(&p).unwrap_or_default().lines().filter_map(|l| l.trim().parse().ok()).filter(|t| now.saturating_sub(*t) < 120).collect();
    if v.len() >= 3 { state_log("reinicio automatico SUPRIMIDO (3 em 2 min)"); return; }
    v.push(now); let _ = std::fs::write(&p, v.iter().map(|t| t.to_string()).collect::<Vec<_>>().join("\n"));
    state_log(&format!("reinicio automatico apos: {why}"));
    if let Ok(exe) = std::env::current_exe() { let _ = std::process::Command::new(exe).arg("--restart").spawn(); }
}
/// Saida imediata e limpa (sem destrutores/DLL detach, que disparavam uma excecao 0x87A no encerramento): usada em Sair, no autoteste e no bench.
pub(crate) static EXITING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub(crate) fn fast_exit(code: u32) -> ! { EXITING.store(true, std::sync::atomic::Ordering::SeqCst); let _ = std::fs::remove_file(data_dir().join("running.txt")); unsafe { let _ = windows::Win32::System::Threading::TerminateProcess(windows::Win32::System::Threading::GetCurrentProcess(), code); } std::process::exit(code as i32) }
unsafe extern "system" fn ueh(ep: *const windows::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS) -> i32 {
    let code = if ep.is_null() { 0 } else { (*(*ep).ExceptionRecord).ExceptionCode.0 as u32 };
    if EXITING.load(std::sync::atomic::Ordering::SeqCst) { return 0; } // encerrando de proposito: nunca reinicia
    state_log(&format!("CRASH: excecao nao tratada 0x{code:X}")); relaunch("excecao nao tratada"); 0
}
/// Log de diagnostico (so com EAVELY_LOG, ou apos panico/violacao de acesso): limitado a 512 KB (gira para .old) e sem titulo/artista/arquivos do usuario.
pub(crate) fn log(s: &str) {
    if std::env::var_os("EAVELY_LOG").is_none() { return; }
    use std::io::Write;
    let p = std::env::temp_dir().join("eavely-spike.log");
    if std::fs::metadata(&p).is_ok_and(|m| m.len() > 512 * 1024) { let _ = std::fs::rename(&p, p.with_extension("old")); }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) { let _ = writeln!(f, "{s}"); }
}

unsafe extern "system" fn veh(ep: *mut windows::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS) -> i32 {
    use windows::Win32::System::LibraryLoader::*;
    let r = &*(*ep).ExceptionRecord;
    if r.ExceptionCode.0 as u32 == 0xC0000005 {
        let mut m = HMODULE::default();
        let _ = GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, windows::core::PCWSTR(r.ExceptionAddress as *const u16), &mut m);
        let mut buf = [0u16; 260]; let n = GetModuleFileNameW(Some(m), &mut buf) as usize;
        std::env::set_var("EAVELY_LOG", "1");
        log(&format!("ACCESS VIOLATION em {:p} modulo {} (offset 0x{:X}) thread {:?}", r.ExceptionAddress, String::from_utf16_lossy(&buf[..n]), r.ExceptionAddress as usize - m.0 as usize, std::thread::current().name()));
    }
    0
}

fn main() {
    unsafe { windows::Win32::System::Diagnostics::Debug::AddVectoredExceptionHandler(1, Some(veh)); }
    unsafe { windows::Win32::System::Diagnostics::Debug::SetUnhandledExceptionFilter(Some(ueh)); }
    std::panic::set_hook(Box::new(|i| { std::env::set_var("EAVELY_LOG", "1"); log(&format!("PANIC: {i}")); state_log(&format!("CRASH: panic: {i}")); relaunch("panic"); }));
    if std::env::args().any(|a| a == "--restart") { std::thread::sleep(std::time::Duration::from_millis(1500)); }
    selftest::init();
    settings::get(); state_log(&format!("animacoes do Windows: {}; respeitar essa opcao: {}", if island::system_anim_off() { "DESLIGADAS" } else { "ligadas" }, settings::respect_anim()));
    // instancia unica (instalador, atalho e "iniciar com o Windows" nao duplicam o app)
    unsafe {
        use windows::{core::w, Win32::{Foundation::{GetLastError, ERROR_ALREADY_EXISTS}, System::Threading::CreateMutexW}};
        match CreateMutexW(None, true, w!("Local\\EavelyNotchSingleInstance")) {
            Ok(m) => { if GetLastError() == ERROR_ALREADY_EXISTS && !(cfg!(feature = "devtools") && std::env::var_os("EAVELY_MULTI").is_some()) { state_log(&format!("segunda instancia ignorada (pid {}, v{}): ja existe outra", std::process::id(), env!("CARGO_PKG_VERSION"))); return; } std::mem::forget(m); } // GetLastError logo apos a chamada
            Err(_) => {} // sem mutex (raro): segue sem a trava de instancia unica
        }
    }
    start_marker();
    pages::set_live_expanded(settings::get().live_mode == "expanded");
    if settings::get().shelf_start_empty { shelf::clear_native(); }
    timer::init();
    tasks::init();
    #[cfg(feature = "devtools")]
    let fake = std::env::var_os("EAVELY_FAKE_MEDIA").is_some(); // fake = demo/benchmark sem player real
    #[cfg(not(feature = "devtools"))]
    let fake = false;
    if fake { #[cfg(feature = "devtools")] media::fake(); } else { media::init(); }
    #[cfg(feature = "devtools")]
    if let Some(k) = std::env::var("EAVELY_UI_K").ok().and_then(|v| v.parse::<f32>().ok()) { layout::set_k(k); } // teste: preset de tamanho (0.6..1.5)
    let island_thread = island::start();
    #[cfg(feature = "devtools")]
    if std::env::var_os("EAVELY_TEST_VOLUME").is_some() { // teste interno: muda o volume do sistema em 2% e volta (dispara o callback real do overlay)
        std::thread::spawn(|| { std::thread::sleep(std::time::Duration::from_secs(4)); let v = vol::get(); vol::set(if v < 0.5 { v + 0.02 } else { v - 0.02 }); std::thread::sleep(std::time::Duration::from_secs(2)); vol::set(v); });
    }
    #[cfg(feature = "devtools")]
    if std::env::var_os("EAVELY_TEST_AUTOSTART").is_some() { // teste interno: liga e desliga "iniciar com o Windows" (sem entrada sintetica)
        std::thread::spawn(|| { std::thread::sleep(std::time::Duration::from_secs(2)); settings::update(|s| s.start_with_windows = true); std::thread::sleep(std::time::Duration::from_secs(4)); settings::update(|s| s.start_with_windows = false); });
    }
    // a ilha vive na sua thread (loop de mensagens proprio); o app encerra pelo menu da bandeja
    // se a thread da ilha acabar (panico ou fim do loop), o processo NAO fica parado e invisivel: registra o motivo e sai (o panico ja agendou o reinicio)
    if let Some(h) = island_thread {
        match h.join() { Ok(()) => { state_log("CRASH: a thread da ilha terminou sem erro (loop de mensagens encerrado)"); relaunch("thread da ilha terminou"); } Err(_) => state_log("CRASH: a thread da ilha terminou por panico") }
        fast_exit(1);
    }
    loop { std::thread::park(); }
}
