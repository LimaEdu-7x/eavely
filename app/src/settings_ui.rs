// Janela de Configuracoes em formato Apple (DESIGN-NATIVO §16): 660x500 sem barra de titulo, sidebar de 190 px + cartoes, D2D proprio, aplica na hora.
// Chaves/valores de §12 continuam; novas opcoes em settings.rs. Janela de tamanho fixo (sem redimensionar); toggles trocam sem a animacao de 160 ms (pendente).
use std::{cell::RefCell, sync::atomic::{AtomicI32, AtomicIsize, Ordering::Relaxed}, time::Instant};
use windows::{
    core::{w, HSTRING},
    Win32::{Foundation::*, Graphics::Dwm::*, System::LibraryLoader::GetModuleHandleW, UI::{Controls::MARGINS, HiDpi::*, Input::KeyboardAndMouse::*, Shell::ShellExecuteW, WindowsAndMessaging::*}},
};
use crate::{gfx::{rgba, Fmt, Gfx, Rgba}, pages::R, settings::{self, Settings}};

const WIN_BG: Rgba = rgba(0x1E1E20, 1.0);
const SIDE_BG: Rgba = rgba(0x1A1A1C, 1.0);
const SIDE_SEL: Rgba = rgba(0x2A2A2D, 1.0);
const SIDE_HOV: Rgba = rgba(0x242427, 1.0);
const CARD: Rgba = rgba(0x26262A, 1.0);
const CTRL: Rgba = rgba(0x3A3A3C, 1.0);
const TRACK: Rgba = rgba(0x1C1C1E, 1.0);
const SEG_ON: Rgba = rgba(0x55555A, 1.0);
const TOG_ON: Rgba = rgba(0x4FA3F7, 1.0);
const TEXT: Rgba = rgba(0xFFFFFF, 1.0);
const TEXT2: Rgba = rgba(0xFFFFFF, 0.64);
const TEXT3: Rgba = rgba(0xFFFFFF, 0.52);
const LINE: Rgba = rgba(0xFFFFFF, 0.07);
const DANGER: Rgba = rgba(0xFF6B6B, 1.0);
const AMBER: Rgba = rgba(0xFFB86B, 1.0);

const W: f32 = 660.0; const H: f32 = 500.0; const SIDE_W: f32 = 190.0; const TOP_H: f32 = 44.0; const CX: f32 = 214.0; const CW: f32 = 422.0; const CONTENT_TOP: f32 = 56.0;
static HW: AtomicIsize = AtomicIsize::new(0);
const WM_REFRESH: u32 = WM_APP + 20;
const T_ANIM: usize = 1;
static PAGE_REQ: AtomicI32 = AtomicI32::new(-1);

const PAGES: [(&str, &str, u32); 11] = [("Geral", "settings", 0x6E6E76), ("Ilha e aparência", "eye", 0x6B7CFF), ("Música e letras", "music", 0xB69CFF), ("Tarefas", "list-checks", 0x5EE0B0), ("Pomodoro", "timer", 0xFF7A66),
    ("Shelf", "inbox", 0xFFC857), ("Avisos do sistema", "bell", 0x4DC3E8), ("Atalhos", "keyboard", 0x9A6BFF), ("Desempenho", "zap", 0xFFB020), ("Mascote e início", "smile", 0x7CC4FF), ("Sobre", "info", 0x6E6E76)];
const P_ABOUT: usize = 10;

#[derive(Clone, Copy, PartialEq, Debug)]
enum SAct { Page(u8), Close, Min, Tog(u8), Seg(u8, u8), Step(u8, i8), Hot(u8), Mon, SlotTop, SlotBottom, Link(u8), Reset }
const T_START: u8 = 0; const T_TRACE: u8 = 1; const T_LIVE: u8 = 2; const T_HIDE: u8 = 3; const T_LYRIC: u8 = 4; const T_AUTO: u8 = 5; const T_NOTIFY: u8 = 6; const T_ONLINE: u8 = 7;
const T_VOL: u8 = 10; const T_BRI: u8 = 11; const T_BAT: u8 = 12; const T_WIFI: u8 = 13; const T_HP: u8 = 14; const T_CAPS: u8 = 15;
const T_PCSTATE: u8 = 26; const T_PCCPU: u8 = 27; const T_PCRAM: u8 = 28; const T_PCGPU: u8 = 29; const T_PCNET: u8 = 30;
const T_MINIMAL: u8 = 16; const T_SPIN: u8 = 17; const T_GRAD: u8 = 18; const T_LYRBAR: u8 = 19; const T_SHELF_EMPTY: u8 = 20; const T_THUMBS: u8 = 21; const T_HIDEFS: u8 = 22; const T_DONELAST: u8 = 23; const T_OVERDUE: u8 = 24; const T_RESPECT: u8 = 25;
const S_PCSLOTS: u8 = 6; const S_PCROT: u8 = 7; const S_PRESET: u8 = 8; const S_FPS: u8 = 9; const S_AQ: u8 = 10; const S_LYRS: u8 = 11; const S_FX: u8 = 12; const S_VIZ: u8 = 13;
const S_LEAVE: u8 = 14; const S_MINI: u8 = 15; const S_PERS: u8 = 16; const S_INIT: u8 = 17; const T_SHELFGLOW: u8 = 31;
const S_LOOK: u8 = 0; const S_LIVEMODE: u8 = 1; const S_ZONE: u8 = 2; const S_SCALE: u8 = 3; const S_OPEN: u8 = 4; const S_SPEED: u8 = 5;
const ST_FOCUS: u8 = 0; const ST_SHORT: u8 = 1; const ST_LONG: u8 = 2; const ST_CYC: u8 = 3;

struct SApp { hwnd: HWND, gfx: Gfx, page: usize, prev: usize, page_t: Instant, scroll: f32, content_h: f32, hover: Option<SAct>, pressed: Option<SAct>, hits: Vec<(R, SAct)>, side: Vec<(R, SAct)>, capture: Option<u8>, scale: f32, dirty: bool, focus_ring: bool }
thread_local! { static APP: RefCell<Option<SApp>> = const { RefCell::new(None) }; }

