// Overlays de sistema (DESIGN-NATIVO §7): pilula 280x48 no topo. Cada fonte so e registrada quando o aviso esta LIGADO nas configuracoes.
//   Volume: IAudioEndpointVolume (callback)   Bateria: WM_POWERBROADCAST   Wi-Fi/rede: NotifyIpInterfaceChange + NLM
//   Fones: IMMNotificationClient (troca do dispositivo padrao)   Caps Lock: GetKeyState a cada 250 ms (so com o aviso ligado)   Brilho: NAO implementado (exige WMI/notebook)
use std::{sync::{Mutex, atomic::{AtomicBool, Ordering::Relaxed}}, time::{Duration, Instant}};
use windows::{
    core::{implement, Result as WR, PCWSTR},
    Win32::{Foundation::*, Media::Audio::{Endpoints::*, *}, NetworkManagement::IpHelper::*, Networking::NetworkListManager::*, Networking::WinSock::AF_UNSPEC,
        System::{Com::*, Power::*}, },
};
use crate::gfx::{rgba, Fmt, Rgba};

#[derive(Clone, Debug)]
pub struct Ov { pub icon: &'static str, pub title: String, pub sub: String, pub value: Option<f32>, pub color: Rgba, pub until: Instant }
static CUR: Mutex<Option<Ov>> = Mutex::new(None);
const HOLD: Duration = Duration::from_millis(1800);
fn lk() -> std::sync::MutexGuard<'static, Option<Ov>> { CUR.lock().unwrap_or_else(|e| e.into_inner()) }

/// Mostra/atualiza o overlay (o mais novo substitui; cada evento reinicia os 1,8 s).
pub fn show(icon: &'static str, title: &str, sub: &str, value: Option<f32>, color: Rgba) {
    crate::log(&format!("overlay: {title} {sub} {value:?}"));
    *lk() = Some(Ov { icon, title: title.into(), sub: sub.into(), value, color, until: Instant::now() + HOLD });
    crate::island::overlay_changed();
}
pub fn current() -> Option<Ov> { lk().clone() }
pub fn active() -> bool { lk().as_ref().is_some_and(|o| Instant::now() < o.until) }
pub fn ms_left() -> u32 { lk().as_ref().map_or(0, |o| o.until.saturating_duration_since(Instant::now()).as_millis() as u32) }

const ACCENT: Rgba = rgba(0x7CC4FF, 1.0);
const AMBER: Rgba = rgba(0xFFB86B, 1.0);
const GREEN: Rgba = rgba(0x7BE0A4, 1.0);
const RED: Rgba = rgba(0xFF6B6B, 1.0);
const VIOLET: Rgba = rgba(0xB69CFF, 1.0);
const TEXT3: Rgba = rgba(0xFFFFFF, 0.52);

// ---------- regras puras (testadas) ----------
/// (carregando, %) antes/depois -> o que avisar. Sem bateria (desktop) nunca chamado.
pub fn battery_decision(prev: (bool, u8), cur: (bool, u8)) -> Option<(&'static str, bool)> {
    // (titulo, critico)
    if cur.0 != prev.0 { return Some((if cur.0 { "Carregando" } else { "Na bateria" }, false)); }
    if !cur.0 && prev.1 > 10 && cur.1 <= 10 { return Some(("Bateria muito baixa", true)); }
    if !cur.0 && prev.1 > 20 && cur.1 <= 20 { return Some(("Bateria fraca", false)); }
    if cur.0 && prev.1 < 100 && cur.1 >= 100 { return Some(("Carga completa", false)); }
    None
}
pub fn looks_like_headphones(name: &str) -> bool {
    let n = name.to_lowercase();
    ["headphone", "headset", "fone", "auricular", "airpods", "buds", "earphone", "earbuds"].iter().any(|k| n.contains(k))
}

pub fn volume_event(vol: f32, muted: bool) {
    let v = if muted { 0.0 } else { vol };
    let icon = if v <= 0.001 { "volume-x" } else if v < 0.5 { "volume-1" } else { "volume-2" };
    show(icon, "Volume", "", Some(v), if muted { TEXT3 } else { ACCENT });
}

fn power() -> Option<(bool, u8)> {
    let mut s = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut s) }.is_err() || s.BatteryFlag & 128 != 0 || s.BatteryLifePercent > 100 { return None; } // 128 = sem bateria
    Some((s.ACLineStatus == 1, s.BatteryLifePercent))
}
static LAST_POWER: Mutex<Option<(bool, u8)>> = Mutex::new(None);
/// WM_POWERBROADCAST / PBT_APMPOWERSTATUSCHANGE
pub fn power_event() {
    let Some(cur) = power() else { crate::state_log("energia: evento recebido, mas sem bateria neste PC"); return };
    crate::state_log(&format!("energia: evento (tomada={}, {}%)", cur.0, cur.1));
    let prev = LAST_POWER.lock().unwrap_or_else(|e| e.into_inner()).replace(cur).unwrap_or(cur);
    if !crate::settings::get().overlays.battery { return; }
    if let Some((title, crit)) = battery_decision(prev, cur) {
        let (icon, col) = if crit || (!cur.0 && cur.1 <= 20) { ("battery-low", if cur.1 <= 10 { RED } else { AMBER }) } else if cur.0 { ("battery-charging", GREEN) } else { ("battery", TEXT3) };
        let sub = if !cur.0 && cur.1 <= 20 { format!("{}% · Conecte o carregador", cur.1) } else { format!("{}%", cur.1) };
        show(icon, title, &sub, None, col);
    }
}

