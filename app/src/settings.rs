// Configuracoes: modelo (settings.json em %APPDATA%\Eavely), validacao, aplicacao imediata e atalhos globais (parse puro e testado).
use serde::{Deserialize, Serialize};
use std::sync::{atomic::{AtomicU8, AtomicU32, Ordering::Relaxed}, RwLock};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct TimerCfg { pub focus_min: u32, pub short_min: u32, pub long_min: u32, pub cycles_to_long: u32, pub auto_next: bool, pub notify: bool }
impl Default for TimerCfg { fn default() -> Self { TimerCfg { focus_min: 25, short_min: 5, long_min: 15, cycles_to_long: 4, auto_next: false, notify: true } } }

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Hotkeys { pub toggle_island: String, pub toggle_timer: String, pub open_shelf: String }
impl Default for Hotkeys { fn default() -> Self { Hotkeys { toggle_island: "Ctrl+Alt+E".into(), toggle_timer: "Ctrl+Alt+T".into(), open_shelf: "Ctrl+Alt+F".into() } } }

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Overlays { pub volume: bool, pub brightness: bool, pub battery: bool, pub wifi: bool, pub headphones: bool, pub caps_lock: bool }

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub online_lyrics: bool, // buscar letras no lrclib.net (envia titulo, artista e album)
    pub start_with_windows: bool, pub show_trace: bool, pub show_live: bool, pub hide_during_event: bool,
    pub monitor: String, // "cursor" | "primary" | "mN" (N = indice)
    pub trigger_zone_px: u32, pub collapsed_look: String, // "clock" | "eyes"
    pub show_lyric_line: bool, pub live_mode: String, // "compact" | "expanded"
    pub timer: TimerCfg, pub hotkeys: Hotkeys, pub overlays: Overlays,
    /// respeitar 'Efeitos de animacao' do Windows (desligado la = sem animacao aqui). Padrao: NAO respeitar (as molas sao parte do produto); ligue para acessibilidade.
    pub respect_system_animation: bool,
    pub ui_scale: String,          // "auto" | "100" | "115" | "130" | "150" (reinicia para aplicar)
    pub minimal_island: bool,      // ilha minima: sem a pilula ao vivo (musica/timer)
    pub open_mode: String,         // "hover" | "click"
    pub anim_speed: String,        // "normal" | "fast" (duracoes x1 / x0,6)
    pub cover_spin: bool, pub cover_gradient: bool, pub lyric_bar: bool, // 'Mostrar letras no notch recolhido': barra de letras e linha do ao vivo (a ilha aberta nao muda)
    pub shelf_start_empty: bool, pub shelf_thumbs: bool, pub hide_fullscreen: bool,
    pub tasks_done_last: bool, pub tasks_overdue_red: bool,
    /// compacto de PC (so com o compacto visivel): estado do momento, CPU/memoria/GPU/rede (rotacionam em `pc_slots` espacos a cada `pc_rotate` s)
    pub pc_state: bool, pub pc_cpu: bool, pub pc_ram: bool, pub pc_gpu: bool, pub pc_net: bool, pub pc_slots: u32, pub pc_rotate: u32,
    /// Desempenho. preset: "max" | "balanced" | "eco" | "custom"; fps_cap: "auto" (taxa do monitor) | "60" | "30"; anim_quality: "smooth" | "balanced" | "light";
    /// lyrics_scroll: "smooth" (a cada quadro) | "line" (por linha); fx_level / viz_level: "on" | "reduced" (| "off" so em fx)
    /// ms que a ilha fica aberta depois que o mouse sai (0, 500, 1000, 2000, 3000)
    pub leave_delay_ms: u32,
    /// brilho atras do arquivo arrastado no Shelf (Economia desliga)
    pub shelf_glow: bool,
    /// mini notch (janela maximizada): false = so ao encostar na forma (folga 1 px); true = pequena folga (6 px)
    pub mini_near: bool,
    pub perf_preset: String, pub fps_cap: String, pub anim_quality: String, pub lyrics_scroll: String, pub fx_level: String, pub viz_level: String,
}
impl Default for Settings {
    fn default() -> Self { Settings { online_lyrics: true, start_with_windows: false, show_trace: true, show_live: true, hide_during_event: false, monitor: "cursor".into(), trigger_zone_px: 40,
        collapsed_look: "clock".into(), show_lyric_line: true, live_mode: "compact".into(), timer: TimerCfg::default(), hotkeys: Hotkeys::default(), overlays: Overlays::default(), respect_system_animation: false,
        ui_scale: "auto".into(), minimal_island: false, open_mode: "hover".into(), anim_speed: "normal".into(), cover_spin: true, cover_gradient: true, lyric_bar: true,
        shelf_start_empty: false, shelf_thumbs: true, hide_fullscreen: true, tasks_done_last: true, tasks_overdue_red: true,
        pc_state: true, pc_cpu: true, pc_ram: true, pc_gpu: true, pc_net: true, pc_slots: 1, pc_rotate: 5,
        leave_delay_ms: 1000, shelf_glow: true, mini_near: false, perf_preset: "balanced".into(), fps_cap: "auto".into(), anim_quality: "smooth".into(), lyrics_scroll: "smooth".into(), fx_level: "on".into(), viz_level: "reduced".into() } }
}
impl Settings {
    /// Presets de desempenho: sobrescrevem so os controles da secao (e o intervalo do compacto de PC)
    pub fn apply_preset(&mut self, p: &str) {
        let (fps, q, lyr, fx, viz, rot) = match p { "max" => ("auto", "smooth", "smooth", "on", "on", 3), "eco" => ("30", "light", "line", "reduced", "reduced", 8), _ => ("auto", "smooth", "smooth", "on", "reduced", 5) };
        self.perf_preset = if matches!(p, "max" | "eco") { p } else { "balanced" }.into();
        self.shelf_glow = p != "eco"; self.fps_cap = fps.into(); self.anim_quality = q.into(); self.lyrics_scroll = lyr.into(); self.fx_level = fx.into(); self.viz_level = viz.into(); self.pc_rotate = rot;
    }
    /// o preset continua marcado so se os controles ainda batem com ele; senao vira "custom"
    fn preset_matches(&self) -> bool { let mut t = self.clone(); t.apply_preset(&self.perf_preset); t.shelf_glow == self.shelf_glow && t.fps_cap == self.fps_cap && t.anim_quality == self.anim_quality && t.lyrics_scroll == self.lyrics_scroll && t.fx_level == self.fx_level && t.viz_level == self.viz_level && t.pc_rotate == self.pc_rotate }
}
impl Settings {
    /// Valores fora do intervalo sao corrigidos ao ler.
    pub fn clamp(&mut self) {
        self.leave_delay_ms = [0u32, 500, 1000, 2000, 3000].into_iter().min_by_key(|v| v.abs_diff(self.leave_delay_ms)).unwrap_or(1000);
        self.trigger_zone_px = match self.trigger_zone_px { 0..=32 => 24, 33..=52 => 40, _ => 64 };
        self.timer.focus_min = self.timer.focus_min.clamp(1, 180); self.timer.short_min = self.timer.short_min.clamp(1, 60);
        self.timer.long_min = self.timer.long_min.clamp(1, 120); self.timer.cycles_to_long = self.timer.cycles_to_long.clamp(2, 8);
        if self.collapsed_look != "eyes" { self.collapsed_look = "clock".into(); }
        if self.live_mode != "expanded" { self.live_mode = "compact".into(); }
        if !matches!(self.ui_scale.as_str(), "100" | "115" | "130" | "150") { self.ui_scale = "auto".into(); }
        if self.open_mode != "click" { self.open_mode = "hover".into(); }
        self.pc_slots = self.pc_slots.clamp(1, 2); if !matches!(self.pc_rotate, 3 | 5 | 8) { self.pc_rotate = 5; }
        if self.anim_speed != "fast" { self.anim_speed = "normal".into(); }
        if !matches!(self.fps_cap.as_str(), "60" | "30") { self.fps_cap = "auto".into(); }
        if !matches!(self.anim_quality.as_str(), "balanced" | "light") { self.anim_quality = "smooth".into(); }
        if self.lyrics_scroll != "line" { self.lyrics_scroll = "smooth".into(); }
        if !matches!(self.fx_level.as_str(), "reduced" | "off") { self.fx_level = "on".into(); }
        if self.viz_level != "on" { self.viz_level = "reduced".into(); }
        if !matches!(self.perf_preset.as_str(), "max" | "eco" | "custom") { self.perf_preset = "balanced".into(); }
        if self.perf_preset != "custom" && !self.preset_matches() { self.perf_preset = "custom".into(); }
        if !(self.monitor == "cursor" || self.monitor == "primary" || self.monitor.strip_prefix('m').is_some_and(|n| n.parse::<u32>().is_ok())) { self.monitor = "cursor".into(); }
        for h in [&mut self.hotkeys.toggle_island, &mut self.hotkeys.toggle_timer, &mut self.hotkeys.open_shelf] { if parse_hotkey(h).is_none() { *h = String::new(); } }
        let d = Hotkeys::default();
        if self.hotkeys.toggle_island.is_empty() { self.hotkeys.toggle_island = d.toggle_island; }
        if self.hotkeys.toggle_timer.is_empty() { self.hotkeys.toggle_timer = d.toggle_timer; }
        if self.hotkeys.open_shelf.is_empty() { self.hotkeys.open_shelf = d.open_shelf; }
    }
}

