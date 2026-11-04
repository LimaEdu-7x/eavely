// Paginas da ilha (Musica, Pomodoro, Shelf, Tarefas), cromo (cabecalho, setas, pontos, dock), peek/ao vivo, entrada do mouse.
// Modo imediato: cada quadro desenha e registra as areas clicaveis em `hits` (coordenadas da ilha, px logicos).
use std::{sync::{Arc, Mutex}, time::Instant};
use windows::Win32::{Foundation::{LPARAM, WPARAM}, System::{Power::*, SystemInformation::GetLocalTime}, UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*}};
use crate::{gfx::{rgba, Fmt, Rgba}, island::{App, St, Tw}, lyrics::Line, media, timer};

// ---- tokens ----
const BG: Rgba = rgba(0x000000, 1.0);
const SURFACE: Rgba = rgba(0xFFFFFF, 0.08);
const SURFACE_H: Rgba = rgba(0xFFFFFF, 0.14);
const LINE_C: Rgba = rgba(0xFFFFFF, 0.14);
const TEXT: Rgba = rgba(0xFFFFFF, 1.0);
const TEXT2: Rgba = rgba(0xFFFFFF, 0.64);
const TEXT3: Rgba = rgba(0xFFFFFF, 0.52);
const ACCENT2: Rgba = rgba(0xFFB86B, 1.0);
const OK: Rgba = rgba(0x7BE0A4, 1.0);
const DANGER: Rgba = rgba(0xFF6B6B, 1.0);
/// ordem das paginas: 0 Musica, 1 Arquivos (Shelf), 2 Pomodoro, 3 Tarefas
pub const PAGE_COL: [Rgba; 4] = [rgba(0xB69CFF, 1.0), rgba(0xFFC857, 1.0), rgba(0xFF7A66, 1.0), rgba(0x5EE0B0, 1.0)];
pub const P_SHELF: usize = 1;
pub const P_POMO: usize = 2;
const PAGE_NAMES: [&str; 4] = ["Música", "Shelf", "Pomodoro", "Tarefas"];
const PAGE_ICONS: [&str; 4] = ["music", "inbox", "timer", "list-checks"];

#[derive(Clone, Copy)] pub struct R { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
impl R { pub fn has(&self, px: f32, py: f32) -> bool { px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h } }
const fn r(x: f32, y: f32, w: f32, h: f32) -> R { R { x, y, w, h } }
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act { TaskToday, TaskToggle(u32), TaskRemove(u32), TaskPick(u32), TaskCal, TaskMonth(i8), TaskDay(u32), VolSlider, ShelfSelect, TaskAdd, TaskEdit, Prev, Play, Next, Seek(R2), LyricSeek(u32), Lyrics, Volume, Page(u8), Arrow(i8), Settings, TimerMode(u8), TimerPlay, TimerReset, TimerEdit, ShelfClear, ShelfRemove(u32), ShelfOpen(u32), ShelfDragStart(u32), Live(u8), None }
#[derive(Clone, Copy, PartialEq, Debug)] pub struct R2(pub i32, pub i32); // faixa x da barra de seek (x, largura) em decimos de px

// ---- estado das paginas ----
/// Troca de musica: a capa gira em Y (largura 62 -> 2 px em ~130 ms ease-in), troca a imagem e reabre (2 -> 62 px em ~450 ms ease-out); titulo/fundo em crossfade.
pub struct Spin { pub t: Instant, pub cover: Option<(u64, Arc<Vec<u8>>)>, pub tint: Option<(f32, f32, f32)>, pub title: String, pub sub: String }
pub const SPIN_IN_MS: f32 = 130.0; pub const SPIN_OUT_MS: f32 = 450.0; pub const SPIN_FADE_MS: f32 = 300.0;
/// largura relativa da capa (1 = inteira) a `ms` do inicio do giro; e se ja virou para a capa nova
pub fn spin_scale(ms: f32) -> (f32, bool) {
    let min = 2.0 / 62.0;
    if ms < SPIN_IN_MS { let x = (ms / SPIN_IN_MS).clamp(0.0, 1.0); (1.0 - (1.0 - min) * x * x, false) }
    else { let x = ((ms - SPIN_IN_MS) / SPIN_OUT_MS).clamp(0.0, 1.0); (min + (1.0 - min) * crate::island::ease(x), true) }
}
fn spin_ms(a: &App, now: Instant) -> Option<f32> { a.p.spin.as_ref().map(|s| if now > s.t { (now - s.t).as_secs_f32() * 1000.0 } else { 0.0 }).filter(|m| *m < SPIN_IN_MS + SPIN_OUT_MS) }
/// cor do fundo com crossfade entre a capa antiga e a nova
fn cur_tint(a: &App, now: Instant) -> Option<(f32, f32, f32)> {
    match (spin_ms(a, now), a.p.spin.as_ref()) {
        (Some(m), Some(s)) => { let f = (m / (SPIN_IN_MS + SPIN_OUT_MS)).clamp(0.0, 1.0); match (s.tint, a.p.tint) { (Some(o), Some(n)) => Some((o.0 + (n.0 - o.0) * f, o.1 + (n.1 - o.1) * f, o.2 + (n.2 - o.2) * f)), (_, n) => n } }
        _ => a.p.tint,
    }
}
#[derive(Default)]
pub struct PageState {
    pub cover: Option<(u64, Arc<Vec<u8>>)>, pub tint: Option<(f32, f32, f32)>,
    pub spin: Option<Spin>, pub shown_title: String, pub shown_sub: String, pub lyr_user: Option<(Instant, f32)>, pub lyr_key_bar: String, pub lyr_prev_soft: bool, pub lyr_cur_soft: bool, pub lyr_key: String, pub lyr_idx: i64, pub lyr_prev: String, pub lyr_t: Option<Instant>, pub scroll: Option<Tw>,
    pub eq_t0: Option<Instant>, pub batt: (u8, bool), pub batt_t: Option<Instant>, pub last_wheel: Option<Instant>,
    pub tasks: Vec<crate::tasks::Task>, pub task_gen: u64, pub task_input: Option<String>, pub task_edit_id: Option<u64>, pub eye_move: Option<Instant>, pub blink_t: Option<Instant>, pub eye_x: Option<Tw>, pub eye_y: Option<Tw>, pub vol_muted: bool, pub pc_t0: Option<Instant>, pub pc_g: usize, pub pc_prev: usize, pub pc_chg: Option<Instant>, pub shelf_del: Vec<(String, Instant, bool)>, pub dots_at: Option<(Instant, usize)>, pub empty_at: Option<Instant>, pub shelf_n: usize, pub items_at: Option<Instant>, pub drag_end: Option<Instant>, pub shelf_new: Vec<(String, Instant)>, pub shelf_ids: Vec<String>, pub shelf_sx: f32, pub shelf_hover: Option<u32>, pub drag_t: Option<Instant>, pub task_edit_mode: bool, pub task_focus: Option<u64>, pub task_cal: bool, pub task_date: Option<(i32, u32, u32)>, pub task_month: (i32, u32), pub task_scroll: usize,
    pub ov_cur: &'static str, pub ov_prev: &'static str, pub ov_v: Option<Tw>, pub ov_ic: Option<Tw>, // overlay: icone atual/anterior (crossfade) e valor animado
    pub vol: f32, pub vol_t: Option<Instant>, pub vol_drag: bool, pub select: bool, pub sel: Vec<usize>, pub edit: Option<String>, pub shelf: Vec<crate::shelf::ShelfItem>, pub shelf_gen: u64, pub lyr_cur: String, pub drag_from: Option<(u32, f32, f32)>,
}

#[derive(Default)] struct LyrSlot { key: String, state: u8, synced: bool, instrumental: bool, lines: Vec<Line> } // state: 0 nada, 1 carregando, 2 pronto, 3 sem letra
static LYR: Mutex<LyrSlot> = Mutex::new(LyrSlot { key: String::new(), state: 0, synced: false, instrumental: false, lines: Vec::new() });
fn lyr() -> std::sync::MutexGuard<'static, LyrSlot> { LYR.lock().unwrap_or_else(|e| e.into_inner()) }

// ---- persistencia (ui.json) ----
fn ui_path() -> std::path::PathBuf { crate::data_dir().join("ui.json") }
fn ui_load() -> serde_json::Value { crate::store::load_or_quarantine(&ui_path()).unwrap_or_default() }
fn ui_save(k: &str, v: serde_json::Value) {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(()); let _g = L.lock().unwrap_or_else(|e| e.into_inner());
    let mut j = ui_load(); if !j.is_object() { j = serde_json::json!({}); }
    j[k] = v; crate::store::write_atomic(&ui_path(), j.to_string().as_bytes());
}
pub fn load_page() -> usize { ui_load()["page"].as_u64().unwrap_or(0).min(3) as usize }
pub fn load_split() -> bool { ui_load()["lyricsSplit"].as_bool().unwrap_or(false) }
pub fn save_page(p: usize) { ui_save("page", p.into()); } // chave nova: a ordem das paginas mudou
pub fn show_line() -> bool { crate::settings::get().show_trace }