pub fn refresh() { let h = HW.load(Relaxed); if h != 0 { unsafe { let _ = PostMessageW(Some(HWND(h as *mut _)), WM_REFRESH, WPARAM(0), LPARAM(0)); } } }
pub fn is_open() -> bool { HW.load(Relaxed) != 0 }
pub fn hwnd() -> isize { HW.load(Relaxed) }
static SCROLL_REQ: AtomicI32 = AtomicI32::new(-1);
/// So para capturas: rola o conteudo / escolhe a pagina sem entrada sintetica
#[cfg(feature = "devtools")]
pub fn debug_scroll(y: i32) { SCROLL_REQ.store(y, Relaxed); refresh(); }
#[cfg(feature = "devtools")]
pub fn debug_page(p: i32) { PAGE_REQ.store(p, Relaxed); refresh(); }

pub fn open() {
    let h = HW.load(Relaxed);
    if h != 0 { unsafe { let hw = HWND(h as *mut _); let _ = ShowWindow(hw, SW_RESTORE); let _ = SetForegroundWindow(hw); } return; }
    static OPENING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if OPENING.swap(true, Relaxed) { return; } // cliques rapidos antes de HW existir: so uma janela
    std::thread::Builder::new().name("settings".into()).spawn(|| unsafe { run(); OPENING.store(false, Relaxed); }).ok();
}

fn monitor_options() -> Vec<(String, String)> {
    let mut v = vec![("cursor".to_string(), "Seguir o cursor".to_string()), ("primary".into(), "Monitor principal".into())];
    for i in 0..crate::island::monitor_count() { v.push((format!("m{i}"), crate::island::monitor_label(i))); }
    v
}

struct Ctx<'a> { g: &'a mut Gfx, hits: &'a mut Vec<(R, SAct)>, hover: Option<SAct>, pressed: Option<SAct>, y: f32, s: Settings, capture: Option<u8>, first_group: bool }
const RIGHT: f32 = CX + CW - 14.0;
impl Ctx<'_> {
    fn hov(&self, a: SAct) -> bool { self.hover == Some(a) }
    /// rotulo de grupo (11/500 text-3) 20 px abaixo do cartao anterior, e cartao com `h` de altura
    fn group(&mut self, label: &str, h: f32) {
        if !self.first_group { self.y += 20.0; } self.first_group = false;
        self.g.text(&label.to_uppercase(), Fmt::Tiny, CX + 4.0, self.y, 300.0, 14.0, TEXT3, false); self.y += 20.0;
        self.g.rrect(CX, self.y, CW, h, 12.0, CARD);
    }
    fn foot(&mut self, t: &str) { self.g.text(t, Fmt::Small, CX + 4.0, self.y + 6.0, CW - 8.0, 14.0, TEXT3, false); self.y += 24.0; }
    fn rh(desc: bool) -> f32 { if desc { 56.0 } else { 44.0 } }
    /// cabecalho de linha: separador, rotulo e descricao (`ctrl_w` = largura reservada ao controle, para o texto nao encostar)
    fn row(&mut self, label: &str, desc: Option<&str>, first: bool, enabled: bool, ctrl_w: f32) {
        if !first { self.g.line(CX + 16.0, self.y, CX + CW, self.y, 1.0, LINE); }
        let al = if enabled { 1.0 } else { 0.4 }; let maxw = CW - 32.0 - ctrl_w - 12.0;
        match desc {
            Some(d) => { self.g.text(label, Fmt::Row, CX + 16.0, self.y + 9.0, maxw, 18.0, (1.0, 1.0, 1.0, al), false); self.g.text(d, Fmt::Small, CX + 16.0, self.y + 30.0, maxw, 14.0, (1.0, 1.0, 1.0, 0.52 * al), false); }
            None => self.g.text(label, Fmt::Row, CX + 16.0, self.y + 13.0, maxw, 18.0, (1.0, 1.0, 1.0, al), false),
        }
    }
    fn toggle(&mut self, key: u8, on: bool, enabled: bool, h: f32) {
        let (x, y) = (RIGHT - 40.0, self.y + (h - 24.0) / 2.0); let al = if enabled { 1.0 } else { 0.4 };
        self.g.rrect(x, y, 40.0, 24.0, 12.0, if on { (TOG_ON.0, TOG_ON.1, TOG_ON.2, al) } else { (CTRL.0, CTRL.1, CTRL.2, al) });
        self.g.circle(if on { x + 30.0 } else { x + 12.0 }, y + 12.0, 10.0, (1.0, 1.0, 1.0, al));
        if enabled { self.hits.push((R { x: x - 4.0, y: self.y, w: 48.0, h }, SAct::Tog(key))); }
    }
    fn segmented(&mut self, key: u8, opts: &[&str], sel: usize, h: f32) {
        let seg_w = opts.iter().map(|o| self.g.measure(o, Fmt::Tiny) + 20.0).fold(0.0f32, f32::max).max(46.0);
        let total = seg_w * opts.len() as f32 + 4.0; let x0 = RIGHT - total; let y = self.y + (h - 26.0) / 2.0;
        self.g.rrect(x0, y, total, 26.0, 8.0, TRACK);
        for (i, o) in opts.iter().enumerate() {
            let x = x0 + 2.0 + i as f32 * seg_w; let act = SAct::Seg(key, i as u8);
            if i == sel { self.g.rrect(x, y + 2.0, seg_w, 22.0, 6.0, SEG_ON); }
            else if self.hov(act) { self.g.rrect(x, y + 2.0, seg_w, 22.0, 6.0, rgba(0xFFFFFF, 0.06)); }
            let tw = self.g.measure(o, Fmt::Tiny); self.g.text(o, Fmt::Tiny, x + (seg_w - tw) / 2.0, y + 3.5, tw + 6.0, 16.0, if i == sel { TEXT } else { TEXT2 }, false);
            self.hits.push((R { x, y, w: seg_w, h: 26.0 }, act));
        }
    }
    fn stepper(&mut self, key: u8, val: u32, unit: &str, h: f32) {
        let (x0, y) = (RIGHT - 108.0, self.y + (h - 24.0) / 2.0);
        for (i, (ic, d)) in [("minus", -1i8), ("plus", 1)].into_iter().enumerate() {
            let x = if i == 0 { x0 } else { x0 + 84.0 }; let act = SAct::Step(key, d);
            self.g.circle(x + 12.0, y + 12.0, 12.0, if self.pressed == Some(act) { rgba(0x5A5A5E, 1.0) } else if self.hov(act) { rgba(0x48484B, 1.0) } else { CTRL });
            self.g.icon(ic, x + 6.0, y + 6.0, 12.0, TEXT, false); self.hits.push((R { x, y, w: 24.0, h: 24.0 }, act));
        }
        let t = format!("{val}{unit}"); let tw = self.g.measure(&t, Fmt::BodyStrong); self.g.text(&t, Fmt::BodyStrong, x0 + 24.0 + (60.0 - tw) / 2.0, y + 4.0, tw + 6.0, 16.0, TEXT, false);
    }
    fn menu(&mut self, label: &str, act: SAct, h: f32) {
        let (x, y) = (RIGHT - 190.0, self.y + (h - 26.0) / 2.0);
        self.g.rrect(x, y, 190.0, 26.0, 7.0, if self.hov(act) { rgba(0x48484B, 1.0) } else { CTRL });
        self.g.text(label, Fmt::Body, x + 10.0, y + 5.0, 150.0, 16.0, TEXT, false); self.g.icon("chevron-down", x + 190.0 - 22.0, y + 6.0, 14.0, TEXT2, false);
        self.hits.push((R { x, y, w: 190.0, h: 26.0 }, act));
    }
    fn hotkey(&mut self, idx: u8, text: &str, h: f32) {
        let (x, y) = (RIGHT - 116.0, self.y + (h - 26.0) / 2.0); let act = SAct::Hot(idx);
        let cap = self.capture == Some(idx); let conflict = settings::HK_STATE[idx as usize].load(Relaxed) == 2 && !cap;
        self.g.rrect(x, y, 116.0, 26.0, 7.0, TRACK);
        self.g.rrect_stroke(x + 0.5, y + 0.5, 115.0, 25.0, 7.0, 1.0, if self.hov(act) { rgba(0x4FA3F7, 0.7) } else { rgba(0xFFFFFF, 0.08) });
        if cap { self.g.rrect_stroke(x + 0.5, y + 0.5, 115.0, 25.0, 7.0, 1.5, TOG_ON); } else if conflict { self.g.rrect_stroke(x + 0.5, y + 0.5, 115.0, 25.0, 7.0, 1.5, AMBER); }
        let label = if cap { "Pressione as teclas…" } else { text }; let label = label.replace('+', " + "); let tw = self.g.measure(&label, Fmt::Tiny);
        self.g.text(&label, Fmt::Tiny, x + (116.0 - tw) / 2.0, y + 5.0, tw + 6.0, 16.0, if cap { TOG_ON } else { TEXT }, false);
        if conflict { self.g.text("Em uso por outro app", Fmt::Small, x - 8.0 - 130.0, y + 6.0, 130.0, 14.0, AMBER, true); }
        self.hits.push((R { x, y, w: 116.0, h: 26.0 }, act));
    }
    fn button(&mut self, x: f32, y: f32, w: f32, label: &str, act: SAct) {
        self.g.rrect(x, y, w, 24.0, 12.0, if self.hov(act) { rgba(0x48484B, 1.0) } else { CTRL });
        let tw = self.g.measure(label, Fmt::Tiny); self.g.text(label, Fmt::Tiny, x + (w - tw) / 2.0, y + 2.5, tw + 6.0, 16.0, TEXT, false); self.hits.push((R { x, y, w, h: 24.0 }, act));
    }
    fn tog_row(&mut self, label: &str, desc: Option<&str>, key: u8, on: bool, first: bool, enabled: bool) {
        let h = Self::rh(desc.is_some()); self.row(label, desc, first, enabled, 52.0); self.toggle(key, on, enabled, h); self.y += h;
    }
}