// ---- atalhos: "Ctrl+Alt+E" <-> (mods, vk) ----
pub const MOD_ALT: u32 = 1; pub const MOD_CONTROL: u32 = 2; pub const MOD_SHIFT: u32 = 4; pub const MOD_WIN: u32 = 8;
/// Exige ao menos Ctrl/Alt/Win (nunca uma tecla solta). vk: A-Z, 0-9, F1-F12.
pub fn parse_hotkey(s: &str) -> Option<(u32, u32)> {
    let (mut mods, mut vk) = (0u32, None);
    for p in s.split('+').map(str::trim) {
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL, "alt" => mods |= MOD_ALT, "shift" => mods |= MOD_SHIFT, "win" => mods |= MOD_WIN,
            k if k.len() == 1 && k.as_bytes()[0].is_ascii_alphanumeric() => vk = Some(k.to_ascii_uppercase().as_bytes()[0] as u32),
            k if k.starts_with('f') && k[1..].parse::<u32>().is_ok_and(|n| (1..=12).contains(&n)) => vk = Some(0x70 + k[1..].parse::<u32>().unwrap() - 1),
            _ => return None,
        }
    }
    (mods & (MOD_CONTROL | MOD_ALT | MOD_WIN) != 0).then_some(()).and(vk).map(|v| (mods, v))
}
pub fn fmt_hotkey(mods: u32, vk: u32) -> String {
    let mut v = Vec::new();
    if mods & MOD_CONTROL != 0 { v.push("Ctrl".to_string()); } if mods & MOD_ALT != 0 { v.push("Alt".into()); } if mods & MOD_SHIFT != 0 { v.push("Shift".into()); } if mods & MOD_WIN != 0 { v.push("Win".into()); }
    v.push(if (0x70..=0x7B).contains(&vk) { format!("F{}", vk - 0x70 + 1) } else { (vk as u8 as char).to_string() });
    v.join("+")
}

