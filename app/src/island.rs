// Ilha nativa: janela DirectComposition + estados (traco/peek/aberto/ao vivo/oculto) + animacao dirigida por tempo (vblank) + entrada.
// Tudo na thread "island" (loop de mensagens proprio). Outras threads so chamam `wake()`.
use std::{cell::RefCell, sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU32, Ordering::Relaxed}, time::{Duration, Instant}};
use windows::{
    core::{w, BOOL},
    Win32::{Foundation::*, Graphics::{Dwm::*, Gdi::*}, System::LibraryLoader::GetModuleHandleW,
        UI::{Accessibility::*, HiDpi::*, WindowsAndMessaging::*}, System::Threading::INFINITE},
};
use crate::gfx::Gfx;

pub const WIN_W: f32 = 600.0; // janela maxima (520 das letras + flares de 18 + folga para o overshoot)
/// ILHA ABERTA: tamanho FIXO em todas as paginas (corpo 420x140; com os flares de 18 px = 457 de largura medido internamente). So as letras alargam (520).
pub const OPEN_W: f32 = 420.0;
pub const OPEN_H: f32 = 140.0;
pub const OPEN_FLARE: f32 = 18.0;
pub const WIN_H: f32 = 176.0; // 164 com dock + folga

const WM_CLASS: u32 = WM_APP + 1;
const WM_WAKE: u32 = WM_APP + 2;
pub const WM_TRAY: u32 = WM_APP + 10;
const T_TICK: usize = 1;
const T_SCAN: usize = 2;
#[cfg(feature = "devtools")]
const T_BENCH: usize = 4;
const T_TRIM: usize = 5;
const WM_REHK: u32 = WM_APP + 3;
const T_PIN: usize = 7;
const T_OVER: usize = 8;
const T_CAPS: usize = 9;
const T_EYES: usize = 10;
const WM_OVL: u32 = WM_APP + 4;
const WM_CAPS: u32 = WM_APP + 5;
const T_NET: usize = 11;
const T_LEAVE: usize = 12;
const T_CLK: usize = 13;
const T_BREATH: usize = 14;
const T_SELFTEST: usize = 16;
const T_METRICS: usize = 17; // compacto de PC: amostra CPU/RAM/GPU/rede/mic a 1 Hz, SO com o compacto visivel
const T_LOOK: usize = 15; // olhos: reconfere o cursor a cada 33 ms enquanto o aspecto 'olhos' esta visivel
/// atraso antes de recolher ao tirar o mouse (ms); sera ajustado ao valor medido
/// MINI NOTCH (janela maximizada): tamanho em px logicos, facil de ajustar (pediu-se 56x5; tambem cogitado 48 ou 64 de largura). Raio do canto de baixo e dos flares limitados pela altura.
pub const MINI_W: f32 = 56.0;
pub const MINI_H: f32 = 5.0;
pub const MINI_R: f32 = 2.5;
pub const MINI_FLARE: f32 = 4.0;
pub static GRACE_MS: AtomicU32 = AtomicU32::new(100);
const WM_NET: u32 = WM_APP + 6;
static PTR_ARMED: AtomicBool = AtomicBool::new(false);
static LOC_HOOK: AtomicIsize = AtomicIsize::new(0);
const T_PTR: usize = 6; // reconfere a posicao do cursor 12 ms apos o ultimo evento (o WM_INPUT chega antes do cursor andar)

static HW: AtomicIsize = AtomicIsize::new(0);
pub fn wake() { let h = HW.load(Relaxed); if h != 0 { unsafe { let _ = PostMessageW(Some(HWND(h as *mut _)), WM_WAKE, WPARAM(0), LPARAM(0)); } } }

// ---- estado compartilhado com o hook do mouse (so atomicos) ----
static MONS: [[AtomicI32; 5]; 8] = [const { [const { AtomicI32::new(0) }; 5] }; 8]; // x, y, w, h, escala*1000 (px fisicos)
static MON_N: AtomicU32 = AtomicU32::new(0);
static ACTIVE: AtomicU32 = AtomicU32::new(0);
static HIT: [AtomicI32; 4] = [const { AtomicI32::new(0) }; 4]; // retangulo "sobre a ilha" (px fisicos) do estado alvo
static HOOK_OPEN: AtomicBool = AtomicBool::new(false);
static LAST_CLASS: AtomicU32 = AtomicU32::new(u32::MAX);
pub static LBUTTON: AtomicBool = AtomicBool::new(false);
static LBTN_DOWN_ON_FILES: AtomicBool = AtomicBool::new(false);
static PRESS_KIND: AtomicU32 = AtomicU32::new(0); // onde o botao foi apertado: 0 nada, 1 janela do Explorer, 2 area de trabalho
static PRESS_XY: [AtomicI32; 2] = [const { AtomicI32::new(0) }; 2];
/// Um ARRASTO REAL de arquivos (nunca so cursor perto): botao esquerdo REALMENTE apertado agora, apertado sobre arquivos (Explorer/area de trabalho), cursor ja andou
/// >= 16 px desde o aperto e, se foi na area de trabalho (onde clicar no vazio so seleciona), o shell esta mostrando a imagem de arrasto (SysDragImage).
/// Devolve a causa (para o state.log) ou None.
fn real_drag(cur: POINT, async_down: bool, kind: u32, press: (i32, i32), drag_img: bool) -> Option<&'static str> {
    if !async_down || kind == 0 { return None; }
    let moved = (cur.x - press.0).abs().max((cur.y - press.1).abs()) >= 16;
    if !moved { return None; }
    match kind { 1 => Some("botao apertado no Explorer + movimento >= 16 px"), 2 if drag_img => Some("botao apertado na area de trabalho + imagem de arrasto do shell"), _ => None }
}
fn real_drag_now() -> Option<&'static str> { unsafe {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    let down = GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000 != 0;
    if !down { LBUTTON.store(false, Relaxed); LBTN_DOWN_ON_FILES.store(false, Relaxed); PRESS_KIND.store(0, Relaxed); return None; } // evento de soltar perdido: nao fica 'preso'
    let mut p = POINT::default(); let _ = GetCursorPos(&mut p);
    let img = FindWindowW(w!("SysDragImage"), None).is_ok_and(|h| IsWindowVisible(h).as_bool());
    real_drag(p, down, PRESS_KIND.load(Relaxed), (PRESS_XY[0].load(Relaxed), PRESS_XY[1].load(Relaxed)), img)
} }

/// Posicao do cursor + botao: classifica (zona de gatilho / sobre a ilha / dock / monitor) e avisa a ilha so quando a classe muda.
/// Alimentado por Raw Input (WM_INPUT, so mouse, sem hook global): evento puro, sem polling.
unsafe fn pointer_moved(pt: POINT, wm: u32) {
    match wm {
        WM_LBUTTONDOWN => { LBUTTON.store(true, Relaxed); let k = files_under(pt); PRESS_KIND.store(k, Relaxed); PRESS_XY[0].store(pt.x, Relaxed); PRESS_XY[1].store(pt.y, Relaxed); LBTN_DOWN_ON_FILES.store(k != 0, Relaxed); }
        WM_LBUTTONUP => { LBUTTON.store(false, Relaxed); LBTN_DOWN_ON_FILES.store(false, Relaxed); PRESS_KIND.store(0, Relaxed); }
        _ => {}
    }
    let (x, y) = (pt.x, pt.y);
    let n = MON_N.load(Relaxed) as usize;
    let mut mi = None;
    for i in 0..n.min(8) { let m = &MONS[i]; let (mx, my, mw, mh) = (m[0].load(Relaxed), m[1].load(Relaxed), m[2].load(Relaxed), m[3].load(Relaxed));
        if x >= mx && x < mx + mw && y >= my && y < my + mh { mi = Some(i); break; } }
    let mut class = 0u32;
    if let Some(i) = mi {
        let m = &MONS[i]; let (mx, my, mw) = (m[0].load(Relaxed), m[1].load(Relaxed), m[2].load(Relaxed)); let s = m[4].load(Relaxed) as f32 / 1000.0;
        let act = ACTIVE.load(Relaxed) as usize;
        let cx = mx + mw / 2;
        let (h0, h1, h2, h3) = (HIT[0].load(Relaxed), HIT[1].load(Relaxed), HIT[2].load(Relaxed), HIT[3].load(Relaxed));
        let inside = i == act && x >= h0 && x < h2 && y >= h1 && y < h3;
        let trig = (x - cx).abs() < (140.0 * s) as i32 && y - my < (crate::settings::TRIG_PX.load(Relaxed) as f32 * s) as i32;
        let zone = if inside { 2 } else if trig { 1 } else { 0 };
        let dock = HOOK_OPEN.load(Relaxed) && inside && (y - my) as f32 / s >= 122.0;
        class = zone | (dock as u32) << 2 | (i as u32) << 3;
    }
    if LAST_CLASS.swap(class, Relaxed) != class || wm == WM_LBUTTONDOWN || wm == WM_LBUTTONUP {
        let h = HW.load(Relaxed); if h != 0 { let _ = PostMessageW(Some(HWND(h as *mut _)), WM_CLASS, WPARAM(class as usize), LPARAM(0)); }
    }
}
unsafe fn handle_raw(lp: LPARAM) {
    use windows::Win32::UI::Input::*;
    let mut buf = [0u64; 12]; let mut size = size_of_val(&buf) as u32;
    let n = GetRawInputData(HRAWINPUT(lp.0 as *mut _), RID_INPUT, Some(buf.as_mut_ptr() as *mut _), &mut size, size_of::<RAWINPUTHEADER>() as u32);
    if n == u32::MAX { return; }
    let ri = &*(buf.as_ptr() as *const RAWINPUT);
    if ri.header.dwType != RIM_TYPEMOUSE.0 { return; }
    let fl = ri.data.mouse.Anonymous.Anonymous.usButtonFlags as u32;
    let wm = if fl & RI_MOUSE_LEFT_BUTTON_DOWN != 0 { WM_LBUTTONDOWN } else if fl & RI_MOUSE_LEFT_BUTTON_UP != 0 { WM_LBUTTONUP } else { WM_MOUSEMOVE };
    let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt);
    pointer_moved(pt, wm);
    let h = HW.load(Relaxed); if h != 0 && !PTR_ARMED.swap(true, Relaxed) { SetTimer(Some(HWND(h as *mut _)), T_PTR, 12, None); } // um timer por rajada de eventos (nao um por evento)
}
/// Botao apertado sobre o Explorer/area de trabalho: candidato a arrasto de arquivos (abre o Shelf ao chegar no topo).
unsafe fn files_under(p: POINT) -> u32 {
    #[cfg(feature = "devtools")]
    if std::env::var_os("EAVELY_DRAG_ANY").is_some() { return 1; } // so para teste automatizado (fonte de arrasto proprio)
    let h = WindowFromPoint(p); if h.0.is_null() { return 0; }
    let r = GetAncestor(h, GA_ROOT);
    let mut cls = [0u16; 40]; let n = GetClassNameW(r, &mut cls).max(0) as usize;
    match String::from_utf16_lossy(&cls[..n]).as_str() { "CabinetWClass" | "ExploreWClass" => 1, "Progman" | "WorkerW" => 2, _ => 0 }
}

