// Mascote Eavy (v0.3): porte nativo (Direct2D) do motor mascote3.js. Mesma matematica: seguir-alvo k = 1 - base^dt, tweens por chaves [alvo, ms, easing],
// molas, piscar aleatorio, sprites pixel nitidos. Dados: assets/mascote/eavy/rig-final.json + skin-areia.json (embutidos no exe).
// 30 fps acordado, 4 fps dormindo (estado `sono`), 0 % com a ilha oculta (o laco so pede quadros com a pagina/compacto visivel).
use std::{collections::{HashMap, HashSet}, rc::Rc, sync::{atomic::{AtomicU32, AtomicU8, Ordering::Relaxed}, OnceLock}, time::{Duration, Instant}};
use serde_json::Value;
use windows_numerics::Matrix3x2;
use crate::gfx::{Gfx, Rgba, Seg};

const RIG: &str = include_str!("../../assets/mascote/eavy/rig-final.json");
const SKIN: &str = include_str!("../../assets/mascote/eavy/skin-areia.json");
const TAU: f32 = std::f32::consts::TAU;
type C3 = [f32; 3];
// ordem do catalogo `cenario` do rig (a ordem de desenho do JS = ordem de insercao)
const CENA: [&str; 9] = ["globo", "toca", "parede", "bateria", "sino", "cartaz", "chave", "xicara", "trofeu"];

pub struct Rig { pub v: Value, pub skin: Value, idx: HashMap<String, usize>, body: Vec<Seg> }
pub fn rig() -> &'static Rig {
    static R: OnceLock<Rig> = OnceLock::new();
    R.get_or_init(|| {
        let v: Value = serde_json::from_str(RIG).unwrap_or(Value::Null); let skin: Value = serde_json::from_str(SKIN).unwrap_or(Value::Null);
        let idx = v["reacoes"].as_array().map(|a| a.iter().enumerate().filter_map(|(i, r)| r["id"].as_str().map(|s| (s.to_string(), i))).collect()).unwrap_or_default();
        let body = parse_path(skin["corpo"].as_str().unwrap_or("")); Rig { v, skin, idx, body }
    })
}
impl Rig {
    pub fn ids(&self) -> Vec<String> { self.v["reacoes"].as_array().map(|a| a.iter().filter_map(|r| r["id"].as_str().map(String::from)).collect()).unwrap_or_default() }
    fn def(&self, i: usize) -> &Value { &self.v["reacoes"][i] }
}
fn f(v: &Value) -> f32 { v.as_f64().unwrap_or(0.0) as f32 }
fn fo(v: &Value, d: f32) -> f32 { v.as_f64().map_or(d, |x| x as f32) }
fn hex(s: &str) -> C3 { let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0); [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0] }
fn mix(a: C3, b: C3, t: f32) -> C3 { [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t] }
fn rgba(c: C3, a: f32) -> Rgba { (c[0], c[1], c[2], a) }
fn clamp(v: f32, a: f32, b: f32) -> f32 { v.max(a).min(b) }
fn ease(n: &str, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    match n { "out" => 1.0 - (1.0 - t).powi(3), "inOut" => if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 },
        "back" => { let c1 = 1.7; let c3 = c1 + 1.0; 1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2) } "easeIn" => t * t * t, _ => t }
}
/// tween por chaves [[alvo, ms, easing?], ...]; `init` = valor antes da 1a chave
fn trk(keys: &Value, el: f32, init: f32) -> f32 {
    let (mut t0, mut v) = (0.0f32, init);
    for k in keys.as_array().into_iter().flatten() { let (tg, ms) = (f(&k[0]), f(&k[1]));
        if el < t0 + ms { return v + (tg - v) * ease(k[2].as_str().unwrap_or("lin"), (el - t0) / if ms == 0.0 { 1.0 } else { ms }); } t0 += ms; v = tg; }
    v
}
fn rot(deg: f32, cx: f32, cy: f32) -> Matrix3x2 { let a = deg.to_radians(); let (s, c) = a.sin_cos();
    Matrix3x2 { M11: c, M12: s, M21: -s, M22: c, M31: cx - cx * c + cy * s, M32: cy - cx * s - cy * c } }
fn tr(x: f32, y: f32) -> Matrix3x2 { Matrix3x2::translation(x, y) }
fn sc(x: f32, y: f32) -> Matrix3x2 { Matrix3x2::scale(x, y) }

// ---------------------------------------------------------------- sprites pixel art (SPRG do mascote3.js)
type Grid = Vec<Vec<u8>>;
fn pal(c: u8) -> C3 { hex(match c { b'o' => "#2a2430", b'w' => "#fffaf0", b'g' => "#d3d7e2", b'G' => "#8e94a8", b'D' => "#5b6078", b'y' => "#ffd23f", b'Y' => "#e09a1a", b'q' => "#fff0a0", b'r' => "#e4585f",
    b'b' => "#4f6fe0", b'B' => "#9fb0ff", b'n' => "#b07a3c", b'N' => "#6b4226", b'k' => "#1d1b25", b'c' => "#8fe3ff", b'C' => "#3fa9e0", b'O' => "#ff9a5a", b'l' => "#3ccf8e", b'p' => "#ff9ac8", b'R' => "#8a2a30", _ => "#ff00ff" }) }