// ---- estado global ----
static S: RwLock<Option<Settings>> = RwLock::new(None);
/// estado dos atalhos globais (0 nao registrado, 1 ok, 2 em uso por outro app) — lido pela janela de configuracoes
pub static HK_STATE: [AtomicU8; 3] = [const { AtomicU8::new(0) }; 3];
pub static TRIG_PX: AtomicU32 = AtomicU32::new(40);
/// copias baratas (sem clonar) das opcoes lidas a cada quadro/avaliacao
pub static FLAGS: [std::sync::atomic::AtomicBool; 16] = [const { std::sync::atomic::AtomicBool::new(false) }; 16];
pub static PC_SLOTS: AtomicU32 = AtomicU32::new(1); pub static PC_ROT: AtomicU32 = AtomicU32::new(5);
pub const F_PC_STATE: usize = 8; pub const F_PC_CPU: usize = 9; pub const F_PC_RAM: usize = 10; pub const F_PC_GPU: usize = 11; pub const F_PC_NET: usize = 12;
pub const F_HIDE_FS: usize = 0; pub const F_FAST: usize = 1; pub const F_CLICK: usize = 2; pub const F_SPIN: usize = 3; pub const F_GRAD: usize = 4; pub const F_LYRBAR: usize = 5; pub const F_THUMBS: usize = 6; pub const F_MINIMAL: usize = 7;
pub fn flag(i: usize) -> bool { FLAGS[i].load(Relaxed) }
pub const F_LYR_LINE: usize = 13; pub const F_SHELF_GLOW: usize = 14;
/// desempenho (lido por quadro): limite de quadros em microssegundos (0 = taxa do monitor), multiplicador de tempo das animacoes x100, equalizador em quadros/s (0 = ritmo do monitor)
pub static CAP_US: AtomicU32 = AtomicU32::new(0); pub static ANIM_X100: AtomicU32 = AtomicU32::new(100); pub static VIZ_FPS: AtomicU32 = AtomicU32::new(20);
fn sync_flags(s: &Settings) {
    let fx = s.fx_level.as_str();
    for (i, v) in [s.hide_fullscreen, s.anim_speed == "fast", s.open_mode == "click", s.cover_spin && fx == "on", s.cover_gradient && fx != "off", s.lyric_bar, s.shelf_thumbs, s.minimal_island, s.pc_state, s.pc_cpu, s.pc_ram, s.pc_gpu, s.pc_net, s.lyrics_scroll == "line", s.shelf_glow].into_iter().enumerate() { FLAGS[i].store(v, Relaxed); }
    crate::island::GRACE_MS.store(s.leave_delay_ms, Relaxed); crate::island::MINI_PAD.store(if s.mini_near { 6 } else { 1 }, Relaxed);
    PC_SLOTS.store(s.pc_slots, Relaxed); PC_ROT.store(s.pc_rotate, Relaxed);
    let light = s.anim_quality == "light";
    CAP_US.store(match (s.fps_cap.as_str(), light) { ("30", _) | (_, true) => 33_333, ("60", _) => 16_667, _ => 0 }, Relaxed);
    ANIM_X100.store(match s.anim_quality.as_str() { "light" => 170, "balanced" => 130, _ => 100 }, Relaxed);
    VIZ_FPS.store(if s.viz_level == "on" { 0 } else { 20 }, Relaxed);
}
/// fator de escala da ilha (so lido na inicializacao: 'reinicia para aplicar')
pub fn ui_scale_factor() -> f32 { match get().ui_scale.as_str() { "100" => 1.0, "115" => 1.15, "130" => 1.3, "150" => 1.5, _ => 1.0 } }
pub fn ui_scale_explicit() -> bool { get().ui_scale != "auto" }
fn file() -> std::path::PathBuf { crate::data_dir().join("settings.json") }
pub fn get() -> Settings {
    if cfg!(test) { return Settings::default(); }
    if let Some(s) = S.read().unwrap_or_else(|e| e.into_inner()).as_ref() { return s.clone(); }
    let mut s: Settings = crate::store::load_or_quarantine(&file()).unwrap_or_default();
    s.clamp(); TRIG_PX.store(s.trigger_zone_px, Relaxed); sync_flags(&s);
    *S.write().unwrap_or_else(|e| e.into_inner()) = Some(s.clone()); s
}
pub fn timer() -> TimerCfg { get().timer }