// ---- compacto de PC ----
/// notebook? (bateria so aparece em notebook)
pub fn has_battery() -> bool { static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new(); *V.get_or_init(|| { let mut s = SYSTEM_POWER_STATUS::default(); unsafe { GetSystemPowerStatus(&mut s).is_ok() && s.BatteryFlag != 128 && s.BatteryFlag != 255 } }) }
fn pc_metrics() -> Vec<crate::metrics::Metric> {
    use crate::{metrics::Metric, settings::{flag, F_PC_CPU, F_PC_GPU, F_PC_NET, F_PC_RAM}};
    let mut v = Vec::new(); if flag(F_PC_CPU) { v.push(Metric::Cpu); } if flag(F_PC_RAM) { v.push(Metric::Ram); } if flag(F_PC_GPU) { v.push(Metric::Gpu); } if flag(F_PC_NET) { v.push(Metric::Net); } v
}
pub fn pc_mode() -> bool { crate::settings::flag(crate::settings::F_PC_STATE) || !pc_metrics().is_empty() }
fn pc_slots() -> usize { (crate::settings::PC_SLOTS.load(std::sync::atomic::Ordering::Relaxed) as usize).clamp(1, 2).min(pc_metrics().len().max(1)) }
const PC_SLOT_W: f32 = 66.0; const PC_STATE_W: f32 = 56.0;
/// largura do compacto: 150 (legado) ou o que o conteudo pede; so depende das OPCOES (nunca do estado ao vivo) para nao ficar mudando de tamanho
pub fn peek_w() -> f32 {
    if !pc_mode() { return 150.0; }
    let ml = pc_metrics();
    (14.0 + 22.0 + 44.0 + 14.0 + if crate::settings::flag(crate::settings::F_PC_STATE) { PC_STATE_W } else { 0.0 } + if ml.is_empty() { 0.0 } else { pc_slots() as f32 * PC_SLOT_W } + 10.0).max(150.0)
}
/// o timer de 1 Hz so existe com o compacto visivel e algo a mostrar
pub fn pc_needs_tick() -> bool { pc_mode() }
pub fn pc_sample() {
    use crate::settings::{flag, F_PC_CPU, F_PC_GPU, F_PC_NET, F_PC_RAM, F_PC_STATE};
    crate::metrics::sample(flag(F_PC_CPU), flag(F_PC_RAM), flag(F_PC_GPU), flag(F_PC_NET), flag(F_PC_STATE));
}

// ---- consultas ----
pub fn is_playing() -> bool { media::view().status == "playing" }
fn timer_running() -> bool { timer::timer_get().map(|t| t.status == timer::Status::Running).unwrap_or(false) }
pub fn is_live() -> bool { is_playing() || timer_running() }
static LIVE_EXP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn live_expanded() -> bool { LIVE_EXP.load(std::sync::atomic::Ordering::Relaxed) }
pub fn set_live_expanded(on: bool) { LIVE_EXP.store(on, std::sync::atomic::Ordering::Relaxed); }
/// forma do estado "ao vivo": compacto 168x30 (padrao) ou expandido 300x52
/// barra de letras: tocando musica com letra sincronizada -> 300x34 com nota + linha atual, degrade preto -> cor da capa
pub fn lyric_bar() -> bool {
    static LAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !is_playing() || live_expanded() || !crate::settings::get().show_lyric_line || !crate::settings::flag(crate::settings::F_LYRBAR) { return false; }
    let l = lyr();
    // ESTAVEL: com a letra carregando mantem o estado anterior (sem encolher/crescer entre faixas); instrumental e lacunas mantem a barra
    let on = match l.state { 2 => l.instrumental || (l.synced && !l.lines.is_empty()), 1 => LAST.load(std::sync::atomic::Ordering::Relaxed), _ => false };
    LAST.store(on, std::sync::atomic::Ordering::Relaxed); on
}
/// texto da barra no instante: (texto, discreto?). Lacuna = intro longa, trecho sem linha por >~4-5 s (ponte/final) ou faixa instrumental.
fn bar_text(pos: u64) -> (String, bool) {
    let l = lyr();
    if l.instrumental { return ("Instrumental".into(), true); }
    let i = lyric_idx(&l, pos);
    const GAP_TXT: &str = "Sem letra neste trecho";
    if i < 0 { return match l.lines.first() { Some(f) if f.time_ms.saturating_sub(pos) > 4000 => (GAP_TXT.into(), true), _ => ("♪".into(), true) }; }
    let cur = &l.lines[i as usize]; let since = pos.saturating_sub(cur.time_ms);
    let t = cur.text.trim();
    if t.is_empty() { return (GAP_TXT.into(), true); }
    match l.lines.get(i as usize + 1) { Some(n) if n.time_ms.saturating_sub(cur.time_ms) > 9000 && since > 5000 => (GAP_TXT.into(), true), None if since > 6000 => (GAP_TXT.into(), true), _ => (t.to_string(), false) }
}
pub fn live_size() -> (f32, f32, f32) { if live_expanded() { (300.0, 52.0, 22.0) } else if lyric_bar() { (300.0, 34.0, 14.0) } else { (live_width(), 30.0, 14.0) } }
pub fn live_width() -> f32 { if is_playing() && timer_running() { 190.0 } else { 168.0 } }
pub fn eq_anim() -> bool { is_playing() && !crate::island::reduced_motion() }
pub fn open_anim(a: &App, _now: Instant) -> bool { eq_anim() && a.state == St::Open }
pub fn on_open(a: &mut App) {
    a.p.batt_t = None;
    // pagina inicial: arrastando arquivo = Shelf (ja tratado pelo arrasto); tocando musica = player; senao a ultima pagina escolhida pelo usuario
    if a.drag_over { return; }
    let want = if is_playing() && crate::settings::get().show_live { 0 } else { load_page() };
    if a.page != want { a.prev_page = a.page; a.page = want; a.page_t = crate::island::Spring::new(1.0); a.p.dots_at = None; } // troca seca: a ilha ainda esta abrindo
}

/// Chamado em `wake()`: faixa mudou -> capa/tint e busca de letra.
pub fn on_wake(a: &mut App) {
    let v = media::view();
    { // troca de faixa com a ilha aberta na Musica: inicia o giro (guarda capa/cor/titulo antigos)
        let key = format!("{}|{}", v.title, v.artist);
        if crate::settings::flag(crate::settings::F_SPIN) && !a.p.lyr_key.is_empty() && key != a.p.lyr_key && !v.title.is_empty() && a.state == St::Open && a.page == 0 && !crate::island::reduced_motion() && a.p.spin.as_ref().is_none_or(|s| s.t.elapsed().as_millis() > 640) {
            a.p.spin = Some(Spin { t: Instant::now(), cover: a.p.cover.clone(), tint: a.p.tint, title: a.p.shown_title.clone(), sub: a.p.shown_sub.clone() });
        }
    }
    match media::cover_px() {
        None => { a.p.cover = None; a.p.tint = None; } // sem capa: placeholder (icone music) e sem gradiente
        Some(px) => if a.p.cover.as_ref().map(|c| c.0) != Some(v.cover_gen) { a.p.tint = dominant(&px, media::COVER_PX as usize); a.p.cover = Some((v.cover_gen, px)); }
    }
    let key = format!("{}|{}", v.title, v.artist);
    if key != a.p.lyr_key {
        a.p.lyr_key = key.clone(); a.p.lyr_idx = -2; a.p.scroll = None;
        if v.title.is_empty() { let mut l = lyr(); *l = LyrSlot::default(); return; }
        { let mut l = lyr(); *l = LyrSlot { key: key.clone(), state: 1, ..Default::default() }; }
        let m = media::current();
        crate::lyrics::request(crate::lyrics::Job { key, title: m.title, artist: m.artist, album: m.album, dur_sec: (m.duration_ms + 500) / 1000 }, lyrics_done);
    }
}
/// Resultado do worker de letras (so chega se a faixa ainda for a mais recente)
fn lyrics_done(key: &str, res: Option<crate::lyrics::Lyrics>) {
    let mut l = lyr(); if l.key != key { return; }
    #[cfg(feature = "devtools")]
    if FORCE_LYR.load(std::sync::atomic::Ordering::Relaxed) != 0 { return; } // captura de tela: estado forcado
    match res { Some(x) => { l.synced = x.synced; l.instrumental = x.instrumental; l.lines = x.lines; l.state = 2; } None => { l.state = 3; } }
    drop(l); crate::island::wake();
}

/// cor dominante da capa (algoritmo do DESIGN-NATIVO §6), L fixado em 0,18..0,32 (HSL)
fn dominant(px: &[u8], side: usize) -> Option<(f32, f32, f32)> {
    if side == 0 || px.len() < side * side * 4 { return None; } // decode truncado: sem gradiente em vez de indice fora do limite
    let step = (side / 24).max(1);
    let mut buckets = [(0.0f32, 0.0f32); 12];
    for y in (0..side).step_by(step) { for x in (0..side).step_by(step) {
        let i = (y * side + x) * 4; let (b, g, rr) = (px[i] as f32 / 255.0, px[i + 1] as f32 / 255.0, px[i + 2] as f32 / 255.0);
        let (mx, mn) = (rr.max(g).max(b), rr.min(g).min(b)); let l = (mx + mn) / 2.0; let d = mx - mn;
        if d == 0.0 { continue; }
        let s = d / (1.0 - (2.0 * l - 1.0).abs()); if s < 0.15 || l < 0.08 || l > 0.92 { continue; }
        let h = if mx == rr { ((g - b) / d).rem_euclid(6.0) } else if mx == g { (b - rr) / d + 2.0 } else { (rr - g) / d + 4.0 } * 60.0;
        let k = ((h / 30.0) as usize).min(11); buckets[k].0 += s; buckets[k].1 += h * s;
    } }
    let (k, b) = buckets.iter().enumerate().max_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap())?;
    if b.0 <= 0.0 { return None; }
    let _ = k; let h = b.1 / b.0;
    Some(hsl(h, 0.6, 0.40)) // L 0,40 -> depois do alfa 0,55 sobre o preto fica na faixa 0,18-0,32 pedida
}
fn hsl(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s; let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs()); let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 { 0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x), 3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x) };
    (r + m, g + m, b + m)
}

fn fmt_t(ms: u64) -> String { let s = ms / 1000; format!("{:02}:{:02}", s / 60, s % 60) }
fn lyric_idx(l: &LyrSlot, pos: u64) -> i64 { if !l.synced || l.lines.is_empty() { return -1; } l.lines.partition_point(|x| x.time_ms <= pos) as i64 - 1 }