fn page_body(c: &mut Ctx, page: usize) {
    let s = c.s.clone();
    match page {
        0 => { // Geral
            c.group("Inicialização", Ctx::rh(true)); c.tog_row("Iniciar com o Windows", Some("Abre o Eavely na bandeja ao entrar no Windows."), T_START, s.start_with_windows, true, true);
            c.group("Tela cheia", Ctx::rh(true)); c.tog_row("Ocultar em tela cheia", Some("Esconde a ilha enquanto um app ou vídeo ocupa a tela inteira."), T_HIDEFS, s.hide_fullscreen, true, true);
            c.group("Monitor", 44.0); c.row("Monitor preferido", None, true, true, 190.0);
            { let opts = monitor_options(); let label = opts.iter().find(|o| o.0 == s.monitor).map(|o| o.1.clone()).unwrap_or_else(|| "Seguir o cursor".into()); c.menu(&label, SAct::Mon, 44.0); } c.y += 44.0;
        }
        1 => { // Ilha e aparencia
            c.group("Ilha", 56.0 + 56.0 + 44.0 + 44.0);
            c.row("Tamanho", Some("Reinicia para aplicar"), true, true, 250.0); c.segmented(S_SCALE, &["Auto", "100%", "115%", "130%", "150%"], ["auto", "100", "115", "130", "150"].iter().position(|x| *x == s.ui_scale).unwrap_or(0), 56.0); c.y += 56.0;
            c.tog_row("Ilha mínima", Some("Sem a pílula ao vivo de música e timer."), T_MINIMAL, s.minimal_island, false, true);
            c.row("Mostrar traço quando ocioso", None, false, true, 52.0); c.toggle(T_TRACE, s.show_trace, true, 44.0); c.y += 44.0;
            c.row("Aparência recolhida", None, false, true, 190.0); c.segmented(S_LOOK, &["Bateria e hora", "Olhos"], (s.collapsed_look == "eyes") as usize, 44.0); c.y += 44.0;
            c.group("Comportamento", 44.0 * 5.0 + 56.0 * 2.0);
            c.row("Abrir a ilha", None, true, true, 190.0); c.segmented(S_OPEN, &["Passar o mouse", "Clique"], (s.open_mode == "click") as usize, 44.0); c.y += 44.0;
            c.row("Recolher após", Some("0 s = na hora"), false, true, 240.0); c.segmented(S_LEAVE, &["0 s", "0,5 s", "1 s", "2 s", "3 s"], match s.leave_delay_ms { 0 => 0, 500 => 1, 2000 => 3, 3000 => 4, _ => 2 }, 56.0); c.y += 56.0;
            c.row("Mini notch", Some("Em janela maximizada"), false, true, 220.0); c.segmented(S_MINI, &["Encostar", "Pequena folga"], s.mini_near as usize, 56.0); c.y += 56.0;
            c.row("Zona de ativação do topo", None, false, true, 190.0); c.segmented(S_ZONE, &["Estreita", "Normal", "Larga"], match s.trigger_zone_px { 24 => 0, 40 => 1, _ => 2 }, 44.0); c.y += 44.0;
            c.row("Velocidade da animação", None, false, true, 150.0); c.segmented(S_SPEED, &["Normal", "Rápida"], (s.anim_speed == "fast") as usize, 44.0); c.y += 44.0;
            c.row("Mostrar a ilha ao vivo", None, false, true, 52.0); c.toggle(T_LIVE, s.show_live, true, 44.0); c.y += 44.0;
            c.row("Ao vivo", None, false, s.show_live, 190.0); c.segmented(S_LIVEMODE, &["Compacto", "Expandido"], (s.live_mode == "expanded") as usize, 44.0); c.y += 44.0;
            c.group("Compacto do PC", 56.0 + 44.0 * 4.0 + 44.0);
            c.tog_row("Estado do momento", Some("Timer restante, equalizador e ponto de microfone/câmera."), T_PCSTATE, s.pc_state, true, true);
            c.tog_row("CPU", None, T_PCCPU, s.pc_cpu, false, true); c.tog_row("Memória", None, T_PCRAM, s.pc_ram, false, true); c.tog_row("GPU", None, T_PCGPU, s.pc_gpu, false, true); c.tog_row("Rede (velocidade)", None, T_PCNET, s.pc_net, false, true);
            c.row("Dados ao mesmo tempo", None, false, true, 120.0); c.segmented(S_PCSLOTS, &["1", "2"], (s.pc_slots.clamp(1, 2) - 1) as usize, 44.0); c.y += 44.0;
            c.foot("As leituras só acontecem com o compacto visível. Intervalo em Desempenho.");
        }
        2 => { // Musica e letras
            c.group("Letras", 56.0 + 44.0 + 56.0);
            c.tog_row("Mostrar letras", Some("Busca no LRCLIB (envia título, artista e álbum)."), T_ONLINE, s.online_lyrics, true, true);
            c.tog_row("Linha de letra na Música", None, T_LYRIC, s.show_lyric_line, false, true);
            c.tog_row("Mostrar letras no notch recolhido", Some("Linha atual no ao vivo; a ilha aberta continua com letras."), T_LYRBAR, s.lyric_bar, false, s.show_lyric_line);
            c.group("Capa", 44.0 * 2.0); c.tog_row("Giro da capa ao trocar de música", None, T_SPIN, s.cover_spin, true, true); c.tog_row("Gradiente da capa", None, T_GRAD, s.cover_gradient, false, true);
        }
        3 => { c.group("Lista", 44.0 * 2.0); c.tog_row("Concluídas no fim da lista", None, T_DONELAST, s.tasks_done_last, true, true); c.tog_row("Atrasadas em vermelho", None, T_OVERDUE, s.tasks_overdue_red, false, true); }
        4 => { // Pomodoro
            c.group("Durações", 44.0 * 4.0);
            c.row("Foco", None, true, true, 108.0); c.stepper(ST_FOCUS, s.timer.focus_min, " min", 44.0); c.y += 44.0;
            c.row("Pausa curta", None, false, true, 108.0); c.stepper(ST_SHORT, s.timer.short_min, " min", 44.0); c.y += 44.0;
            c.row("Pausa longa", None, false, true, 108.0); c.stepper(ST_LONG, s.timer.long_min, " min", 44.0); c.y += 44.0;
            c.row("Focos até a pausa longa", None, false, true, 108.0); c.stepper(ST_CYC, s.timer.cycles_to_long, "", 44.0); c.y += 44.0;
            c.group("Ciclo", 44.0 * 2.0); c.tog_row("Iniciar o próximo ciclo automaticamente", None, T_AUTO, s.timer.auto_next, true, true); c.tog_row("Aviso e som ao terminar", None, T_NOTIFY, s.timer.notify, false, true);
        }
        5 => { c.group("Shelf", 44.0 * 2.0); c.tog_row("Começar com a Shelf vazia", None, T_SHELF_EMPTY, s.shelf_start_empty, true, true); c.tog_row("Mostrar miniaturas", None, T_THUMBS, s.shelf_thumbs, false, true);
            c.foot("Limpar a Shelf só remove as referências; nenhum arquivo é apagado do disco."); }
        6 => { // Avisos do sistema
            let panel = crate::overlay::has_internal_panel();
            let mut ind: Vec<(&str, u8, bool)> = vec![("Volume", T_VOL, s.overlays.volume)]; if panel { ind.push(("Brilho", T_BRI, s.overlays.brightness)); } ind.push(("Caps Lock", T_CAPS, s.overlays.caps_lock));
            c.group("Indicadores", 44.0 * ind.len() as f32); for (i, (l, k, on)) in ind.iter().enumerate() { c.tog_row(l, None, *k, *on, i == 0, true); }
            c.foot("Caps Lock verifica a tecla a cada 250 ms só com isto ligado; nenhuma tecla é lida.");
            c.group("Avisos", 44.0 * 3.0); c.tog_row("Bateria e carregador", None, T_BAT, s.overlays.battery, true, true); c.tog_row("Wi‑Fi", None, T_WIFI, s.overlays.wifi, false, true); c.tog_row("Fones", None, T_HP, s.overlays.headphones, false, true);
        }
        7 => { // Atalhos
            c.group("Atalhos globais", 44.0 * 3.0);
            c.row("Mostrar/ocultar a ilha", None, true, true, 116.0); c.hotkey(0, &s.hotkeys.toggle_island, 44.0); c.y += 44.0;
            c.row("Iniciar/pausar o timer", None, false, true, 116.0); c.hotkey(1, &s.hotkeys.toggle_timer, 44.0); c.y += 44.0;
            c.row("Abrir a Shelf", None, false, true, 116.0); c.hotkey(2, &s.hotkeys.open_shelf, 44.0); c.y += 44.0;
            c.foot("Clique no campo e pressione as teclas. Esc cancela, Backspace limpa.");
        }
        8 => { // Desempenho
            let sel = |v: &str, o: &[&str]| o.iter().position(|x| *x == v).unwrap_or(0);
            c.group("Predefinições", 44.0);
            c.row("Modo", None, true, true, 289.0);
            c.segmented(S_PRESET, &["Máxima fluidez", "Equilibrado", "Economia"], match s.perf_preset.as_str() { "max" => 0, "balanced" => 1, "eco" => 2, _ => 9 }, 44.0); c.y += 44.0;
            c.foot("Os modos ajustam os controles abaixo; mexer em um deles vira \"personalizado\".");
            c.group("Quadros e animações", 56.0 * 2.0);
            c.row("Taxa de quadros", Some("Auto segue o monitor; menos = menos CPU"), true, true, 150.0); c.segmented(S_FPS, &["Auto", "60", "30"], sel(&s.fps_cap, &["auto", "60", "30"]), 56.0); c.y += 56.0;
            c.row("Animações da ilha", Some("Mais suave usa mais CPU/GPU"), false, true, 190.0); c.segmented(S_AQ, &["Suave", "Equilibrada", "Leve"], sel(&s.anim_quality, &["smooth", "balanced", "light"]), 56.0); c.y += 56.0;
            c.group("Letras", 56.0);
            c.row("Rolagem das letras", Some("Suave usa mais CPU; por linha poupa"), true, true, 150.0); c.segmented(S_LYRS, &["Suave", "Por linha"], (s.lyrics_scroll == "line") as usize, 56.0); c.y += 56.0;
            c.group("Efeitos", 56.0 * 3.0);
            c.row("Capa e efeitos", Some("Mais efeitos = mais GPU"), true, true, 215.0); c.segmented(S_FX, &["Ligado", "Reduzido", "Desligado"], sel(&s.fx_level, &["on", "reduced", "off"]), 56.0); c.y += 56.0;
            c.row("Equalizador e olhos", Some("Ligado segue o monitor; usa mais CPU"), false, true, 150.0); c.segmented(S_VIZ, &["Ligado", "Reduzido"], (s.viz_level != "on") as usize, 56.0); c.y += 56.0;
            c.tog_row("Brilho do Shelf", Some("Halo atrás do arquivo ao arrastar; desligue p/ poupar GPU"), T_SHELFGLOW, s.shelf_glow, false, true);
            c.group("Compacto do PC", 56.0);
            c.row("Atualizar métricas a cada", Some("Menos frequente = menos CPU"), true, true, 150.0); c.segmented(S_PCROT, &["3 s", "5 s", "8 s"], match s.pc_rotate { 3 => 0, 8 => 2, _ => 1 }, 56.0); c.y += 56.0;
            c.group("Sistema", Ctx::rh(true)); c.tog_row("Respeitar animações do Windows", Some("Com os efeitos de animação desligados no Windows, a ilha não anima."), T_RESPECT, s.respect_system_animation, true, true);
            c.foot("Ociosa, a ilha não gasta CPU em nenhum modo.");
        }
        9 => { // Mascote e inicio
            c.group("Mascote", Ctx::rh(true));
            c.row("Personalidade", Some("Como reage a cliques."), true, true, 250.0); c.segmented(S_PERS, &["Calmo", "Travesso", "Dramático"], match s.personality.as_str() { "travesso" => 1, "dramatico" => 2, _ => 0 }, 56.0); c.y += 56.0;
            c.group("Início", Ctx::rh(true));
            c.row("Página inicial", Some("Música tocando abre o player."), true, true, 250.0); c.segmented(S_INIT, &["Última usada", "Mascote", "Música"], match s.initial_page.as_str() { "mascot" => 1, "music" => 2, _ => 0 }, 56.0); c.y += 56.0;
            c.group("Slots da página Mascote", 44.0 * 2.0 + 0.0);
            c.row("Slot de cima", None, true, true, 190.0); c.menu(crate::slots::Widget::parse(&s.slot_top).label(), SAct::SlotTop, 44.0); c.y += 44.0;
            c.row("Slot de baixo", None, false, true, 190.0); c.menu(crate::slots::Widget::parse(&s.slot_bottom).label(), SAct::SlotBottom, 44.0); c.y += 44.0;
            c.foot("Sem repetir opção; Nenhum dá a altura toda ao outro.");
        }
        _ => { // Sobre
            c.group("Eavely", 190.0);
            c.g.thumb(0xEA7E_1060, include_bytes!("../../assets/app/logo-sobre.bgra"), 96, 96, CX + 16.0, c.y + 16.0, 56.0, 56.0, 0.0, 1.0); // logo oficial (scripts/make-about-logo.py)
            c.g.text("Eavely", Fmt::Head, CX + 88.0, c.y + 20.0, 200.0, 24.0, TEXT, false);
            c.g.text(&format!("Versão {} (compilação {})", env!("CARGO_PKG_VERSION"), env!("EAVELY_BUILD")), Fmt::Small, CX + 88.0, c.y + 46.0, 300.0, 14.0, TEXT2, false);
            c.g.text("Notch leve para Windows. Sem telemetria: a rede é usada só", Fmt::Small, CX + 16.0, c.y + 88.0, CW - 32.0, 14.0, TEXT2, false);
            c.g.text("para buscar letras (LRCLIB), e apenas se você deixar ligado.", Fmt::Small, CX + 16.0, c.y + 104.0, CW - 32.0, 14.0, TEXT2, false);
            let by = c.y + 138.0; c.button(CX + 16.0, by, 132.0, "Licenças de terceiros", SAct::Link(0)); c.button(CX + 156.0, by, 140.0, "Código-fonte (GitHub)", SAct::Link(1)); c.button(CX + 304.0, by, 100.0, "Licença MIT", SAct::Link(2));
            c.y += 190.0; c.y += 14.0; let t = "© 2026 Eavely · MIT"; let tw = c.g.measure(t, Fmt::Small); c.g.text(t, Fmt::Small, CX + (CW - tw) / 2.0, c.y, tw + 6.0, 14.0, TEXT3, false); c.y += 16.0;
        }
    }
}