/// Altera, valida, grava (atomico) e aplica na hora.
/// leitura barata (sem clonar) para o motor de animacao
pub fn respect_anim() -> bool { S.read().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|s| s.respect_system_animation) }
pub fn update(f: impl FnOnce(&mut Settings)) {
    static UPD: std::sync::Mutex<()> = std::sync::Mutex::new(()); // um update por vez: le -> altera -> grava sem perder alteracoes de outra thread
    let _g = UPD.lock().unwrap_or_else(|e| e.into_inner());
    let old = get(); let mut s = old.clone(); f(&mut s); s.clamp();
    if s == old { return; }
    *S.write().unwrap_or_else(|e| e.into_inner()) = Some(s.clone());
    crate::store::write_atomic(&file(), serde_json::to_string_pretty(&s).unwrap_or_default().as_bytes());
    TRIG_PX.store(s.trigger_zone_px, Relaxed); sync_flags(&s);
    crate::pages::set_live_expanded(s.live_mode == "expanded");
    if s.start_with_windows != old.start_with_windows { set_autostart(s.start_with_windows); }
    if s.hotkeys != old.hotkeys { crate::island::reapply_hotkeys(); }
    if s.overlays != old.overlays { crate::overlay::reconfigure(); }
    crate::island::wake(); crate::settings_ui::refresh();
}

/// "Iniciar com o Windows": HKCU\...\Run (so este usuario)
pub fn set_autostart(on: bool) {
    use windows::{core::{w, HSTRING}, Win32::System::Registry::*};
    unsafe {
        if on {
            let exe = std::env::current_exe().map(|p| format!("\"{}\"", p.display())).unwrap_or_default();
            let v: Vec<u16> = exe.encode_utf16().chain([0]).collect();
            let _ = RegSetKeyValueW(HKEY_CURRENT_USER, w!(r"Software\Microsoft\Windows\CurrentVersion\Run"), &HSTRING::from("Eavely"), REG_SZ.0, Some(v.as_ptr() as *const _), (v.len() * 2) as u32);
        } else { let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, w!(r"Software\Microsoft\Windows\CurrentVersion\Run"), &HSTRING::from("Eavely")); }
    }
}