// ---- desenho ----
pub fn draw(a: &mut App, now: Instant, w: f32, h: f32, rad: f32, hits: &mut Vec<(R, Act)>) {
    let st = a.state;
    // corpo: preto (traco: 85% + borda inferior)
    a.gfx.rrect(0.0, -rad, w, h + rad, rad, (0.0, 0.0, 0.0, 1.0));
    // flares: 6 px recolhido -> 18 px aberto (largura total 457 = 420 + 2x18, medido internamente)
    if h > 3.0 { a.gfx.flares(w, (crate::layout::compact_flare() + (crate::layout::open_flare() - crate::layout::compact_flare()) * ((h - crate::layout::compact_h()) / (crate::layout::open_h() - crate::layout::compact_h())).clamp(0.0, 1.0)).min(h * 0.75), (0.0, 0.0, 0.0, 1.0)); }
    if st == St::Line || h < 6.0 { return; } // mini notch: so a forma
    // dai em diante o conteudo e desenhado em UNIDADES DE PROJETO (420 de largura aberta) sob a escala k do preset de tamanho; a forma acima e em px reais
    let k = crate::layout::k(); let ix = (crate::layout::win_w() - w) / 2.0; a.gfx.set_t(ix, a.ty.pos.min(0.0), k);
    let (w, h, rad) = (w / k, h / k, rad / k);
    let open_f = ((h - 30.0) / (140.0 - 30.0)).clamp(0.0, 1.0);
    let ao = ((open_f - 0.4) / 0.6).clamp(0.0, 1.0); // conteudo do aberto: entra depois da forma abrir ~40%
    let ap = if st == St::Over { ((h - 8.0) / 30.0).clamp(0.0, 1.0) } else if st == St::Live && live_expanded() { ((h - 8.0) / 20.0).clamp(0.0, 1.0) } else if st == St::Live && lyric_bar() { ((h - 8.0) / 20.0).clamp(0.0, 1.0) } else { ((h - 8.0) / 20.0).clamp(0.0, 1.0) * (1.0 - (h - 30.0) / 30.0).clamp(0.0, 1.0) }; // peek/ao vivo
    let t_s = a.p.eq_t0.get_or_insert(now).elapsed().as_secs_f32();
    if st == St::Over && ap > 0.01 { if let Some(ov) = crate::overlay::current() { a.gfx.layer(ap); crate::overlay::draw(a, &ov, w, ap, now); a.gfx.unlayer(); } }
    else if ap > 0.01 { a.gfx.layer(ap); if st == St::Live && live_expanded() { draw_live_expanded(a, w, t_s); } else if st == St::Live && lyric_bar() { draw_lyric_bar(a, w, h, rad, now); } else { draw_peek_live(a, w, t_s); } a.gfx.unlayer(); }
    if ao > 0.01 {
        a.gfx.rclip(w, h, rad);
        // gradiente da cor da capa (so Musica)
        if a.page == 0 && crate::settings::flag(crate::settings::F_GRAD) { if let Some((tr, tg, tb)) = cur_tint(a, now) { a.gfx.vgrad(0.0, 0.0, w, h, (tr, tg, tb, 0.55), 0.35, ao); } }
        a.gfx.layer(ao);
        draw_open(a, now, w, h, t_s, hits);
        a.gfx.unlayer();
        a.gfx.unrclip();
    }
}

pub fn rest_clock() -> (String, String) { clock_str() }
/// bateria para a forma de repouso em GDI (sem cache de App)
pub fn rest_battery() -> (u8, bool) {
    let mut s = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut s) }.is_ok() { (if s.BatteryLifePercent <= 100 { s.BatteryLifePercent } else { 100 }, s.ACLineStatus == 1) } else { (100, true) }
}
fn clock_str() -> (String, String) {
    let t = unsafe { GetLocalTime() };
    const D: [&str; 7] = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"]; const M: [&str; 12] = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
    (format!("{:02}:{:02}", t.wHour, t.wMinute), format!("{} {} {}", D[t.wDayOfWeek as usize % 7], t.wDay, M[(t.wMonth as usize).clamp(1, 12) - 1]))
}
fn battery(a: &mut App) -> (u8, bool) {
    if a.p.batt_t.is_none_or(|t| t.elapsed().as_secs() >= 30) {
        let mut s = SYSTEM_POWER_STATUS::default();
        a.p.batt = if unsafe { GetSystemPowerStatus(&mut s) }.is_ok() { (if s.BatteryLifePercent <= 100 { s.BatteryLifePercent } else { 100 }, s.ACLineStatus == 1) } else { (100, true) };
        a.p.batt_t = Some(Instant::now());
    }
    a.p.batt
}
/// icone de estado do sistema: bateria (notebook) ou rede (PC sem bateria): Wi-Fi com barras, cabeada ou sem rede
fn status_icon(a: &mut App, x: f32, y: f32, sz: f32) {
    use crate::netstat::Net;
    if has_battery() { let (pc, ch) = battery(a); a.gfx.icon(if ch { "battery-charging" } else if pc <= 20 { "battery-low" } else { "battery" }, x, y, sz, batt_col(pc, ch), false); return; }
    match crate::netstat::get() {
        Net::Eth => a.gfx.icon("ethernet-port", x, y, sz, TEXT2, false),
        Net::Off => a.gfx.icon("wifi-off", x, y, sz, DANGER, false),
        Net::Wifi(b) => { a.gfx.icon("wifi", x, y, sz, (1.0, 1.0, 1.0, 0.28), false); a.gfx.icon(match b { 4 => "wifi", 3 => "wifi-high", 2 => "wifi-low", _ => "wifi-zero" }, x, y, sz, TEXT2, false); }
    }
}
fn batt_col(p: u8, ch: bool) -> Rgba { if ch { OK } else if p <= 10 { DANGER } else if p <= 20 { ACCENT2 } else { TEXT2 } }

fn eq_bars(a: &mut App, x: f32, y: f32, playing: bool, t: f32, col: Rgba) {
    let periods = [0.9f32, 1.15, 1.4]; let phases = [0.0f32, 0.33, 0.66];
    for i in 0..3 {
        let hgt = if playing { if crate::island::reduced_motion() { [8.0, 12.0, 6.0][i] } else { 4.0 + 10.0 * (0.5 + 0.5 * (std::f32::consts::TAU * (t / periods[i] + phases[i])).sin()) } } else { 4.0 };
        a.gfx.rrect(x + i as f32 * 5.0, y + 14.0 - hgt, 3.0, hgt, 1.5, if playing { col } else { TEXT3 });
    }
}

fn draw_live_expanded(a: &mut App, w: f32, t: f32) {
    let v = media::view();
    if is_playing() || (!v.title.is_empty() && v.status != "stopped") {
        if let Some((g, px)) = a.p.cover.clone() { a.gfx.cover(g, &px, media::COVER_PX, 10.0, 8.0, 36.0, 8.0, 1.0); } else { a.gfx.rrect(10.0, 8.0, 36.0, 36.0, 8.0, SURFACE); a.gfx.icon("music", 17.0, 17.0, 18.0, TEXT3, false); }
        a.gfx.text(&v.title, Fmt::BodyStrong, 54.0, 7.0, 190.0, 18.0, TEXT, false);
        let l = if crate::settings::flag(crate::settings::F_LYRBAR) { lyric_line(a, Instant::now()) } else { None }.unwrap_or_else(|| v.artist.clone()); // 'letras no recolhido' desligado: o ao vivo mostra o artista
        a.gfx.text(&l, Fmt::Body, 54.0, 26.0, 190.0, 16.0, TEXT2, false);
        eq_bars(a, w - 48.0, 19.0, is_playing(), t, PAGE_COL[0]); // margens iguais de 10 px (QA5 C8)
        a.gfx.icon("eye-off", w - 26.0, 18.0, 16.0, TEXT3, false); // ocultar durante o evento (clique: N2/Configuracoes)
        if timer_running() { a.gfx.circle(w - 13.0, 12.0, 3.0, PAGE_COL[2]); }
    } else if timer_running() {
        let tm = timer::timer_get().unwrap();
        a.gfx.ring(26.0, 26.0, 13.0, 3.0, SURFACE_H); a.gfx.arc(26.0, 26.0, 13.0, 3.0, tm.remaining_sec as f32 / tm.total_sec.max(1) as f32, PAGE_COL[2]);
        a.gfx.text(&format!("{:02}:{:02}", tm.remaining_sec / 60, tm.remaining_sec % 60), Fmt::ClockLive, 52.0, 8.0, 120.0, 18.0, TEXT, false);
        let base = match tm.mode { timer::Mode::Focus => "Foco", timer::Mode::Short => "Pausa curta", timer::Mode::Long => "Pausa longa" };
        a.gfx.text(&if tm.cycle > 0 { format!("{base} · ciclo {}", tm.cycle) } else { base.to_string() }, Fmt::Small, 52.0, 28.0, 140.0, 14.0, TEXT2, false);
    }
}

/// So para screenshots/teste: estado da letra (1 carregando, 3 sem letra).
static FORCE_LYR: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
#[cfg(feature = "devtools")]
pub fn debug_fake_lines() { let mut l = lyr(); l.state = 2; l.instrumental = false; l.synced = true; l.key = "debug".into();
    l.lines = [(18000, "We're no strangers to love"), (22000, "You know the rules and so do I"), (27000, "A full commitment's what I'm thinking of"), (32000, "You wouldn't get this from any other guy"), (40000, "I just wanna tell you how I'm feeling"), (52000, "Gotta make you understand"), (56000, "Never gonna give you up"), (59000, "Never gonna let you down")].iter().map(|(t, x)| Line { time_ms: *t, text: x.to_string() }).collect(); }
#[cfg(feature = "devtools")]
pub fn debug_long_line() { let mut l = lyr(); l.state = 2; l.instrumental = false; l.synced = true; l.lines = vec![Line { time_ms: 0, text: "Esta é uma linha de letra bem longa que não cabe inteira na barra compacta".into() }, Line { time_ms: 500_000, text: "fim".into() }]; }
#[cfg(feature = "devtools")]
pub fn debug_instrumental() { let mut l = lyr(); l.state = 2; l.instrumental = true; l.synced = false; l.lines.clear(); }
#[cfg(feature = "devtools")]
pub fn debug_lyr(state: u8) { FORCE_LYR.store(state, std::sync::atomic::Ordering::Relaxed); let mut l = lyr(); l.state = state; l.lines.clear(); }