fn draw(a: &mut SApp) {
    if a.gfx.begin((0.0, 0.0, W + 2.0, H + 2.0)).is_err() { return; }
    a.gfx.set_t(0.0, 0.0, 1.0);
    // janela: fundo arredondado (cantos transparentes) + borda de 1 px
    a.gfx.rrect(0.0, 0.0, W, H, 14.0, WIN_BG);
    a.gfx.rrect(0.0, 0.0, SIDE_W, H, 14.0, SIDE_BG); a.gfx.rrect(SIDE_W - 20.0, 0.0, 20.0, H, 0.0, SIDE_BG); // sidebar (canto direito reto)
    a.gfx.rrect_stroke(0.5, 0.5, W - 1.0, H - 1.0, 14.0, 1.0, rgba(0xFFFFFF, 0.10));
    let s = settings::get(); let hover = a.hover; let pressed = a.pressed;
    // controles da janela: fechar (ativo), minimizar (ativo), maximizar (desabilitado)
    a.side.clear();
    for (i, (col, act)) in [(rgba(0xFF5F57, 1.0), Some(SAct::Close)), (rgba(0x3A3A3C, 1.0), Some(SAct::Min)), (rgba(0x3A3A3C, 0.5), None)].into_iter().enumerate() {
        let (cx, cy) = (22.0 + 18.0 * i as f32, 23.0); a.gfx.circle(cx, cy, 6.0, col);
        if let Some(ac) = act { if hover == Some(ac) { a.gfx.icon(if i == 0 { "x" } else { "minus" }, cx - 4.0, cy - 4.0, 8.0, rgba(0x000000, 0.7), false); } a.side.push((R { x: cx - 8.0, y: cy - 8.0, w: 16.0, h: 16.0 }, ac)); }
    }
    // sidebar
    for (i, (name, icon, col)) in PAGES.iter().enumerate() {
        let y = if i == P_ABOUT { H - 48.0 } else { 52.0 + 34.0 * i as f32 };
        if i == P_ABOUT { a.gfx.line(10.0, H - 62.0, SIDE_W - 10.0, H - 62.0, 1.0, LINE); }
        let act = SAct::Page(i as u8); let sel = a.page == i;
        if sel { a.gfx.rrect(10.0, y, 170.0, 32.0, 8.0, SIDE_SEL); } else if hover == Some(act) { a.gfx.rrect(10.0, y, 170.0, 32.0, 8.0, SIDE_HOV); }
        a.gfx.rrect(18.0, y + 5.0, 22.0, 22.0, 6.0, rgba(*col, 1.0)); a.gfx.icon(icon, 22.0, y + 9.0, 14.0, TEXT, false);
        a.gfx.text(name, if sel { Fmt::RowStrong } else { Fmt::Row }, 48.0, y + 7.0, 126.0, 18.0, TEXT, false);
        if a.focus_ring && sel { a.gfx.rrect_stroke(10.5, y + 0.5, 169.0, 31.0, 8.0, 2.0, TOG_ON); }
        a.side.push((R { x: 10.0, y, w: 170.0, h: 32.0 }, act));
    }
    // conteudo: crossfade de 200 ms entre paginas; titulo fixo, corpo rola sob a faixa de arrasto
    let f = ((a.page_t.elapsed().as_secs_f32() / 0.2).min(1.0)).clamp(0.0, 1.0);
    let mut hits = Vec::new(); let mut content_h = 0.0;
    let pages: Vec<(usize, f32)> = if f < 1.0 { vec![(a.prev, 1.0 - f), (a.page, f)] } else { vec![(a.page, 1.0)] };
    a.gfx.clip(SIDE_W, CONTENT_TOP, W - SIDE_W, H - CONTENT_TOP);
    for (p, al) in pages {
        a.gfx.layer(al); a.gfx.set_t(0.0, CONTENT_TOP - a.scroll, 1.0);
        let mut c = Ctx { g: &mut a.gfx, hits: &mut hits, hover, pressed, y: 10.0, s: s.clone(), capture: a.capture, first_group: true };
        page_body(&mut c, p); if p == a.page { content_h = c.y + 16.0; }
        if p != a.page { hits.clear(); }
        a.gfx.set_t(0.0, 0.0, 1.0); a.gfx.unlayer();
    }
    a.gfx.unclip();
    a.hits = hits; a.content_h = content_h;
    // titulo da pagina (fixo) sobre uma faixa que oculta o conteudo rolado
    a.gfx.rrect(SIDE_W, 0.0, W - SIDE_W, CONTENT_TOP - 4.0, 0.0, WIN_BG); a.gfx.rrect(W - 20.0, 0.0, 20.0, 20.0, 0.0, WIN_BG);
    a.gfx.rrect_stroke(0.5, 0.5, W - 1.0, H - 1.0, 14.0, 1.0, rgba(0xFFFFFF, 0.10));
    a.gfx.text(PAGES[a.page].0, Fmt::Head, CX, 20.0, 400.0, 24.0, TEXT, false);
    let view = H - CONTENT_TOP;
    if a.content_h > view { let th = (view * view / a.content_h).max(30.0); let ty = CONTENT_TOP + (view - th) * (a.scroll / (a.content_h - view)); a.gfx.rrect(W - 9.0, ty, 5.0, th, 2.5, rgba(0xFFFFFF, 0.2)); }
    let _ = a.gfx.end();
    a.dirty = f < 1.0;
}

