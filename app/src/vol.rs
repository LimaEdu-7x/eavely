// Volume do sistema (endpoint de saida padrao). Fallback do slider de volume (DESIGN-NATIVO §4.1: "senao, volume do sistema").
use windows::Win32::{Media::Audio::{Endpoints::IAudioEndpointVolume, *}, System::Com::*};

unsafe fn ep() -> Option<IAudioEndpointVolume> {
    let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
    en.GetDefaultAudioEndpoint(eRender, eMultimedia).ok()?.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()
}
/// 0..1 (mudo = 0)
pub fn get() -> f32 { unsafe { ep().map_or(0.5, |e| if e.GetMute().map(|m| m.as_bool()).unwrap_or(false) { 0.0 } else { e.GetMasterVolumeLevelScalar().unwrap_or(0.5) }) } }
pub fn set(v: f32) { unsafe { if let Some(e) = ep() { let v = v.clamp(0.0, 1.0); let _ = e.SetMasterVolumeLevelScalar(v, std::ptr::null()); let _ = e.SetMute(v <= 0.001, std::ptr::null()); } } }

// ---- volume do APP de midia (sessao de audio do processo do player), nao do sistema ----
use windows::core::Interface;
/// chave do app a partir do AUMID do SMTC: "Spotify.exe" -> "spotify"; "SpotifyAB.SpotifyMusic_xxx!Spotify" -> "spotify"
fn app_key(aumid: &str) -> String { let a = aumid.rsplit('!').next().unwrap_or(aumid); a.trim_end_matches(".exe").rsplit('.').next().unwrap_or(a).to_ascii_lowercase() }
unsafe fn exe_stem(pid: u32) -> String {
    use windows::Win32::{Foundation::CloseHandle, System::Threading::*};
    let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
    let mut buf = [0u16; 520]; let mut n = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut n).is_ok(); let _ = CloseHandle(h);
    if !ok { return String::new(); }
    let p = String::from_utf16_lossy(&buf[..n as usize]); p.rsplit(['\\', '/']).next().unwrap_or("").trim_end_matches(".exe").to_ascii_lowercase()
}
/// sessoes de audio cujo processo corresponde ao app (varias para navegadores)
unsafe fn sessions(app: &str) -> Vec<ISimpleAudioVolume> {
    let key = app_key(app); let mut out = Vec::new();
    if key.is_empty() || key == "fake" { return out; }
    let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return out };
    let Ok(dev) = en.GetDefaultAudioEndpoint(eRender, eMultimedia) else { return out };
    let Ok(mgr) = dev.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { return out };
    let Ok(list) = mgr.GetSessionEnumerator() else { return out };
    for i in 0..list.GetCount().unwrap_or(0) {
        let Ok(c) = list.GetSession(i) else { continue };
        let Ok(c2) = c.cast::<IAudioSessionControl2>() else { continue };
        let stem = exe_stem(c2.GetProcessId().unwrap_or(0));
        if !stem.is_empty() && (stem.contains(&key) || key.contains(&stem)) { if let Ok(v) = c.cast::<ISimpleAudioVolume>() { out.push(v); } }
    }
    out
}
/// 0..1 (mudo = 0) do app; se o app nao tem sessao de audio, o do sistema
pub fn app_get(app: &str) -> f32 { unsafe { let s = sessions(app); if s.is_empty() { return get(); } s.iter().map(|v| if v.GetMute().map(|m| m.as_bool()).unwrap_or(false) { 0.0 } else { v.GetMasterVolume().unwrap_or(0.5) }).fold(0.0, f32::max) } }
pub fn app_set(app: &str, vol: f32) { unsafe { let s = sessions(app); if s.is_empty() { set(vol); return; } let v = vol.clamp(0.0, 1.0); for x in s { let _ = x.SetMasterVolume(v, std::ptr::null()); let _ = x.SetMute(v <= 0.001, std::ptr::null()); } } }
/// alterna mudo do app (ou do sistema); devolve o novo estado de mudo
pub fn app_toggle_mute(app: &str) -> bool { unsafe {
    let s = sessions(app);
    if s.is_empty() { return match ep() { Some(e) => { let m = !e.GetMute().map(|m| m.as_bool()).unwrap_or(false); let _ = e.SetMute(m, std::ptr::null()); m } None => false }; }
    let m = !s.iter().any(|v| v.GetMute().map(|m| m.as_bool()).unwrap_or(false));
    for x in s { let _ = x.SetMute(m, std::ptr::null()); } m
} }