/// Olhos ociosos (aparencia recolhida): pupilas seguem o cursor (max 2 px, suavizado 80 ms), piscam a cada 3-7 s
fn draw_eyes(a: &mut App, w: f32, now: Instant) {
    let mut p = windows::Win32::Foundation::POINT::default(); unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut p); }
    let m = crate::island::cursor_mon_center(); let s = a.gfx.scale;
    let (tx, ty) = ((((p.x - m.0) as f32 / s) / 70.0).clamp(-1.0, 1.0) * 2.0, (((p.y - m.1) as f32 / s) / 30.0).clamp(0.0, 1.0) * 2.0 - 1.0);
    let ex = a.p.eye_x.get_or_insert(Tw::new(0.0)); ex.go(now, tx, 80.0); let px = ex.val(now);
    let ey = a.p.eye_y.get_or_insert(Tw::new(0.0)); ey.go(now, ty, 80.0); let py = ey.val(now);
    let sleepy = a.p.eye_move.is_some_and(|t| t.elapsed().as_secs() >= 60);
    let blink = a.p.blink_t.map_or(1.0, |t| { let ms = t.elapsed().as_millis() as f32; if ms < 120.0 { 1.0 - 0.9 * ms / 120.0 } else if ms < 240.0 { 0.1 + 0.9 * (ms - 120.0) / 120.0 } else { 1.0 } });
    let lid = if sleepy { blink.min(0.55) } else { blink };
    if sleepy { a.gfx.text("z", Fmt::Small, w / 2.0 + 18.0, 4.0, 10.0, 12.0, rgba(0xFFFFFF, 0.7), false); }
    for dx in [-9.0f32, 9.0] {
        a.gfx.ellipse(w / 2.0 + dx, 15.0, 4.0, 6.0 * lid, (1.0, 1.0, 1.0, 1.0));
        a.gfx.ellipse(w / 2.0 + dx + px * 0.8, 15.0 + py * 1.2 * lid, 2.5, (2.5 * lid).max(0.4), (0.0, 0.0, 0.0, 1.0));
    }
}

/// corta com reticencias para caber em `maxw` (linhas longas da letra)
fn fit(a: &App, s: &str, f: Fmt, maxw: f32) -> String {
    if a.gfx.measure(s, f) <= maxw { return s.to_string(); }
    let mut v: Vec<char> = s.chars().collect();
    while v.len() > 1 { v.pop(); let t: String = v.iter().collect::<String>().trim_end().to_string() + "…"; if a.gfx.measure(&t, f) <= maxw { return t; } }
    "…".into()
}
fn draw_lyric_bar(a: &mut App, w: f32, h: f32, rad: f32, now: Instant) {
    if let Some((tr, tg, tb)) = a.p.tint { a.gfx.rclip(w, h, rad); a.gfx.vgrad(0.0, 0.0, w, h, (tr, tg, tb, 1.0), 0.3, 1.0); a.gfx.unrclip(); } else { a.gfx.rclip(w, h, rad); a.gfx.vgrad(0.0, 0.0, w, h, (0.35, 0.35, 0.35, 1.0), 0.3, 1.0); a.gfx.unrclip(); }
    a.gfx.icon("music", 14.0, h / 2.0 - 6.0, 12.0, TEXT3, false);
    let (txt, soft) = bar_text(ui_pos(a, &media::view(), now));
    let key = format!("{soft}|{txt}");
    if key != a.p.lyr_key_bar { a.p.lyr_prev = a.p.lyr_cur.clone(); a.p.lyr_prev_soft = a.p.lyr_cur_soft; a.p.lyr_cur = txt.clone(); a.p.lyr_cur_soft = soft; a.p.lyr_key_bar = key; a.p.lyr_t = Some(now); }
    let f = crate::island::ease(a.p.lyr_t.map_or(1.0, |t| if crate::settings::flag(crate::settings::F_LYR_LINE) { 1.0 } else { (now - t).as_secs_f32() / 0.26 })); // troca de linha: deslize HORIZONTAL com mola leve (~260 ms)
    let put = |a: &mut App, s: &str, soft: bool, al: f32, dx: f32| { if s.is_empty() || al < 0.01 { return; } let fm = if soft { Fmt::Body } else { Fmt::BodyStrong }; let s = &fit(a, s, fm, w - 64.0); let tw = a.gfx.measure(s, fm).min(w - 64.0); a.gfx.layer(al * if soft { 0.62 } else { 1.0 }); a.gfx.text(s, fm, (w - tw) / 2.0 + dx, h / 2.0 - 9.0, tw + 4.0, 18.0, TEXT, false); a.gfx.unlayer(); };
    a.gfx.clip(30.0, 0.0, w - 60.0, h);
    if f < 1.0 { let (p, ps) = (a.p.lyr_prev.clone(), a.p.lyr_prev_soft); put(a, &p, ps, 1.0 - f, -22.0 * f); }
    let (c, cs) = (a.p.lyr_cur.clone(), a.p.lyr_cur_soft); put(a, &c, cs, f, 22.0 * (1.0 - f));
    a.gfx.unclip();
}

/// COMPACTO DE PC: [bateria (so notebook)] hora [ponto mic/camera] [estado: timer restante ou equalizador] [1-2 dados rotativos CPU/RAM/GPU/rede]
fn draw_peek_pc(a: &mut App, w: f32, t: f32, now: Instant) {
    let sn = crate::metrics::snap();
    let mut x = 14.0;
    status_icon(a, x, 7.0, 16.0); x += 22.0;
    let (hm, _) = clock_str(); let cw = a.gfx.measure(&hm, Fmt::ClockLive);
    a.gfx.text(&hm, Fmt::ClockLive, x, 5.0, cw + 4.0, 20.0, TEXT, false); x += cw + 8.0;
    if sn.mic || sn.cam { a.gfx.circle(x + 3.0, 15.0, 3.0, if sn.mic { ACCENT2 } else { OK }); x += 12.0; } // ponto: laranja = microfone, verde = camera em uso
    if crate::settings::flag(crate::settings::F_PC_STATE) {
        if timer_running() { if let Ok(tm) = timer::timer_get() { a.gfx.icon("timer", x, 9.0, 12.0, PAGE_COL[2], false); a.gfx.text(&format!("{:02}:{:02}", tm.remaining_sec / 60, tm.remaining_sec % 60), Fmt::Small, x + 16.0, 8.0, 40.0, 14.0, TEXT, false); } }
        else if is_playing() { eq_bars(a, x + 2.0, 8.0, true, t, PAGE_COL[0]); }
    }
    // dados rotativos: grupos de `slots` itens, troca a cada PC_ROT s com deslize horizontal (300 ms)
    let ml = pc_metrics(); if ml.is_empty() { return; }
    let n = pc_slots(); let groups = ml.len().div_ceil(n); let rot = crate::settings::PC_ROT.load(std::sync::atomic::Ordering::Relaxed).max(1) as f32;
    let t0 = *a.p.pc_t0.get_or_insert(now); let g = if groups > 1 { ((now - t0).as_secs_f32() / rot) as usize % groups } else { 0 };
    if g != a.p.pc_g { a.p.pc_prev = a.p.pc_g; a.p.pc_g = g; a.p.pc_chg = Some(now); }
    let f = crate::island::ease(a.p.pc_chg.map_or(1.0, |c| (now - c).as_secs_f32() / 0.3));
    let x0 = w - 10.0 - n as f32 * PC_SLOT_W;
    a.gfx.clip(x0 - 2.0, 0.0, n as f32 * PC_SLOT_W + 4.0, 30.0);
    let mut draw_group = |a: &mut App, gi: usize, al: f32, dx: f32| { if al < 0.01 { return; } a.gfx.layer(al);
        for k in 0..n { let idx = gi * n + k; let Some(m) = ml.get(idx) else { continue }; pc_item(a, *m, &sn, x0 + k as f32 * PC_SLOT_W + dx); } a.gfx.unlayer(); };
    if f < 1.0 { let p = a.p.pc_prev; draw_group(a, p, 1.0 - f, -14.0 * f); }
    draw_group(a, g, f, 14.0 * (1.0 - f));
    a.gfx.unclip();
}
fn pc_item(a: &mut App, m: crate::metrics::Metric, s: &crate::metrics::Snap, x: f32) {
    use crate::metrics::Metric;
    let bar = |a: &mut App, pct: f32, col: Rgba| { let col = if pct >= 90.0 { DANGER } else if pct >= 75.0 { ACCENT2 } else { col }; a.gfx.rrect(x, 24.0, PC_SLOT_W - 8.0, 2.0, 1.0, SURFACE_H); a.gfx.rrect(x, 24.0, (PC_SLOT_W - 8.0) * (pct / 100.0).clamp(0.0, 1.0), 2.0, 1.0, col); };
    match m {
        Metric::Cpu | Metric::Ram | Metric::Gpu => {
            let (lbl, v, col) = match m { Metric::Cpu => ("CPU", Some(s.cpu), PAGE_COL[0]), Metric::Ram => ("RAM", Some(s.ram), rgba(0x7CC4FF, 1.0)), _ => ("GPU", s.gpu, rgba(0xFF9F6B, 1.0)) };
            a.gfx.text(lbl, Fmt::Small, x, 4.0, 28.0, 14.0, TEXT3, false);
            let txt = v.map_or("—".to_string(), |v| format!("{:.0}%", v)); a.gfx.text(&txt, Fmt::BodyStrong, x + 28.0, 3.0, 34.0, 16.0, TEXT, false);
            bar(a, v.unwrap_or(0.0), col);
        }
        Metric::Net => {
            a.gfx.text(&format!("↓ {}", crate::metrics::fmt_rate(s.down)), Fmt::Small, x, 2.0, PC_SLOT_W - 4.0, 13.0, TEXT, false);
            a.gfx.text(&format!("↑ {}", crate::metrics::fmt_rate(s.up)), Fmt::Small, x, 15.0, PC_SLOT_W - 4.0, 13.0, TEXT2, false);
        }
    }
}