fn apply(a: &mut SApp, act: SAct) {
    match act {
        SAct::Page(p) => switch_page(a, p as usize),
        SAct::Close => unsafe { let _ = DestroyWindow(a.hwnd); },
        SAct::Min => unsafe { let _ = ShowWindow(a.hwnd, SW_MINIMIZE); },
        SAct::Reset => {}
        SAct::Tog(k) => settings::update(|s| match k {
            T_ONLINE => s.online_lyrics ^= true, T_START => s.start_with_windows ^= true, T_TRACE => s.show_trace ^= true, T_LIVE => s.show_live ^= true, T_HIDE => s.hide_during_event ^= true,
            T_LYRIC => s.show_lyric_line ^= true, T_AUTO => s.timer.auto_next ^= true, T_NOTIFY => s.timer.notify ^= true,
            T_VOL => s.overlays.volume ^= true, T_BRI => s.overlays.brightness ^= true, T_BAT => s.overlays.battery ^= true, T_WIFI => s.overlays.wifi ^= true, T_HP => s.overlays.headphones ^= true, T_CAPS => s.overlays.caps_lock ^= true,
            T_MINIMAL => s.minimal_island ^= true, T_SPIN => s.cover_spin ^= true, T_GRAD => s.cover_gradient ^= true, T_LYRBAR => s.lyric_bar ^= true, T_SHELF_EMPTY => s.shelf_start_empty ^= true, T_THUMBS => s.shelf_thumbs ^= true,
            T_PCSTATE => s.pc_state ^= true, T_PCCPU => s.pc_cpu ^= true, T_PCRAM => s.pc_ram ^= true, T_PCGPU => s.pc_gpu ^= true, T_PCNET => s.pc_net ^= true,
            T_HIDEFS => s.hide_fullscreen ^= true, T_DONELAST => s.tasks_done_last ^= true, T_OVERDUE => s.tasks_overdue_red ^= true, T_RESPECT => s.respect_system_animation ^= true, T_SHELFGLOW => s.shelf_glow ^= true, _ => {} }),
        SAct::Seg(k, v) => settings::update(|s| match k {
            S_LOOK => s.collapsed_look = if v == 1 { "eyes" } else { "clock" }.into(), S_LIVEMODE => s.live_mode = if v == 1 { "expanded" } else { "compact" }.into(),
            S_ZONE => s.trigger_zone_px = [24, 40, 64][v as usize % 3], S_SCALE => s.ui_scale = ["auto", "100", "115", "130", "150"][v as usize % 5].into(),
            S_PCSLOTS => s.pc_slots = v as u32 + 1, S_PCROT => s.pc_rotate = [3, 5, 8][v as usize % 3],
            S_PRESET => s.apply_preset(["max", "balanced", "eco"][v as usize % 3]), S_FPS => s.fps_cap = ["auto", "60", "30"][v as usize % 3].into(),
            S_AQ => s.anim_quality = ["smooth", "balanced", "light"][v as usize % 3].into(), S_LYRS => s.lyrics_scroll = if v == 1 { "line" } else { "smooth" }.into(),
            S_FX => s.fx_level = ["on", "reduced", "off"][v as usize % 3].into(), S_VIZ => s.viz_level = if v == 0 { "on" } else { "reduced" }.into(),
            S_MINI => s.mini_near = v == 1,
            S_PERS => s.personality = ["calmo", "travesso", "dramatico"][v as usize % 3].into(),
            S_INIT => s.initial_page = ["last", "mascot", "music"][v as usize % 3].into(),
            S_LEAVE => s.leave_delay_ms = [0, 500, 1000, 2000, 3000][v as usize % 5],
            S_OPEN => s.open_mode = if v == 1 { "click" } else { "hover" }.into(), S_SPEED => s.anim_speed = if v == 1 { "fast" } else { "normal" }.into(), _ => {} }),
        SAct::Step(k, d) => settings::update(|s| { let f = |x: u32| (x as i32 + d as i32).max(0) as u32; match k {
            ST_FOCUS => s.timer.focus_min = f(s.timer.focus_min), ST_SHORT => s.timer.short_min = f(s.timer.short_min), ST_LONG => s.timer.long_min = f(s.timer.long_min), ST_CYC => s.timer.cycles_to_long = f(s.timer.cycles_to_long), _ => {} } }),
        SAct::Hot(i) => { a.capture = Some(i); }
        SAct::SlotTop | SAct::SlotBottom => { let top = act == SAct::SlotTop; let cur = if top { settings::get().slot_top } else { settings::get().slot_bottom }; let all = crate::slots::Widget::ALL; let i = all.iter().position(|w| w.key() == cur).unwrap_or(0); let next = all[(i + 1) % all.len()].key().to_string(); settings::update(|s| if top { s.slot_top = next.clone() } else { s.slot_bottom = next.clone() }); }
        SAct::Mon => { let opts = monitor_options(); let cur = settings::get().monitor; let i = opts.iter().position(|o| o.0 == cur).unwrap_or(0); let next = opts[(i + 1) % opts.len()].0.clone(); settings::update(|s| s.monitor = next); }
        SAct::Link(i) => unsafe {
            let url = ["https://github.com/LimaEdu-7x/eavely/blob/main/THIRD-PARTY-LICENSES.md", "https://github.com/LimaEdu-7x/eavely", "https://github.com/LimaEdu-7x/eavely/blob/main/LICENSE"][i as usize % 3];
            ShellExecuteW(None, w!("open"), &HSTRING::from(url), None, None, SW_SHOWNORMAL);
        },
    }
    a.dirty = true;
}
fn switch_page(a: &mut SApp, p: usize) {
    if p == a.page || p >= PAGES.len() { return; }
    a.prev = a.page; a.page = p; a.page_t = Instant::now(); a.scroll = 0.0; a.capture = None; a.dirty = true;
    unsafe { SetTimer(Some(a.hwnd), T_ANIM, 16, None); }
}