fn mg(w: usize, h: usize) -> Grid { vec![vec![b'.'; w]; h] }
fn gr(g: &mut Grid, x: i32, y: i32, w: i32, h: i32, c: u8) { for j in y..y + h { for i in x..x + w { if j >= 0 && (j as usize) < g.len() && i >= 0 && (i as usize) < g[0].len() { g[j as usize][i as usize] = c; } } } }
fn gp(g: &mut Grid, x: i32, y: i32, c: u8) { gr(g, x, y, 1, 1, c); }
fn gd(g: &mut Grid, cx: f32, cy: f32, r: f32, c: u8) { for j in 0..g.len() { for i in 0..g[0].len() { if (i as f32 + 0.5 - cx).powi(2) + (j as f32 + 0.5 - cy).powi(2) <= r * r { g[j][i] = c; } } } }
fn gl(g: &mut Grid, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) { let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1); for k in 0..=n { gp(g, (x0 as f32 + (x1 - x0) as f32 * k as f32 / n as f32).round() as i32, (y0 as f32 + (y1 - y0) as f32 * k as f32 / n as f32).round() as i32, c); } }
/// opcoes de sprite (toca-discos: gravando/braco/rotulo; vinil: rotulo); Copy, serve de chave do cache
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub struct SpO { rec: bool, arm: bool, lab: u8 }
const SP0: SpO = SpO { rec: true, arm: true, lab: b'O' };
fn spo(o: &Value, b: SpO) -> SpO { SpO { rec: o["rec"].as_f64().map_or(b.rec, |x| x != 0.0), arm: o["arm"].as_f64().map_or(b.arm, |x| x != 0.0), lab: o["lab"].as_str().and_then(|s| s.bytes().next()).unwrap_or(b.lab) } }
fn sprite_grid(name: &str, fr: i32, ph: f32, o: &SpO) -> Option<Grid> {
    let ell = |g: &mut Grid, cx: f32, cy: f32, rx: f32, ry: f32, c: u8| { for j in 0..g.len() { for i in 0..g[0].len() { let (dx, dy) = ((i as f32 + 0.5 - cx) / rx, (j as f32 + 0.5 - cy) / ry); if dx * dx + dy * dy <= 1.0 { g[j][i] = c; } } } };
    Some(match name {
        "toca" => { let (rec, arm, lab) = (o.rec, o.arm, o.lab); let mut g = mg(22, 15); gr(&mut g, 1, 7, 20, 7, b'G'); gr(&mut g, 1, 7, 20, 1, b'g'); gr(&mut g, 1, 13, 20, 1, b'D');
            ell(&mut g, 9.5, 6.2, 7.6, 3.1, if rec { b'k' } else { b'D' }); if rec { ell(&mut g, 9.5, 6.2, 2.3, 1.05, lab); }
            if rec && arm { let a = ph * TAU * 4.0; gp(&mut g, (9.5 + a.cos() * 5.2).round() as i32, (6.2 + a.sin() * 2.1).round() as i32, b'G'); } else if !rec { gp(&mut g, 9, 6, b'g'); }
            if arm { gl(&mut g, 19, 3, 13, 6, b'g'); } else { gl(&mut g, 19, 3, 17, 0, b'g'); } gr(&mut g, 18, 2, 3, 3, b'G'); gr(&mut g, 3, 10, 2, 2, b'r'); gr(&mut g, 7, 10, 2, 2, if rec && arm { b'l' } else { b'y' }); g }
        "globo" => { let mut g = mg(16, 20); gr(&mut g, 7, 1, 1, 5, b'g'); gr(&mut g, 6, 5, 3, 2, b'D');
            for j in 0..20 { for i in 0..16 { if (i as f32 + 0.5 - 8.0).powi(2) + (j as f32 + 0.5 - 13.0).powi(2) <= 36.0 { g[j][i] = [b'w', b'B', b'g', b'C'][(i + j + fr as usize) % 4]; } } }
            gp(&mut g, 5, 9, b'w'); gp(&mut g, 6, 8, b'w'); g }
        "vinil" => { let mut g = mg(14, 14); gd(&mut g, 7.0, 7.0, 6.0, b'k'); for j in 0..14 { for i in 0..14 { let d = (i as f32 + 0.5 - 7.0).hypot(j as f32 + 0.5 - 7.0); if d > 3.6 && d < 4.4 { g[j][i] = b'D'; } } }
            gd(&mut g, 7.0, 7.0, 2.2, o.lab); let a = ph * TAU * 4.0; gp(&mut g, 7 + (a.cos() * 4.8).round() as i32, 7 + (a.sin() * 4.8).round() as i32, b'G'); g }
        "trofeu" => { let mut g = mg(14, 16); gr(&mut g, 3, 1, 8, 5, b'y'); gr(&mut g, 4, 6, 6, 2, b'y'); gr(&mut g, 5, 8, 4, 1, b'y'); gr(&mut g, 6, 9, 2, 2, b'Y'); gr(&mut g, 4, 11, 6, 2, b'Y');
            for (x, y) in [(2, 2), (1, 3), (2, 4), (11, 2), (12, 3), (11, 4), (7, 3)] { gp(&mut g, x, y, b'Y'); } gr(&mut g, 4, 2, 1, 3, b'q'); g }
        "medalha" => { let mut g = mg(12, 16); gr(&mut g, 2, 1, 3, 3, b'r'); gr(&mut g, 3, 4, 2, 2, b'r'); gp(&mut g, 4, 6, b'r'); gr(&mut g, 7, 1, 3, 3, b'b'); gr(&mut g, 7, 4, 2, 2, b'b'); gp(&mut g, 7, 6, b'b');
            gd(&mut g, 6.0, 11.0, 3.8, b'y'); gr(&mut g, 5, 10, 2, 2, b'Y'); gp(&mut g, 4, 9, b'q'); g }
        "pasta" => { let mut g = mg(16, 13); gr(&mut g, 1, 1, 6, 2, b'Y'); gr(&mut g, 1, 2, 14, 9, b'Y'); gr(&mut g, 3, 3, 10, 3, b'w'); gr(&mut g, 1, 5, 14, 7, b'y'); gr(&mut g, 1, 5, 14, 1, b'q'); g }
        "pilha" => { let mut g = mg(18, 11); gr(&mut g, 1, 1, 14, 9, b'D'); gr(&mut g, 15, 4, 2, 3, b'G'); gr(&mut g, 1, 1, 14, 1, b'G'); if (fr / 3) % 2 == 0 { gr(&mut g, 2, 2, 3, 7, b'r'); } g }
        "bateria" => { let mut g = mg(14, 22); gr(&mut g, 5, 0, 4, 2, b'G'); gr(&mut g, 2, 2, 10, 19, b'g'); gr(&mut g, 3, 3, 8, 17, b'D'); gr(&mut g, 3, 3, 1, 17, b'G');
            for y in [4, 8, 12] { gr(&mut g, 4, y, 6, 3, b'k'); } gr(&mut g, 4, 16, 6, 3, if (fr / 3) % 2 == 0 { b'r' } else { b'R' }); gp(&mut g, 6, 1, b'w');
            for (x, y) in [(7, 8), (6, 9), (7, 9), (8, 9), (6, 10)] { gp(&mut g, x, y, b'D'); } g }
        "plug" => { let mut g = mg(18, 12); gr(&mut g, 2, 1, 9, 9, b'G'); gr(&mut g, 2, 1, 9, 1, b'g'); gr(&mut g, 2, 9, 9, 1, b'D'); gr(&mut g, 11, 3, 5, 2, b'g'); gr(&mut g, 11, 7, 5, 2, b'g');
            for (x, y) in [(7, 2), (6, 3), (5, 4), (6, 4), (7, 4), (6, 5), (5, 6)] { gp(&mut g, x, y, b'y'); } gr(&mut g, 0, 5, 2, 2, b'D'); g }
        "sino" => { let mut g = mg(14, 15); gd(&mut g, 7.0, 6.0, 4.8, b'y'); gr(&mut g, 2, 6, 10, 4, b'y'); gr(&mut g, 1, 10, 12, 2, b'Y'); gd(&mut g, 7.0, 13.0, 1.5, b'Y'); gp(&mut g, 7, 1, b'Y'); gr(&mut g, 4, 3, 1, 4, b'q'); g }
        "cartaz" => { let mut g = mg(16, 21); gr(&mut g, 1, 1, 14, 11, b'w'); gr(&mut g, 1, 11, 14, 1, b'g');
            for (r, row) in [".CCC.", "C...C", "....C", "...C.", "..C..", ".....", "..C.."].iter().enumerate() { for (c, ch) in row.bytes().enumerate() { if ch != b'.' { gp(&mut g, 5 + c as i32, 3 + r as i32, ch); } } }
            gr(&mut g, 7, 12, 2, 8, b'n'); gr(&mut g, 8, 12, 1, 8, b'N'); g }
        "chave" => { let mut g = mg(14, 16); gr(&mut g, 6, 6, 3, 9, b'G'); gr(&mut g, 6, 6, 1, 9, b'g'); gd(&mut g, 7.5, 4.2, 3.7, b'G'); gr(&mut g, 6, 0, 3, 4, b'.'); gp(&mut g, 5, 3, b'g'); g }
        "xicara" => { let mut g = mg(14, 14); for i in 0..3 { gp(&mut g, 3 + i * 3, 1 + ((fr + i) % 3), b'g'); }
            gr(&mut g, 1, 5, 8, 7, b'w'); gr(&mut g, 1, 5, 8, 1, b'N'); gr(&mut g, 1, 11, 8, 1, b'g'); gr(&mut g, 9, 6, 2, 1, b'w'); gr(&mut g, 10, 7, 1, 3, b'w'); gr(&mut g, 9, 10, 2, 1, b'w'); gr(&mut g, 0, 12, 11, 1, b'G'); g }
        "touca" => { let mut g = mg(26, 16); for j in 1..=9usize { for i in 0..26usize { if (i as f32 + 0.5 - 11.0).powi(2) + (j as f32 + 0.5 - 10.0).powi(2) <= 85.0 { g[j][i] = if (i + j) % 6 < 2 { b'B' } else { b'b' }; } } }
            gr(&mut g, 1, 10, 22, 3, b'w'); gr(&mut g, 20, 5, 3, 6, b'b'); gr(&mut g, 21, 3, 3, 3, b'b'); gd(&mut g, 23.0, 3.0, 2.2, b'w'); g }
        "lupa" => { let mut g = mg(18, 18); gd(&mut g, 7.0, 7.0, 5.6, b'n'); gd(&mut g, 7.0, 7.0, 3.8, b'c'); gp(&mut g, 5, 5, b'w'); gp(&mut g, 6, 5, b'w'); gp(&mut g, 5, 6, b'w'); gl(&mut g, 11, 11, 16, 16, b'N'); gl(&mut g, 12, 11, 16, 15, b'n'); g }
        "jornal" => { let mut g = mg(26, 16); gr(&mut g, 1, 2, 12, 13, b'w'); gr(&mut g, 13, 2, 12, 13, b'w'); gr(&mut g, 12, 2, 2, 13, b'g'); gr(&mut g, 1, 1, 24, 1, b'g');
            gr(&mut g, 2, 3, 9, 2, b'k'); gr(&mut g, 15, 3, 8, 2, b'k'); gr(&mut g, 2, 6, 4, 3, b'D'); gr(&mut g, 7, 6, 4, 1, b'G'); gr(&mut g, 7, 8, 4, 1, b'G'); gr(&mut g, 2, 10, 9, 1, b'G'); gr(&mut g, 2, 12, 9, 1, b'G');
            gr(&mut g, 15, 6, 8, 1, b'G'); gr(&mut g, 15, 8, 8, 1, b'G'); gr(&mut g, 15, 10, 4, 3, b'D'); gr(&mut g, 20, 10, 3, 1, b'G'); gr(&mut g, 20, 12, 3, 1, b'G'); g }
        "parede" => { let mut g = mg(16, 26); gr(&mut g, 4, 0, 12, 26, b'D'); gr(&mut g, 4, 0, 1, 26, b'G'); gr(&mut g, 4, 23, 12, 3, b'k'); gr(&mut g, 1, 9, 6, 9, b'w'); gr(&mut g, 6, 9, 1, 9, b'g'); gr(&mut g, 1, 11, 3, 1, b'k'); gr(&mut g, 1, 15, 3, 1, b'k'); gp(&mut g, 5, 13, b'G'); g }
        _ => return None,
    })
}
/// grade -> corridas horizontais (cor, i, j, comprimento) com contorno 'o' nas bordas, em cache por (nome, quadro, opcoes)
type Runs = std::rc::Rc<(usize, usize, Vec<(u8, u16, u16, u16)>)>;
fn sprite_runs(name: &'static str, tm: f32, o: &SpO) -> Option<Runs> {
    thread_local! { static CACHE: std::cell::RefCell<HashMap<(&'static str, i32, SpO), Option<Runs>>> = std::cell::RefCell::new(HashMap::new()); }
    let fr = ((tm * 7.0).floor() as i32).rem_euclid(28); let key = (name, fr, *o);
    CACHE.with(|c| { let mut m = c.borrow_mut(); if let Some(r) = m.get(&key) { return r.clone(); }
        let r = sprite_grid(name, fr, fr as f32 / 28.0, o).map(|g| {
            let (h, w) = (g.len(), g[0].len()); let mut q = g.clone();
            for j in 0..h { for i in 0..w { if g[j][i] == b'.' && [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)].iter().any(|(a, b)| { let (x, y) = (i as i32 + a, j as i32 + b); x >= 0 && y >= 0 && (y as usize) < h && (x as usize) < w && g[y as usize][x as usize] != b'.' }) { q[j][i] = b'o'; } } }
            let mut runs = Vec::new();
            for j in 0..h { let mut i = 0; while i < w { let c = q[j][i]; if c == b'.' { i += 1; continue; } let s = i; while i < w && q[j][i] == c { i += 1; } runs.push((c, s as u16, j as u16, (i - s) as u16)); } }
            Rc::new((w, h, runs)) });
        if m.len() > 600 { m.clear(); } m.insert(key, r.clone()); r })
}
fn sprite_dim(name: &'static str, o: &SpO) -> (f32, f32) { sprite_runs(name, 0.0, o).map_or((1.0, 1.0), |r| (r.0 as f32, r.1 as f32)) }
/// desenha o sprite centrado na origem atual; u = tamanho do "pixel"; dim escurece (camada de tras)
fn sprite(g: &Gfx, name: &'static str, tm: f32, u: f32, o: &SpO, dim: f32, a: f32, clipx: f32) {
    let Some(r) = sprite_runs(name, tm, o) else { return }; let (w, h) = (r.0 as f32, r.1 as f32);
    g.aa(false);
    for &(c, i, j, n) in &r.2 { let p = pal(c); let x0 = (i as f32 - w / 2.0) * u; let x1 = (x0 + n as f32 * u).min(clipx); if x1 <= x0 { continue; } // clipx: corte vertical no espaco do sprite (pinos somem na tomada)
        g.rrect(x0, (j as f32 - h / 2.0) * u, x1 - x0, u, 0.0, (p[0] * dim, p[1] * dim, p[2] * dim, a)); }
    g.aa(true);
}

// ---------------------------------------------------------------- particulas
#[derive(Clone, Copy, PartialEq, Debug)] enum PT { Star, Spark, Sweat, Z, Dots, Note, Bang, Bolt, File, Conf, Refl, Fumaca, Puff }
#[derive(Clone, Debug)] struct Part { ty: PT, age: f32, life: f32, x: f32, y: f32, vx: f32, vy: f32, size: f32, rot: f32, c: C3, delay: f32, seed: f32 }
#[derive(Clone, Copy, Default)] struct Hand { x: f32, y: f32, r: f32, v: f32 }
#[derive(Clone, Default)] struct Scn { a: f32, t0: f32, vis: bool, par: Option<&'static Value>, traj: Option<&'static Value> }
#[derive(Clone)] struct Cur { gx: f32, gy: f32, tilt: f32, oy: f32, ox: f32, sx: f32, sy: f32, blush: f32, tint_a: f32, tint: C3, mc: f32, mw: f32, mo: f32 }
#[derive(Clone)] struct Tw { ox: f32, oy: f32, sx: f32, sy: f32, roll: f32, tilt: f32, es: f32, blush: f32, hands: f32, hd: f32, cheek: f32, mouth: f32, mc: f32, glow: f32, jit: f32, wave: f32, orbit: bool }
impl Default for Tw { fn default() -> Self { Tw { ox: 0.0, oy: 0.0, sx: 1.0, sy: 1.0, roll: 0.0, tilt: 0.0, es: 1.0, blush: 0.0, hands: 0.0, hd: 0.0, cheek: 0.0, mouth: 0.0, mc: 0.0, glow: 0.0, jit: 0.0, wave: 0.0, orbit: false } } }
struct FxRun { d: usize, t: f32, done: HashSet<usize> }

/// Motor do Eavy (equivalente a `class Mascot` do mascote3.js).
pub struct Eavy {
    t: f32, dt: f32, s_t: f32, seed: u32, cur: Cur, base: usize, base_id: &'static str, fx: Option<FxRun>, parts: Vec<Part>, timers: HashMap<(usize, &'static str), f32>,
    dance: f32, blink_v: f32, next_blink: f32, blink_q: Vec<f32>, ptr: Option<(f32, f32)>, ptr_t: f32, badge_s: f32,
    sc: HashMap<&'static str, Scn>, hand: [Hand; 2], cab: (f32, f32), prev_ox_px: f32, read_lift: f32, el_now: f32,
    tw: Tw, d_roll: f32, dsx: f32, dsy: f32, eye: &'static str, tint_a: f32, badge: Option<&'static str>, face_x: Option<f32>,
    frozen: bool, pub ctx_musica: bool, pub n_eyes: u32, pub n_mouths: u32, pub n_hands: u32,
}
fn bd(r: f32, dt: f32) -> f32 { 1.0 - r.powf(dt) }
impl Eavy {
    pub fn new() -> Eavy {
        let r = rig(); let base = r.v["estado_inicial"].as_str().and_then(|n| r.idx.get(n)).copied().unwrap_or(0);
        let mut e = Eavy { t: 0.0, dt: 0.033, s_t: 0.0, seed: 7, cur: Cur { gx: 0.0, gy: 0.0, tilt: 0.0, oy: 0.0, ox: 0.0, sx: 1.0, sy: 1.0, blush: 0.12, tint_a: 0.0, tint: [1.0; 3], mc: 0.8, mw: fo(&r.skin["bocaDW"], 5.0), mo: 0.0 },
            base, base_id: r.def(base)["id"].as_str().unwrap_or(""), fx: None, parts: Vec::new(), timers: HashMap::new(), dance: 0.0, blink_v: 1.0, next_blink: 0.0, blink_q: Vec::new(), ptr: None, ptr_t: -99.0, badge_s: 0.0,
            sc: HashMap::new(), hand: [Hand { x: 2.0, y: 66.0, r: -35.0, v: 0.0 }, Hand { x: 98.0, y: 66.0, r: 35.0, v: 0.0 }], cab: (0.0, 0.0), prev_ox_px: 0.0, read_lift: 0.0, el_now: 0.0,
            tw: Tw::default(), d_roll: 0.0, dsx: 1.0, dsy: 1.0, eye: "open", tint_a: 0.0, badge: None, face_x: None, frozen: false, ctx_musica: true, n_eyes: 0, n_mouths: 0, n_hands: 0 };
        e.next_blink = 1.5 + e.rnd() * 2.0; e
    }
    fn rnd(&mut self) -> f32 { // mulberry32 (igual ao rng() do JS)
        self.seed = self.seed.wrapping_add(0x6D2B79F5); let s = self.seed; let mut t = (s ^ (s >> 15)).wrapping_mul(1 | s);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t; ((t ^ (t >> 14)) as f64 / 4294967296.0) as f32 }
    /// teste/captura: toca `id` e avanca `ms` ms em quadros de 1/30 s
    pub fn seek(&mut self, id: &str, ms: u32) { let m = self.ctx_musica; *self = Eavy::new(); self.ctx_musica = m; self.play(id); for _ in 0..(ms as f32 / 33.333).round() as u32 { self.step(1.0 / 30.0); } self.frozen = true; }
    /// seed por abertura (a fixa repetia a mesma sequencia de piscadas/emissores); seek/capturas seguem com a seed 7
    pub fn reseed(&mut self, seed: u32) { self.seed = seed; self.next_blink = 1.5 + self.rnd() * 2.0; }
    pub fn dbg(&self) -> String { format!("face_x={:?} cur.ox={} tw.ox={} hand_r=({:.1},{:.1}) v={:.2}", self.face_x, self.cur.ox, self.tw.ox, self.hand[1].x, self.hand[1].y, self.hand[1].v) }
    pub fn unfreeze(&mut self) { self.frozen = false; }
    pub fn base_id(&self) -> &'static str { self.base_id }
    pub fn gesture_running(&self) -> bool { self.fx.is_some() }
    pub fn gesture_id(&self) -> Option<&'static str> { self.fx.as_ref().and_then(|f| rig().def(f.d)["id"].as_str()) }
    pub fn sleeping(&self) -> bool { self.base_id == "sono" }
    /// ponteiro relativo ao centro do mascote, px de tela (esquece em 4 s sem mexer)
    pub fn pointer(&mut self, dx: f32, dy: f32) { self.ptr = Some((dx, dy)); self.ptr_t = self.t; }
    pub fn play(&mut self, id: &str) {
        self.frozen = false;
        let r = rig(); let Some(&i) = r.idx.get(id) else { return }; let d = r.def(i);
        if !d["estado"].is_null() { self.base = i; self.base_id = d["id"].as_str().unwrap_or(""); self.fx = None; self.s_t = 0.0; self.blink_q.push(self.t); return; }
        if let Some(bn) = d["base_now"].as_str() { if let Some(&bi) = r.idx.get(bn) { self.base = bi; self.base_id = r.def(bi)["id"].as_str().unwrap_or(""); self.s_t = 0.0; } }
        self.fx = Some(FxRun { d: i, t: 0.0, done: HashSet::new() });
    }
    fn timer(&mut self, k: (usize, &'static str), every: f32) -> bool {
        let dt = self.dt; let v = self.timers.entry(k).or_insert(every * 0.6); *v -= dt; if *v <= 0.0 { *v += every; true } else { false } }
    fn spawn(&mut self, ty: &str, n: u32, o: &Value) { for i in 0..n { let p = self.mk(ty, i, n, o); self.parts.push(p); } }
    fn mk(&mut self, ty: &str, i: u32, n: u32, o: &Value) -> Part {
        let (fi, fnn) = (i as f32, n as f32); let mid = fi - (fnn - 1.0) / 2.0;
        let mut p = Part { ty: PT::Star, age: 0.0, life: 1.4, x: 50.0, y: 20.0, vx: 0.0, vy: 0.0, size: 7.0, rot: 0.0, c: hex("#ffd45e"), delay: 0.0, seed: 0.0 };
        let oc = o["c"].as_str().map(hex);
        match ty {
            "star" => { p.ty = PT::Star; p.x = 50.0 + mid * 15.0; p.y = 6.0; p.vx = mid * 5.0; p.vy = -22.0 - self.rnd() * 8.0; p.life = fo(&o["life"], 1.5); p.size = 6.0 + self.rnd() * 2.0; p.rot = self.rnd(); p.c = oc.unwrap_or(hex("#ffc94a")); p.delay = fi * 0.14; }
            "spark" => { p.ty = PT::Spark; let a = self.rnd() * TAU; p.x = 50.0 + a.cos() * 40.0; p.y = 46.0 + a.sin() * 36.0; p.vx = a.cos() * 26.0; p.vy = a.sin() * 26.0; p.life = 0.7; p.size = 5.0 + self.rnd() * 3.0; p.c = oc.unwrap_or(hex("#fff6c8")); p.delay = self.rnd() * 0.1;
                if !o["x"].is_null() { p.x = f(&o["x"]); p.y = f(&o["y"]); p.vx = (self.rnd() - 0.5) * 70.0; p.vy = (self.rnd() - 0.5) * 70.0; p.life = 0.5; p.delay = 0.0; } }
            "sweat" => { p.ty = PT::Sweat; p.x = 84.0; p.y = 18.0; p.vy = 26.0; p.life = 1.0; p.size = 5.0; p.c = hex("#8cc8ff"); }
            "z" => { let k = fo(&o["k"], 0.0); p.ty = PT::Z; p.x = 80.0; p.y = 12.0; p.vx = 9.0; p.vy = -14.0; p.life = 1.7; p.size = 9.0 + k * 2.0; p.c = hex("#c9d4f2"); p.delay = k * 0.3; }
            "dots" => { p.ty = PT::Dots; p.x = 84.0; p.y = 4.0; p.life = 1.5; }
            "note" => { p.ty = PT::Note; p.x = 20.0 + self.rnd() * 60.0; p.y = 4.0; p.vx = (self.rnd() - 0.5) * 14.0; p.vy = -18.0; p.life = 1.6; p.size = 11.0; p.c = hex("#ff9ae0"); }
            "bang" => { p.ty = PT::Bang; p.x = 82.0; p.y = -2.0; p.life = 0.9; p.size = 14.0; p.c = hex("#ffb020"); }
            "bolt" => { p.ty = PT::Bolt; let a = self.rnd() * TAU; let d = 44.0 + self.rnd() * 10.0; p.x = 50.0 + a.cos() * d; p.y = 48.0 + a.sin() * d * 0.85; p.rot = a; p.life = 0.3 + self.rnd() * 0.12; p.size = 12.0 + self.rnd() * 10.0;
                p.c = if self.rnd() < 0.5 { hex("#ffe45c") } else { hex("#6ff2ff") }; p.seed = self.rnd() * 99.0; }
            "file" => { p.ty = PT::File; p.x = 50.0; p.y = -28.0; p.vy = 100.0; p.life = 0.42; p.size = 9.0; p.c = hex("#cfe3ff"); }
            "conf" => { p.ty = PT::Conf; p.x = 8.0 + self.rnd() * 84.0; p.y = -14.0; p.vx = (self.rnd() - 0.5) * 26.0; p.vy = 34.0 + self.rnd() * 26.0; p.life = 1.5; p.rot = self.rnd() * 6.0;
                p.c = hex(["#ff5a6a", "#ffc94a", "#4f9bf0", "#1fbf84", "#ef7fae"][(self.rnd() * 5.0) as usize % 5]); p.delay = self.rnd() * 0.35; }
            "refl" => { p.ty = PT::Refl; p.x = -20.0 + self.rnd() * 150.0; p.y = -26.0 + self.rnd() * 110.0; p.life = 0.55; p.size = 3.0; p.c = hex(["#ff7ad9", "#6ff2ff", "#ffe45c", "#9fb0ff", "#ffffff"][(self.rnd() * 5.0) as usize % 5]); }
            "fumaca" => { p.ty = PT::Fumaca; p.x = 40.0 + self.rnd() * 20.0; p.y = 22.0; p.vx = (self.rnd() - 0.5) * 8.0; p.vy = -16.0; p.life = 1.1; p.size = 3.5; p.c = hex("#8f95a8"); }
            "puff" => { p.ty = PT::Puff; p.x = if o["x"].is_null() { 20.0 + self.rnd() * 60.0 } else { f(&o["x"]) + (self.rnd() - 0.5) * 12.0 }; p.y = fo(&o["y"], 80.0); p.vx = (self.rnd() - 0.5) * 30.0; p.vy = -8.0; p.life = 0.45; p.size = 3.0; p.c = hex("#cfcfdc"); }
            _ => {}
        }
        p
    }

    /// avanca `dt` segundos (equivale a step(dt) do JS)
    pub fn step(&mut self, dt: f32) {
        if self.frozen { return; }
        let dt = dt.clamp(1e-3, if self.sleeping() { 0.3 } else { 0.1 }); let r = rig(); let rg = &r.v; let k = &rg["seguir"];
        self.t += dt; self.dt = dt; self.s_t += dt;
        let (k_gen, k_look, k_col) = (bd(f(&k["kGen"]), dt), bd(f(&k["kLook"]), dt), bd(f(&k["kCor"]), dt));
        let base_d: &'static Value = r.def(self.base); let b: &'static Value = &base_d["estado"]; let has_b = !b.is_null();
        let mut tw = Tw::default(); let mut fx_cena_on: Option<&'static Value> = None; let mut fx_par: Option<&'static Value> = None; let mut fx_traj: Option<&'static Value> = None;
        let mut boca_ov: Option<&'static Value> = None; let mut eye_ov: Option<&str> = None; let mut look: Option<(f32, f32)> = None; let mut tint_ov: Option<(C3, f32)> = None;
        let mut el = 0.0f32; let mut fx_d: Option<&'static Value> = None; let mut finish: Option<usize> = None; let mut events: Vec<(usize, &'static Value)> = Vec::new();
        if let Some(fxr) = self.fx.as_mut() {
            let d: &'static Value = r.def(fxr.d); fx_d = Some(d); fxr.t += dt; el = fxr.t * 1000.0;
            if let Some(tk) = d["tracks"].as_object() { for (p, keys) in tk { let init = if matches!(p.as_str(), "sx" | "sy" | "es") { 1.0 } else { 0.0 }; let v = trk(keys, el, init);
                match p.as_str() { "ox" => tw.ox = v, "oy" => tw.oy = v, "sx" => tw.sx = v, "sy" => tw.sy = v, "roll" => tw.roll = v, "tilt" => tw.tilt = v, "es" => tw.es = v, "blush" => tw.blush = v, "hands" => tw.hands = v, "hd" => tw.hd = v, "cheek" => tw.cheek = v, "mouth" => tw.mouth = v, "mc" => tw.mc = v, "glow" => tw.glow = v, "jit" => tw.jit = v, _ => {} } } }
            for e in d["eyes"].as_array().into_iter().flatten() { if el >= f(&e[0]) && el < f(&e[1]) { eye_ov = e[2].as_str(); } }
            let dur = f(&d["dur"]); let (ct0, ct1) = if d["cena_t"].is_null() { (0.0, dur) } else { (f(&d["cena_t"][0]), f(&d["cena_t"][1])) };
            if !d["cena_on"].is_null() && el >= ct0 && el < ct1 { fx_cena_on = Some(&d["cena_on"]); }
            if !d["cena_par"].is_null() { fx_par = Some(&d["cena_par"]); } if !d["cena_traj"].is_null() { fx_traj = Some(&d["cena_traj"]); }
            if let Some(bo) = d["boca"].as_array() { if el >= f(&bo[0]) && el < f(&bo[1]) { boca_ov = Some(&bo[2]); } }
            for l in d["look"].as_array().into_iter().flatten() { if el >= f(&l[0]) { look = Some((f(&l[1]), f(&l[2]))); } }
            if let Some(ti) = d["tint"].as_array() { let (a, z, am) = (f(&ti[0]), f(&ti[1]), f(&ti[3])); let fct = ((el - a) / 150.0).min((z - el) / 300.0).min(1.0).max(0.0); if fct > 0.0 { tint_ov = Some((hex(ti[2].as_str().unwrap_or("#ffffff")), am * fct)); } }
            for (i, ev) in d["events"].as_array().into_iter().flatten().enumerate() { if el >= f(&ev["t"]) && !fxr.done.contains(&i) { fxr.done.insert(i); events.push((i, ev)); } }
            if el >= dur { finish = Some(fxr.d); }
        }
        for (_, ev) in &events { match ev["do"].as_str() { Some("blink") => self.blink_q.push(self.t), Some("spawn") => { let ty = ev["type"].as_str().unwrap_or(""); self.spawn(ty, ev["n"].as_u64().unwrap_or(1) as u32, ev); } _ => {} } }
        if let Some(d) = fx_d { for pr in d["proc"].as_array().into_iter().flatten() { let (from, to) = (f(&pr["from"]), f(&pr["to"])); if el >= from && el <= to {
            match pr["type"].as_str() {
                Some("wave") => { let w = (TAU * f(&pr["hz"]) * (el - from) / 1000.0).sin(); tw.tilt += f(&pr["t0"]) + w * f(&pr["t1"]); tw.wave = (13.0 * (el - from) / 1000.0).sin() * 0.35; }
                Some("chew") => { let ph = (TAU * f(&pr["hz"]) * (el - from) / 1000.0).sin(); tw.mouth = f(&pr["base"]) + f(&pr["amp"]) * ph; tw.cheek = tw.cheek.max(0.85 + 0.15 * ph); }
                Some("walk") => { let ph = (el - from) / 1000.0 * fo(&pr["hz"], 3.2); let sa = (std::f32::consts::PI * ph).sin().abs(); tw.oy -= sa * 0.035; tw.tilt += (std::f32::consts::PI * ph).sin() * 0.05; tw.sy *= 1.0 - 0.035 * (1.0 - sa); tw.sx *= 1.0 + 0.02 * (1.0 - sa); }
                Some("electric") => { tw.jit = f(&pr["amp"]) * ((to - el) / 500.0).min(1.0); if self.timer((usize::MAX, "eb"), 0.09) { self.spawn("bolt", 1, &Value::Null); } }
                _ => {} } } }
            if !d["orbit"].is_null() && el < f(&d["dur"]) { tw.orbit = true; } }
        if let Some(i) = finish { self.fx = None; let d = r.def(i);
            if !d["then_cond"].is_null() { let tc = &d["then_cond"]; let id = if self.ctx_musica { tc["se"].as_str() } else { tc["senao"].as_str() }; if let Some(id) = id { self.play(id); } }
            else if let Some(id) = d["then"].as_str() { self.play(id); } }
        // ---- cenario persistente, maos, itens
        let fx_on = self.fx.is_some(); let _ = fx_on;
        self.el_now = if fx_d.is_some() { el } else { self.s_t };
        let mut want_on = [""; 8]; let mut nw = 0usize;
        for v in [fx_cena_on, if has_b { Some(&b["cena_on"]) } else { None }].into_iter().flatten() { for s in v.as_array().into_iter().flatten() { if let Some(s) = s.as_str() { if nw < 8 { want_on[nw] = s; nw += 1; } } } }
        let cat = &rg["cenario"]; let pp: Option<&'static Value> = fx_par.or(if has_b && !b["cena_par"].is_null() { Some(&b["cena_par"]) } else { None }); let en = self.el_now;
        for id in CENA { if cat[id].is_null() { continue; } let w = want_on[..nw].contains(&id); let t = self.t;
            let s = self.sc.entry(id).or_default(); if w && !s.vis { s.vis = true; s.t0 = t; } if !w { s.vis = false; }
            s.a = clamp(s.a + if w { dt / 0.3 } else { -dt / 0.3 }, 0.0, 1.0); s.par = None;
            if let Some(p) = pp { for pr in p[id].as_array().into_iter().flatten() { if en >= f(&pr[0]) { s.par = Some(&pr[1]); } } }
            s.traj = fx_traj.and_then(|t| if t[id].is_null() { None } else { Some(&t[id]) }); if s.traj.is_some() && w { s.a = 1.0; } }
        let segs: Option<&'static Value> = fx_d.filter(|d| !d["maosK"].is_null()).map(|d| &d["maosK"]).or(if has_b && !b["maosK"].is_null() { Some(&base_d["estado"]["maosK"]) } else { None });
        let k_h = 1.0 - 0.0004f32.powf(dt);
        for (hi, h) in ["L", "R"].iter().enumerate() {
            let s = segs.and_then(|sg| sg.as_array()).and_then(|a| a.iter().find(|z| z["h"].as_str() == Some(h) && (z["a"].is_null() || (en >= f(&z["a"]) && en < f(&z["z"])))));
            let (tx, ty, tr, tv) = if let Some(s) = s {
                let sx = if s["swingx"].is_null() { 0.0 } else { (TAU * f(&s["swingx"]["hz"]) * self.t).sin() * f(&s["swingx"]["a"]) };
                let sy = if s["swing"].is_null() { 0.0 } else { (TAU * f(&s["swing"]["hz"]) * self.t + if *h == "R" { std::f32::consts::PI } else { 0.0 }).sin() * f(&s["swing"]["a"]) };
                (f(&s["x"]) + sx, f(&s["y"]) + sy, f(&s["r"]), 1.0) } else if hi == 0 { (2.0, 66.0, -35.0, 0.0) } else { (98.0, 66.0, 35.0, 0.0) };
            let q = &mut self.hand[hi]; q.x += (tx - q.x) * k_h; q.y += (ty - q.y) * k_h; q.r += (tr - q.r) * k_h; q.v += (tv - q.v) * (1.0 - 0.00002f32.powf(dt)); }
        if has_b && !b["leitura"].is_null() { let p = fo(&b["leitura"]["p"], 5.6); let ph = self.s_t % p; self.read_lift = if ph < 3.6 { 0.0 } else if ph < 4.1 { ease("inOut", (ph - 3.6) / 0.5) } else if ph < 5.1 { 1.0 } else { 1.0 - ease("inOut", (ph - 5.1) / 0.5) }; } else { self.read_lift = 0.0; }
        { let ox_px = (self.cur.ox + tw.ox) * 100.0; let vx = (ox_px - self.prev_ox_px) / dt.max(1e-3); self.prev_ox_px = ox_px; self.cab.1 += (-38.0 * self.cab.0 - 4.5 * self.cab.1 - vx * 0.35) * dt; self.cab.0 = clamp(self.cab.0 + self.cab.1 * dt, -16.0, 16.0); }
        // ---- alvos continuos do estado de base
        let dd_eye = if has_b { b["eye"].as_str().unwrap_or("open") } else { "open" };
        let dd_tint = if has_b { b["tint"].as_str().unwrap_or("#ffffff") } else { "#ffffff" }; let dd_tint_a = if has_b { fo(&b["tintA"], 0.0) } else { 0.0 };
        if has_b && !b["dance"].is_null() { let on = f(&b["dance"]) != 0.0; self.dance = clamp(self.dance + dt / if on { 0.3 } else { 0.5 } * if on { 1.0 } else { -1.0 }, 0.0, 1.0); } else { self.dance = clamp(self.dance - dt / 0.5, 0.0, 1.0); }
        let (mut t_tilt, mut t_sx, mut t_sy, t_blush, mut t_oy, mut t_ox) = (if has_b { fo(&b["tilt"], 0.0) } else { 0.0 }, if has_b { fo(&b["sx"], 1.0) } else { 1.0 }, if has_b { fo(&b["sy"], 1.0) } else { 1.0 }, if has_b { fo(&b["blush"], 0.12) } else { 0.12 }, 0.0f32, 0.0f32);
        if has_b && !b["breathe"].is_null() { let s = (f(&b["breathe"]["w"]) * self.t).sin() * f(&b["breathe"]["a"]); t_sy *= 1.0 + s; t_sx *= 1.0 - s * 0.57; }
        if has_b && !b["bounce"].is_null() { t_oy -= (f(&b["bounce"]["w"]) * self.t).sin().abs() * f(&b["bounce"]["a"]); }
        let (mut d_roll, mut dsx, mut dsy) = (0.0f32, 1.0f32, 1.0f32);
        if self.dance > 0.0 { let beat = self.t * f(&rg["danca_bpm"]) / 60.0; let hop = (std::f32::consts::PI * beat).sin().abs(); let land = (1.0 - hop).powi(6); let sw = (std::f32::consts::PI * beat / 2.0).sin(); let a = self.dance;
            t_oy -= 0.1 * hop * a; t_ox += 0.06 * sw * a; d_roll = 0.1 * sw * a; dsx = 1.0 + 0.045 * land * a; dsy = 1.0 - 0.06 * land * a; if self.timer((usize::MAX - 1, "note"), 1.1) { self.spawn("note", 1, &Value::Null); } }
        let (bl, tc) = match tint_ov { Some((c, a)) => (a, c), None => (dd_tint_a, hex(dd_tint)) };
        let k_m = 1.0 - 0.00002f32.powf(dt); let skin_dw = fo(&r.skin["bocaDW"], 5.0);
        let bb = boca_ov.or(if has_b && !b["boca"].is_null() { Some(&b["boca"]) } else { None });
        let (bc, bw, bo) = bb.map_or((0.8, skin_dw, 0.0), |v| (fo(&v["c"], 0.8), fo(&v["w"], skin_dw), fo(&v["o"], 0.0)));
        self.cur.mc += (bc - self.cur.mc) * k_m; self.cur.mw += (bw - self.cur.mw) * k_m; self.cur.mo += (bo - self.cur.mo) * k_m;
        let c = &mut self.cur; c.tilt += (t_tilt - c.tilt) * k_gen; c.sx += (t_sx - c.sx) * k_gen; c.sy += (t_sy - c.sy) * k_gen; c.oy += (t_oy - c.oy) * k_gen; c.ox += (t_ox - c.ox) * k_gen; c.blush += (t_blush - c.blush) * k_gen;
        c.tint_a += (bl - c.tint_a) * k_col; c.tint = mix(c.tint, tc, k_col); t_tilt = 0.0; let _ = t_tilt;
        // olhar: gesto > leitura > estado > ponteiro global (esquece em 4 s)
        let mut gt = (0.0f32, 0.0f32);
        if let Some(l) = look { gt = l; } else if has_b && !b["leitura"].is_null() && self.read_lift < 0.4 { gt = (0.0, 0.7); } else if has_b && !b["look"].is_null() { gt = (f(&b["look"][0]), f(&b["look"][1])); }
        else if has_b && !b["scan"].is_null() { gt = ((TAU * self.t / f(&b["scan"]["p"])).sin() * f(&b["scan"]["a"]), 0.0); }
        else if let Some((x, y)) = self.ptr { if self.t - self.ptr_t < f(&rg["olhar"]["memoria_s"]) { let d = x.hypot(y).max(1e-3); let m = (d / f(&rg["olhar"]["px_pleno"])).min(1.0); gt = (x / d * m, y / d * m * 0.75); } }
        self.cur.gx += (gt.0 - self.cur.gx) * k_look; self.cur.gy += (gt.1 - self.cur.gy) * k_look;
        // piscar (sono/linha/feliz/cansado nao piscam)
        let eye = eye_ov.unwrap_or(dd_eye); let no_blink = eye != "open" && eye != "wide";
        if !no_blink && self.t >= self.next_blink { self.blink_q.push(self.t); let (mn, mx) = (f(&rg["piscar"]["min_s"]), f(&rg["piscar"]["max_s"])); self.next_blink = self.t + mn + self.rnd() * (mx - mn); if self.rnd() < f(&rg["piscar"]["duplo"]) { self.blink_q.push(self.t + 0.23); } }
        let (tnow, mut bv) = (self.t, 1.0f32);
        self.blink_q.retain(|t0| { let e = (tnow - t0) * 1000.0; if e < 0.0 { return true; } if e < 70.0 { bv = bv.min(1.0 - 0.94 * ease("inOut", e / 70.0)); } else if e < 200.0 { bv = bv.min(0.06 + 0.94 * ease("out", (e - 70.0) / 130.0)); } e < 200.0 });
        self.blink_v = if no_blink { 1.0 } else { bv };
        // emissores do estado
        if has_b { for e in b["emit"].as_array().into_iter().flatten() { let ty: &'static str = e["type"].as_str().unwrap_or("");
            if self.timer((self.base, ty), f(&e["every"])) && self.rnd() < fo(&e["p"], 1.0) { self.spawn(ty, e["n"].as_u64().unwrap_or(1) as u32, e); } } }
        self.parts.retain_mut(|p| { if p.delay > 0.0 { p.delay -= dt; return true; } p.age += dt; p.x += p.vx * dt; p.y += p.vy * dt; p.age < p.life });
        self.badge = if has_b { b["badge"].as_str() } else { None };
        self.badge_s += ((if self.badge.is_some() { 1.0 } else { 0.0 }) - self.badge_s) * k_gen;
        self.tw = tw; self.d_roll = d_roll; self.dsx = dsx; self.dsy = dsy; self.eye = eye; self.tint_a = self.cur.tint_a;
    }

    // ---------------------------------------------------------------- desenho
    /// `m0` = transformacao da base (px reais); (x, y) = centro-base do corpo (x do centro, y do chao) em unidades de projeto; l = largura do corpo (92 u do rig) em unidades de projeto.
    /// `mini` = so o corpo/rosto (compacto): sem cenario nem particulas.
    pub fn draw(&mut self, g: &Gfx, x: f32, y: f32, l: f32, mini: bool) {
        self.n_eyes = 0; self.n_mouths = 0; self.n_hands = 0;
        let m0 = g.get_matrix(); let s = l / 92.0;
        let u = sc(s, s) * tr(x - 50.0 * s, y - 86.0 * s) * m0; // unidades do rig -> tela (base y = 86)
        g.set_matrix(u);
        let r = rig(); let skin = &r.skin; let c = self.cur.clone(); let tw = self.tw.clone(); let (w100, h90) = (100.0f32, 90.0f32);
        let jit = if tw.jit != 0.0 { (self.t * 190.0).sin() * tw.jit } else { 0.0 };
        let ox = (c.ox + tw.ox + jit + c.gx * 0.02) * w100; let oy = (c.oy + tw.oy) * h90; let roll = (tw.roll + self.d_roll + c.tilt + tw.tilt + c.gx * 0.04) * 180.0 / std::f32::consts::PI;
        let (sx, sy) = (c.sx * tw.sx * self.dsx, c.sy * tw.sy * self.dsy);
        let pal_c = |k: &str| hex(skin["paleta"][k].as_str().unwrap_or("#ffffff")); let (corpo, sombra, acento) = (pal_c("corpo"), pal_c("sombra"), pal_c("acento")); let tm = fo(&skin["tint_corpo"], 1.0);
        let top = mix(corpo, c.tint, c.tint_a * 0.14 * tm); let bot = mix(sombra, c.tint, c.tint_a * 0.55 * tm); let ink = hex(skin["olhos"]["cor"].as_str().unwrap_or("#14121a"));
        if !mini { // brilho de estado + camada de tras + sombra de contato
            let ga = clamp(c.tint_a * 0.7 + tw.glow, 0.0, 1.0); if ga > 0.01 { g.vignette(50.0, 50.0, 68.0, rgba(c.tint, ga)); }
            self.layers(g, false, ox, oy, top, bot);
            if skin["chao"].as_bool().unwrap_or(false) { let h = clamp(-oy / 40.0, 0.0, 1.0); let a = 0.34 * (1.0 - 0.55 * h); let (cx, rx, ry) = (50.0 + ox, 38.0 * (1.0 - 0.25 * h), 3.6 * (1.0 - 0.3 * h));
                for k in 0..3 { let f2 = 1.0 + 0.35 * (1.0 - k as f32 / 2.0); g.ellipse(cx, 88.0, rx * f2 * (0.78 + 0.11 * k as f32), ry * f2, (0.0, 0.0, 0.0, a / 2.6)); } }
        }
        // ---- corpo (pose): translate(ox,oy) rotate(roll @50,50) squash ancorado em (50,86)
        let pose = tr(-50.0, -86.0) * sc(sx, sy) * tr(50.0, 86.0) * rot(roll, 50.0, 50.0) * tr(ox, oy); let pm = pose * u; g.set_matrix(pm);
        let hv = clamp(tw.hands, 0.0, 1.0); let hd = clamp(tw.hd, 0.0, 1.0);
        for (i, sgn) in [-1.0f32, 1.0].iter().enumerate() { let a = hv.max(hd); if a < 0.02 { continue; }
            let hx = 50.0 + sgn * 49.0; let hy = 64.0 - hv * 40.0 + if *sgn > 0.0 { tw.wave * 8.0 } else { 0.0 } + hd * (TAU * self.t * 1.6 + if *sgn > 0.0 { std::f32::consts::PI } else { 0.0 }).sin() * 7.0; let _ = i;
            g.set_matrix(rot(sgn * 38.0, 0.0, 0.0) * tr(hx, hy) * pm); mitten(g, rgba(bot, a), rgba(sombra, a), 1.1); self.n_hands += 1; }
        g.set_matrix(pm);
        let segs = &r.body;
        g.path_vgrad(segs, 22.0, 86.0, rgba(top, 1.0), rgba(bot, 1.0));
        let tcol = mix(acento, c.tint, c.tint_a * 0.6);
        if skin["detalhe"].as_str() == Some("ranhura") { for x in [64.0, 71.0, 78.0] { g.rrect(x, 64.0, 3.0, 13.0, 1.5, rgba(tcol, 0.85)); } }
        let ck = clamp(tw.cheek, 0.0, 1.0); if ck > 0.03 { let (rx, ry) = (3.0 + 5.0 * ck, 2.6 + 4.0 * ck); for cx in [25.0, 75.0] { g.ellipse(cx, 68.0, rx, ry, rgba(corpo, 1.0)); g.path(&ellipse_segs(cx, 68.0, rx, ry), None, Some((0.9, rgba(sombra, 1.0)))); } }
        // olhos (2)
        let es = tw.es; let (gx, gy) = (c.gx, c.gy); let bv = self.blink_v; let ex = [fo(&skin["olhos"]["x"][0], 36.0), fo(&skin["olhos"]["x"][1], 64.0)]; let ey = fo(&skin["olhos"]["y"], 55.0) + gy * 4.5;
        for (i, s2) in [-1.0f32, 1.0].iter().enumerate() { let far = if s2 * gx < 0.0 { 1.0 - 0.22 * gx.abs() } else { 1.0 }; let mut em: &str = self.eye; if em == "wink" { em = if i == 1 { "happy" } else { "open" }; }
            g.set_matrix(sc(far, 1.0) * tr(ex[i] + gx * 7.0, ey) * pm); self.n_eyes += 1; let ic = rgba(ink, 1.0); let st = Some((2.8, ic));
            match em {
                "closed" => g.path(&[Seg::M(-5.5, -1.5), Seg::Q(0.0, 4.5, 5.5, -1.5)], None, st), "happy" => g.path(&[Seg::M(-5.5, 2.5), Seg::Q(0.0, -4.0, 5.5, 2.5)], None, st),
                "tired" => g.path(&[Seg::M(-5.5, 0.0), Seg::C(-5.5, 0.0, -5.5, 4.5, 0.0, 4.5), Seg::C(5.5, 4.5, 5.5, 0.0, 5.5, 0.0), Seg::Z], Some(ic), None),
                "line" => g.rrect(-5.5, -1.2, 11.0, 2.6, 1.3, ic),
                "spiral" => { let mut p = Vec::new(); for k in 0..=40 { let a = k as f32 / 40.0 * TAU * 2.2; let rr = k as f32 / 40.0 * 6.0; p.push(if k == 0 { Seg::M(a.cos() * rr, a.sin() * rr) } else { Seg::L(a.cos() * rr, a.sin() * rr) }); }
                    g.set_matrix(rot(self.t * 540.0 * s2, 0.0, 0.0) * sc(far, 1.0) * tr(ex[i] + gx * 7.0, ey) * pm); g.path(&p, None, Some((1.7, ic))); }
                _ => { let sx2 = if es > 1.0 { 1.0 + (es - 1.0) * 0.5 } else { 1.0 }; let sy2 = es * bv.max(0.12); g.set_matrix(sc(sx2, sy2) * sc(far, 1.0) * tr(ex[i] + gx * 7.0, ey) * pm); g.rrect(-3.65, -8.35, 7.3, 16.7, 3.65, ic); } } }
        g.set_matrix(pm);
        // boca (1): M x1 y Q 50 qy x2 y Q 50 qy+o*24 x1 y Z, preenchida e com contorno na cor da tinta
        { let c2 = c.mc + tw.mc; let w = c.mw; let o = clamp(c.mo + tw.mouth, 0.0, 1.0); let y0 = fo(&skin["bocaY"], 69.0); let ff = fo(&skin["bocaF"], 6.4); let qy = y0 + c2 * ff; let (x1, x2) = (50.0 - w, 50.0 + w);
          g.path(&[Seg::M(x1, y0), Seg::Q(50.0, qy, x2, y0), Seg::Q(50.0, qy + o * 24.0, x1, y0), Seg::Z], Some(rgba(ink, 1.0)), Some((fo(&skin["bocaW"], 2.2), rgba(ink, 1.0)))); self.n_mouths += 1; }
        if !mini { self.items(g, ox, oy, top); }
        g.set_matrix(u);
        if !mini { self.layers(g, true, ox, oy, top, bot); self.particles(g, ox, oy); }
        g.set_matrix(m0);
    }

    /// camadas de cenario: back (atras, escurecida) ou front (na frente), com paralaxe do olhar
    fn layers(&mut self, g: &Gfx, front: bool, ox: f32, oy: f32, _top: C3, _bot: C3) {
        let r = rig(); let cat = &r.v["cenario"]; let c = &self.cur; let el = self.el_now; let t = self.t; let base = g.get_matrix();
        g.set_matrix(if front { tr(c.gx * 4.0, c.gy * 1.6) } else { tr(-c.gx * 7.0, -c.gy * 2.5) } * base);
        let lm = g.get_matrix(); let mut face_x = self.face_x; if !front { face_x = None; }
        for id in CENA { let sp = &cat[id]; if sp.is_null() { continue; } if (sp["z"].as_str() == Some("f")) != front { continue; }
            let Some(q) = self.sc.get(id) else { continue }; if q.a < 0.01 { continue; }
            let d = fo(&sp["d"], 0.0); let us = fo(&sp["u"], 2.6) * (1.0 - 0.2 * d) * if sp["z"].as_str() == Some("f") { 1.06 } else { 1.0 };
            let mut par = spo(&sp["opts"], SP0); if let Some(po) = q.par { par = spo(po, par); } let sname: &'static str = sp["t"].as_str().unwrap_or("");
            let (dw, dh) = sprite_dim(sname, &par);
            let mut cy = if sp["base"].is_null() { f(&sp["y"]) } else { f(&sp["base"]) - (dh / 2.0 - 1.0) * us };
            if !sp["desce"].is_null() { cy += (1.0 - ease("back", (t - q.t0) * 1000.0 / f(&sp["desce"]["ms"]))) * f(&sp["desce"]["de"]); }
            let mut rt = fo(&sp["r"], 0.0); let sh = match q.par { Some(p) if !p["_shake"].is_null() => &p["_shake"], _ => &sp["shake"] }; if !sh.is_null() { rt += (TAU * f(&sh["hz"]) * t).sin() * f(&sh["amp"]); }
            let (mut xx, mut a, mut shd) = (f(&sp["x"]), q.a, 1.0f32);
            if let Some(kf) = q.traj.and_then(|v| v.as_array()) { let mut k = 0usize; while k + 2 < kf.len() && el > f(&kf[k + 1][0]) { k += 1; } let (k0, k1) = (&kf[k], &kf[k + 1]);
                let fct = ease(k1[4].as_str().unwrap_or("lin"), clamp((el - f(&k0[0])) / (f(&k1[0]) - f(&k0[0])).max(1e-3), 0.0, 1.0)); let rv = |v: &Value, d0: f32| if v.is_null() { d0 } else { f(v) };
                let (x0, x1, y0, y1, r0, r1) = (rv(&k0[1], xx), rv(&k1[1], xx), rv(&k0[2], cy), rv(&k1[2], cy), rv(&k0[3], rt), rv(&k1[3], rt));
                xx = x0 + (x1 - x0) * fct; cy = y0 + (y1 - y0) * fct; rt = r0 + (r1 - r0) * fct; a = 1.0; shd = clamp(1.0 - (f(&sp["base"]) - (cy + dh / 2.0 * us)) / 130.0, 0.0, 1.0); }
            if sname == "parede" { face_x = Some(f(&sp["x"]) + (1.0 - dw / 2.0) * us); }
            if !sp["base"].is_null() { let (rx, ry) = (dw * us * 0.4, dw * us * 0.07); for k in 0..2 { g.ellipse(xx, f(&sp["base"]) + 1.5, rx * (1.0 + 0.3 * (1.0 - k as f32)), ry * (1.0 + 0.4 * (1.0 - k as f32)), (0.0, 0.0, 0.0, 0.2 * a * shd)); } }
            let dim = if !front { if d > 0.35 { 0.66 } else if d > 0.08 { 0.82 } else { 1.0 } } else { 1.0 };
            g.set_matrix(rot(rt, 0.0, 0.0) * tr(xx, cy) * lm); sprite(g, sname, t, us, &par, dim, a, f32::INFINITY); g.set_matrix(lm); }
        if front { let _ = (ox, oy); } else { self.face_x = face_x.or(self.face_x); }
        g.set_matrix(base);
        if !front && self.face_x.is_some() && self.sc.get("parede").is_none_or(|q| q.a < 0.01) { self.face_x = None; }
    }

    /// itens do corpo: segurados pelas maos, trajetos, jornal, cabo+tomada, e as maos "ativas" (maosK)
    fn items(&mut self, g: &Gfx, ox: f32, _oy: f32, top: C3) {
        let r = rig(); let base_d = r.def(self.base); let fx_d = self.fx.as_ref().map(|x| r.def(x.d)); let el = self.el_now; let t = self.t; let pm = g.get_matrix();
        let items: Option<&Value> = fx_d.and_then(|d| if d["itens"].is_null() { None } else { Some(&d["itens"]) }).or(if base_d["estado"]["itens"].is_null() { None } else { Some(&base_d["estado"]["itens"]) });
        let mut cable: Option<&Value> = None; let mut plug: Option<(f32, f32, f32, f32)> = None; let mut deferred: Vec<(Matrix3x2, &'static str, f32, SpO, f32, f32)> = Vec::new(); let mut hands_extra: Vec<(Matrix3x2, f32)> = Vec::new();
        for it in items.and_then(|v| v.as_array()).into_iter().flatten() {
            if it["t"].as_str() == Some("cabo") { cable = Some(it); continue; }
            let w = &it["win"]; let in_fx = self.fx.is_some() && !w.is_null(); if in_fx && (el < f(&w[0]) || el > f(&w[1])) { continue; }
            let mut a = if in_fx { clamp(((el - f(&w[0])) / 120.0).min((f(&w[1]) - el) / 120.0), 0.0, 1.0) } else { 1.0 }; let (x, y); let mut scl = 1.0f32;
            if !it["hold"].is_null() { let q = self.hand[if it["hold"]["h"].as_str() == Some("L") { 0 } else { 1 }]; x = q.x + fo(&it["hold"]["dx"], 0.0); y = q.y + fo(&it["hold"]["dy"], 0.0); a *= (q.v * 2.0).min(1.0); }
            else if let Some(p) = it["path"].as_array() { let mut k = 0usize; while k + 2 < p.len() && el > f(&p[k + 1][0]) { k += 1; } let (t0, x0, y0) = (f(&p[k][0]), f(&p[k][1]), f(&p[k][2])); let (t1, x1, y1) = (f(&p[k + 1][0]), f(&p[k + 1][1]), f(&p[k + 1][2]));
                let fct = clamp((el - t0) / (t1 - t0).max(1e-3), 0.0, 1.0); x = x0 + (x1 - x0) * fct; y = y0 + (y1 - y0) * fct; }
            else if it["proc"].as_str() == Some("jornal") { let us = fo(&it["u"], 2.6); let l = self.read_lift; let yy = (if it["pos"].is_null() { 80.0 } else { f(&it["pos"][1]) }) + l * 26.0; let (dw, dh) = sprite_dim("jornal", &SP0);
                let top2 = yy - dh / 2.0 * us; let hw = dw / 2.0 * us - 4.0; deferred.push((rot(l * 3.0, 0.0, 0.0) * tr(50.0, yy) * pm, "jornal", us, SP0, 1.0, f32::INFINITY));
                for sg in [-1.0f32, 1.0] { hands_extra.push((rot(sg * -22.0, 0.0, 0.0) * tr(50.0 + sg * hw, top2 + 7.0) * pm, 1.0)); } continue; }
            else { x = f(&it["pos"][0]); y = f(&it["pos"][1]); }
            if !it["esc"].is_null() && el >= f(&it["esc"][0]) { scl = 1.0 - clamp((el - f(&it["esc"][0])) / (f(&it["esc"][1]) - f(&it["esc"][0])), 0.0, 1.0); }
            if scl < 0.03 || a < 0.02 { continue; }
            let sh = if it["shake"].is_null() { 0.0 } else { (TAU * f(&it["shake"]["hz"]) * t).sin() * f(&it["shake"]["amp"]) };
            let osc = if it["osc"].is_null() { (0.0, 0.0) } else { let o = &it["osc"]; ((TAU * t / f(&o["p"])).sin() * f(&o["ax"]), (TAU * t / f(&o["p"])).cos() * f(&o["ay"])) };
            let m = sc(fo(&it["s"], 1.0) * scl, fo(&it["s"], 1.0) * scl) * rot(fo(&it["r"], 0.0) + sh, 0.0, 0.0) * tr(x + osc.0, y + osc.1) * pm;
            if it["t"].as_str() == Some("plug") { plug = Some((x, y, fo(&it["u"], 2.4), a)); }
            deferred.push((m, it["t"].as_str().unwrap_or(""), fo(&it["u"], 2.6), spo(&it["opts"], SP0), a, if it["clip"].as_bool() == Some(true) { self.face_x.map_or(f32::INFINITY, |fx| (fx - ox - (x + osc.0)) / (fo(&it["s"], 1.0) * scl)) } else { f32::INFINITY })); }
        if let (Some(cb), Some(p)) = (cable, plug) { let p0 = (f(&cb["port"][0]), f(&cb["port"][1])); let p1 = (p.0 - 7.0 * p.2, p.1 + 1.0); let dist = (p1.0 - p0.0).hypot(p1.1 - p0.1);
            let sag = clamp(0.6 * (fo(&cb["L"], 46.0) - dist), 0.0, 30.0); let (cx, cy) = ((p0.0 + p1.0) / 2.0 + self.cab.0, (p0.1 + p1.1) / 2.0 + sag);
            let d = [Seg::M(p0.0, p0.1), Seg::Q(cx, cy, p1.0, p1.1)]; g.path(&d, None, Some((4.6, (0.165, 0.141, 0.188, p.3)))); g.path(&d, None, Some((2.6, (0.416, 0.439, 0.518, p.3)))); }
        for (m, name, us, opts, a, cl) in deferred { g.set_matrix(m); sprite(g, name, t, us, &opts, 1.0, a, cl); }
        for (m, a) in hands_extra { g.set_matrix(m); mitten(g, rgba(top, a), (0.165, 0.141, 0.188, 0.5 * a), 1.1); self.n_hands += 1; }
        let hands = self.hand; for q in hands.iter() { if q.v > 0.03 { g.set_matrix(rot(q.r, 0.0, 0.0) * tr(q.x, q.y) * pm); mitten(g, rgba(top, q.v.min(1.0)), (0.165, 0.141, 0.188, 0.5 * q.v.min(1.0)), 1.1); self.n_hands += 1; } }
        g.set_matrix(pm);
    }

    fn particles(&mut self, g: &Gfx, ox: f32, oy: f32) {
        let m = g.get_matrix(); let skip = |g: &Gfx| g.set_matrix(m);
        for q in &self.parts { if q.delay > 0.0 { continue; } let a = q.age / q.life; let fo_ = if a > 0.65 { 1.0 - (a - 0.65) / 0.35 } else { 1.0 }; let pop = (a / 0.12).min(1.0); let col = rgba(q.c, fo_);
            let at = tr(q.x, q.y) * m;
            match q.ty {
                PT::Star => { g.set_matrix(sc(pop, pop) * rot(q.rot * 40.0 + a * 120.0, 0.0, 0.0) * at); g.path(&star(q.size, 0.45), Some(col), None); }
                PT::Spark => { let s2 = pop * (1.0 - a * 0.5); g.set_matrix(sc(s2, s2) * at); g.path(&spark(q.size), Some(col), None); }
                PT::Sweat => { g.set_matrix(sc(q.size / 5.0, q.size / 5.0) * at); g.path(&[Seg::M(0.0, -6.0), Seg::C(3.0, -1.0, 4.0, 2.0, 0.0, 4.0), Seg::C(-4.0, 2.0, -3.0, -1.0, 0.0, -6.0), Seg::Z], Some(col), Some((0.8, (0.169, 0.435, 0.839, fo_)))); }
                PT::Z => { let z = q.size * (0.6 + 0.5 * a) / 9.0 * 4.0; g.set_matrix(at); g.path(&[Seg::M(-z, -z), Seg::L(z, -z), Seg::L(-z, z), Seg::L(z, z)], None, Some((1.8, col))); }
                PT::Dots => { g.set_matrix(m); for i in 0..3 { if a > i as f32 * 0.2 { g.circle(q.x + i as f32 * 8.0, q.y - i as f32 * 6.0, 2.2 + i as f32 * 0.9, (0.8, 0.75, 1.0, fo_)); } } }
                PT::Note => { g.set_matrix(at); let z = q.size / 11.0; g.ellipse(0.0, 0.0, 3.0 * z, 2.2 * z, col); g.path(&[Seg::M(2.6 * z, 0.0), Seg::L(2.6 * z, -9.0 * z), Seg::Q(2.6 * z, -6.0 * z, 6.0 * z, -5.0 * z)], None, Some((1.6 * z, col))); }
                PT::Bang => { let z = q.size / 14.0 * pop * (1.0 + (1.0 - (a * 3.0).min(1.0)) * 0.4); g.set_matrix(sc(z, z) * at); g.path(&[Seg::M(0.0, -8.0), Seg::L(0.0, 0.0)], None, Some((3.0, col))); g.circle(0.0, 4.0, 1.7, col); }
                PT::Bolt => { let (rr, s2) = (q.size, q.seed); g.set_matrix(rot(q.rot.to_degrees(), 0.0, 0.0) * at); if a < 0.8 { g.path(&[Seg::M(0.0, 0.0), Seg::L(rr * 0.3, s2.sin() * 4.0), Seg::L(rr * 0.55, -(s2 * 2.0).cos() * 5.0), Seg::L(rr * 0.8, (s2 * 3.0).sin() * 4.0), Seg::L(rr, 0.0)], None, Some((2.2, col))); } }
                PT::File => { g.set_matrix(at); sprite(g, "pasta", 0.0, 2.2, &SP0, 1.0, 1.0, f32::INFINITY); }
                PT::Conf => { g.set_matrix(m); g.aa(false); g.rrect((q.x / 3.0).round() * 3.0, (q.y / 3.0).round() * 3.0, 3.0, 3.0, 0.0, col); g.aa(true); }
                PT::Refl => { g.set_matrix(m); g.aa(false); g.rrect(q.x.round(), q.y.round(), 3.0, 3.0, 0.0, rgba(q.c, fo_ * if (q.age * 40.0).sin() > -0.3 { 1.0 } else { 0.4 })); g.aa(true); }
                PT::Fumaca => { g.set_matrix(m); let z = (q.size + a * 5.0).round(); g.aa(false); g.rrect((q.x / 2.0).round() * 2.0, (q.y / 2.0).round() * 2.0, z, z, 0.0, rgba(q.c, 0.7 * fo_)); g.aa(true); }
                PT::Puff => { g.set_matrix(m); g.circle(q.x, q.y, q.size + a * 6.0, rgba(q.c, 0.5 * fo_)); } } }
        if self.badge_s > 0.02 { if let Some(bg) = self.badge { let bob = (self.t * 5.0).sin() * 2.0; let z = self.badge_s; let q = bg == "qmark"; g.set_matrix(sc(z, z) * tr(88.0, 2.0 + bob) * m);
            g.circle(0.0, 0.0, 9.0, rgba(hex(if q { "#4cc9f0" } else { "#ffb020" }), 1.0));
            if q { g.path(&[Seg::M(-2.6, -3.0), Seg::Q(-2.6, -6.0, 0.0, -6.0), Seg::Q(2.8, -6.0, 2.8, -3.0), Seg::Q(2.8, -1.0, 0.0, 0.0), Seg::L(0.0, 1.8)], None, Some((1.8, (1.0, 1.0, 1.0, 1.0)))); g.circle(0.0, 5.0, 1.1, (1.0, 1.0, 1.0, 1.0)); }
            else { g.path(&[Seg::M(0.0, -6.0), Seg::L(0.0, 1.5)], None, Some((2.0, (1.0, 1.0, 1.0, 1.0)))); g.circle(0.0, 5.0, 1.2, (1.0, 1.0, 1.0, 1.0)); } } }
        if self.tw.orbit { for i in 0..3 { let a = self.t * 7.0 + i as f32 * TAU / 3.0; g.set_matrix(tr(50.0 + a.cos() * 36.0 + ox, 4.0 + a.sin() * 8.0 + oy) * m); g.path(&star(5.0, 0.45), Some(rgba(hex("#ffd45e"), 1.0)), None); } }
        skip(g);
    }
}
fn mitten(g: &Gfx, fill: Rgba, stroke: Rgba, w: f32) { g.rrect(-4.0, -7.5, 8.0, 15.0, 4.0, fill); g.rrect_stroke(-4.0, -7.5, 8.0, 15.0, 4.0, w, stroke); }
fn ellipse_segs(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<Seg> { let k = 0.5523; vec![Seg::M(cx + rx, cy), Seg::C(cx + rx, cy + ry * k, cx + rx * k, cy + ry, cx, cy + ry), Seg::C(cx - rx * k, cy + ry, cx - rx, cy + ry * k, cx - rx, cy), Seg::C(cx - rx, cy - ry * k, cx - rx * k, cy - ry, cx, cy - ry), Seg::C(cx + rx * k, cy - ry, cx + rx, cy - ry * k, cx + rx, cy), Seg::Z] }
fn star(r: f32, k: f32) -> Vec<Seg> { (0..10).map(|i| { let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0; let q = if i % 2 == 1 { r * k } else { r }; if i == 0 { Seg::M(a.cos() * q, a.sin() * q) } else { Seg::L(a.cos() * q, a.sin() * q) } }).chain([Seg::Z]).collect() }
fn spark(r: f32) -> Vec<Seg> { let c = r * 0.15; vec![Seg::M(0.0, -r), Seg::Q(c, -c, r, 0.0), Seg::Q(c, c, 0.0, r), Seg::Q(-c, c, -r, 0.0), Seg::Q(-c, -c, 0.0, -r), Seg::Z] }
/// SVG path (so M L H V C Q Z absolutos, como o corpo do skin) -> segmentos
pub fn parse_path(d: &str) -> Vec<Seg> {
    let mut out = Vec::new(); let (mut cx, mut cy) = (0.0f32, 0.0f32); let mut cmd = ' '; let mut nums: Vec<f32> = Vec::new(); let mut tok = String::new();
    let mut flush = |cmd: char, nums: &mut Vec<f32>, out: &mut Vec<Seg>, cx: &mut f32, cy: &mut f32| { let n = nums.as_slice();
        match cmd { 'M' if n.len() >= 2 => { out.push(Seg::M(n[0], n[1])); *cx = n[0]; *cy = n[1]; } 'L' if n.len() >= 2 => { out.push(Seg::L(n[0], n[1])); *cx = n[0]; *cy = n[1]; }
            'H' if !n.is_empty() => { out.push(Seg::L(n[0], *cy)); *cx = n[0]; } 'V' if !n.is_empty() => { out.push(Seg::L(*cx, n[0])); *cy = n[0]; }
            'C' if n.len() >= 6 => { out.push(Seg::C(n[0], n[1], n[2], n[3], n[4], n[5])); *cx = n[4]; *cy = n[5]; } 'Q' if n.len() >= 4 => { out.push(Seg::Q(n[0], n[1], n[2], n[3])); *cx = n[2]; *cy = n[3]; }
            'Z' => out.push(Seg::Z), _ => {} } nums.clear(); };
    for ch in d.chars().chain([' ']) {
        if ch.is_ascii_alphabetic() { if !tok.is_empty() { nums.push(tok.parse().unwrap_or(0.0)); tok.clear(); } flush(cmd, &mut nums, &mut out, &mut cx, &mut cy); cmd = ch; if ch == 'Z' { flush('Z', &mut nums, &mut out, &mut cx, &mut cy); } }
        else if ch == ' ' || ch == ',' { if !tok.is_empty() { nums.push(tok.parse().unwrap_or(0.0)); tok.clear(); } }
        else if ch == '-' && !tok.is_empty() { nums.push(tok.parse().unwrap_or(0.0)); tok.clear(); tok.push(ch); } else { tok.push(ch); } }
    out
}

// ---------------------------------------------------------------- integracao com o app (gatilhos reais)
/// erros reais do app (falha ao abrir da Shelf, rede das letras): quem falha chama `report_error()`; o Eavy faz a reacao `erro` quando a pagina esta visivel
pub static ERRORS: AtomicU32 = AtomicU32::new(0);
pub fn report_error() { ERRORS.fetch_add(1, Relaxed); }
/// notificacao nova (gesto) e atencao sustentada (1 approval, 2 pergunta, 3 busca; 0 nada); o app chama quando tiver a origem
pub static NOTIFS: AtomicU32 = AtomicU32::new(0);
pub static ATTENTION: AtomicU8 = AtomicU8::new(0);
pub fn report_notification() { NOTIFS.fetch_add(1, Relaxed); }
pub fn set_attention(kind: u8) { ATTENTION.store(kind, Relaxed); }

/// Dados do app lidos a cada quadro (sem alocar). `timer`: 0 parado, 1 rodando, 2 pausado, 3 concluido.
#[derive(Clone, Copy, Default)]
pub struct Ctx { pub playing: bool, pub track: u64, pub charging: Option<bool>, pub batt_pct: u8, pub night: bool, pub timer: u8, pub cycle: u32, pub shelf_adds: u32, pub pointer: Option<(f32, f32)>, pub reduced: bool }
/// Estado logico do Eavy: detecta as bordas (tocar/pausar, faixa, plugar, timer, Shelf, erro...) e decide o estado de base. So avanca com a pagina/compacto visivel;
/// ao reabrir depois de uma pausa (> 1,5 s) ressincroniza com o estado ATUAL e descarta gestos vencidos.
pub struct Mascot {
    pub e: Eavy, last: Option<Instant>, born: Instant, init: bool, prev: Ctx, key: u64, cand: (u64, Instant), last_ptr: Option<(f32, f32)>, pub touch: Instant,
    asleep: bool, yawned: bool, pending: Option<&'static str>, slaps: Vec<Instant>, dizzies: Vec<Instant>, cool_until: Option<Instant>, pub mini: bool, errors: u32, notifs: u32, last_greet: Option<Instant>, reduced: bool, sleep_lock: bool, pinned: bool, low: bool,
}
impl Default for Mascot { fn default() -> Self { let mut e = Eavy::new(); e.reseed(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |d| d.subsec_nanos() ^ d.as_secs() as u32)); Mascot { e, last: None, born: Instant::now(), init: false, prev: Ctx::default(), key: 0, cand: (0, Instant::now()), last_ptr: None, touch: Instant::now(), asleep: false, yawned: false,
    pending: None, slaps: Vec::new(), dizzies: Vec::new(), cool_until: None, mini: false, errors: 0, notifs: 0, last_greet: None, reduced: false, sleep_lock: false, pinned: false, low: false } } }
/// frase do balao do irritado (deve caber inteira em BALLOON_MAX_W; ha teste em gfx.rs)
pub const IRRITADO_TXT: &str = "Dou um tempo. Volto em 5 s.";
const COOLDOWN: Duration = Duration::from_secs(5); const FAR_PX: f32 = 150.0; const YAWN_S: f32 = 60.0; const SLEEP_S: f32 = 120.0; const TRACK_DEBOUNCE: Duration = Duration::from_millis(300);
impl Mascot {
    pub fn age_ms(&self, now: Instant) -> u128 { now.duration_since(self.born).as_millis() }
    pub fn touched(&mut self, now: Instant) { self.touch = now; if !self.sleep_lock { self.asleep = false; } self.yawned = false; }
    /// Clique no mascote (no mouse-down, sem espera). Escalonamento (tempos do Coucou: janela de 1,7 s, 3 tapas = tontura; o Coucou ignora tapas enquanto tonto):
    /// tapa -> 3 em 1,7 s tonto -> uma 2a tontura em ate 10 s = irritado: balao + 5 s de cooldown ignorando cliques.
    pub fn on_click(&mut self, now: Instant) {
        if self.cooling(now) { return; }
        self.touched(now); if self.e.gesture_id() == Some("tonto") { return; }
        self.slaps.retain(|t| now.duration_since(*t) < Duration::from_millis(1700)); self.slaps.push(now);
        if self.slaps.len() >= 3 {
            self.slaps.clear(); self.dizzies.retain(|t| now.duration_since(*t) < Duration::from_secs(10)); self.dizzies.push(now);
            if self.dizzies.len() >= 2 { self.dizzies.clear(); self.cool_until = Some(now + COOLDOWN); self.e.play("irritado"); } else { self.e.play("tonto"); }
        } else { self.e.play("clique-tapa"); }
    }
    fn cooling(&self, now: Instant) -> bool { self.cool_until.is_some_and(|t| now < t) }
    /// balao de texto durante o cooldown: (texto, opacidade 0..1)
    pub fn balloon(&self, now: Instant) -> Option<(&'static str, f32)> {
        let t = self.cool_until?; if now >= t { return None; } let left = t.duration_since(now).as_secs_f32();
        Some((IRRITADO_TXT, (left / 0.25).min(1.0).min(((COOLDOWN.as_secs_f32() - left) / 0.15).min(1.0))))
    }
    pub fn error(&mut self) { self.e.play("erro"); }
    /// captura/teste: toca `id`, avanca `ms` e congela (o app nao reaplica o estado de base nem avanca)
    pub fn seek(&mut self, id: &str, ms: u32) { self.e.seek(id, ms); self.pinned = true; }
    /// captura: estado irritado com balao (cooldown de 5 s iniciado agora, congelado em 1 s)
    pub fn seek_irritado(&mut self) { let now = Instant::now(); self.cool_until = Some(now + COOLDOWN - Duration::from_millis(1000)); self.e.seek("irritado", 900); self.pinned = true; }
    pub fn unpin(&mut self) { self.pinned = false; self.e.unfreeze(); }
    /// teste/captura: dorme (ignora atividade)
    pub fn bench_sleep(&mut self) { self.asleep = true; self.sleep_lock = true; self.e.unfreeze(); }
    /// quadros continuos so acordado e sem animacoes reduzidas; dormindo ou reduzido = so o timer de 250 ms (4 fps)
    pub fn busy(&self, _now: Instant, _over: bool) -> bool { !self.mini && !self.e.sleeping() && !self.reduced }
    pub fn wake_ms(&self, _now: Instant) -> Option<u32> { if self.e.sleeping() || self.reduced { Some(250) } else if self.mini { Some(if self.e.gesture_running() { 33 } else { 100 }) } else { None } }
    pub fn raging(&self) -> bool { false }
    fn want(&self, c: &Ctx) -> &'static str {
        if self.cool_until.is_some() { return "irritado"; }
        match ATTENTION.load(Relaxed) { 1 => return "approval", 2 => return "pergunta", 3 => return "busca", _ => {} }
        if self.low { "bateria-baixa" } else if c.timer == 1 { "pomodoro-inicio" } else if c.playing { "danca" } else if c.charging == Some(true) { "carregando-ativo" } else if self.asleep { "sono" } else { "idle-pisca" }
    }
    fn resync(&mut self, c: &Ctx, now: Instant, greet: bool) {
        self.prev = *c; self.key = c.track; self.cand = (c.track, now); self.errors = ERRORS.load(Relaxed); self.notifs = NOTIFS.load(Relaxed); self.slaps.clear(); self.pending = None;
        self.touched(now); self.e.play(self.want(c)); if greet { self.e.play("saudacao"); self.last_greet = Some(now); }
    }
    /// atualiza gatilhos e avanca o motor (chamado por quadro desenhado)
    pub fn step(&mut self, c: &Ctx, now: Instant) {
        if self.pinned { return; }
        // bateria baixa com histerese: entra <= 20 % fora da tomada; sai >= 25 % ou ao plugar (batt_pct 0 = desconhecido)
        if c.charging == Some(true) || c.batt_pct >= 25 { self.low = false; } else if c.charging == Some(false) && c.batt_pct > 0 && c.batt_pct <= 20 { self.low = true; }
        let gap = self.last.map(|t| now.duration_since(t));
        let dt = gap.map_or(1.0 / 30.0, |g| g.as_secs_f32()); self.last = Some(now); self.reduced = c.reduced;
        self.e.ctx_musica = c.playing; if self.cool_until.is_some_and(|t| now >= t) { self.cool_until = None; }
        if !self.init || gap.is_some_and(|g| g > Duration::from_millis(1500)) {
            let greet = self.last_greet.is_none_or(|t| now.duration_since(t) > Duration::from_secs(60)); self.init = true; self.resync(c, now, greet);
        } else {
            let p = self.prev; let mut act = false;
            if c.shelf_adds != p.shelf_adds { self.e.play("engolir"); act = true; }
            let er = ERRORS.load(Relaxed); if er != self.errors { self.errors = er; self.e.play("erro"); act = true; }
            let nf = NOTIFS.load(Relaxed); if nf != self.notifs { self.notifs = nf; self.e.play("notificacao"); act = true; }
            if c.playing != p.playing { if !c.playing { self.e.play("musica-pausa"); } act = true; }
            // troca de faixa com debounce de 300 ms (titulo e artista chegam em 2 eventos do SMTC)
            if c.track != self.cand.0 { self.cand = (c.track, now); }
            if self.cand.0 != self.key && now.duration_since(self.cand.1) >= TRACK_DEBOUNCE { if self.key != 0 && self.cand.0 != 0 { self.e.play("troca-faixa"); act = true; } self.key = self.cand.0; }
            if c.charging != p.charging { if c.charging == Some(true) && p.charging.is_some() { self.e.play("carregando"); } act = true; }
            if c.timer != p.timer { act = true; if c.timer == 3 { self.e.play("pomodoro-fim"); if c.cycle > 0 && c.cycle % 4 == 0 { self.pending = Some("orgulho"); } } }
            self.prev = *c;
            // atividade: mouse perto movendo, clique ou evento (cursor longe e parado nao conta)
            let near = c.pointer.is_none_or(|(x, y)| x.hypot(y) < FAR_PX);
            if let Some((x, y)) = c.pointer { if self.last_ptr.is_none_or(|q| (q.0 - x).abs() + (q.1 - y).abs() > 1.5) { self.last_ptr = Some((x, y)); if near { act = true; } self.e.pointer(x, y); } }
            if act && !self.sleep_lock { self.touched(now); }
            // ocioso: 60 s bocejo (uma vez), 120 s com o mouse longe (ou noite) dorme
            let idle = now.duration_since(self.touch).as_secs_f32(); let calm = !c.playing && c.timer != 1 && !self.low && c.charging != Some(true);
            if calm && !self.sleep_lock {
                if idle >= YAWN_S && !self.yawned && !self.asleep { self.yawned = true; self.e.play("bocejo"); }
                if !self.asleep && ((idle >= SLEEP_S && !near) || (c.night && idle >= 20.0)) { self.asleep = true; }
            } else if !self.sleep_lock { self.asleep = false; }
            if let Some(g) = self.pending { if !self.e.gesture_running() { self.pending = None; self.e.play(g); } }
        }
        // estado de base: depois de um gesto volta ao estado ATUAL (danca se musica, carregando-ativo se plugado...), nao a idle
        if !self.e.gesture_running() { let w = self.want(c); if self.e.base_id() != w { self.e.play(w); } }
        // tempo real: a 4 fps (dormindo/reduzido) o passo de 0,25 s vira subpassos de <= 50 ms (gestos na velocidade certa)
        let n = (dt / 0.05).ceil().clamp(1.0, 30.0) as u32; for _ in 0..n { self.e.step(dt / n as f32); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// seed fixa nos testes (o Default real usa o relogio)
    fn seeded() -> Mascot { let mut m = Mascot::default(); m.e.reseed(7); m }
    fn steps(m: &mut Mascot, c: &Ctx, t: &mut Instant, secs: f32) { for _ in 0..(secs * 30.0) as u32 { *t += Duration::from_millis(33); m.step(c, *t); } }
    #[test] fn rig_embutido_tem_22_reacoes_e_skin() { let r = rig(); assert_eq!(r.ids().len(), 23); assert_eq!(r.skin["id"], "areia-final"); assert!(r.idx.contains_key("carregando-ativo")); }
    #[test] fn rig_embutido_sem_textos_de_terceiros() { let t = RIG.to_lowercase(); assert!(!t.contains("coucou") && !t.contains("mochi")); }
    #[test] fn caminho_do_corpo_vira_segmentos_fechados() { let s = &rig().body; assert!(matches!(s[0], Seg::M(26.0, 22.0))); assert!(matches!(s.last(), Some(Seg::Z))); assert_eq!(s.iter().filter(|x| matches!(x, Seg::C(..))).count(), 4); }
    #[test] fn seguir_alvo_independe_do_fps() {
        let run = |fps: f32| { let mut e = Eavy::new(); e.pointer(300.0, 0.0); for _ in 0..(fps as i32) { e.step(1.0 / fps); } e.cur.gx };
        let (a, b) = (run(30.0), run(144.0)); assert!((a - b).abs() < 0.02, "{a} {b}"); assert!(a > 0.0 && a < 1.0); }
    #[test] fn todas_as_reacoes_rodam_sem_nan_e_terminam() {
        let r = rig(); for id in r.ids() { let mut e = Eavy::new(); e.play(&id); let gesto = e.fx.is_some();
            for _ in 0..(30 * 12) { e.step(1.0 / 30.0); assert!(e.cur.ox.is_finite() && e.cur.oy.is_finite() && e.cur.sx.is_finite() && e.cur.mw.is_finite() && e.tw.roll.is_finite(), "{id}"); assert!(e.cur.sx > 0.2 && e.cur.sy > 0.2, "{id}"); }
            if gesto { assert!(e.fx.is_none(), "{id} nao terminou"); } } }
    #[test] fn troca_de_faixa_volta_para_danca_se_a_musica_segue() {
        let mut e = Eavy::new(); e.ctx_musica = true; e.play("danca"); e.play("troca-faixa"); for _ in 0..(30 * 6) { e.step(1.0 / 30.0); } assert_eq!(e.base_id(), "danca");
        let mut e = Eavy::new(); e.ctx_musica = false; e.play("troca-faixa"); for _ in 0..(30 * 6) { e.step(1.0 / 30.0); } assert_eq!(e.base_id(), "idle-pisca"); }
    #[test] fn gatilhos_reais_e_volta_ao_estado_atual() {
        let mut m = seeded(); let mut t = Instant::now(); let mut c = Ctx { track: 1, charging: Some(false), ..Default::default() }; steps(&mut m, &c, &mut t, 6.0);
        c.playing = true; steps(&mut m, &c, &mut t, 0.2); assert_eq!(m.e.base_id(), "danca");
        c.track = 2; steps(&mut m, &c, &mut t, 0.1); assert!(!m.e.gesture_running(), "debounce: ainda nao troca"); steps(&mut m, &c, &mut t, 0.5); assert!(m.e.gesture_running(), "troca de faixa"); steps(&mut m, &c, &mut t, 6.0); assert_eq!(m.e.base_id(), "danca");
        c.charging = Some(true); steps(&mut m, &c, &mut t, 0.2); assert!(m.e.gesture_running(), "carregando"); steps(&mut m, &c, &mut t, 7.0); assert_eq!(m.e.base_id(), "danca", "musica tem prioridade sobre carregando-ativo");
        c.playing = false; steps(&mut m, &c, &mut t, 3.0); assert_eq!(m.e.base_id(), "carregando-ativo", "pausar a musica nao perde o carregador");
        c.timer = 1; steps(&mut m, &c, &mut t, 0.2); assert_eq!(m.e.base_id(), "pomodoro-inicio"); c.timer = 3; c.playing = true; steps(&mut m, &c, &mut t, 0.2); assert!(m.e.gesture_running(), "pomodoro-fim");
        steps(&mut m, &c, &mut t, 4.0); assert_eq!(m.e.base_id(), "danca", "pomodoro-fim volta ao estado atual"); }
    #[test] fn titulo_e_artista_em_dois_eventos_disparam_uma_troca() {
        let mut m = seeded(); let mut t = Instant::now(); let mut c = Ctx { track: 1, playing: true, ..Default::default() }; steps(&mut m, &c, &mut t, 6.0);
        c.track = 2; steps(&mut m, &c, &mut t, 0.1); c.track = 3; steps(&mut m, &c, &mut t, 0.5); assert!(m.e.gesture_running()); steps(&mut m, &c, &mut t, 6.0);
        let mut n = 0; let mut was = false; for _ in 0..200 { t += Duration::from_millis(33); m.step(&c, t); let g = m.e.gesture_running(); if g && !was { n += 1; } was = g; } assert_eq!(n, 0, "so uma troca"); }
    #[test] fn reabrir_descarta_gestos_vencidos_e_mostra_o_estado_atual() {
        let mut m = seeded(); let mut t = Instant::now(); let mut c = Ctx { track: 1, ..Default::default() }; steps(&mut m, &c, &mut t, 6.0);
        c.playing = true; c.timer = 3; t += Duration::from_secs(30); m.step(&c, t); assert_eq!(m.e.base_id(), "danca"); }
    #[test] fn ocioso_bocejo_sono_e_acorda() {
        let mut m = seeded(); let mut t = Instant::now(); let mut c = Ctx { track: 1, pointer: Some((400.0, 0.0)), ..Default::default() }; steps(&mut m, &c, &mut t, 59.0); assert!(!m.e.gesture_running());
        steps(&mut m, &c, &mut t, 2.0); assert!(m.e.gesture_running(), "bocejo aos 60 s"); steps(&mut m, &c, &mut t, 62.0); assert!(m.e.sleeping(), "sono aos 120 s com mouse longe");
        c.pointer = Some((40.0, 0.0)); steps(&mut m, &c, &mut t, 0.3); assert!(!m.e.sleeping(), "mouse perto acorda");
        let mut m = seeded(); let mut t = Instant::now(); let c = Ctx { track: 1, pointer: Some((10.0, 0.0)), ..Default::default() }; steps(&mut m, &c, &mut t, 130.0); assert!(!m.e.sleeping(), "mouse perto nao dorme"); }
    #[test] fn dormindo_e_oculto_ressincroniza_sem_gesto_velho() {
        let mut m = seeded(); let mut t = Instant::now(); let mut c = Ctx { track: 1, pointer: Some((400.0, 0.0)), ..Default::default() }; steps(&mut m, &c, &mut t, 125.0); assert!(m.e.sleeping());
        c.shelf_adds = 3; t += Duration::from_secs(40); m.step(&c, t); assert!(!m.e.gesture_running() || m.e.fx.as_ref().map(|f| rig().def(f.d)["id"].as_str()) == Some(Some("saudacao")), "sem engolir velho"); }
    #[test] fn passo_de_4_fps_usa_tempo_real() {
        let run = |fps: f32| { let mut m = seeded(); let mut t = Instant::now(); let c = Ctx { track: 1, ..Default::default() }; m.step(&c, t); m.e.play("engolir"); let step = Duration::from_secs_f32(1.0 / fps); for _ in 0..(fps * 1.5) as u32 { t += step; m.step(&c, t); } m.e.fx.as_ref().map_or(-1.0, |f| f.t) };
        let (a, b) = (run(30.0), run(4.0)); assert!((a - b).abs() < 0.3, "{a} {b}"); }
    #[test] fn clique_e_imediato_e_escalona_tapa_tonto_irritado() {
        let mut m = seeded(); let mut t = Instant::now(); let c = Ctx { track: 1, ..Default::default() }; steps(&mut m, &c, &mut t, 6.0);
        m.on_click(t); assert_eq!(m.e.gesture_id(), Some("clique-tapa"), "tapa no mesmo instante, sem espera");
        t += Duration::from_millis(300); m.on_click(t); t += Duration::from_millis(300); m.on_click(t); assert_eq!(m.e.gesture_id(), Some("tonto"), "3 em 1,7 s");
        let n = m.slaps.len(); m.on_click(t); assert_eq!(m.slaps.len(), n, "tonto ignora tapas (como o Coucou)");
        steps(&mut m, &c, &mut t, 2.5); assert!(m.cool_until.is_none() && m.e.gesture_id().is_none());
        for _ in 0..3 { m.on_click(t); t += Duration::from_millis(200); } assert!(m.cooling(t), "2a tontura em 10 s = irritado");
        assert_eq!(m.e.base_id(), "irritado"); assert!(m.balloon(t).is_some());
        m.on_click(t); assert!(m.slaps.is_empty(), "cliques ignorados no cooldown");
        steps(&mut m, &c, &mut t, 5.5); assert!(!m.cooling(t) && m.balloon(t).is_none()); assert_eq!(m.e.base_id(), "idle-pisca"); }
    #[test] fn olhos_seguem_o_cursor_global() {
        let mut m = seeded(); let mut t = Instant::now(); let mut c = Ctx { track: 1, pointer: Some((300.0, 0.0)), ..Default::default() }; steps(&mut m, &c, &mut t, 1.0); assert!(m.e.cur.gx > 0.3, "{}", m.e.cur.gx);
        c.pointer = Some((-300.0, 0.0)); steps(&mut m, &c, &mut t, 1.0); assert!(m.e.cur.gx < -0.3); }
    #[test] fn erro_real_e_bateria_baixa() {
        let mut m = seeded(); let mut t = Instant::now(); let mut c = Ctx { track: 1, ..Default::default() }; steps(&mut m, &c, &mut t, 6.0);
        m.error(); steps(&mut m, &c, &mut t, 0.2); assert!(m.e.gesture_running()); /* report_error usa contador global: nao chamado aqui para nao afetar testes paralelos */ c.charging = Some(false); c.batt_pct = 18; steps(&mut m, &c, &mut t, 3.0); assert_eq!(m.e.base_id(), "bateria-baixa"); c.batt_pct = 22; steps(&mut m, &c, &mut t, 3.0); assert_eq!(m.e.base_id(), "bateria-baixa", "histerese: 22 % ainda baixa"); c.batt_pct = 25; steps(&mut m, &c, &mut t, 3.0); assert_eq!(m.e.base_id(), "idle-pisca"); }
}