fn draw_peek_live(a: &mut App, w: f32, t: f32) {
    if a.state == St::Peek && crate::settings::get().collapsed_look == "eyes" { return draw_eyes(a, w, Instant::now()); }
    if a.state == St::Peek && pc_mode() { return draw_peek_pc(a, w, t, Instant::now()); }
    let (hm, _) = clock_str(); let (pc, ch) = battery(a);
    let bx = 14.0; let _ = (pc, ch);
    status_icon(a, bx, 7.0, 16.0);
    if a.state == St::Peek { a.gfx.text(&hm, Fmt::ClockLive, w - 14.0 - 64.0, 5.0, 64.0, 20.0, TEXT, true); } // peek: hora a direita (mock 22)
    else { a.gfx.text(&hm, Fmt::ClockLive, 40.0, 5.0, 56.0, 20.0, TEXT, false); }
    if a.state == St::Live || is_live() && w > 160.0 {
        let (pl, tm) = (is_playing(), timer_running());
        let mut x = w - 14.0 - 13.0;
        if pl { eq_bars(a, x, 8.0, true, t, PAGE_COL[0]); x -= 22.0; }
        if tm { a.gfx.icon("timer", x - 3.0, 7.0, 16.0, PAGE_COL[2], false); }
    }
}

fn draw_open(a: &mut App, now: Instant, w: f32, h: f32, t: f32, hits: &mut Vec<(R, Act)>) {
    let dx = (w - 420.0) / 2.0; // layout normal fica centrado enquanto a largura anima
    // split segue o ESTADO (nao a largura: a mola da largura passa de 420 ao abrir e mostrava a letra por baixo do player)
    a.split_t.go(now, if a.split && a.page == 0 { 1.0 } else { 0.0 }, 240.0); let split_f = a.split_t.val(now).clamp(0.0, 1.0);
    let pv = a.page_t.pos; let busy = a.page_t.busy(now); let slide = crate::island::PAGE_SLIDE_PX * a.dir;
    let (pa, pb) = (if busy { 1.0 - pv.clamp(0.0, 1.0) } else { 0.0 }, if busy { pv.clamp(0.0, 1.0) } else { 1.0 });
    let prev = a.prev_page;
    if busy && pa > 0.01 { page_layer(a, now, prev, dx, pa, -slide * pv.clamp(0.0, 1.0), split_f, t, w, &mut Vec::new()); }
    page_layer(a, now, a.page, dx, pb, if busy { slide * (1.0 - pv) } else { 0.0 }, split_f, t, w, hits);
    chrome(a, now, w, h, dx, split_f, t, hits);
}

fn page_layer(a: &mut App, now: Instant, page: usize, dx: f32, alpha: f32, off: f32, split_f: f32, t: f32, w: f32, hits: &mut Vec<(R, Act)>) {
    if alpha < 0.01 { return; }
    a.gfx.layer(alpha);
    match page {
        0 => {
            if split_f < 0.99 { a.gfx.layer(1.0 - split_f); a.gfx.set_t(origin(a, w) + (dx + off) * crate::layout::k(), 0.0, crate::layout::k()); music_normal(a, now, t, hits, dx + off); a.gfx.unlayer(); }
            if split_f > 0.01 { a.gfx.layer(split_f); a.gfx.set_t(origin(a, w), 0.0, crate::layout::k()); music_split(a, now, hits); a.gfx.unlayer(); }
        }
        1 => { a.gfx.set_t(origin(a, w) + (dx + off) * crate::layout::k(), 0.0, crate::layout::k()); crate::pages2::shelf_bg(a, now); header(a, "Shelf", 1, t, now); shelf(a, hits, dx + off, now); }
        2 => { a.gfx.set_t(origin(a, w) + (dx + off) * crate::layout::k(), 0.0, crate::layout::k()); header(a, "Pomodoro", 2, t, now); pomodoro(a, hits, dx + off, now); }
        _ => { a.gfx.set_t(origin(a, w) + (dx + off) * crate::layout::k(), 0.0, crate::layout::k()); header(a, "Tarefas", 3, t, now); tarefas(a, hits, dx + off); }
    }
    a.gfx.set_t(origin(a, w), 0.0, crate::layout::k());
    a.gfx.unlayer();
}
/// origem x (px logicos da janela) da ilha atual
fn origin(_a: &App, w: f32) -> f32 { (crate::layout::win_w() - w * crate::layout::k()) / 2.0 } // `w` em unidades de projeto

fn header(a: &mut App, title: &str, page: usize, t: f32, now: Instant) {
    // Shelf: o cabecalho sobe 4 px (excecao documentada, DESIGN-NATIVO 14.1) para caber a caixa de 96
    let up = -4.0; // titulo com y=8 em TODAS as paginas (QA5)
    a.gfx.text(title, Fmt::Title, 24.0, 12.0 + up, 208.0, 20.0, PAGE_COL[page], false);
    // Shelf com itens: os botoes ocupam o lugar de data/hora/bateria; some/volta em fade (0,3 s)
    let ha = if page == P_SHELF { crate::pages2::shelf_header_alpha(a, now) } else { 1.0 };
    if ha > 0.01 {
        a.gfx.layer(ha); let k = crate::layout::k(); a.gfx.set_t((crate::layout::win_w() - 420.0 * k) / 2.0, up * k, k);
        let (hm, date) = clock_str(); let (pc, ch) = battery(a);
        // grupo bateria · data · hora alinhado a direita, 8 px entre itens; termina em x=372 (o indicador ao vivo fica em 382..404; com timer+musica cede 18 px)
        let edge = 372.0 - if is_playing() && timer_running() { 18.0 } else { 0.0 };
        let (wh, wd) = (a.gfx.measure(&hm, Fmt::BodyStrong), a.gfx.measure(&date, Fmt::Body));
        a.gfx.text(&hm, Fmt::BodyStrong, edge - wh - 2.0, 14.0, wh + 4.0, 16.0, TEXT, true);
        a.gfx.text(&date, Fmt::Body, edge - wh - 8.0 - wd - 2.0, 14.0, wd + 4.0, 16.0, TEXT2, true);
        let _ = (pc, ch); status_icon(a, edge - wh - 8.0 - wd - 8.0 - 16.0, 14.0, 16.0);
        a.gfx.set_t((crate::layout::win_w() - 420.0 * k) / 2.0, 0.0, k); a.gfx.unlayer();
    }
    live_indicator(a, t);
}
fn live_indicator(a: &mut App, t: f32) {
    let (pl, tm) = (is_playing(), timer_running());
    let mut x = 404.0 - 13.0;
    if pl { eq_bars(a, x, 12.0, true, t, PAGE_COL[0]); x -= 20.0; }
    if tm { a.gfx.icon("timer", x - 1.0, 12.0, 14.0, PAGE_COL[2], false); }
}

fn hov(a: &App, act: Act) -> bool { a.hover == Some(act) }
fn button_bg(a: &mut App, hits: &mut Vec<(R, Act)>, rect: R, act: Act, off: f32, radius: f32) {
    let pressed = a.pressed == Some(act);
    if hov(a, act) || pressed { a.gfx.rrect(rect.x, rect.y, rect.w, rect.h, radius, if pressed { rgba(0xFFFFFF, 0.20) } else { SURFACE_H }); }
    hits.push((R { x: rect.x + off, ..rect }, act));
}