unsafe extern "system" fn winevent(_: HWINEVENTHOOK, ev: u32, hwnd: HWND, idobj: i32, _: i32, _: u32, _: u32) {
    if ev == EVENT_SYSTEM_FOREGROUND { // LOCATIONCHANGE so da thread da janela em primeiro plano: o sistema nao nos chama para as demais
        let mut pid = 0u32; let tid = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let old = LOC_HOOK.swap(0, Relaxed); if old != 0 { let _ = UnhookWinEvent(HWINEVENTHOOK(old as *mut _)); }
        if tid != 0 { let nh = SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(winevent), pid, tid, WINEVENT_OUTOFCONTEXT); LOC_HOOK.store(nh.0 as isize, Relaxed); }
    }
    if ev == EVENT_OBJECT_LOCATIONCHANGE && (idobj != 0 || hwnd != GetForegroundWindow()) { return; }
    let h = HW.load(Relaxed); if h != 0 { SetTimer(Some(HWND(h as *mut _)), T_SCAN, 200, None); } // debounce
}

// ---- monitores e janelas em tela cheia / maximizadas ----
#[derive(Clone, Copy, PartialEq)] pub struct Mon { pub x: i32, pub y: i32, pub w: i32, pub h: i32, pub s: f32 }
unsafe extern "system" fn mon_cb(hm: HMONITOR, _: HDC, r: *mut RECT, lp: LPARAM) -> BOOL {
    let v = &mut *(lp.0 as *mut Vec<Mon>); let r = &*r;
    let (mut dx, mut dy) = (96u32, 96u32); let _ = GetDpiForMonitor(hm, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
    v.push(Mon { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top, s: dx as f32 / 96.0 }); TRUE
}
fn monitors() -> Vec<Mon> { let mut v: Vec<Mon> = Vec::new(); unsafe { let _ = EnumDisplayMonitors(None, None, Some(mon_cb), LPARAM(&mut v as *mut _ as isize)); }
    // Tamanho da ilha: Auto = escala do Windows; 100/115/130/150% = fator ABSOLUTO sobre o layout (lido ao iniciar: 'reinicia para aplicar')
    if crate::settings::ui_scale_explicit() { static F: std::sync::OnceLock<f32> = std::sync::OnceLock::new(); let f = *F.get_or_init(crate::settings::ui_scale_factor); for m in &mut v { m.s = f; } }
    v }

#[derive(Debug, PartialEq)] pub enum Cover { None, Max, Full }
/// Tela cheia x maximizada x nada, para UMA janela e UM monitor (retangulos em px fisicos; mon = x,y,w,h).
/// - tela cheia: cobre o monitor INTEIRO e (nao tem barra de titulo [jogo/video borderless] OU o retangulo e EXATAMENTE o do monitor [fullscreen de navegador/F11])
/// - maximizada: IsZoomed com barra de titulo; o retangulo extrapola alguns px (bordas invisiveis) ou nao alcanca a barra de tarefas, nunca e exato com o monitor todo
pub fn classify(r: (i32, i32, i32, i32), mon: (i32, i32, i32, i32), has_caption: bool, zoomed: bool) -> Cover {
    let (m_x, m_y, m_w, m_h) = mon;
    let covers = r.0 <= m_x + 1 && r.1 <= m_y + 1 && r.2 >= m_x + m_w - 1 && r.3 >= m_y + m_h - 1;
    let exact = r == (m_x, m_y, m_x + m_w, m_y + m_h);
    if covers && (!has_caption || exact) { Cover::Full }
    else if zoomed && (r.2.min(m_x + m_w) - r.0.max(m_x)) > m_w / 2 && r.1 <= m_y + 8 { Cover::Max } // a maioria da janela esta neste monitor (as bordas invisiveis de 8 px de uma maximizada invadem o vizinho)
    else { Cover::None }
}
struct Scan { mons: Vec<Mon>, fs: Vec<bool>, mx: Vec<bool>, own: HWND }
unsafe extern "system" fn scan_cb(h: HWND, lp: LPARAM) -> BOOL {
    let sc = &mut *(lp.0 as *mut Scan);
    if h == sc.own || !IsWindowVisible(h).as_bool() || IsIconic(h).as_bool() { return TRUE; }
    let mut cloaked = 0u32; let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
    if cloaked != 0 { return TRUE; }
    let (st, ex) = (GetWindowLongPtrW(h, GWL_STYLE) as u32, GetWindowLongPtrW(h, GWL_EXSTYLE) as u32);
    if ex & (WS_EX_TRANSPARENT.0 | WS_EX_TOOLWINDOW.0) != 0 { return TRUE; }
    let mut cls = [0u16; 40]; let n = GetClassNameW(h, &mut cls).max(0) as usize;
    if matches!(String::from_utf16_lossy(&cls[..n]).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "EavelyIsland") { return TRUE; }
    let mut r = RECT::default(); if GetWindowRect(h, &mut r).is_err() { return TRUE; }
    let zoomed = IsZoomed(h).as_bool();
    for (i, m) in sc.mons.iter().enumerate() {
        match classify((r.left, r.top, r.right, r.bottom), (m.x, m.y, m.w, m.h), st & WS_CAPTION.0 == WS_CAPTION.0, zoomed) { Cover::Full => sc.fs[i] = true, Cover::Max => sc.mx[i] = true, Cover::None => {} }
    }
    TRUE
}
fn scan(mons: &[Mon], own: HWND) -> (Vec<bool>, Vec<bool>) {
    let mut sc = Scan { mons: mons.to_vec(), fs: vec![false; mons.len()], mx: vec![false; mons.len()], own };
    unsafe { let _ = EnumWindows(Some(scan_cb), LPARAM(&mut sc as *mut _ as isize)); }
    (sc.fs, sc.mx)
}

// ---- animacao ----
/// cubic-bezier(.2,.8,.2,1) (ease-out do design)
pub fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let (x1, y1, x2, y2) = (0.2f32, 0.8f32, 0.2f32, 1.0f32);
    let bx = |t: f32| 3.0 * (1.0 - t).powi(2) * t * x1 + 3.0 * (1.0 - t) * t * t * x2 + t.powi(3);
    let by = |t: f32| 3.0 * (1.0 - t).powi(2) * t * y1 + 3.0 * (1.0 - t) * t * t * y2 + t.powi(3);
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..24 { let m = (lo + hi) / 2.0; if bx(m) < x { lo = m } else { hi = m } }
    by((lo + hi) / 2.0)
}
#[derive(Clone, Copy)]
pub struct Tw { from: f32, to: f32, t0: Instant, dur: f32 }
impl Tw {
    pub fn new(v: f32) -> Tw { Tw { from: v, to: v, t0: Instant::now(), dur: 0.0 } }
    pub fn val(&self, now: Instant) -> f32 { if self.dur <= 0.0 { return self.to; } let t = (now - self.t0).as_secs_f32() * 1000.0 / self.dur; self.from + (self.to - self.from) * ease(t) }
    pub fn go(&mut self, now: Instant, to: f32, dur_ms: f32) { if (to - self.to).abs() < 1e-4 { return; } self.from = self.val(now); self.to = to; self.t0 = now; self.dur = if reduced_motion() { 0.0 } else { dur_ms }; }
    pub fn target(&self) -> f32 { self.to }
    pub fn busy(&self, now: Instant) -> bool { self.dur > 0.0 && (now - self.t0).as_secs_f32() * 1000.0 < self.dur + 20.0 } // +20 ms: garante um quadro final com o valor exato (e o repouso so depois dele)
}
// ---------------- motor de molas (spring) ----------------
/// Parametros de uma mola: forca = -rigidez*(pos-alvo) - amortecimento*vel ; a = forca/massa.  zeta = amortecimento / (2*sqrt(rigidez*massa)):
/// zeta < 1 => overshoot (estilingue); quanto menor, mais balanca.
#[derive(Clone, Copy, Debug)] pub struct SpringParams { pub stiffness: f32, pub damping: f32, pub mass: f32 }
/// TABELA DE AJUSTE (os valores finais virao da medicao interna). Provisorio: zeta ~0,55 abrindo (overshoot ~12%), ~0,68 fechando.
pub const SPRING_OPEN: SpringParams = SpringParams { stiffness: 144.0, damping: 10.6, mass: 1.0 };   // crescer (largura/altura/raio)
pub const SPRING_CLOSE: SpringParams = SpringParams { stiffness: 272.0, damping: 21.0, mass: 1.0 };  // encolher
pub const SPRING_DOCK: SpringParams = SpringParams { stiffness: 310.0, damping: 35.2, mass: 1.0 };   // dock/calendario (+28 px), mais seco
/// medido internamente (interno): altura 30->140 passa a 148 (+7%), letras 419->519 passa a 527 (+8%), "respirar" antes de abrir.
pub const SPRING_OPEN_H: SpringParams = SpringParams { stiffness: 144.0, damping: 15.6, mass: 1.0 };
pub const SPRING_LYRICS: SpringParams = SpringParams { stiffness: 219.0, damping: 18.7, mass: 1.0 };
pub const SPRING_BREATH: SpringParams = SpringParams { stiffness: 900.0, damping: 50.0, mass: 1.0 };
pub const BREATH_MS: u64 = 70;             // medido: respira 164x30 -> 175x34 e espera ~50 ms antes do pico
pub const BREATH_GROW: (f32, f32) = (11.0, 4.0);
/// CORTE POR DURACAO: a mola teorica balanca alem dos 0,43 s medidos; depois disso ela fica 3x mais dura e criticamente amortecida (sem salto), e aos 0,8 s assenta de vez.
pub const SPRING_CUT_S: f32 = 0.43;
pub const SPRING_HARD_S: f32 = 0.8;
/// troca lateral de abas (DESIGN-NATIVO sec.17): mola k420 c34, deslize de 48 px junto com o crossfade
pub const SPRING_PAGE: SpringParams = SpringParams { stiffness: 420.0, damping: 34.0, mass: 1.0 };
pub const PAGE_SLIDE_PX: f32 = 48.0;
pub const SPRING_REST: f32 = 0.05; // px: abaixo disso (e velocidade baixa) a mola assenta e para de pedir quadros
#[derive(Clone, Copy, Debug)]
pub struct Spring { pub pos: f32, pub vel: f32, pub to: f32, p: SpringParams, age: f32 }
impl Spring {
    pub fn new(v: f32) -> Spring { Spring { pos: v, vel: 0.0, to: v, p: SPRING_OPEN, age: 0.0 } }
    pub fn val(&self, _now: Instant) -> f32 { self.pos }
    /// novo alvo; `dur` e ignorado (a duracao vem da fisica). Reduced-motion: assenta na hora.
    pub fn go(&mut self, _now: Instant, to: f32, dur: f32) {
        if (to - self.to).abs() < 1e-4 { return; }
        self.p = if dur <= 210.0 && dur > 0.0 { SPRING_DOCK } else if to >= self.pos { SPRING_OPEN } else { SPRING_CLOSE };
        self.start(to);
    }
    /// como `go`, mas com a mola escolhida (so ao crescer; encolher usa SPRING_CLOSE)
    pub fn go_p(&mut self, to: f32, grow: SpringParams) {
        if (to - self.to).abs() < 1e-4 { return; }
        self.p = if to >= self.pos { grow } else { SPRING_CLOSE };
        self.start(to);
    }
    fn start(&mut self, to: f32) { self.to = to; self.age = 0.0; if reduced_motion() { self.pos = to; self.vel = 0.0; } }
    pub fn busy(&self, _now: Instant) -> bool { !self.settled() }
    pub fn settled(&self) -> bool { (self.pos - self.to).abs() < SPRING_REST && self.vel.abs() < 1.0 }
    /// integracao semi-implicita em passos de ate 4 ms (estavel e independente da taxa de quadros)
    pub fn step(&mut self, dt: f32) {
        if self.settled() { self.pos = self.to; self.vel = 0.0; return; }
        let mut left = dt.clamp(0.0, 0.05); self.age += left;
        if self.age > SPRING_HARD_S { self.pos = self.to; self.vel = 0.0; return; }
        let (k, c) = if self.age > SPRING_CUT_S { let k = self.p.stiffness * 3.0; (k, 2.0 * (k * self.p.mass).sqrt()) } else { (self.p.stiffness, self.p.damping) };
        while left > 0.0 {
            let h = left.min(0.004); left -= h;
            let a = (-k * (self.pos - self.to) - c * self.vel) / self.p.mass;
            self.vel += a * h; self.pos += self.vel * h;
        }
        if self.settled() { self.pos = self.to; self.vel = 0.0; }
    }
}