/// Monitor interno (notebook)? DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL em algum caminho ativo. Cacheado.
pub fn has_internal_panel() -> bool {
    use windows::Win32::Devices::Display::*;
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| unsafe {
        let (mut np, mut nm) = (0u32, 0u32);
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != WIN32_ERROR(0) { return false; }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize]; let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
        if QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), None) != WIN32_ERROR(0) { return false; }
        paths.iter().take(np as usize).any(|p| p.targetInfo.outputTechnology == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL)
    })
}
const G_BRIGHT: windows::core::GUID = windows::core::GUID::from_u128(0xaded5e82_b909_4619_9949_f5d71dac0bcb); // GUID_DEVICE_POWER_POLICY_VIDEO_BRIGHTNESS
const G_ACDC: windows::core::GUID = windows::core::GUID::from_u128(0x5d3e9a59_e9d5_4b00_a6bd_ff34ff516548); // GUID_ACDC_POWER_SOURCE
/// Pede ao Windows os eventos de energia (fonte AC/DC e brilho) na janela da ilha: evento puro, nada de polling.
/// As alcas (HPOWERNOTIFY) nao sao guardadas de proposito: o app sai por TerminateProcess (fast_exit) e o Windows libera o registro com o processo.
pub fn register_power(h: HWND) { unsafe {
    use windows::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_WINDOW_HANDLE;
    for (g, n) in [(G_ACDC, "fonte AC/DC"), (G_BRIGHT, "brilho")] {
        let r = RegisterPowerSettingNotification(HANDLE(h.0), &g, DEVICE_NOTIFY_WINDOW_HANDLE);
        crate::state_log(&format!("energia: registro de evento {n} = {}", r.is_ok()));
    }
} }
static LAST_BR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);
static BR_LOGS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static LAST_ACDC: Mutex<Option<Instant>> = Mutex::new(None);
/// WM_POWERBROADCAST / PBT_POWERSETTINGCHANGE (lparam = POWERBROADCAST_SETTING)
pub fn power_setting(p: *const core::ffi::c_void) { unsafe {
    let Some(s) = (p as *const POWERBROADCAST_SETTING).as_ref() else { return };
    if s.DataLength < 4 { return; }
    let v = std::ptr::read_unaligned(s.Data.as_ptr() as *const u32);
    if s.PowerSetting == G_ACDC { *LAST_ACDC.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now()); crate::state_log(&format!("energia: fonte mudou = {v} (0 tomada, 1 bateria)")); }
    else if s.PowerSetting == G_BRIGHT { brightness_value(v.min(100), "evento de energia", false); }
} }
/// Valor de brilho novo (de qualquer fonte). `real`=true: o evento SEMPRE e uma mudanca (WMI); false: o primeiro valor e so o estado inicial.
fn brightness_value(v: u32, via: &str, real: bool) {
    let prev = LAST_BR.swap(v, Relaxed);
    let n = BR_LOGS.fetch_add(1, Relaxed);
    if n < 4 { crate::state_log(&format!("brilho: {v}% via {via}{}", if prev == u32::MAX && !real { " (estado inicial, sem aviso)" } else { "" })); }
    if (prev == u32::MAX && !real) || prev == v { return; } // inicial, ou a outra fonte ja avisou este valor
    if LAST_ACDC.lock().unwrap_or_else(|e| e.into_inner()).is_some_and(|t| t.elapsed() < Duration::from_secs(2)) { return; } // trocar tomada/bateria muda o brilho sozinho: sem aviso
    if !crate::settings::get().overlays.brightness || !has_internal_panel() { return; }
    show("sun", "Brilho", "", Some(v as f32 / 100.0), AMBER);
}
static BR_EN: AtomicBool = AtomicBool::new(false);
static BR_THREAD: AtomicBool = AtomicBool::new(false);
/// Brilho: WmiMonitorBrightnessEvent (ROOT\\WMI), evento puro; so notebook; so com o aviso ligado. NAO testado em hardware (este PC e desktop).
fn brightness(on: bool) {
    BR_EN.store(on && has_internal_panel(), Relaxed);
    crate::state_log(&format!("brilho: aviso {}; painel interno: {}; fontes: evento de energia + WMI (monitor externo/DDC nao suportado)", if on { "ligado" } else { "desligado" }, if has_internal_panel() { "sim" } else { "NAO (sem painel interno o aviso nao e usado)" }));
    if !BR_EN.load(Relaxed) || BR_THREAD.swap(true, Relaxed) { return; }
    std::thread::Builder::new().name("wmi-brilho".into()).spawn(|| unsafe {
        use windows::{core::BSTR, Win32::System::{Variant::*, Wmi::*}};
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let run = || -> WR<()> {
            let loc: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;
            let svc = loc.ConnectServer(&BSTR::from("ROOT\\WMI"), &BSTR::new(), &BSTR::new(), &BSTR::new(), 0, &BSTR::new(), None)?;
            let en = svc.ExecNotificationQuery(&BSTR::from("WQL"), &BSTR::from("SELECT * FROM WmiMonitorBrightnessEvent"), WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY, None)?;
            loop {
                let mut objs: [Option<IWbemClassObject>; 1] = [None]; let mut n = 0u32;
                en.Next(WBEM_INFINITE, &mut objs, &mut n).ok()?; // erro (servico WMI reiniciado): sai e o laco externo recria a consulta com espera
                if n == 0 { return Err(windows::core::Error::from(E_FAIL)); }
                if let Some(o) = objs[0].take() { let mut v = VARIANT::default();
                    if o.Get(windows::core::w!("Brightness"), 0, &mut v, None, None).is_ok() {
                        // VT_UI1 = 17: so le bVal se o tipo for esse; sempre limpa o VARIANT
                        if v.Anonymous.Anonymous.vt == VARENUM(17) && BR_EN.load(Relaxed) { brightness_value((v.Anonymous.Anonymous.Anonymous.bVal as u32).min(100), "WMI", true); }
                        let _ = VariantClear(&mut v);
                    } }
            }
        };
        // espera crescente (1 s .. 30 s) entre tentativas: nunca gira sem pausa
        let mut wait = 1u64;
        loop { if let Err(e) = run() { crate::log(&format!("wmi brilho: {e}; nova tentativa em {wait}s")); if wait == 1 { crate::state_log(&format!("brilho: WMI indisponivel ({e}); segue so o evento de energia")); } } std::thread::sleep(Duration::from_secs(wait)); wait = (wait * 2).min(30); }
    }).ok();
}