/// capa com o giro de troca de faixa (so a largura encolhe/volta; leve perda de altura = perspectiva)
fn draw_cover(a: &mut App, now: Instant, x: f32, y: f32, size: f32, r: f32, stroke: bool) {
    let ms = spin_ms(a, now);
    let (cover, sx) = match (ms, a.p.spin.as_ref()) { (Some(m), Some(s)) => { let (k, new) = spin_scale(m); (if new { a.p.cover.clone() } else { s.cover.clone() }, k) } _ => (a.p.cover.clone(), 1.0) };
    let m = if sx < 0.999 { Some(a.gfx.scale_xy(x + size / 2.0, y + size / 2.0, sx, 1.0 - 0.07 * (1.0 - sx))) } else { None };
    if let Some((g, px)) = cover { a.gfx.cover(g, &px, media::COVER_PX, x, y, size, r, 1.0); if stroke { a.gfx.rrect_stroke(x + 0.5, y + 0.5, size - 1.0, size - 1.0, r, 1.0, LINE_C); } }
    else { a.gfx.rrect(x, y, size, size, r, SURFACE); a.gfx.icon("music", x + size / 2.0 - 12.0, y + size / 2.0 - 12.0, 24.0, TEXT3, false); }
    if let Some(m) = m { a.gfx.set_matrix(m); }
}
fn music_normal(a: &mut App, now: Instant, t: f32, hits: &mut Vec<(R, Act)>, off: f32) {
    let v = media::view();
    let empty = v.title.is_empty() && v.status == "stopped";
    // capa
    draw_cover(a, now, 24.0, 12.0, 62.0, 10.0, true);
    let col = if empty { rgba(0xFFFFFF, 0.35) } else { TEXT };
    if empty {
        a.gfx.text("Nada tocando", Fmt::Title, 98.0, 18.0, 260.0, 20.0, TEXT2, false);
        a.gfx.text("Abra um player de música", Fmt::Body, 98.0, 40.0, 260.0, 16.0, TEXT3, false);
    } else {
        let sub = media::current().album; let sub = if sub.is_empty() { v.artist.clone() } else { format!("{} — {}", v.artist, sub) };
        let lyr_on = crate::settings::get().show_lyric_line;
        let has_lyric = lyr_on && lyric_line(a, now).is_some();
        let sy = if has_lyric { 33.0 } else { 31.0 };
        let fade = spin_ms(a, now).map(|m| (m / SPIN_FADE_MS).clamp(0.0, 1.0)).filter(|f| *f < 1.0);
        if let (Some(f), Some((ot, os))) = (fade, a.p.spin.as_ref().map(|s| (s.title.clone(), s.sub.clone()))) { // crossfade titulo/subtitulo antigo -> novo
            a.gfx.layer(1.0 - f); a.gfx.text(&ot, Fmt::Title, 98.0, 12.0, 262.0, 20.0, TEXT, false); a.gfx.text(&os, Fmt::Body, 98.0, sy, 270.0, 14.0, TEXT2, false); a.gfx.unlayer();
            a.gfx.layer(f); a.gfx.text(&v.title, Fmt::Title, 98.0, 12.0, 262.0, 20.0, TEXT, false); a.gfx.text(&sub, Fmt::Body, 98.0, sy, 270.0, 14.0, TEXT2, false); a.gfx.unlayer();
        } else { a.gfx.text(&v.title, Fmt::Title, 98.0, 12.0, 262.0, 20.0, TEXT, false); a.gfx.text(&sub, Fmt::Body, 98.0, sy, 270.0, 14.0, TEXT2, false); }
        a.p.shown_title = v.title.clone(); a.p.shown_sub = sub.clone();
        if lyr_on { draw_lyric_line(a, now); }
    }
    live_indicator(a, t);
    // progresso (seek otimista)
    let pos = ui_pos(a, &v, now);
    let dur = v.dur_ms.max(1);
    let frac = (pos as f32 / dur as f32).clamp(0.0, 1.0);
    a.gfx.text(&fmt_t(pos), Fmt::Body, 24.0, 78.0, 44.0, 14.0, col, false);
    a.gfx.text(&format!("-{}", fmt_t(v.dur_ms.saturating_sub(pos))), Fmt::Body, 352.0, 78.0, 44.0, 14.0, col, true);
    let big = hov(a, Act::Seek(R2(720, 2760))) || a.seek_drag;
    let bh = if big { 6.0 } else { 4.0 };
    a.gfx.rrect(72.0, 85.0 - bh / 2.0, 276.0, bh, 2.0, SURFACE_H);
    a.gfx.rrect(72.0, 85.0 - bh / 2.0, (276.0 * frac).max(0.0), bh, 2.0, TEXT);
    hits.push((r(72.0 + off, 76.0, 276.0, 18.0), Act::Seek(R2(720, 2760))));
    // controles
    let dis = if empty { 0.35 } else { 1.0 };
    let vol_open = a.p.vol_t.is_some_and(|t| now.duration_since(t).as_millis() < 1500) || a.p.vol_drag;
    if vol_open { // slider ~65 px ao lado do alto-falante; os controles de transporte continuam visiveis (mock 03)
        let vv = a.p.vol.clamp(0.0, 1.0);
        a.gfx.rrect(296.0, 107.0, 65.0, 6.0, 3.0, SURFACE_H); a.gfx.rrect(296.0, 107.0, 65.0 * vv, 6.0, 3.0, TEXT);
        hits.push((r(290.0 + off, 98.0, 77.0, 24.0), Act::VolSlider));
    }
    let (p0, n0) = (r(160.0, 97.0, 26.0, 26.0), r(234.0, 97.0, 26.0, 26.0)); // grupo centrado em x=210 (QA5 C1: prev 173 / play 210 / next 247)
    if !empty { button_bg(a, hits, p0, Act::Prev, off, 13.0); } if true { a.gfx.icon("skip-back", 165.0, 102.0, 16.0, (1.0, 1.0, 1.0, dis), true);
    button_bg(a, hits, n0, Act::Next, off, 13.0); a.gfx.icon("skip-forward", 239.0, 102.0, 16.0, (1.0, 1.0, 1.0, dis), true); }
    let pc = if a.pressed == Some(Act::Play) { rgba(0xCFCFCF, 1.0) } else if hov(a, Act::Play) { rgba(0xE6E6E6, 1.0) } else { rgba(0xFFFFFF, dis) };
    {
    a.gfx.rrect(192.0, 95.0, 36.0, 30.0, 15.0, pc); // 7 px abaixo da barra (antes encostava)
    a.gfx.icon(if v.status == "playing" { "pause" } else { "play" }, if v.status == "playing" { 201.0 } else { 202.0 }, 101.0, 18.0, (0.0, 0.0, 0.0, 1.0), true); // play: +1 px (compensacao optica)
    if !empty { hits.push((r(192.0 + off, 95.0, 36.0, 30.0), Act::Play)); } }
    let lb = r(20.0, 98.0, 24.0, 24.0); // icone do balao em x=24..40 (alinha com o tempo)
    if a.split { a.gfx.rrect(lb.x, lb.y, lb.w, lb.h, 9.0, rgba(0xFFFFFF, 0.16)); } else { button_bg(a, hits, lb, Act::Lyrics, off, 9.0); }
    a.gfx.icon("message-square-quote", 24.0, 102.0, 16.0, if a.split { TEXT } else { TEXT2 }, false);
    hits.push((R { x: lb.x + off, ..lb }, Act::Lyrics));
    let vb = r(376.0, 98.0, 24.0, 24.0); // icone de volume em x=380..396 (alinha com o tempo restante)
    button_bg(a, hits, vb, Act::Volume, off, 9.0); a.gfx.icon(if a.p.vol_muted || (vol_open && a.p.vol <= 0.001) { "volume-x" } else if vol_open && a.p.vol < 0.5 { "volume-1" } else { "volume-2" }, 380.0, 102.0, 16.0, if vol_open { TEXT } else { TEXT2 }, false);
}

/// posicao mostrada: otimista por 400 ms apos o seek
fn ui_pos(a: &mut App, v: &media::View, now: Instant) -> u64 {
    if let Some((p, t0)) = a.seek_opt {
        let el = (now - t0).as_millis() as u64;
        if a.seek_drag { return p; }
        if el < 400 { return p + if v.status == "playing" { el } else { 0 }; }
        a.seek_opt = None;
    }
    v.pos_ms
}

fn lyric_line(a: &mut App, _now: Instant) -> Option<String> {
    let v = media::view(); let l = lyr();
    if l.state != 2 || !l.synced { return None; }
    let i = lyric_idx(&l, v.pos_ms); if i < 0 { return None; }
    let _ = a;
    let t = l.lines[i as usize].text.trim().to_string(); Some(if t.is_empty() { "♪".into() } else { t })
}
fn draw_lyric_line(a: &mut App, now: Instant) {
    let cur = lyric_line(a, now);
    if lyr().state != 2 { a.p.lyr_cur.clear(); a.p.lyr_prev.clear(); a.p.lyr_idx = -2; return; } // carregando/sem letra: nenhuma linha
    let idx = { let v = media::view(); let l = lyr(); lyric_idx(&l, v.pos_ms) };
    if idx != a.p.lyr_idx { a.p.lyr_prev = a.p.lyr_prev_text(); a.p.lyr_idx = idx; a.p.lyr_t = Some(now); a.p.lyr_cur = cur.clone().unwrap_or_default(); }
    let f = a.p.lyr_t.map_or(1.0, |t| if crate::settings::flag(crate::settings::F_LYR_LINE) { 1.0 } else { crate::island::ease((now - t).as_secs_f32() / 0.22) });
    if f < 1.0 {
        a.gfx.layer(1.0 - f); a.gfx.text(&a.p.lyr_prev.clone(), Fmt::Body, 98.0, 52.0 - 6.0 * f, 262.0, 18.0, TEXT, false); a.gfx.unlayer();
        a.gfx.layer(f); a.gfx.text(&a.p.lyr_cur.clone(), Fmt::Body, 98.0, 52.0 + 6.0 * (1.0 - f), 262.0, 18.0, TEXT, false); a.gfx.unlayer();
    } else { a.gfx.text(&a.p.lyr_cur.clone(), Fmt::Body, 98.0, 52.0, 262.0, 18.0, TEXT, false); }
}
impl PageState { fn lyr_prev_text(&self) -> String { self.lyr_cur.clone() } }