/// 'Efeitos de animacao' do Windows desligados? (cache; so e consultado se a opcao de respeitar estiver ligada)
pub fn reduced_motion() -> bool { crate::settings::respect_anim() && system_anim_off() }
pub fn system_anim_off() -> bool { static R: AtomicU32 = AtomicU32::new(2); let v = R.load(Relaxed);
    if v != 2 { return v == 1; }
    let mut on = BOOL(1); unsafe { let _ = SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, Some(&mut on as *mut _ as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)); }
    R.store(!on.as_bool() as u32, Relaxed); !on.as_bool() }

#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum St { Hidden, Line, Peek, Open, Live, Over }

pub struct Stats { pub iv: Vec<f32>, pub cpu: Vec<f32>, pub last: Option<Instant>, pub streaks: u32, pub hz: f32 }
impl Stats {
    fn report(&mut self, tag: &str) {
        if self.iv.len() < 3 { self.iv.clear(); self.cpu.clear(); return; }
        let mut v = self.iv.clone(); v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p = |q: f32| v[((v.len() as f32 * q) as usize).min(v.len() - 1)];
        let avg = v.iter().sum::<f32>() / v.len() as f32;
        let cpu = self.cpu.iter().sum::<f32>() / self.cpu.len().max(1) as f32;
        crate::log(&format!("PERF {tag}: quadros={} intervalo avg={avg:.2}ms p50={:.2} p95={:.2} p99={:.2} max={:.2} | fps~{:.0} | desenho(CPU) avg={cpu:.2}ms | monitor={:.0}Hz", v.len(), p(0.5), p(0.95), p(0.99), v[v.len() - 1], 1000.0 / avg, self.hz));
        self.iv.clear(); self.cpu.clear(); self.streaks += 1;
    }
}

/// Renderer sob demanda: o D3D/D2D/DWrite custa ~25 MB, entao so existe enquanto a ilha esta visivel (o traco usa uma janela GDI minuscula).
pub struct Slot { g: Option<Gfx>, pub scale: f32, pub size: (u32, u32) }
impl std::ops::Deref for Slot { type Target = Gfx; fn deref(&self) -> &Gfx { self.g.as_ref().expect("gfx") } }
impl std::ops::DerefMut for Slot { fn deref_mut(&mut self) -> &mut Gfx { self.g.as_mut().expect("gfx") } }
impl Slot {
    pub fn has(&self) -> bool { self.g.is_some() }
    pub fn resize(&mut self, w: u32, h: u32, s: f32) { self.scale = s; self.size = (w, h); if let Some(g) = self.g.as_mut() { g.resize(w, h, s); } }
}

pub struct App {
    pub hwnd: HWND, pub trace: HWND, pub vis: bool, pub gfx: Slot, pub mons: Vec<Mon>, pub active: usize, pub fs: Vec<bool>, pub mx: Vec<bool>,
    pub enabled: bool, pub state: St, pub page: usize, pub prev_page: usize, pub page_t: Spring, pub dir: f32,
    pub split: bool, pub dock: bool, pub drag_over: bool,
    pub ty: Spring, pub tw: Spring, pub th: Spring, pub tr: Spring, pub last_step: Instant, pub dock_t: Tw, pub split_t: Tw,
    pub class: u32, pub hover: Option<crate::pages::Act>, pub pressed: Option<crate::pages::Act>, pub hits: Vec<(crate::pages::R, crate::pages::Act)>,
    pub seek_drag: bool, pub seek_opt: Option<(u64, Instant)>,
    pub rest_shape: (f32, f32, f32), pub pin: Option<Instant>, pub leave_at: Option<Instant>, pub breathe: Option<(f32, f32, f32)>, pub kb_prev: Option<isize>, pub click_open: bool, pub look_eyes: bool, pub eye_last: (i32, i32), pub drag_prev_page: Option<usize>, pub rect1: (f32, f32, f32, f32), pub rect2: (f32, f32, f32, f32), pub stats: Stats, pub dirty: bool, pub started: Instant, pub force: Option<St>, pub region: (i32, i32),
    pub p: crate::pages::PageState,
}

thread_local! { static APP: RefCell<Option<App>> = const { RefCell::new(None) }; }

impl App {
    fn mon(&self) -> Mon { self.mons[self.active.min(self.mons.len() - 1)] }
    pub fn now() -> Instant { Instant::now() }

    /// forma alvo do estado atual (w, h, raio)
    pub fn target(&self) -> (f32, f32, f32) {
        match self.state {
            St::Hidden | St::Line => (MINI_W, MINI_H, MINI_R), // mini notch (janela maximizada)
            St::Peek => (crate::pages::peek_w(), 30.0, 14.0),
            St::Live => crate::pages::live_size(),
            St::Over => (280.0, 48.0, 22.0),
            St::Open if self.breathe.is_some() => self.breathe.unwrap(),
            St::Open => (if self.split && self.page == 0 { 520.0 } else { OPEN_W }, OPEN_H + if self.dock || (self.page == 3 && self.p.task_cal) { 32.0 } else { 0.0 }, 28.0),
        }
    }
    fn is_live(&self) -> bool { crate::pages::is_live() }