fn capture_key(a: &mut SApp, vk: u32) -> bool {
    let Some(i) = a.capture else { return false };
    if matches!(vk, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0..=0xA5) { return true; } // so modificadores: espera a tecla
    if vk == 0x1B { a.capture = None; a.dirty = true; return true; }
    if vk == 0x08 { let d = settings::Hotkeys::default(); settings::update(|s| match i { 0 => s.hotkeys.toggle_island = d.toggle_island, 1 => s.hotkeys.toggle_timer = d.toggle_timer, _ => s.hotkeys.open_shelf = d.open_shelf }); a.capture = None; a.dirty = true; return true; }
    let down = |k: i32| unsafe { GetKeyState(k) < 0 };
    let mods = (down(0x11) as u32) * settings::MOD_CONTROL | (down(0x12) as u32) * settings::MOD_ALT | (down(0x10) as u32) * settings::MOD_SHIFT | ((down(0x5B) || down(0x5C)) as u32) * settings::MOD_WIN;
    let ok_key = (0x30..=0x39).contains(&vk) || (0x41..=0x5A).contains(&vk) || (0x70..=0x7B).contains(&vk);
    if !ok_key || mods & (settings::MOD_CONTROL | settings::MOD_ALT | settings::MOD_WIN) == 0 { return true; } // exige Ctrl/Alt/Win
    let t = settings::fmt_hotkey(mods, vk);
    settings::update(|s| match i { 0 => s.hotkeys.toggle_island = t.clone(), 1 => s.hotkeys.toggle_timer = t.clone(), _ => s.hotkeys.open_shelf = t.clone() });
    a.capture = None; a.dirty = true; true
}