fn music_split(a: &mut App, now: Instant, hits: &mut Vec<(R, Act)>) {
    let v = media::view();
    draw_cover(a, now, 20.0, 14.0, 56.0, 9.0, false);
    a.gfx.text(if v.title.is_empty() { "Nada tocando" } else { &v.title }, Fmt::Title, 88.0, 14.0, 148.0, 18.0, TEXT, false);
    a.gfx.text(&v.artist, Fmt::Body, 88.0, 34.0, 148.0, 14.0, TEXT2, false);
    let pos = ui_pos(a, &v, now); let dur = v.dur_ms.max(1); let frac = (pos as f32 / dur as f32).clamp(0.0, 1.0);
    a.gfx.text(&fmt_t(pos), Fmt::Body, 18.0, 76.0, 34.0, 14.0, TEXT, false);
    a.gfx.text(&format!("-{}", fmt_t(v.dur_ms.saturating_sub(pos))), Fmt::Body, 200.0, 76.0, 40.0, 14.0, TEXT, true);
    let act = Act::Seek(R2(560, 1400)); let big = hov(a, act) || a.seek_drag; let bh = if big { 6.0 } else { 4.0 };
    a.gfx.rrect(56.0, 83.0 - bh / 2.0, 140.0, bh, 2.0, SURFACE_H); a.gfx.rrect(56.0, 83.0 - bh / 2.0, 140.0 * frac, bh, 2.0, TEXT);
    hits.push((r(56.0, 74.0, 140.0, 18.0), act));
    let lb = r(16.0, 98.0, 24.0, 24.0); a.gfx.rrect(lb.x, lb.y, lb.w, lb.h, 9.0, rgba(0xFFFFFF, 0.16)); a.gfx.icon("message-square-quote", 20.0, 102.0, 16.0, TEXT, false); hits.push((lb, Act::Lyrics));
    button_bg(a, hits, r(72.0, 97.0, 26.0, 26.0), Act::Prev, 0.0, 13.0); a.gfx.icon("skip-back", 77.0, 102.0, 16.0, TEXT, true);
    button_bg(a, hits, r(146.0, 97.0, 26.0, 26.0), Act::Next, 0.0, 13.0); a.gfx.icon("skip-forward", 151.0, 102.0, 16.0, TEXT, true);
    a.gfx.rrect(104.0, 95.0, 36.0, 30.0, 15.0, if hov(a, Act::Play) { rgba(0xE6E6E6, 1.0) } else { TEXT });
    a.gfx.icon(if v.status == "playing" { "pause" } else { "play" }, 113.0, 101.0, 18.0, (0.0, 0.0, 0.0, 1.0), true); hits.push((r(104.0, 95.0, 36.0, 30.0), Act::Play));
    a.gfx.line(250.5, 16.0, 250.5, 120.0, 1.0, LINE_C);
    // painel de letras
    let l = lyr();
    let (vx, vy, vw, vh) = (268.0f32, 14.0f32, 236.0f32, 108.0f32);
    match l.state {
        1 => { a.gfx.text("Carregando letras…", Fmt::Body, vx, vy + 44.0, vw, 18.0, TEXT2, false); }
        3 => { a.gfx.icon("message-square-quote", vx + vw / 2.0 - 10.0, 40.0, 20.0, TEXT3, false); a.gfx.text("Sem letra para esta faixa", Fmt::Body, vx, 68.0, vw, 18.0, TEXT2, false); }
        2 if l.instrumental => { a.gfx.icon("music", vx + vw / 2.0 - 10.0, 40.0, 20.0, TEXT3, false); a.gfx.text("Faixa instrumental", Fmt::Body, vx, 68.0, vw, 18.0, TEXT2, false); }
        2 if l.synced => {
            let idx = lyric_idx(&l, v.pos_ms).max(0);
            // rolagem: acompanha a linha atual; se o usuario rolou com a roda, pausa o auto-scroll por 3 s e depois volta suave
            let follow = match a.p.lyr_user { Some((t, c)) if t.elapsed().as_millis() < 3000 => c, _ => idx as f32 };
            let sc = a.p.scroll.get_or_insert(Tw::new(follow));
            if sc.target() != follow { sc.go(now, follow, if crate::settings::flag(crate::settings::F_LYR_LINE) { 0.0 } else { 350.0 }); } // "por linha": salta sem interpolar
            let s = sc.val(now); let base = s.round() as i64;
            let hov_line = if let Some(Act::LyricSeek(j)) = a.hover { Some(j as i64) } else { None };
            a.gfx.clip(vx - 8.0, vy, vw + 16.0, vh);
            for k in -3i64..=3 {
                let j = base + k; if j < 0 || j as usize >= l.lines.len() { continue; }
                let y = 68.0 + (j as f32 - s) * 22.0 - 9.0;
                let edge = ((y + 9.0 - vy).min(vy + vh - (y + 9.0)) / 28.0).clamp(0.0, 1.0);
                let is_now = j == idx; let hv = hov_line == Some(j);
                if is_now || hv { a.gfx.rrect(vx - 6.0, y - 2.0, vw + 12.0, 22.0, 8.0, rgba(0xFFFFFF, if is_now { 0.10 } else { 0.06 } * edge)); }
                let txt = l.lines[j as usize].text.trim(); let txt = if txt.is_empty() { "♪" } else { txt };
                a.gfx.text(txt, if is_now { Fmt::BodyStrong } else { Fmt::Body }, vx, y, vw, 18.0, (1.0, 1.0, 1.0, edge * if is_now { 1.0 } else if hv { 0.8 } else { 0.42 }), false);
                if edge > 0.3 { hits.push((r(vx - 6.0, y - 2.0, vw + 12.0, 22.0), Act::LyricSeek(j as u32))); }
            }
            a.gfx.unclip();
            if media::seek_failed_within(2500) { let m = "Este player não permite pular"; let tw = a.gfx.measure(m, Fmt::Small); a.gfx.rrect(vx + (vw - tw) / 2.0 - 8.0, vy + vh - 22.0, tw + 16.0, 20.0, 10.0, rgba(0x000000, 0.8)); a.gfx.text(m, Fmt::Small, vx + (vw - tw) / 2.0, vy + vh - 20.0, tw + 4.0, 16.0, rgba(0xFFC857, 1.0), false); }
        }
        2 => { let mut y = 24.0; for ln in l.lines.iter().take(5) { a.gfx.text(&ln.text, Fmt::Body, vx, y, vw, 18.0, TEXT2, false); y += 22.0; } }
        _ => { a.gfx.text("Toque uma música para ver a letra", Fmt::Body, vx, 60.0, vw, 18.0, TEXT2, false); }
    }
}

fn pomodoro(a: &mut App, hits: &mut Vec<(R, Act)>, off: f32, _now: Instant) { crate::pages2::pomodoro(a, hits, off); }
fn shelf(a: &mut App, hits: &mut Vec<(R, Act)>, off: f32, now: Instant) { crate::pages2::shelf(a, hits, off, now); }
fn tarefas(a: &mut App, hits: &mut Vec<(R, Act)>, off: f32) { crate::tasks::draw(a, hits, off); }

/// setas, pontos, dock
fn chrome(a: &mut App, now: Instant, w: f32, _h: f32, dx: f32, split_f: f32, _t: f32, hits: &mut Vec<(R, Act)>) {
    a.gfx.set_t(origin(a, w) + dx * crate::layout::k(), 0.0, crate::layout::k());
    let dock_f = a.dock_t.val(now).clamp(0.0, 1.0);
    if split_f < 0.5 {
        let al = 1.0 - split_f * 2.0;
        a.gfx.layer(al);
        for (x, ic, d) in [(3.0f32, "chevron-left", -1i8), (399.0, "chevron-right", 1)] {
            let act = Act::Arrow(d);
            a.gfx.circle(x + 9.0, 75.0, 9.0, if hov(a, act) { SURFACE_H } else { SURFACE });
            a.gfx.icon(ic, x + 3.0, 69.0, 12.0, TEXT2, false);
            hits.push((r(x + dx, 66.0, 18.0, 18.0), act));
        }
        if dock_f < 0.99 && !(a.page == 3 && a.p.task_cal) { a.gfx.layer(1.0 - dock_f);
            // pontos: 4 px, ativo = pilula 15x4, espacamento 8,5 px. Na troca a pilula antiga encolhe e a nova cresce (~0,23 s), sem gota
            let tt = a.p.dots_at.map_or(1.0, |(t0, _)| ((now - t0).as_secs_f32() / 0.23).min(1.0)); let from = a.p.dots_at.map_or(a.page, |d| d.1); let e = crate::island::ease(tt);
            let wd: Vec<f32> = (0..4).map(|i| { let wgt = if tt >= 1.0 { (i == a.page) as u8 as f32 } else if i == a.page { e } else if i == from { 1.0 - e } else { 0.0 }; 4.0 + 11.0 * wgt }).collect();
            let gap = 8.5; let total: f32 = wd.iter().sum::<f32>() + 3.0 * gap; let mut x = 210.0 - total / 2.0; 
            for i in 0..4 { let act_on = i == a.page; a.gfx.rrect(x, 131.0, wd[i], 4.0, 2.0, if act_on { (1.0, 1.0, 1.0, 1.0) } else { (1.0, 1.0, 1.0, 0.55) }); x += wd[i] + gap; }
            a.gfx.unlayer(); }
        a.gfx.unlayer();
    }
    if dock_f > 0.01 {
        a.gfx.layer(dock_f);
        let y0 = 136.0; let x0 = 114.0;
        for i in 0..4usize {
            let rect = r(x0 + 36.0 * i as f32, y0, 36.0, 28.0); let on = a.page == i; let act = Act::Page(i as u8);
            if on { a.gfx.rrect(rect.x + 2.0, 139.0, 32.0, 22.0, 9.0, (PAGE_COL[i].0, PAGE_COL[i].1, PAGE_COL[i].2, 0.16)); }
            let ic = if on { PAGE_COL[i] } else if hov(a, act) { TEXT } else { TEXT2 };
            a.gfx.icon(PAGE_ICONS[i], rect.x + 10.0, 142.0, 16.0, ic, false);
            hits.push((R { x: rect.x + dx, ..rect }, act));
        }
        a.gfx.line(x0 + 144.0 + 6.0, 144.0, x0 + 144.0 + 6.0, 156.0, 1.0, LINE_C);
        let sr = r(x0 + 156.0, y0, 36.0, 28.0);
        let ic = if hov(a, Act::Settings) { TEXT } else { TEXT2 };
        a.gfx.icon("settings", sr.x + 10.0, 142.0, 16.0, ic, false);
        hits.push((R { x: sr.x + dx, ..sr }, Act::Settings));
        a.gfx.unlayer();
    }
    let _ = PAGE_NAMES;
}