    /// decide o estado a partir do cursor (class), tela cheia/maximizada, "ao vivo"
    pub fn evaluate_pub(&mut self) { self.evaluate(); }
    /// Arrasto de arquivos sobre o notch: vai para Arquivos sozinho; se o arrasto sai sem soltar, volta a pagina anterior (apos um drop, fica).
    pub fn set_drag(&mut self, on: bool) {
        if on == self.drag_over { return; }
        if on && crate::droptarget::OVER.load(Relaxed) { crate::state_log("Shelf aberto automaticamente (causa: OLE DragEnter na janela)"); }
        self.drag_over = on;
        if on { crate::droptarget::DROPPED.store(false, Relaxed); if self.page != crate::pages::P_SHELF { self.drag_prev_page = Some(self.page); self.go_page(crate::pages::P_SHELF); } }
        else if let Some(p) = self.drag_prev_page.take() { if !crate::droptarget::DROPPED.swap(false, Relaxed) { self.go_page(p); } }
    }
    /// Olhos (aparencia recolhida): proxima piscada em 3-7 s enquanto o peek esta visivel (um timer, sem quadros continuos)
    pub fn eye_blink(&mut self) {
        if self.state != St::Peek || crate::settings::get().collapsed_look != "eyes" { return; }
        self.p.blink_t = Some(Instant::now()); self.dirty = true;
        let ms = 3000 + (Instant::now().duration_since(self.started).subsec_nanos() as u64 / 1000 % 4000) as u32;
        unsafe { SetTimer(Some(self.hwnd), T_EYES, ms, None); }
    }
    fn evaluate(&mut self) {
        let now = Instant::now();
        let zone = self.class & 3; let want_dock = self.class & 4 != 0 && !(self.page == 3 && self.p.task_cal); let mi = (self.class >> 3) as usize;
        let n = self.mons.len();
        if let Some(f) = self.force { if f != self.state { self.set_state(f, now); } else { self.publish_hit(); } return; }
        // monitor preferido: seguir o cursor (padrao), o principal, ou um fixo
        let pref = crate::settings::get().monitor;
        let want = match pref.as_str() { "cursor" => None, "primary" => Some(self.mons.iter().position(|m| m.x <= 0 && m.y <= 0 && m.x + m.w > 0 && m.y + m.h > 0).unwrap_or(0)), p => p.strip_prefix('m').and_then(|v| v.parse::<usize>().ok()).filter(|i| *i < n).or(Some(0)) };
        match want {
            Some(i) => if i != self.active && self.state != St::Open { self.active = i; ACTIVE.store(i as u32, Relaxed); self.place(); },
            None => if mi < n && mi != self.active && self.state != St::Open && self.class != 0 { self.active = mi; ACTIVE.store(mi as u32, Relaxed); self.place(); },
        }
        if let Some(p) = self.pin { if Instant::now() < p { self.set_state(St::Open, now); return; } else { self.pin = None; } }
        let a = self.active.min(n - 1);
        if zone >= 1 && self.enabled && !self.fs[a] && self.gfx.g.is_none() { self.ensure_gfx(); } // aquece o renderer ao chegar perto: abrir nao espera ~150 ms
        let fs_hide = self.fs[a] && crate::settings::flag(crate::settings::F_HIDE_FS);
        let new = if !self.enabled || fs_hide { St::Hidden }
            else if zone == 2 && self.state != St::Hidden && (!crate::settings::flag(crate::settings::F_CLICK) || self.click_open || self.state == St::Open) { St::Open }
            else if zone >= 1 && matches!(self.state, St::Line | St::Peek) { St::Peek }
            else { self.rest_state(a) };
        // overlay de sistema: cobre o estado de repouso (nao aparece com a ilha aberta nem em tela cheia)
        let new = if new != St::Open && new != St::Hidden && crate::overlay::active() { St::Over } else { new };
        // arrastar arquivos (botao apertado sobre o Explorer) ate o topo: abre o Shelf
        let rd = if self.enabled && !self.fs[a] && zone >= 1 && LBTN_DOWN_ON_FILES.load(Relaxed) { real_drag_now() } else { None };
        if let Some(c) = rd { if !self.drag_over { crate::state_log(&format!("Shelf aberto automaticamente (causa: {c})")); } }
        let new = if rd.is_some() { self.set_drag(true); St::Open }
            else { if zone == 0 && !crate::droptarget::OVER.load(Relaxed) { self.set_drag(false); } new };
        // atraso (grace) antes de recolher: ao tirar o mouse a ilha espera GRACE_MS (a menos que va para tela cheia/desligada)
        let new = if self.state == St::Open && new != St::Open && new != St::Hidden && !self.pin.is_some() {
            let g = GRACE_MS.load(Relaxed);
            match self.leave_at {
                None => { self.leave_at = Some(now + Duration::from_millis(g as u64)); unsafe { SetTimer(Some(self.hwnd), T_LEAVE, g.max(10), None); } St::Open }
                Some(t) if now < t => St::Open,
                Some(_) => { self.leave_at = None; new }
            }
        } else { if new == St::Open || self.state != St::Open { self.leave_at = None; } new };
        if self.state == St::Open && new == St::Open { // dock por histerese
            if want_dock != self.dock { self.dock = want_dock; self.dock_t.go(now, if self.dock { 1.0 } else { 0.0 }, 200.0); self.retarget(now, 200.0); }
        }
        if new != St::Open { self.click_open = false; }
        if new != self.state { if new != St::Open { self.dock = false; } self.set_state(new, now); } else { self.publish_hit(); }
    }
    fn rest_state(&self, a: usize) -> St {
        let cfg = crate::settings::get();
        let live = self.is_live() && cfg.show_live && !cfg.hide_during_event && !cfg.minimal_island;
        // padrao = COMPACTO 150x30 (bateria+hora); janela maximizada = mini notch; tela cheia = nada (tratado antes); ao vivo = pilula 168x30
        if self.mx[a] { if cfg.show_trace || self.is_live() { St::Line } else { St::Hidden } } else if live { St::Live } else if cfg.show_trace || (self.is_live() && cfg.show_live) { St::Peek } else { St::Hidden }
    }
    pub fn set_state(&mut self, new: St, now: Instant) {
        let old = self.state;
        // "respira" antes de abrir: a pilula cresce um pouco (164x30 -> 176x34) e so entao abre
        let breath = new == St::Open && matches!(old, St::Peek | St::Live) && !reduced_motion() && !self.drag_over && self.pin.is_none();
        let prev = self.target();
        self.state = new;
        if breath { self.breathe = Some((prev.0 + BREATH_GROW.0, prev.1 + BREATH_GROW.1, prev.2)); unsafe { SetTimer(Some(self.hwnd), T_BREATH, BREATH_MS as u32, None); } } else if new != St::Open || old != St::Open { self.breathe = None; }
        if new == St::Over && old != St::Over { self.ty = Spring::new(-26.0); self.ty.to = 0.0; self.ty.p = SPRING_OPEN; } // entra deslizando do topo com overshoot
        if new == St::Open && old != St::Open { self.split = false; self.dock = false; HOOK_OPEN.store(true, Relaxed); crate::pages::on_open(self); }
        if new != St::Open { HOOK_OPEN.store(false, Relaxed); }
        let dur = match (old, new) { (_, St::Open) => 240.0, (St::Open, _) => 220.0, _ => 200.0 };
        match new {
            St::Hidden => { self.show_rest(false); self.show_island(false); unsafe { SetTimer(Some(self.hwnd), T_TRIM, 30000, None); } }
            St::Line | St::Peek if !self.vis && self.gdi_rest(new) && !(unsafe { IsWindowVisible(self.trace).as_bool() } && old != new && matches!(old, St::Line | St::Peek)) => { self.show_rest(true); unsafe { SetTimer(Some(self.hwnd), T_TRIM, 30000, None); } } // ja em repouso: so a forma GDI
            _ => {
                if !self.vis { // saindo do repouso: a forma comeca no tamanho do traco, sem piscar
                    let (rw, rh, rr) = self.rest_shape; self.tw = Spring::new(rw); self.th = Spring::new(rh); self.tr = Spring::new(rr);
                    if self.ensure_gfx() { self.show_island(true); unsafe { let _ = KillTimer(Some(self.hwnd), T_TRIM); } self.show_rest(false); }
                }
            }
        }
        self.retarget(now, dur);
        self.publish_hit();
        self.rearm_tick();
        self.dirty = true;
        if new == St::Peek && crate::settings::get().collapsed_look == "eyes" { unsafe { SetTimer(Some(self.hwnd), T_LOOK, 33, None); } self.p.eye_move = Some(Instant::now()); unsafe { SetTimer(Some(self.hwnd), T_EYES, 3000, None); } } else { unsafe { let _ = KillTimer(Some(self.hwnd), T_EYES); } }
        if new == St::Peek { self.p.pc_t0 = Some(now); self.p.pc_g = 0; }
        self.sync_metrics();
        crate::log(&format!("estado {old:?} -> {new:?}"));
    }
    pub fn retarget(&mut self, now: Instant, dur: f32) {
        let (w, h, r) = self.target();
        let dur_w = if self.state == St::Open && (w - self.tw.to).abs() > 50.0 && self.tw.to > 300.0 { if w > self.tw.to { 320.0 } else { 300.0 } } else { dur };
        if !(self.tw.busy(now) || self.th.busy(now) || self.tr.busy(now)) { self.last_step = Instant::now(); } // mola parada: o dt comeca agora
        let lyr = self.state == St::Open && self.breathe.is_none() && w > self.tw.to + 50.0 && self.tw.to > 300.0;
        let breath_k = self.state == St::Open && self.breathe.is_some();
        if lyr { self.tw.go_p(w, SPRING_LYRICS); } else if breath_k { self.tw.go_p(w, SPRING_BREATH); } else { self.tw.go(now, w, dur_w); }
        if breath_k { self.th.go_p(h, SPRING_BREATH); self.tr.go(now, r, dur); } else if self.state == St::Open && h > self.th.to + 60.0 { self.th.go_p(h, SPRING_OPEN_H); self.tr.go(now, r, dur); } else { self.th.go(now, h, dur); self.tr.go(now, r, dur); }
        self.update_region(); self.dirty = true;
    }
    /// regiao da janela = uniao da forma atual com a alvo (so a ilha captura cliques)
    fn update_region(&mut self) {
        let now = Instant::now(); let s = self.gfx.scale;
        // regiao JUSTA: so a forma desenhada (corpo + flares) captura cliques; o resto da janela deixa o mouse passar para a area de trabalho
        let hh = self.th.pos.clamp(1.0, WIN_H - 2.0); let fl = (6.0 + (OPEN_FLARE - 6.0) * ((hh - 30.0) / (OPEN_H - 30.0)).clamp(0.0, 1.0)).min(hh * 0.75);
        let w = (self.tw.pos.clamp(1.0, WIN_W - 4.0 - 2.0 * OPEN_FLARE) + 2.0 * fl + 2.0).min(WIN_W); let h = (hh + 2.0).min(WIN_H);
        let (rw, rh) = ((w * s).ceil() as i32, (h * s).ceil() as i32);
        if self.region == (rw, rh) { return; }
        self.region = (rw, rh);
        let ox = ((WIN_W * s - rw as f32) / 2.0) as i32;
        unsafe { let rg = CreateRectRgn(ox, 0, ox + rw, rh); SetWindowRgn(self.hwnd, Some(rg), false); }
    }
    fn publish_hit(&self) {
        let m = self.mon(); let s = m.s; let (w, h, _) = self.target();
        let (cw, ch) = match self.state { St::Line | St::Hidden => (w.max(80.0), 24.0), St::Peek | St::Live | St::Over => (w + 16.0, h + 8.0), St::Open => (w + 16.0, h + 8.0) };
        let cx = m.x + m.w / 2;
        HIT[0].store(cx - (cw * s / 2.0) as i32, Relaxed); HIT[1].store(m.y, Relaxed);
        HIT[2].store(cx + (cw * s / 2.0) as i32, Relaxed); HIT[3].store(m.y + (ch * s) as i32, Relaxed);
    }
    fn place(&mut self) {
        let m = self.mon();
        let (w, h) = ((WIN_W * m.s).round() as u32, (WIN_H * m.s).round() as u32);
        self.gfx.resize(w, h, m.s);
        unsafe { let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), m.x + (m.w - w as i32) / 2, m.y, w as i32, h as i32, SWP_NOACTIVATE); }
        if self.gdi_rest(self.state) && !self.vis { self.show_rest(true); }
        self.region = (0, 0); self.update_region(); self.publish_hit(); self.dirty = true;
    }
    pub fn go_page(&mut self, p: usize) { let d = if p > self.page { 1.0 } else { -1.0 }; self.go_page_dir(p, d); }
    /// troca de aba LATERAL: `dir` = sentido do deslize (pela ordem das abas; setas/roda dao a volta com o sentido do clique)
    pub fn go_page_dir(&mut self, p: usize, dir: f32) {
        if p == self.page { return; }
        let now = Instant::now();
        self.dir = dir;
        self.prev_page = self.page; self.page = p; self.page_t = Spring::new(0.0); self.page_t.go_p(1.0, SPRING_PAGE); if self.tw.settled() && self.th.settled() && self.tr.settled() { self.last_step = Instant::now(); } self.p.dots_at = Some((now, self.prev_page));
        let (w, _, _) = self.target(); self.tw.go(now, w, 300.0);
        crate::pages::save_page(p); self.dirty = true;
    }
    /// ha algo animando? (precisa de quadros no ritmo do monitor)
    /// animacao de forma/pagina/dock (ou pedido de redesenho): um quadro por vblank
    fn animating_full(&self, now: Instant) -> bool {
        self.dirty || (self.page == crate::pages::P_SHELF && self.p.shelf_busy()) || !self.ty.settled() || self.p.blink_t.is_some_and(|t| t.elapsed().as_millis() < 260) || self.tw.busy(now) || self.th.busy(now) || self.tr.busy(now) || self.page_t.busy(now) || self.dock_t.busy(now) || self.p.lyr_t.is_some_and(|t| t.elapsed().as_millis() < 260) || self.p.scroll.is_some_and(|s| s.busy(now)) || self.p.pc_chg.is_some_and(|t| self.state == St::Peek && t.elapsed().as_millis() < 340) || self.p.spin.as_ref().is_some_and(|s| s.t.elapsed().as_millis() < 640)
    }
    fn animating(&self, now: Instant) -> bool {
        self.animating_full(now) || (self.state == St::Open && self.page == crate::pages::P_SHELF && self.drag_over) || (self.state == St::Live && crate::pages::eq_anim()) || (self.state == St::Open && crate::pages::open_anim(self, now))
    }
    fn rearm_tick(&self) {
        unsafe {
            let ms = match self.state { St::Open => if crate::pages::is_playing() && crate::island::reduced_motion() { 250 } else { 1000 }, St::Live => 1000, St::Peek => if crate::settings::get().collapsed_look == "eyes" { 1000 } else { let t = windows::Win32::System::SystemInformation::GetLocalTime(); ((60 - t.wSecond as u32) * 1000 + 50).max(500) }, _ => 0 }; // compacto: acorda so na virada do minuto
            if ms == 0 { let _ = KillTimer(Some(self.hwnd), T_TICK); } else { SetTimer(Some(self.hwnd), T_TICK, ms, None); }
        }
    }

    fn frame(&mut self) {
        if !self.gfx.has() || !self.vis { self.dirty = false; return; }
        let t0 = Instant::now();
        let now = t0;
        let mut dt = (now - self.last_step).as_secs_f32(); self.last_step = now;
        if dt > 0.1 { dt = 0.016; } // quadro atrasado (criacao do renderer): a mola nao 'pula' o tempo perdido
        if crate::settings::flag(crate::settings::F_FAST) { dt *= 1.0 / 0.6; } // velocidade 'Rapida': duracoes x0,6
        dt *= crate::settings::ANIM_X100.load(Relaxed) as f32 / 100.0; // Animacoes "Equilibrada"/"Leve": molas mais curtas
        if matches!(self.state, St::Live | St::Peek) { let t = self.target(); if (t.0 - self.tw.to).abs() > 0.5 || (t.1 - self.th.to).abs() > 0.5 { self.retarget(now, 240.0); } } // barra de letras 300x34 entra/sai quando a letra chega
        self.tw.step(dt); self.th.step(dt); self.tr.step(dt); self.ty.step(dt); self.page_t.step(dt);
        // overshoot nunca passa da janela nem fica negativo
        let (w, h, r) = (self.tw.pos.clamp(1.0, WIN_W - 4.0 - 2.0 * OPEN_FLARE), self.th.pos.clamp(1.0, WIN_H - 2.0), self.tr.pos.clamp(0.0, 60.0));
        if crate::selftest::on() { crate::selftest::rec(self.started.elapsed().as_secs_f32() * 1000.0, dt * 1000.0, w, h, self.page_t.pos); }
        #[cfg(feature = "devtools")] crate::bench::flog(self.started.elapsed().as_secs_f32() * 1000.0, dt * 1000.0, w, h, &format!("{:?}", self.state), self.page);
        self.update_region();
        let ix = (WIN_W - w) / 2.0;
        let cur = (ix - OPEN_FLARE - 2.0, 0.0, w + 2.0 * OPEN_FLARE + 4.0, h + 4.0); // inclui os flares laterais (ate 18 px)
        let u = |a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)| { let (x0, y0) = (a.0.min(b.0), a.1.min(b.1)); let (x1, y1) = ((a.0 + a.2).max(b.0 + b.2), (a.1 + a.3).max(b.1 + b.3)); (x0, y0, x1 - x0, y1 - y0) };
        let clr = u(u(cur, self.rect1), self.rect2); self.rect2 = self.rect1; self.rect1 = cur;
        if let Err(e) = self.gfx.begin(clr) { crate::log(&format!("frame: begin falhou {e}")); self.recover(); return; }
        if self.state != St::Hidden {
            self.gfx.set_t(ix, self.ty.pos.min(0.0), 1.0); // posicao vertical (mola): o overlay desce de cima
            let mut hits = Vec::new();
            crate::pages::draw(self, now, w, h, r, &mut hits);
            self.hits = hits;
            self.gfx.set_t(0.0, 0.0, 1.0);
        }
        let cpu = t0.elapsed().as_secs_f32() * 1000.0;
        if let Err(e) = self.gfx.end() { crate::log(&format!("frame: end falhou {e}")); self.recover(); return; }
        let done = Instant::now();
        // metricas por rajada de animacao
        if let Some(l) = self.stats.last { let iv = (done - l).as_secs_f32() * 1000.0; if iv < 60.0 { self.stats.iv.push(iv); self.stats.cpu.push(cpu); } else { self.stats.report("rajada"); } }
        self.stats.last = Some(done);
        if self.stats.iv.len() >= 240 { self.stats.report("continuo"); }
        self.dirty = false;
        if !self.animating(Instant::now()) { self.stats.report(&format!("{:?}", self.state)); self.stats.last = None; self.update_region_settled(); self.settle_line(); }
    }
    fn update_region_settled(&mut self) { self.update_region(); } // regiao justa (corpo + flares) tambem em repouso
    fn recover(&mut self) { crate::log("gfx: perda de dispositivo, recriando"); self.gfx.g = None; self.ensure_gfx(); }
    fn ensure_gfx(&mut self) -> bool {
        if self.gfx.g.is_none() {
            let (w, h) = self.gfx.size;
            match Gfx::new(self.hwnd, w, h, self.gfx.scale) { Ok(g) => { self.gfx.g = Some(g); crate::log("gfx: criado"); } Err(e) => { crate::log(&format!("gfx falhou: {e}")); return false; } }
        }
        true
    }
    fn show_island(&mut self, on: bool) { if on != self.vis { self.vis = on; unsafe { let _ = ShowWindow(self.hwnd, if on { SW_SHOWNOACTIVATE } else { SW_HIDE }); } } }
    /// Forma de repouso desenhada em GDI (sem D3D): mini notch 64x8 (janela maximizada) ou compacto 150x30 com bateria e hora. Ambos com flares.
    fn show_rest(&mut self, on: bool) {
        unsafe {
            if !on { let _ = ShowWindow(self.trace, SW_HIDE); let _ = KillTimer(Some(self.hwnd), T_CLK); return; }
            let compact = self.state == St::Peek;
            let (bw, bh, rad, fl) = if compact { (150.0f32, 30.0f32, 14.0f32, 6.0f32) } else { (MINI_W, MINI_H, MINI_R, MINI_FLARE) };
            self.rest_shape = (bw, bh, rad);
            let m = self.mon(); let s = m.s;
            let (w, h) = (((bw + 2.0 * fl) * s).round() as i32, (bh * s).round().max(2.0) as i32);
            let (x0, r, f) = (fl * s, rad * s, fl * s);
            let screen = GetDC(None); let mem = CreateCompatibleDC(Some(screen));
            let bmi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, ..Default::default() }, ..Default::default() };
            let mut bits = std::ptr::null_mut();
            if let Ok(bmp) = CreateDIBSection(Some(mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
                let old = SelectObject(mem, bmp.into());
                let px = std::slice::from_raw_parts_mut(bits as *mut u32, (w * h) as usize);
                let mut cov = vec![0f32; px.len()];
                let (bx0, bx1) = (x0, w as f32 - x0);
                for y in 0..h { for x in 0..w {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let c = if fx >= bx0 && fx <= bx1 { // corpo: topo reto, cantos de baixo arredondados
                        let cx = fx.clamp(bx0 + r, bx1 - r); let d = if fy > h as f32 - r { ((fx - cx).powi(2) + (fy - (h as f32 - r)).powi(2)).sqrt() } else { 0.0 }; (r - d + 0.5).clamp(0.0, 1.0)
                    } else if fy <= f { // flares concavos: quadrado menos o quarto de disco centrado no canto
                        let (cx, cy) = if fx < bx0 { (0.0, f) } else { (w as f32, f) }; (((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt() - f + 0.5).clamp(0.0, 1.0)
                    } else { 0.0 };
                    cov[(y * w + x) as usize] = c;
                } }
                if compact { // bateria (formas) + hora (GDI)
                    let (pc, charging) = crate::pages::rest_battery();
                    let col = if charging { (123u32, 224u32, 164u32) } else if pc <= 10 { (255, 107, 107) } else if pc <= 20 { (255, 184, 107) } else { (190, 190, 190) };
                    let (ox, oy, bw2, bh2) = ((x0 + 14.0 * s) as i32, ((h as f32 - 8.0 * s) / 2.0) as i32, (16.0 * s) as i32, (8.0 * s).max(6.0) as i32);
                    let mut put = |x: i32, y: i32, c: (u32, u32, u32)| { if x >= 0 && y >= 0 && x < w && y < h { px[(y * w + x) as usize] = 0xFF00_0000 | c.0 << 16 | c.1 << 8 | c.2; } };
                    for yy in 0..bh2 { for xx in 0..bw2 { let edge = xx == 0 || yy == 0 || xx == bw2 - 1 || yy == bh2 - 1; let fill = xx < (bw2 as f32 * pc as f32 / 100.0) as i32;
                        if edge || fill { put(ox + xx, oy + yy, col); } } }
                    for yy in (bh2 / 4)..(bh2 - bh2 / 4) { for xx in 0..(2.0 * s).max(2.0) as i32 { put(ox + bw2 + xx, oy + yy, col); } }
                    SetBkMode(mem, TRANSPARENT); SetTextColor(mem, COLORREF(0x00FF_FFFF));
                    let font = CreateFontW(-(14.0 * s).round() as i32, 0, 0, 0, 600, 0, 0, 0, FONT_CHARSET(1), FONT_OUTPUT_PRECISION(0), FONT_CLIP_PRECISION(0), ANTIALIASED_QUALITY, 0, w!("Segoe UI"));
                    let of = SelectObject(mem, font.into());
                    let (hm, _) = crate::pages::rest_clock(); let mut v: Vec<u16> = hm.encode_utf16().collect();
                    let mut rc = RECT { left: (x0 + 60.0 * s) as i32, top: ((h as f32 - 20.0 * s) / 2.0) as i32, right: (w as f32 - x0 - 14.0 * s) as i32, bottom: ((h as f32 + 20.0 * s) / 2.0) as i32 };
                    DrawTextW(mem, &mut v, &mut rc, DT_RIGHT | DT_SINGLELINE | DT_NOPREFIX | DT_VCENTER);
                    SelectObject(mem, of); let _ = DeleteObject(font.into());
                }
                for (q, c) in px.iter_mut().zip(&cov) { let a = (*c * 255.0) as u32; let rgb = *q & 0x00FF_FFFF; // GDI nao escreve alfa: reimpoe a cobertura (pre-multiplicado)
                    let mul = |v: u32| (v * a / 255) & 0xFF; *q = a << 24 | mul((rgb >> 16) & 0xFF) << 16 | mul((rgb >> 8) & 0xFF) << 8 | mul(rgb & 0xFF); }
                let bf = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8, ..Default::default() };
                let _ = UpdateLayeredWindow(self.trace, Some(screen), Some(&POINT { x: m.x + (m.w - w) / 2, y: m.y }), Some(&SIZE { cx: w, cy: h }), Some(mem), Some(&POINT::default()), COLORREF(0), Some(&bf), ULW_ALPHA);
                SelectObject(mem, old); let _ = DeleteObject(bmp.into());
            }
            let _ = DeleteDC(mem); ReleaseDC(None, screen);
            let _ = SetWindowPos(self.trace, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
            if compact { // relogio: reagenda para o proximo minuto
                let t = windows::Win32::System::SystemInformation::GetLocalTime(); SetTimer(Some(self.hwnd), T_CLK, ((60 - t.wSecond as u32) * 1000 + 50).max(500), None);
            } else { let _ = KillTimer(Some(self.hwnd), T_CLK); }
        }
    }
    /// Estados desenhados em GDI no repouso (sem renderer D3D): mini notch e compacto com relogio (o compacto com olhos precisa do D3D).
    /// (b) FOCO DO TECLADO: a janela e NOACTIVATE; durante a edicao (nova tarefa, MM:SS do pomodoro) ela vira ativavel, pega o foco e devolve ao app anterior ao terminar.
    pub fn sync_kb(&mut self) {
        let editing = self.p.edit.is_some() || self.p.task_input.is_some();
        unsafe {
            if editing && self.kb_prev.is_none() {
                let fg = GetForegroundWindow(); self.kb_prev = Some(fg.0 as isize);
                let ex = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE); SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, ex & !(WS_EX_NOACTIVATE.0 as isize));
                let (me, other) = (windows::Win32::System::Threading::GetCurrentThreadId(), GetWindowThreadProcessId(fg, None));
                let att = other != 0 && other != me && windows::Win32::System::Threading::AttachThreadInput(me, other, true).as_bool();
                let _ = SetForegroundWindow(self.hwnd); let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(self.hwnd));
                if att { let _ = windows::Win32::System::Threading::AttachThreadInput(me, other, false); }
                crate::log("teclado: foco assumido para edicao");
            } else if !editing && self.kb_prev.is_some() {
                let prev = self.kb_prev.take().unwrap_or(0);
                let ex = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE); SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
                let ph = HWND(prev as *mut _); if prev != 0 && ph != self.hwnd && IsWindow(Some(ph)).as_bool() { let _ = SetForegroundWindow(ph); }
                crate::log("teclado: foco devolvido");
            }
        }
    }
    /// (h) aparencia recolhida aplicada AO VIVO: troca relogio <-> olhos sem desligar/ligar o notch
    /// o compacto de PC esta na tela e ha o que mostrar? (so entao o timer de 1 Hz existe)
    fn metrics_active(&self) -> bool { self.enabled && self.state == St::Peek && self.vis && !self.look_eyes && crate::pages::pc_needs_tick() }
    pub fn sync_metrics(&mut self) {
        unsafe { if self.metrics_active() { SetTimer(Some(self.hwnd), T_METRICS, 1000, None); } else { let _ = KillTimer(Some(self.hwnd), T_METRICS); crate::metrics::reset(); } }
    }
    pub fn apply_look(&mut self) {
        let eyes = crate::settings::get().collapsed_look == "eyes";
        if eyes == self.look_eyes { return; }
        self.look_eyes = eyes;
        if self.state == St::Peek {
            self.show_rest(false); if eyes && self.vis { /* ja no renderer */ } else { self.show_island(false); }
            let now = Instant::now(); let s = self.state; self.set_state(s, now);
            if !eyes { unsafe { let _ = KillTimer(Some(self.hwnd), T_LOOK); } self.settle_line(); }
        }
    }
    fn gdi_rest(&self, st: St) -> bool { st == St::Line } // mini notch (so forma) em GDI; o compacto usa o MESMO renderer/fonte/icone do resto (sec.19)
    /// Repouso no traco concluido: troca a ilha DComp pela janela GDI e agenda a liberacao do renderer.
    fn settle_line(&mut self) {
        if !self.gdi_rest(self.state) || !self.vis { return; }
        self.show_rest(true); self.show_island(false);
        unsafe { SetTimer(Some(self.hwnd), T_TRIM, 30000, None); }
    }
    fn trim(&mut self) {
        if (self.gdi_rest(self.state) || self.state == St::Hidden) && !self.vis && self.gfx.g.is_some() {
            self.gfx.g = None; crate::log("gfx: liberado (repouso)");
            unsafe { let _ = windows::Win32::System::Threading::SetProcessWorkingSetSize(windows::Win32::System::Threading::GetCurrentProcess(), usize::MAX, usize::MAX); }
        }
    }

    fn refresh_monitors(&mut self) {
        let n = monitors(); if n.is_empty() { return; }
        self.mons = n; if self.active >= self.mons.len() { self.active = 0; }
        for (i, m) in self.mons.iter().enumerate().take(8) { for (j, v) in [m.x, m.y, m.w, m.h, (m.s * 1000.0) as i32].into_iter().enumerate() { MONS[i][j].store(v, Relaxed); } }
        MON_N.store(self.mons.len().min(8) as u32, Relaxed); ACTIVE.store(self.active as u32, Relaxed);
        let (fs, mx) = scan(&self.mons, self.hwnd); self.fs = fs; self.mx = mx;
    }
}

fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

unsafe extern "system" fn trace_proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if m == WM_CLOSE { let i = HW.load(Relaxed); if i != 0 { let _ = PostMessageW(Some(HWND(i as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0)); } return LRESULT(0); } // taskkill sem /F fecha o traco visivel: encerra o app de forma limpa
    DefWindowProcW(h, m, wp, lp)
}

static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
unsafe extern "system" fn wndproc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if m != 0 && m == TASKBAR_CREATED.load(Relaxed) { crate::state_log("TaskbarCreated (explorer reiniciou): bandeja recriada"); crate::tray::add(h); crate::log("TaskbarCreated: icone da bandeja recriado"); return LRESULT(0); } // explorer.exe reiniciou
    match m {
        WM_MOUSEACTIVATE => { let grab = with_app(|a| a.kb_prev.is_some()).unwrap_or(false); return LRESULT(if grab { MA_ACTIVATE } else { MA_NOACTIVATE } as isize) }
        0x00FF => { handle_raw(lp); return DefWindowProcW(h, m, wp, lp) }
        WM_CLASS => { with_app(|a| { a.class = wp.0 as u32; a.evaluate(); }); return LRESULT(0) }
        WM_WAKE => { with_app(|a| { a.apply_look(); a.sync_metrics(); a.sync_kb(); crate::pages::on_wake(a); if crate::droptarget::OVER.load(Relaxed) { a.set_drag(true); } else if !LBTN_DOWN_ON_FILES.load(Relaxed) { a.set_drag(false); } a.evaluate(); a.rearm_tick(); a.dirty = true; }); return LRESULT(0) }
        WM_TIMER => {
            match wp.0 {
                T_TICK => { with_app(|a| { if !(a.state != St::Peek && crate::pages::eq_anim()) { a.dirty = true; } }); } // com equalizador ja ha quadros periodicos
                T_SCAN => { let _ = KillTimer(Some(h), T_SCAN); with_app(|a| { let (fs, mx) = scan(&a.mons, a.hwnd); if fs != a.fs || mx != a.mx { crate::log(&format!("tela cheia={fs:?} maximizada={mx:?}")); a.fs = fs; a.mx = mx; a.evaluate(); } }); }
                T_CLK => { with_app(|a| { if a.state == St::Peek && !a.vis && a.gdi_rest(St::Peek) { a.show_rest(true); } else { unsafe { let _ = KillTimer(Some(a.hwnd), T_CLK); } } }); }
                T_BREATH => { let _ = KillTimer(Some(h), T_BREATH); with_app(|a| if a.breathe.take().is_some() && a.state == St::Open { a.retarget(Instant::now(), 240.0); }); }
                T_LEAVE => { let _ = KillTimer(Some(h), T_LEAVE); with_app(|a| a.evaluate()); }
                T_NET => { let _ = KillTimer(Some(h), T_NET); crate::overlay::net_check(); }
                T_PTR => { PTR_ARMED.store(false, Relaxed); let _ = KillTimer(Some(h), T_PTR); let mut pt = POINT::default(); let _ = GetCursorPos(&mut pt); pointer_moved(pt, WM_MOUSEMOVE); }
                T_OVER => { let _ = KillTimer(Some(h), T_OVER); with_app(|a| { a.evaluate(); a.dirty = true; }); }
                T_CAPS => { let on = windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x14) & 1 != 0; if CAPS_LAST.swap(on, Relaxed) != on { crate::overlay::caps_event(on); } }
                T_METRICS => { with_app(|a| { if a.metrics_active() { crate::pages::pc_sample(); a.dirty = true; a.frame(); } else { a.sync_metrics(); } }); }
                T_SELFTEST => { with_app(|a| crate::selftest::tick(a)); }
                T_LOOK => { with_app(|a| { if a.state == St::Peek && a.vis && a.look_eyes { let mut p = POINT::default(); unsafe { let _ = GetCursorPos(&mut p); } if (p.x, p.y) != a.eye_last { a.eye_last = (p.x, p.y); a.frame(); } } else { unsafe { let _ = KillTimer(Some(a.hwnd), T_LOOK); } } }); }
                T_EYES => { let _ = KillTimer(Some(h), T_EYES); with_app(|a| a.eye_blink()); }
                T_PIN => { let _ = KillTimer(Some(h), T_PIN); with_app(|a| { a.pin = None; a.evaluate(); }); }
                T_TRIM => { let _ = KillTimer(Some(h), T_TRIM); with_app(|a| a.trim()); }
                #[cfg(feature = "devtools")]
                T_BENCH => { with_app(|a| crate::bench::step(a)); }
                _ => {}
            }
            return LRESULT(0)
        }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => { if m == WM_DISPLAYCHANGE { crate::state_log("WM_DISPLAYCHANGE (monitores mudaram)"); } with_app(|a| { a.refresh_monitors(); a.place(); a.evaluate(); }); return DefWindowProcW(h, m, wp, lp) }
        WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEWHEEL | 0x02A3 => { with_app(|a| { crate::pages::mouse(a, m, wp, lp); if m != WM_MOUSEMOVE { a.sync_kb(); } }); return LRESULT(0) }
        WM_CHAR => { with_app(|a| { crate::pages2::key(a, 0, char::from_u32(wp.0 as u32)); a.sync_kb(); }); return LRESULT(0) }
        WM_KEYDOWN => { with_app(|a| { crate::pages2::key(a, wp.0 as u32, None); a.sync_kb(); }); return LRESULT(0) }
        WM_KILLFOCUS => { with_app(|a| { let e = a.p.edit.take().is_some() | a.p.task_input.take().is_some(); if e { a.dirty = true; } a.sync_kb(); }); return LRESULT(0) }
        0x0312 => { let id = wp.0; with_app(|a| hotkey(a, id)); return LRESULT(0) } // WM_HOTKEY
        WM_OVL => { with_app(|a| { a.evaluate(); a.dirty = true; let ms = crate::overlay::ms_left() + 40; SetTimer(Some(a.hwnd), T_OVER, ms, None); }); return LRESULT(0) }
        0x0218 => { if wp.0 == 0xA { crate::overlay::power_event(); } return LRESULT(1) } // WM_POWERBROADCAST / PBT_APMPOWERSTATUSCHANGE
        WM_CAPS => { with_app(|a| { if CAPS_EN.load(Relaxed) { CAPS_LAST.store(windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(0x14) & 1 != 0, Relaxed); SetTimer(Some(a.hwnd), T_CAPS, 250, None); } else { let _ = KillTimer(Some(a.hwnd), T_CAPS); } }); return LRESULT(0) }
        WM_REHK => { apply_hotkeys(h); return LRESULT(0) }
        WM_TRAY => { let st = with_app(|a| (a.hwnd, a.enabled)); if let Some((hw, en)) = st { crate::tray::on_msg(hw, en, lp); } return LRESULT(0) }
        WM_CLOSE => { with_app(quit); return LRESULT(0) } // fechamento gracioso (instalador/taskkill sem /F)
        WM_NET => { SetTimer(Some(h), T_NET, 500, None); return LRESULT(0) } // rajada de eventos de rede: um unico teste 500 ms depois
        WM_COMMAND => { with_app(|a| crate::tray::on_cmd(a, wp.0 as u32 & 0xFFFF)); return LRESULT(0) }
        _ => {}
    }
    DefWindowProcW(h, m, wp, lp)
}