pub fn caps_event(on: bool) { show("arrow-big-up", if on { "Caps Lock ativado" } else { "Caps Lock desativado" }, "", None, if on { rgba(0xFFFFFF, 1.0) } else { TEXT3 }); }

// ---------- fontes (registro sob demanda) ----------
struct SendBox<T>(T); unsafe impl<T> Send for SendBox<T> {}
static VOL_REG: Mutex<Option<SendBox<(IAudioEndpointVolume, IAudioEndpointVolumeCallback)>>> = Mutex::new(None);
static NOTIF_REG: Mutex<Option<SendBox<(IMMDeviceEnumerator, IMMNotificationClient)>>> = Mutex::new(None);
static IPCHG: Mutex<Option<usize>> = Mutex::new(None); // HANDLE de NotifyIpInterfaceChange
static NET_UP: AtomicBool = AtomicBool::new(true);
static PHONES: AtomicBool = AtomicBool::new(false);

#[implement(IAudioEndpointVolumeCallback)]
struct VolCb;
impl IAudioEndpointVolumeCallback_Impl for VolCb_Impl {
    fn OnNotify(&self, p: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> WR<()> { unsafe { if let Some(d) = p.as_ref() { volume_event(d.fMasterVolume, d.bMuted.as_bool()); } } Ok(()) }
}

fn device_name(en: &IMMDeviceEnumerator, id: &PCWSTR) -> String { unsafe {
    let Ok(dev) = en.GetDevice(*id) else { return String::new() };
    let Ok(ps) = dev.OpenPropertyStore(STGM_READ) else { return String::new() };
    // PKEY_Device_FriendlyName {a45c254e-df1c-4efd-8020-67d146a850e0}, 14
    let key = PROPERTYKEY { fmtid: windows::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0), pid: 14 };
    ps.GetValue(&key).ok().map(|v| v.to_string()).unwrap_or_default()
} }