// ---- entrada ----
pub fn mouse(a: &mut App, m: u32, wp: WPARAM, lp: LPARAM) {
    if m == WM_LBUTTONDOWN && a.state != St::Open && crate::settings::flag(crate::settings::F_CLICK) { a.click_open = true; a.evaluate_pub(); return; } // modo 'Clique': o primeiro clique no notch abre
    let s = a.gfx.scale; let (cx, cy) = ((lp.0 & 0xFFFF) as i16 as f32 / s, ((lp.0 >> 16) & 0xFFFF) as i16 as f32 / s);
    let w = a.tw.val(Instant::now()); let kk = crate::layout::k(); let (x, y) = ((cx - (crate::layout::win_w() - w) / 2.0) / kk, cy / kk); // coordenadas de projeto
    let hit = a.hits.iter().rev().find(|(rc, _)| rc.has(x, y)).map(|(rc, act)| (*rc, *act));
    match m {
        WM_MOUSEMOVE => {
            let nh = hit.map(|h| h.1);
            if nh != a.hover { a.hover = nh; a.dirty = true; }
            if matches!(nh, Some(Act::Volume) | Some(Act::VolSlider)) && !a.p.vol_drag { // hover no volume: mostra o slider (some ~0,8 s depois de sair)
                let was_open = a.p.vol_t.is_some_and(|t| t.elapsed().as_millis() < 1500);
                if !was_open { let app = media::current().app_name.unwrap_or_default(); a.p.vol = crate::vol::app_get(&app); a.p.vol_muted = a.p.vol <= 0.001; }
                a.p.vol_t = Some(Instant::now() - std::time::Duration::from_millis(700)); a.dirty = true;
                unsafe { SetTimer(Some(a.hwnd), crate::island::T_VOL, 900, None); } // um quadro quando o prazo vence (parado nao ha quadros)
            }
            if let Some((i, x0, y0)) = a.p.drag_from { if (x - x0).abs() + (y - y0).abs() > 6.0 && LBUTTON_DOWN_NOW() {
                // arrastar o card para fora: DoDragDrop (so copia) numa thread propria; o clique nao abre o arquivo
                a.p.drag_from = None; a.pressed = None;
                if let Some(it) = a.p.shelf.get(i as usize) { crate::log(&format!("arrastar card {i} -> {:?}", crate::shelf::start_drag(&it.id.clone()))); }
            } }
            if a.p.vol_drag { if let Some((rc, _)) = a.hits.iter().find(|(_, ac)| *ac == Act::VolSlider).copied() { vol_to(a, x, rc); } }
            if a.seek_drag { if let Some((rc, Act::Seek(_))) = a.hits.iter().find(|(_, ac)| matches!(ac, Act::Seek(_))).copied() { seek_to(a, x, rc, false); } }
            unsafe { let mut t = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: a.hwnd, dwHoverTime: 0 }; let _ = TrackMouseEvent(&mut t); }
        }
        0x02A3 => { a.hover = None; a.dirty = true; }
        WM_LBUTTONDOWN => {
            crate::log(&format!("clique em ({x:.0},{y:.0}) -> {:?} | hits={}", hit.map(|h| h.1), a.hits.len()));
            if let Some((rc, act)) = hit {
                a.pressed = Some(act); a.dirty = true;
                if act == Act::VolSlider { a.p.vol_drag = true; vol_to(a, x, rc); }
                if let Act::ShelfOpen(i) = act { if !a.p.select { a.p.drag_from = Some((i, x, y)); } }
                if let Act::Seek(_) = act { a.seek_drag = true; seek_to(a, x, rc, false); }
            }
        }
        WM_LBUTTONUP => {
            let was = a.pressed.take(); a.dirty = true; a.p.drag_from = None;
            if a.p.vol_drag { a.p.vol_drag = false; a.p.vol_t = Some(Instant::now()); return; }
            if a.seek_drag { a.seek_drag = false; if let Some((rc, _)) = a.hits.iter().find(|(_, ac)| matches!(ac, Act::Seek(_))).copied() { seek_to(a, x, rc, true); } return; }
            if let (Some(p), Some((_, act))) = (was, hit) { if p == act { click(a, act); } }
        }
        WM_MOUSEWHEEL => {
            let d = ((wp.0 >> 16) & 0xFFFF) as i16;
            if a.page == 3 && a.p.tasks.len() > 3 && x > 222.0 && x < 396.0 && y > 40.0 && y < 118.0 { // roda sobre a lista de tarefas: rola a lista
                let n = a.p.tasks.len(); if d < 0 { a.p.task_scroll = (a.p.task_scroll + 1).min(n - 3); } else { a.p.task_scroll = a.p.task_scroll.saturating_sub(1); } a.dirty = true; return;
            }
            if a.split && a.page == 0 && x >= 262.0 && x <= 512.0 && y >= 10.0 && y <= 126.0 && { let l = lyr(); l.state == 2 && l.synced && !l.lines.is_empty() } {
                let n = lyr().lines.len() as f32; let cur = { let v = media::view(); lyric_idx(&lyr(), v.pos_ms).max(0) as f32 };
                let c = match a.p.lyr_user { Some((t, c)) if t.elapsed().as_millis() < 3000 => c, _ => cur };
                a.p.lyr_user = Some((Instant::now(), (c + if d < 0 { 1.0 } else { -1.0 }).clamp(0.0, n - 1.0))); a.dirty = true; return;
            }
            if a.page == P_SHELF && a.p.shelf.len() > 5 && y > 30.0 && y < 118.0 { let mx = (a.p.shelf.len() as f32 * 66.0 - 330.0).max(0.0); a.p.shelf_sx = (a.p.shelf_sx + if d < 0 { 66.0 } else { -66.0 }).clamp(0.0, mx); a.dirty = true; return; }
            if a.p.last_wheel.is_none_or(|t| t.elapsed().as_millis() > 380) { a.p.last_wheel = Some(Instant::now()); step_page(a, if d < 0 { 1 } else { -1 }); }
        }
        _ => {}
    }
}
fn vol_to(a: &mut App, x: f32, rc: R) {
    let f = ((x - (rc.x + 6.0)) / 65.0).clamp(0.0, 1.0); a.p.vol = f; a.p.vol_t = Some(Instant::now()); crate::vol::app_set(&media::current().app_name.unwrap_or_default(), f); a.p.vol_muted = false; a.dirty = true;
}
fn step_page(a: &mut App, d: i32) { let p = (a.page as i32 + d).rem_euclid(4) as usize; a.go_page_dir(p, if d > 0 { 1.0 } else { -1.0 }); }
fn seek_to(a: &mut App, x: f32, rc: R, commit: bool) {
    let v = media::view(); let f = ((x - rc.x) / rc.w).clamp(0.0, 1.0); let ms = (f * v.dur_ms as f32) as u64;
    a.seek_opt = Some((ms, Instant::now())); a.dirty = true;
    if commit { crate::log(&format!("seek -> {ms} ms (otimista; player={:?})", media::media_seek(ms))); }
}
fn click(a: &mut App, act: Act) {
    match act {
        Act::Prev => { let _ = media::media_control(media::Action::Prev); }
        Act::Next => { let _ = media::media_control(media::Action::Next); }
        Act::Play => { let _ = media::media_control(media::Action::Toggle); }
        Act::Volume => { // clicar no icone muta/desmuta o APP de midia (senao o sistema) e mostra o slider
            let app = media::current().app_name.unwrap_or_default(); a.p.vol_muted = crate::vol::app_toggle_mute(&app); a.p.vol = crate::vol::app_get(&app); a.p.vol_t = Some(Instant::now()); }
        Act::LyricSeek(j) => { // seek EXATO no tempo da linha (TryChangePlaybackPositionAsync); a UI mostra a posicao otimista e o player confirma
            let ms = { let l = lyr(); l.lines.get(j as usize).map(|x| x.time_ms) };
            if let Some(ms) = ms { a.seek_opt = Some((ms, Instant::now())); a.p.lyr_user = None; a.dirty = true; crate::log(&format!("seek por linha -> {ms} ms ({:?})", media::media_seek(ms))); } }
        Act::Lyrics => { a.split = !a.split; ui_save("lyricsSplit", a.split.into()); let now = Instant::now(); a.retarget(now, 240.0); }
        Act::Page(p) => a.go_page(p as usize),
        Act::Settings => crate::settings_ui::open(),
        Act::Arrow(d) => step_page(a, d as i32),
        Act::TaskToday | Act::TaskAdd | Act::TaskEdit | Act::TaskToggle(_) | Act::TaskRemove(_) | Act::TaskPick(_) | Act::TaskCal | Act::TaskMonth(_) | Act::TaskDay(_) => crate::tasks::click(a, act),
        Act::ShelfSelect | Act::TimerMode(_) | Act::TimerPlay | Act::TimerReset | Act::TimerEdit | Act::ShelfClear | Act::ShelfRemove(_) | Act::ShelfOpen(_) | Act::ShelfDragStart(_) => crate::pages2::click(a, act),
        _ => {}
    }
    a.dirty = true;
}

#[allow(non_snake_case)] fn LBUTTON_DOWN_NOW() -> bool { crate::island::LBUTTON.load(std::sync::atomic::Ordering::Relaxed) }

#[cfg(test)]
mod tests {
    use super::*;
    fn solid(b: u8, g: u8, r: u8) -> Vec<u8> { (0..128 * 128).flat_map(|_| [b, g, r, 255]).collect() }
    /// luminancia HSL do tint depois do alfa 0,55 sobre o preto (o que o usuario ve)
    fn l_vista(c: (f32, f32, f32)) -> f32 { let (mx, mn) = (c.0.max(c.1).max(c.2) * 0.55, c.0.min(c.1).min(c.2) * 0.55); (mx + mn) / 2.0 }
    #[test] fn gradiente_fica_na_faixa_de_luminancia_com_capa_clara_e_escura() {
        for (nome, px) in [("clara (amarelo)", solid(120, 230, 255)), ("escura (azul marinho)", solid(90, 30, 20)), ("vermelha", solid(40, 40, 220))] {
            let c = dominant(&px, 128).unwrap_or_else(|| panic!("{nome}: sem cor"));
            let l = l_vista(c);
            assert!((0.18..=0.32).contains(&l), "{nome}: L visto = {l}");
        }
        assert!(dominant(&solid(128, 128, 128), 128).is_none()); // cinza: sem gradiente
    }
}

#[cfg(test)]
mod spin_tests {
    use super::*;
    #[test] fn giro_da_capa_62_para_2_px_e_volta() {
        assert!((spin_scale(0.0).0 - 1.0).abs() < 1e-4);
        let (k, new) = spin_scale(SPIN_IN_MS); assert!((k * 62.0 - 2.0).abs() < 0.1 && new, "no meio do giro a capa tem ~2 px: {}", k * 62.0);
        assert!(!spin_scale(SPIN_IN_MS - 1.0).1 && spin_scale(SPIN_IN_MS + 1.0).1, "a imagem troca exatamente no ponto mais fino");
        let (end, _) = spin_scale(SPIN_IN_MS + SPIN_OUT_MS); assert!((end - 1.0).abs() < 1e-3);
        let mut prev = 1.0; for i in 0..=130 { let k = spin_scale(i as f32).0; assert!(k <= prev + 1e-4, "encolhe monotonicamente"); prev = k; }
        let mut prev = 0.0; for i in 131..=580 { let k = spin_scale(i as f32).0; assert!(k >= prev - 1e-4, "reabre monotonicamente"); prev = k; }
    }
}