static CAPS_EN: AtomicBool = AtomicBool::new(false);
static CAPS_LAST: AtomicBool = AtomicBool::new(false);
/// Caps Lock: polling de 250 ms SO enquanto o aviso estiver ligado (GetKeyState, sem hook de teclado)
pub fn caps_poll(on: bool) { CAPS_EN.store(on, Relaxed); let h = HW.load(Relaxed); if h != 0 { unsafe { let _ = PostMessageW(Some(HWND(h as *mut _)), WM_CAPS, WPARAM(0), LPARAM(0)); } } }
pub fn overlay_changed() { let h = HW.load(Relaxed); if h != 0 { unsafe { let _ = PostMessageW(Some(HWND(h as *mut _)), WM_OVL, WPARAM(0), LPARAM(0)); } } }
/// centro (px fisicos) do topo do monitor ativo
pub fn cursor_mon_center() -> (i32, i32) { let i = (ACTIVE.load(Relaxed) as usize).min(7); let m = &MONS[i]; (m[0].load(Relaxed) + m[2].load(Relaxed) / 2, m[1].load(Relaxed)) }
pub fn net_changed() { let h = HW.load(Relaxed); if h != 0 { unsafe { let _ = PostMessageW(Some(HWND(h as *mut _)), WM_NET, WPARAM(0), LPARAM(0)); } } }
pub fn monitor_count() -> u32 { MON_N.load(Relaxed) }
pub fn monitor_label(i: u32) -> String { let m = &MONS[(i as usize).min(7)]; format!("Monitor {} — {}×{}", i + 1, m[2].load(Relaxed), m[3].load(Relaxed)) }
pub fn reapply_hotkeys() { let h = HW.load(Relaxed); if h != 0 { unsafe { let _ = PostMessageW(Some(HWND(h as *mut _)), WM_REHK, WPARAM(0), LPARAM(0)); } } }
/// Registra os 3 atalhos globais (RegisterHotKey); falha = em uso por outro app (HK_STATE=2)
fn apply_hotkeys(h: HWND) {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let c = crate::settings::get().hotkeys;
    for (i, s) in [&c.toggle_island, &c.toggle_timer, &c.open_shelf].into_iter().enumerate() {
        unsafe { let _ = UnregisterHotKey(Some(h), i as i32 + 1);
            let st = match crate::settings::parse_hotkey(s) { Some((m, vk)) => if RegisterHotKey(Some(h), i as i32 + 1, HOT_KEY_MODIFIERS(m | 0x4000), vk).is_ok() { 1 } else { 2 }, None => 0 };
            crate::settings::HK_STATE[i].store(st, Relaxed); crate::log(&format!("atalho {} = {s}: {}", i + 1, ["nao registrado", "ok", "em uso por outro app"][st as usize])); }
    }
    crate::settings_ui::refresh();
}
fn hotkey(a: &mut App, id: usize) {
    crate::log(&format!("atalho global {id}"));
    match id {
        1 => { let on = !a.enabled; set_enabled(a, on, "atalho global (alternar notch)"); }
        2 => crate::timer::toggle_play(),
        3 => { a.go_page(crate::pages::P_SHELF); a.pin = Some(Instant::now() + Duration::from_secs(4)); unsafe { SetTimer(Some(a.hwnd), T_PIN, 4000, None); } a.evaluate(); }
        _ => {}
    }
}
pub fn set_enabled(a: &mut App, on: bool, cause: &str) { a.enabled = on; a.evaluate(); crate::log(&format!("ilha {}", if on { "ligada" } else { "desligada" })); crate::state_log(&format!("ilha {} (causa: {cause})", if on { "LIGADA" } else { "DESLIGADA" })); }
pub fn quit(a: &mut App) { crate::state_log("sair (menu da bandeja)"); crate::tray::remove(a.hwnd); crate::log("sair"); crate::fast_exit(0); }