fn with_app<R>(f: impl FnOnce(&mut SApp) -> R) -> Option<R> { APP.with(|a| a.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f))) }

unsafe extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_NCCALCSIZE => return LRESULT(0), // sem moldura do sistema: toda a area e cliente
        WM_NCHITTEST => { // faixa de arrasto de 44 px (a direita dos controles); o resto e cliente
            let (x, y) = (((lp.0 & 0xFFFF) as i16) as i32, (((lp.0 >> 16) & 0xFFFF) as i16) as i32);
            let mut r = RECT::default(); let _ = GetWindowRect(h, &mut r); let s = with_app(|a| a.scale).unwrap_or(1.0);
            let (lx, ly) = ((x - r.left) as f32 / s, (y - r.top) as f32 / s);
            if ly < TOP_H && lx > 76.0 { return LRESULT(HTCAPTION as isize) }
            return LRESULT(HTCLIENT as isize)
        }
        WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEWHEEL => {
            with_app(|a| {
                let (cx, cy) = ((lp.0 & 0xFFFF) as i16 as f32 / a.scale, ((lp.0 >> 16) & 0xFFFF) as i16 as f32 / a.scale);
                if m == WM_MOUSEWHEEL { let d = ((wp.0 >> 16) & 0xFFFF) as i16 as f32 / 120.0;
                    a.scroll = (a.scroll - d * 48.0).clamp(0.0, (a.content_h - (H - CONTENT_TOP)).max(0.0)); a.dirty = true; return; }
                let hit = if cx < SIDE_W || cy < CONTENT_TOP { a.side.iter().rev().find(|(r, _)| r.has(cx, cy)).map(|h| h.1) }
                          else { let (x, y) = (cx, cy - CONTENT_TOP + a.scroll); a.hits.iter().rev().find(|(r, _)| r.has(x, y)).map(|h| h.1) };
                let hit = hit.or_else(|| if cx < SIDE_W { None } else { None });
                match m {
                    WM_MOUSEMOVE => { if hit != a.hover { a.hover = hit; a.dirty = true; } let mut t = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: a.hwnd, dwHoverTime: 0 }; let _ = TrackMouseEvent(&mut t); }
                    WM_LBUTTONDOWN => { a.pressed = hit; a.focus_ring = false; if hit.is_none() && a.capture.is_some() { a.capture = None; } a.dirty = true; }
                    _ => { let p = a.pressed.take(); if let (Some(p), Some(hh)) = (p, hit) { if p == hh { apply(a, hh); } } a.dirty = true; }
                }
            });
            return LRESULT(0)
        }
        0x02A3 => { with_app(|a| { a.hover = None; a.dirty = true; }); return LRESULT(0) }
        WM_TIMER => { with_app(|a| { a.dirty = true; if a.page_t.elapsed().as_millis() > 260 { let _ = KillTimer(Some(a.hwnd), T_ANIM); } }); return LRESULT(0) }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            if with_app(|a| capture_key(a, wp.0 as u32)).unwrap_or(false) { return LRESULT(0) }
            let vk = wp.0 as u32;
            with_app(|a| match vk { 0x1B => { let _ = DestroyWindow(a.hwnd); } // Esc fecha
                0x26 => { let p = (a.page + PAGES.len() - 1) % PAGES.len(); a.focus_ring = true; switch_page(a, p); } // seta para cima/baixo trocam de pagina
                0x28 => { let p = (a.page + 1) % PAGES.len(); a.focus_ring = true; switch_page(a, p); } _ => {} });
        }
        WM_KILLFOCUS => { with_app(|a| { a.capture = None; a.dirty = true; }); }
        WM_REFRESH => { with_app(|a| { let r = SCROLL_REQ.swap(-1, Relaxed); if r >= 0 { a.scroll = (r as f32).min((a.content_h - (H - CONTENT_TOP)).max(0.0)); }
            let p = PAGE_REQ.swap(-1, Relaxed); if p >= 0 { a.prev = a.page; a.page = (p as usize).min(PAGES.len() - 1); a.page_t = Instant::now() - std::time::Duration::from_secs(1); a.scroll = 0.0; } a.dirty = true; }); return LRESULT(0) }
        WM_CLOSE => { let _ = DestroyWindow(h); return LRESULT(0) }
        WM_DESTROY => { HW.store(0, Relaxed); PostQuitMessage(0); return LRESULT(0) }
        _ => {}
    }
    DefWindowProcW(h, m, wp, lp)
}