#[cfg(test)]
mod tests {
    #[test] fn presets_coerentes_e_personalizado() {
        let mut s = Settings::default(); s.clamp(); assert_eq!(s.perf_preset, "balanced");
        s.apply_preset("eco"); s.clamp(); assert_eq!((s.perf_preset.as_str(), s.fps_cap.as_str(), s.pc_rotate), ("eco", "30", 8));
        s.fps_cap = "60".into(); s.clamp(); assert_eq!(s.perf_preset, "custom");
        s.apply_preset("max"); s.clamp(); assert_eq!(s.perf_preset, "max");
        let mut j: Settings = serde_json::from_str("{\"fpsCap\":\"zzz\",\"fxLevel\":\"x\"}").unwrap(); j.clamp(); assert_eq!((j.fps_cap.as_str(), j.fx_level.as_str()), ("auto", "on"));
    }
    use super::*;
    #[test] fn atraso_de_recolher_e_brilho() {
        let mut s = Settings::default(); s.clamp(); assert_eq!((s.leave_delay_ms, s.shelf_glow), (1000, true));
        s.leave_delay_ms = 1400; s.clamp(); assert_eq!(s.leave_delay_ms, 1000); s.leave_delay_ms = 99999; s.clamp(); assert_eq!(s.leave_delay_ms, 3000);
        s.apply_preset("eco"); assert!(!s.shelf_glow); s.clamp(); assert_eq!(s.perf_preset, "eco");
        s.shelf_glow = true; s.clamp(); assert_eq!(s.perf_preset, "custom"); s.apply_preset("max"); assert!(s.shelf_glow);
    }
    #[test] fn atalhos_ida_e_volta() {
        assert_eq!(parse_hotkey("Ctrl+Alt+E"), Some((MOD_CONTROL | MOD_ALT, b'E' as u32)));
        assert_eq!(parse_hotkey("ctrl + shift + f5"), Some((MOD_CONTROL | MOD_SHIFT, 0x74)));
        assert_eq!(fmt_hotkey(MOD_CONTROL | MOD_ALT, b'T' as u32), "Ctrl+Alt+T");
        assert_eq!(fmt_hotkey(MOD_WIN, 0x7B), "Win+F12");
        assert_eq!(parse_hotkey("E"), None);          // sem Ctrl/Alt/Win
        assert_eq!(parse_hotkey("Shift+E"), None);    // so Shift nao basta
        assert_eq!(parse_hotkey("Ctrl+Alt"), None);   // sem tecla
        assert_eq!(parse_hotkey("Ctrl+Alt+F13"), None);
        assert_eq!(parse_hotkey("lixo"), None);
    }
    #[test] fn valores_fora_do_intervalo_sao_corrigidos() {
        let mut s: Settings = serde_json::from_str(r#"{"triggerZonePx": 500, "timer": {"focusMin": 0, "shortMin": 999, "cyclesToLong": 1}, "monitor": "xx", "liveMode": "?", "hotkeys": {"toggleIsland": "E"}}"#).unwrap();
        s.clamp();
        assert_eq!((s.trigger_zone_px, s.timer.focus_min, s.timer.short_min, s.timer.cycles_to_long), (64, 1, 60, 2));
        assert_eq!((s.monitor.as_str(), s.live_mode.as_str(), s.hotkeys.toggle_island.as_str()), ("cursor", "compact", "Ctrl+Alt+E"));
        assert_eq!(s.timer.long_min, 15); // chave ausente = padrao
    }
    #[test] fn padroes_e_roundtrip() {
        let d = Settings::default(); assert!(d.show_trace && d.show_live && !d.hide_during_event && !d.start_with_windows && d.monitor == "cursor" && d.trigger_zone_px == 40);
        assert!(!d.overlays.volume && !d.overlays.caps_lock); // avisos do sistema desligados por padrao
        let j = serde_json::to_string(&d).unwrap(); assert!(j.contains("\"startWithWindows\":false") && j.contains("\"toggleIsland\":\"Ctrl+Alt+E\""));
        assert_eq!(serde_json::from_str::<Settings>(&j).unwrap(), d);
    }
}