pub fn start() {
    std::thread::Builder::new().name("island".into()).spawn(move || unsafe {
        let _ = windows::Win32::System::Ole::OleInitialize(None);
        let hi: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: hi, lpszClassName: w!("EavelyIsland"), ..Default::default() };
        RegisterClassW(&wc);
        let mut mons = monitors(); if mons.is_empty() { mons.push(Mon { x: 0, y: 0, w: 1920, h: 1080, s: 1.0 }); } // sessao sem monitor (remota/bloqueada): padrao ate o WM_DISPLAYCHANGE
        let mut p = POINT::default(); let _ = GetCursorPos(&mut p);
        let active = mons.iter().position(|m| p.x >= m.x && p.x < m.x + m.w && p.y >= m.y && p.y < m.y + m.h).unwrap_or(0);
        let m = mons[active];
        let (pw, ph) = ((WIN_W * m.s).round() as i32, (WIN_H * m.s).round() as i32);
        let hwnd = CreateWindowExW(WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, w!("EavelyIsland"), w!("Eavely"), WS_POPUP,
            m.x + (m.w - pw) / 2, m.y, pw, ph, None, None, Some(hi), None).unwrap();
        HW.store(hwnd.0 as isize, Relaxed);
        crate::log(&format!("hwnd={}", hwnd.0 as isize));
        let wc2 = WNDCLASSW { lpfnWndProc: Some(trace_proc), hInstance: hi, lpszClassName: w!("EavelyTrace"), ..Default::default() };
        RegisterClassW(&wc2);
        let trace = CreateWindowExW(WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST, w!("EavelyTrace"), w!(""), WS_POPUP, 0, 0, 1, 1, None, None, Some(hi), None).unwrap();
        let gfx = Slot { g: None, scale: m.s, size: (pw as u32, ph as u32) };
        let mut hz = 60.0f32;
        let mut ti = DWM_TIMING_INFO { cbSize: size_of::<DWM_TIMING_INFO>() as u32, ..Default::default() };
        if DwmGetCompositionTimingInfo(hwnd, &mut ti).is_ok() && ti.rateRefresh.uiDenominator != 0 { hz = ti.rateRefresh.uiNumerator as f32 / ti.rateRefresh.uiDenominator as f32; }
        let mut app = App { hwnd, trace, vis: false, gfx, mons: mons.clone(), active, fs: vec![false; mons.len()], mx: vec![false; mons.len()], enabled: true, state: St::Hidden,
            breathe: None, kb_prev: None, click_open: false, look_eyes: crate::settings::get().collapsed_look == "eyes", eye_last: (0, 0), page: crate::pages::load_page(), prev_page: 0, page_t: Spring::new(1.0), dir: 1.0, split: crate::pages::load_split(), dock: false, drag_over: false,
            ty: Spring::new(0.0), tw: Spring::new(MINI_W), th: Spring::new(MINI_H), tr: Spring::new(MINI_R), last_step: Instant::now(), dock_t: Tw::new(0.0), split_t: Tw::new(0.0), class: 0, hover: None, pressed: None, hits: Vec::new(),
            seek_drag: false, seek_opt: None, rest_shape: (MINI_W, MINI_H, MINI_R), pin: None, leave_at: None, drag_prev_page: None, rect1: (0.0, 0.0, WIN_W, WIN_H), rect2: (0.0, 0.0, WIN_W, WIN_H), stats: Stats { iv: Vec::new(), cpu: Vec::new(), last: None, streaks: 0, hz }, dirty: true, started: Instant::now(), force: None, region: (0, 0),
            p: crate::pages::PageState::default() };
        app.refresh_monitors();
        crate::log(&format!("ilha: monitor {}x{} @{:.2}x, {hz:.0} Hz, {} monitores", m.w, m.h, m.s, mons.len()));
        app.place();
        TASKBAR_CREATED.store(RegisterWindowMessageW(w!("TaskbarCreated")), Relaxed);
        crate::tray::add(hwnd);
        apply_hotkeys(hwnd);
        crate::overlay::reconfigure();
        crate::droptarget::register(hwnd);
        #[cfg(feature = "devtools")]
        if let Ok(f) = std::env::var("EAVELY_FORCE") { app.force = Some(match f.as_str() { "open" => St::Open, "peek" => St::Peek, "live" => St::Live, "line" => St::Line, _ => St::Hidden }); }
        #[cfg(feature = "devtools")]
        if let Ok(p) = std::env::var("EAVELY_PAGE") { app.page = p.parse().unwrap_or(0); }
        #[cfg(feature = "devtools")]
        if std::env::var_os("EAVELY_SPLIT").is_some() { app.split = true; }
        #[cfg(feature = "devtools")]
        if std::env::var_os("EAVELY_LIVE_EXPANDED").is_some() { crate::pages::set_live_expanded(true); }
        #[cfg(feature = "devtools")]
        if std::env::var_os("EAVELY_DOCK").is_some() { app.dock = true; }
        crate::pages::on_wake(&mut app);
        app.evaluate();
        APP.with(|a| *a.borrow_mut() = Some(app));
        if crate::selftest::on() { SetTimer(Some(hwnd), T_SELFTEST, 100, None); }
        #[cfg(feature = "devtools")]
        if std::env::var_os("EAVELY_BENCH").is_some() || std::env::var_os("EAVELY_SHOTS").is_some() { SetTimer(Some(hwnd), T_BENCH, 1500, None); }
        { use windows::Win32::UI::Input::*; // Raw Input so de mouse (RIDEV_INPUTSINK): nao e hook global, nao injeta nada nos outros processos
            let rid = [RAWINPUTDEVICE { usUsagePage: 1, usUsage: 2, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd }];
            crate::log(&format!("raw input mouse: {:?}", RegisterRawInputDevices(&rid, size_of::<RAWINPUTDEVICE>() as u32).is_ok())); }
        for (a, b) in [(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND), (EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZEEND), (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND)] {
            let _ = SetWinEventHook(a, b, None, Some(winevent), 0, 0, WINEVENT_OUTOFCONTEXT);
        }
        // laco: com animacao, acorda no ritmo do vblank (swap chain com espera); sem animacao, dorme ate chegar mensagem
        let mut msg = MSG::default();
        let mut last_frame = Instant::now(); let mut next_due = last_frame;
        'outer: loop {
            let (full, eq) = APP.with(|a| a.try_borrow().ok().and_then(|g| g.as_ref().map(|x| { let n = Instant::now(); (x.animating_full(n) && x.vis, x.animating(n) && x.vis) })).unwrap_or((false, false)));
            let viz = crate::settings::VIZ_FPS.load(Relaxed); let full = full || (eq && viz == 0); // equalizador "ligado": no ritmo do monitor
            let fps = if viz == 0 { 60 } else { viz };
            // parado: dorme ate chegar mensagem. Forma/pagina animando: DwmFlush bloqueia ate o proximo vblank (60/120/144 Hz) -> um quadro por refresh.
            // So equalizador/relogio: ~30 fps (EQ_FPS), sem acordar a cada vblank.
            if !eq { let _ = MsgWaitForMultipleObjectsEx(None, INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE); }
            else if !full { let _ = MsgWaitForMultipleObjectsEx(None, 1000 / fps, QS_ALLINPUT, MWMO_INPUTAVAILABLE); }
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() { if msg.message == WM_QUIT { break 'outer; } let _ = TranslateMessage(&msg); DispatchMessageW(&msg); }
            if full { let _ = DwmFlush(); let n = Instant::now();
                // limite de quadros (60/30): um quadro so quando o proximo "prazo" chegou (nao depende de saber o Hz do monitor)
                let cap = Duration::from_micros(crate::settings::CAP_US.load(Relaxed) as u64);
                if cap.is_zero() || n + Duration::from_millis(1) >= next_due {
                    next_due = if next_due + cap < n { n } else { next_due + cap }; last_frame = n;
                    with_app(|a| if a.vis && a.animating(Instant::now()) { a.frame() });
                } }
            else if eq && last_frame.elapsed() >= Duration::from_millis(1000 / fps as u64 - 2) { last_frame = Instant::now(); with_app(|a| if a.vis && a.animating(Instant::now()) { a.frame() }); }
        }
    }).ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    const M: (i32, i32, i32, i32) = (0, 0, 1920, 1080);
    #[test] fn jogo_borderless_e_tela_cheia() { assert_eq!(classify((0, 0, 1920, 1080), M, false, false), Cover::Full); }
    #[test] fn video_do_navegador_com_estilo_de_caption_mas_retangulo_exato_e_tela_cheia() { assert_eq!(classify((0, 0, 1920, 1080), M, true, false), Cover::Full); }
    #[test] fn janela_maximizada_com_bordas_invisiveis_nao_e_tela_cheia() {
        assert_eq!(classify((-8, -8, 1928, 1040), M, true, true), Cover::Max);      // barra de tarefas embaixo
        assert_eq!(classify((-8, -8, 1928, 1088), M, true, true), Cover::Max);      // barra de tarefas auto-ocultar: cobre tudo, mas nao e exato
    }
    #[test] fn segundo_monitor_e_janela_menor() {
        let m2 = (1920, 0, 1366, 768);
        assert_eq!(classify((1920, 0, 3286, 768), m2, false, false), Cover::Full);
        assert_eq!(classify((100, 100, 900, 700), M, true, false), Cover::None);
        assert_eq!(classify((1912, -8, 3294, 776), m2, true, true), Cover::Max);
        assert_eq!(classify((-8, -8, 1928, 1040), m2, true, true), Cover::None);   // maximizada no monitor 1 nao afeta o 2
    }
}