#[implement(IMMNotificationClient)]
struct DevCb;
impl IMMNotificationClient_Impl for DevCb_Impl {
    fn OnDeviceStateChanged(&self, _: &PCWSTR, _: DEVICE_STATE) -> WR<()> { Ok(()) }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> WR<()> { Ok(()) }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> WR<()> { Ok(()) }
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, _role: ERole, id: &PCWSTR) -> WR<()> {
        if flow != eRender { return Ok(()); }
        let cfg = crate::settings::get().overlays;
        if cfg.volume { std::thread::spawn(|| unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); volume_apply(true); }); } // volume segue o NOVO dispositivo padrao (fora do callback: evita deadlock)
        if !cfg.headphones { return Ok(()); }
        let name = unsafe { CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL).map(|en| device_name(&en, id)).unwrap_or_default() };
        let ph = looks_like_headphones(&name); let was = PHONES.swap(ph, Relaxed);
        if ph { show("headphones", "Fones conectados", &name, None, VIOLET); } else if was { show("headphones", "Fones desconectados", &name, None, TEXT3); }
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> WR<()> { Ok(()) }
}

unsafe extern "system" fn ip_changed(_: *const core::ffi::c_void, _: *const MIB_IPINTERFACE_ROW, _: MIB_NOTIFICATION_TYPE) { crate::island::net_changed(); } // so avisa a ilha (debounce la)

/// Roda na thread da ilha (STA com OLE), 500 ms depois da ultima mudanca de rede.
pub fn net_check() {
    if !crate::pages::has_battery() && crate::netstat::refresh() { crate::island::status_changed(); }
    if !crate::settings::get().overlays.wifi { return; }
    let up = unsafe { CoCreateInstance::<_, INetworkListManager>(&NetworkListManager, None, CLSCTX_ALL).and_then(|m| m.IsConnectedToInternet()).map(|b| b.as_bool()).unwrap_or(true) };
    if NET_UP.swap(up, Relaxed) != up { show(if up { "wifi" } else { "wifi-off" }, if up { "Conectado" } else { "Sem conexão" }, "", None, if up { ACCENT } else { RED }); }
}