unsafe fn run() {
    let hi: HINSTANCE = GetModuleHandleW(None).unwrap().into();
    let wc = WNDCLASSW { lpfnWndProc: Some(proc), hInstance: hi, lpszClassName: w!("EavelySettings"), hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(), hIcon: crate::tray::app_icon(), ..Default::default() };
    RegisterClassW(&wc);
    let mut p = POINT::default(); let _ = GetCursorPos(&mut p);
    let mon = windows::Win32::Graphics::Gdi::MonitorFromPoint(p, windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST);
    let (mut dx, mut dy) = (96u32, 96u32); let _ = GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
    let s = dx as f32 / 96.0;
    let (ww, wh) = ((W * s).round() as i32, (H * s).round() as i32);
    let mut mi = windows::Win32::Graphics::Gdi::MONITORINFO { cbSize: size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32, ..Default::default() };
    let _ = windows::Win32::Graphics::Gdi::GetMonitorInfoW(mon, &mut mi);
    let (x, y) = (mi.rcWork.left + (mi.rcWork.right - mi.rcWork.left - ww) / 2, mi.rcWork.top + ((mi.rcWork.bottom - mi.rcWork.top - wh) / 2).max(0));
    let Ok(hwnd) = CreateWindowExW(WS_EX_NOREDIRECTIONBITMAP | WS_EX_APPWINDOW, w!("EavelySettings"), w!("Configurações — Eavely"), WS_POPUP | WS_MINIMIZEBOX | WS_SYSMENU, x, y, ww, wh, None, None, Some(hi), None) else { return };
    let dark = 1i32; let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const _ as *const _, 4);
    let round = 2i32; let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &round as *const _ as *const _, 4);
    let mg = MARGINS { cxLeftWidth: 1, cxRightWidth: 1, cyTopHeight: 1, cyBottomHeight: 1 }; let _ = DwmExtendFrameIntoClientArea(hwnd, &mg); // sombra da janela
    let Ok(gfx) = Gfx::new(hwnd, ww as u32, wh as u32, s) else { let _ = DestroyWindow(hwnd); return };
    APP.with(|a| *a.borrow_mut() = Some(SApp { hwnd, gfx, page: 0, prev: 0, page_t: Instant::now() - std::time::Duration::from_secs(1), scroll: 0.0, content_h: 0.0, hover: None, pressed: None, hits: Vec::new(), side: Vec::new(), capture: None, scale: s, dirty: true, focus_ring: false }));
    HW.store(hwnd.0 as isize, Relaxed);
    crate::log(&format!("settings hwnd={}", hwnd.0 as isize));
    let _ = ShowWindow(hwnd, SW_SHOW); let _ = SetForegroundWindow(hwnd);
    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        let _ = TranslateMessage(&msg); DispatchMessageW(&msg);
        with_app(|a| if a.dirty { draw(a) });
    }
    APP.with(|a| *a.borrow_mut() = None); // libera o renderer
    HW.store(0, Relaxed);
}