#[cfg(test)]
mod spring_tests {
    use super::*;
    fn run(p: SpringParams, target: f32) -> (f32, f32, usize) {
        let mut s = Spring::new(0.0); s.to = target; s.p = p;
        let (mut peak, mut frames) = (0.0f32, 0usize);
        while !s.settled() && frames < 600 { s.step(1.0 / 75.0); peak = peak.max(s.pos); frames += 1; }
        (peak, s.pos, frames)
    }
    #[test] fn mola_abrindo_tem_overshoot_e_assenta_sem_nan() {
        let (peak, fin, frames) = run(SPRING_OPEN, 100.0);
        assert!(peak > 102.0 && peak < 130.0, "overshoot esperado ~12%: pico {peak}");
        assert_eq!(fin, 100.0); assert!(frames < 120, "assenta em < 1,6 s: {frames} quadros"); // a 75 Hz
    }
    #[test] fn mola_fechando_e_independente_da_taxa_de_quadros() {
        let a = run(SPRING_CLOSE, 50.0); let mut s = Spring::new(0.0); s.to = 50.0; s.p = SPRING_CLOSE;
        let mut peak = 0.0f32; for _ in 0..400 { s.step(1.0 / 240.0); peak = peak.max(s.pos); if s.settled() { break; } }
        assert!((a.0 - peak).abs() < 1.5, "pico a 75 Hz {} x a 240 Hz {}", a.0, peak);
    }
    #[test] fn corte_por_duracao_assenta_antes_de_0_9_s_e_letras_passam_pouco() {
        for (p, to) in [(SPRING_OPEN, 293.0f32), (SPRING_OPEN_H, 110.0), (SPRING_LYRICS, 100.0)] {
            let mut s = Spring::new(0.0); s.to = to; s.p = p; let (mut peak, mut t) = (0.0f32, 0.0f32);
            while !s.settled() && t < 2.0 { s.step(1.0 / 120.0); t += 1.0 / 120.0; peak = peak.max(s.pos); }
            assert!(t < 0.9, "assenta em {t} s"); assert_eq!(s.pos, to);
            if p.damping == SPRING_LYRICS.damping { assert!(peak > to * 1.04 && peak < to * 1.12, "letras: pico {peak}"); }
            if p.damping == SPRING_OPEN_H.damping { assert!(peak > to * 1.03 && peak < to * 1.12, "altura: pico {peak}"); }
        }
    }
    #[test] fn assentada_nao_pede_quadros_e_dt_enorme_nao_explode() {
        let mut s = Spring::new(10.0); assert!(s.settled());
        s.step(5.0); assert_eq!(s.pos, 10.0);
        s.to = 200.0; s.step(100.0); assert!(s.pos.is_finite() && s.pos < 400.0); // dt e limitado a 50 ms
    }
}

#[cfg(test)]
mod drag_tests {
    use super::*;
    const P: POINT = POINT { x: 500, y: 300 };
    #[test] fn so_arrasto_real_abre_o_shelf() {
        let far = POINT { x: 700, y: 10 };
        assert!(real_drag(far, false, 1, (500, 300), true).is_none(), "botao solto: nunca");
        assert!(real_drag(far, true, 0, (500, 300), true).is_none(), "aperto fora de arquivos: nunca");
        assert!(real_drag(POINT { x: 505, y: 303 }, true, 1, (500, 300), true).is_none(), "sem andar 16 px: e so um clique");
        assert!(real_drag(far, true, 1, (500, 300), false).is_some(), "Explorer + botao + movimento");
        assert!(real_drag(far, true, 2, (500, 300), false).is_none(), "area de trabalho sem imagem de arrasto = selecao por retangulo");
        assert!(real_drag(far, true, 2, (500, 300), true).is_some());
        let _ = P;
    }
}