/// (Re)registra o callback de volume no endpoint PADRAO atual; `on=false` so remove. Chamado de uma thread MTA propria (nunca de dentro de um callback).
unsafe fn volume_apply(on: bool) {
    let mut g = VOL_REG.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(SendBox((ep, cb))) = g.take() { let _ = ep.UnregisterControlChangeNotify(&cb); }
    if !on { return; }
    if let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) {
        if let Ok(ep) = en.GetDefaultAudioEndpoint(eRender, eMultimedia).and_then(|d| d.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)) {
            let cb: IAudioEndpointVolumeCallback = VolCb.into();
            crate::log(&format!("overlay volume: RegisterControlChangeNotify {:?}", ep.RegisterControlChangeNotify(&cb).is_ok())); *g = Some(SendBox((ep, cb)));
        }
    }
}

/// Liga/desliga cada fonte conforme settings.overlays (chamado no inicio e a cada mudanca de configuracao).
pub fn reconfigure() {
    let o = crate::settings::get().overlays;
    std::thread::Builder::new().name("overlay-cfg".into()).spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        // volume: so registra se ainda nao ha registro (a troca de dispositivo re-registra pelo DevCb)
        if o.volume { if VOL_REG.lock().unwrap_or_else(|e| e.into_inner()).is_none() { volume_apply(true); } } else { volume_apply(false); }
        // notificacoes de dispositivo: fones OU volume (o volume precisa saber da troca da saida padrao)
        { let mut g = NOTIF_REG.lock().unwrap_or_else(|e| e.into_inner());
          let want = o.headphones || o.volume;
          if want && g.is_none() {
              if let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) { let cb: IMMNotificationClient = DevCb.into();
                  crate::log(&format!("overlay dispositivos: RegisterEndpointNotificationCallback {:?}", en.RegisterEndpointNotificationCallback(&cb).is_ok())); *g = Some(SendBox((en, cb))); }
          } else if !want { if let Some(SendBox((en, cb))) = g.take() { let _ = en.UnregisterEndpointNotificationCallback(&cb); } } }
        // rede
        { let mut g = IPCHG.lock().unwrap_or_else(|e| e.into_inner());
          let want_ip = o.wifi || !crate::pages::has_battery(); // PC sem bateria: o icone de rede do compacto tambem precisa do evento
          if want_ip && g.is_none() { let mut h = HANDLE::default();
              let r = NotifyIpInterfaceChange(AF_UNSPEC, Some(ip_changed), None, false, &mut h); crate::log(&format!("overlay rede: NotifyIpInterfaceChange {:?}", r)); if r.0 == 0 { *g = Some(h.0 as usize); }
          } else if !want_ip { if let Some(h) = g.take() { let _ = CancelMibChangeNotify2(HANDLE(h as *mut _)); } } }
        if !crate::pages::has_battery() { crate::netstat::watch(); if crate::netstat::refresh() { crate::island::status_changed(); } }
        // bateria: so guarda o estado inicial (o evento vem por WM_POWERBROADCAST)
        brightness(o.brightness);
        if o.battery { *LAST_POWER.lock().unwrap_or_else(|e| e.into_inner()) = power(); }
        crate::island::caps_poll(o.caps_lock);
    }).ok();
}

// ---------- desenho (coordenadas da ilha 280x48) ----------
/// Entrada: escala 0,92 -> 1 + deslize curto de 6 px + fade, a partir da propria forma do notch (a forma cresce por mola). Troca de icone = crossfade;
/// barra/percentual interpolam. Modo simples (Economia / animacoes do Windows desligadas) = so fade.
pub fn draw(a: &mut crate::island::App, ov: &Ov, w: f32, ap: f32, now: Instant) {
    use crate::island::Tw;
    let simple = crate::island::overlay_simple();
    if a.p.ov_cur.is_empty() { a.p.ov_cur = ov.icon; a.p.ov_prev = ""; a.p.ov_ic = None; a.p.ov_v = Some(Tw::new(ov.value.unwrap_or(0.0))); }
    else if a.p.ov_cur != ov.icon { a.p.ov_prev = a.p.ov_cur; a.p.ov_cur = ov.icon; let mut t = Tw::new(0.0); t.go(now, 1.0, if simple { 1.0 } else { 200.0 }); a.p.ov_ic = Some(t); }
    if let (Some(v), Some(t)) = (ov.value, a.p.ov_v.as_mut()) { t.go(now, v, if simple { 1.0 } else { 150.0 }); }
    let vv = a.p.ov_v.as_ref().map_or(ov.value.unwrap_or(0.0), |t| t.val(now));
    let ip = a.p.ov_ic.as_ref().map_or(1.0, |t| t.val(now)).clamp(0.0, 1.0);
    let ix = (crate::island::WIN_W - w) / 2.0;
    let (s, dy) = if simple { (1.0, 0.0) } else { (0.92 + 0.08 * ap, -6.0 * (1.0 - ap)) };
    a.gfx.set_t(ix + (w - 280.0) / 2.0 + 140.0 * (1.0 - s), dy + 24.0 * (1.0 - s), s);
    let fill = ov.icon == "arrow-big-up"; // seta do Caps Lock cheia
    if ip < 1.0 && !a.p.ov_prev.is_empty() {
        a.gfx.icon(a.p.ov_prev, 16.0, 14.0, 20.0, (ov.color.0, ov.color.1, ov.color.2, ov.color.3 * (1.0 - ip)), a.p.ov_prev == "arrow-big-up");
        a.gfx.icon(ov.icon, 16.0, 14.0, 20.0, (ov.color.0, ov.color.1, ov.color.2, ov.color.3 * ip), fill);
    } else { a.gfx.icon(ov.icon, 16.0, 14.0, 20.0, ov.color, fill); }
    match ov.value {
        Some(_) => {
            a.gfx.rrect(48.0, 21.0, 168.0, 6.0, 3.0, rgba(0xFFFFFF, 0.14)); a.gfx.rrect(48.0, 21.0, (168.0 * vv.clamp(0.0, 1.0)).max(0.0), 6.0, 3.0, ov.color);
            a.gfx.text(&format!("{:.0}%", vv.clamp(0.0, 1.0) * 100.0), Fmt::Body, 224.0, 15.0, 40.0, 14.0, rgba(0xFFFFFF, 1.0), true); // 2 px acima: centro das maiusculas alinhado ao trilho (QA5 C9)
        }
        None if ov.sub.is_empty() => a.gfx.text(&ov.title, Fmt::BodyStrong, 48.0, 15.0, 216.0, 18.0, rgba(0xFFFFFF, 1.0), false),
        None => { a.gfx.text(&ov.title, Fmt::BodyStrong, 48.0, 9.0, 216.0, 16.0, rgba(0xFFFFFF, 1.0), false); a.gfx.text(&ov.sub, Fmt::Small, 48.0, 27.0, 216.0, 14.0, rgba(0xFFFFFF, 0.64), false); }
    }
    a.gfx.set_t(ix, 0.0, 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn bateria_avisa_nas_transicoes() {
        assert_eq!(battery_decision((false, 50), (true, 50)).map(|x| x.0), Some("Carregando"));
        assert_eq!(battery_decision((true, 50), (false, 50)).map(|x| x.0), Some("Na bateria"));
        assert_eq!(battery_decision((false, 21), (false, 20)).map(|x| x.0), Some("Bateria fraca"));
        assert_eq!(battery_decision((false, 11), (false, 10)), Some(("Bateria muito baixa", true)));
        assert_eq!(battery_decision((false, 20), (false, 19)), None);   // ja estava abaixo de 20
        assert_eq!(battery_decision((true, 99), (true, 100)).map(|x| x.0), Some("Carga completa"));
        assert_eq!(battery_decision((true, 100), (true, 100)), None);
    }
    #[test] fn fones_por_nome() {
        assert!(looks_like_headphones("Headphones (Realtek)") && looks_like_headphones("Fones de ouvido (USB)") && looks_like_headphones("Galaxy Buds2"));
        assert!(!looks_like_headphones("Alto-falantes (Realtek High Definition Audio)"));
    }
}
