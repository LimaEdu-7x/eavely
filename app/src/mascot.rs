// Mascote do Eavely (v0.2.0): rosto-pilula desenhado por curvas Bezier a partir de um rig em dados (assets/mascote/rig.json).
// Cada expressao usa as MESMAS partes (2 olhos, 2 sobrancelhas, boca); o morph e o lerp dos numeros. So pede quadros enquanto a pagina Mascote esta visivel E ha
// algo animando (morph, piscar, olhar, respiracao); parado/ilha fechada = 0 % de CPU (piscar por UM timer; clique/mouse chegam por evento).
use std::{collections::HashMap, sync::OnceLock, time::{Duration, Instant}};
use serde_json::Value;
use windows_numerics::Matrix3x2;
use crate::{gfx::Rgba, island::App};

const RIG: &str = include_str!("../../assets/mascote/rig.json");

#[derive(Clone, Copy, Debug, Default, PartialEq)] pub struct Stroke { pub p: [f32; 8], pub w: f32, pub a: f32 }
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pose {
    pub fill: [f32; 3], pub ink: [f32; 3], pub color: [f32; 3], // color = cor da emocao (olhos soltos do peek pequeno)
    pub eye: [Stroke; 2], pub sp: [[f32; 2]; 2], // sp[olho] = [alfa do X, alfa do coracao]
    pub brow: [Stroke; 2],
    pub up: [f32; 8], pub low: [f32; 8], pub mw: f32, pub ma: f32, pub mfill: f32, pub mcol: [f32; 3],
    pub extras: Vec<String>,
}
pub struct Rig { pub poses: HashMap<String, Pose>, pub eye_c: [(f32, f32); 2], pub personalities: HashMap<String, Personality>, pub dur: HashMap<String, HashMap<String, f32>>, pub default_ms: f32, pub min_hold: HashMap<String, f32> }
#[derive(Clone, Debug, Default)] pub struct Personality { pub irritado: (u32, f32), pub raiva: (u32, f32), pub furioso: (u32, f32), pub amplitude: f32, pub morph_x: f32 }

fn hex(s: &str) -> [f32; 3] { let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0); [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0] }
fn arr8(v: &Value) -> [f32; 8] { let mut o = [0.0; 8]; if let Some(a) = v.as_array() { for (i, x) in a.iter().take(8).enumerate() { o[i] = x.as_f64().unwrap_or(0.0) as f32; } } o }
fn stroke(v: &Value) -> Stroke { Stroke { p: arr8(&v["pts"]), w: v["w"].as_f64().unwrap_or(4.0) as f32, a: v["alpha"].as_f64().unwrap_or(1.0) as f32 } }
fn parse_pose(e: &Value) -> Pose {
    let eye = |k: &str| -> (Stroke, [f32; 2]) { let o = &e[k]; let sa = o["special_alpha"].as_f64().unwrap_or(0.0) as f32; let sp = match o["special"].as_str() { Some("x") => [sa, 0.0], Some("heart") => [0.0, sa], _ => [0.0, 0.0] }; (stroke(&o["stroke"]), sp) };
    let (l, sl) = eye("eye_L"); let (r, sr) = eye("eye_R");
    let m = &e["mouth"]; let ink = hex(e["ink"].as_str().unwrap_or("#15151C"));
    Pose { fill: hex(e["face_fill"].as_str().unwrap_or("#f6f9ff")), color: hex(e["color"].as_str().unwrap_or("#D4E4FF")), ink, eye: [l, r], sp: [sl, sr], brow: [stroke(&e["brow_L"]), stroke(&e["brow_R"])],
        up: arr8(&m["up"]), low: arr8(&m["low"]), mw: m["stroke_w"].as_f64().unwrap_or(5.5) as f32, ma: m["alpha"].as_f64().unwrap_or(1.0) as f32, mfill: m["fill_alpha"].as_f64().unwrap_or(0.0) as f32,
        mcol: if m["fill_color"].as_str() == Some("white") { [1.0, 1.0, 1.0] } else { ink }, extras: e["extras"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() }
}
pub fn rig() -> &'static Rig {
    static R: OnceLock<Rig> = OnceLock::new();
    R.get_or_init(|| {
        let d: Value = serde_json::from_str(RIG).unwrap_or(Value::Null);
        let poses = d["expressoes"].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), parse_pose(v))).collect()).unwrap_or_default();
        let c = |k: &str| (d["eye_centers"][k][0].as_f64().unwrap_or(0.0) as f32, d["eye_centers"][k][1].as_f64().unwrap_or(0.0) as f32);
        let cl = |s: &Value| -> (u32, f32) { let t: Vec<f32> = s.as_str().unwrap_or("").split(|ch: char| !ch.is_ascii_digit()).filter_map(|x| x.parse().ok()).collect(); (t.first().copied().unwrap_or(99.0) as u32, t.get(1).copied().unwrap_or(3.0)) };
        let personalities = d["personalidades"].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), Personality { irritado: cl(&v["cliques"]["irritado"]), raiva: cl(&v["cliques"]["raiva"]), furioso: cl(&v["cliques"]["furioso"]), amplitude: v["amplitude"].as_f64().unwrap_or(1.0) as f32, morph_x: v["morph_x"].as_f64().unwrap_or(1.0) as f32 })).collect()).unwrap_or_default();
        let mut dur: HashMap<String, HashMap<String, f32>> = HashMap::new();
        for (set, key) in [("normal", "durations_normal_ms"), ("calmo", "durations_calmo_ms")] { dur.insert(set.into(), d["morph"][key].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_f64().unwrap_or(180.0) as f32)).collect()).unwrap_or_default()); }
        let min_hold = d["morph"]["min_hold_ms"].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_f64().unwrap_or(1200.0) as f32)).collect()).unwrap_or_default();
        Rig { poses, eye_c: [c("L"), c("R")], personalities, dur, default_ms: d["morph"]["default_ms"].as_f64().unwrap_or(180.0) as f32, min_hold }
    })
}
fn static_name(s: &str) -> &'static str { for n in ["neutro", "feliz", "curioso", "surpreso", "irritado", "raiva", "furioso", "choro", "triste", "preocupado", "sono", "sonolento", "entediado"] { if n == s { return n; } } "neutro" }
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] { [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)] }
fn lerp8(a: [f32; 8], b: [f32; 8], t: f32) -> [f32; 8] { let mut o = [0.0; 8]; for i in 0..8 { o[i] = lerp(a[i], b[i], t); } o }
fn lerp_stroke(a: Stroke, b: Stroke, t: f32) -> Stroke { Stroke { p: lerp8(a.p, b.p, t), w: lerp(a.w, b.w, t), a: lerp(a.a, b.a, t) } }
/// morph: lerp de TODOS os numeros (pontos, larguras, alfas, cor do rosto); extras = os da pose de destino
pub fn lerp_pose(a: &Pose, b: &Pose, t: f32) -> Pose {
    Pose { fill: lerp3(a.fill, b.fill, t), ink: lerp3(a.ink, b.ink, t), color: lerp3(a.color, b.color, t), eye: [lerp_stroke(a.eye[0], b.eye[0], t), lerp_stroke(a.eye[1], b.eye[1], t)],
        sp: [[lerp(a.sp[0][0], b.sp[0][0], t), lerp(a.sp[0][1], b.sp[0][1], t)], [lerp(a.sp[1][0], b.sp[1][0], t), lerp(a.sp[1][1], b.sp[1][1], t)]],
        brow: [lerp_stroke(a.brow[0], b.brow[0], t), lerp_stroke(a.brow[1], b.brow[1], t)], up: lerp8(a.up, b.up, t), low: lerp8(a.low, b.low, t), mw: lerp(a.mw, b.mw, t), ma: lerp(a.ma, b.ma, t), mfill: lerp(a.mfill, b.mfill, t), mcol: lerp3(a.mcol, b.mcol, t),
        extras: if t < 0.5 { a.extras.clone() } else { b.extras.clone() } }
}
fn ease_out(t: f32) -> f32 { 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3) }
fn pose_of(name: &str) -> Pose { let r = rig(); r.poses.get(name).or_else(|| r.poses.get("neutro")).cloned().unwrap_or_default() }

/// Entradas de contexto (lidas pela pagina a cada quadro)
pub struct Ctx<'a> { pub playing: bool, pub dragging: bool, pub mouse: Option<(f32, f32)>, pub personality: &'a str, pub calm_fx: bool, pub night: bool, pub low_batt: bool }
/// tedio/sono por inatividade (calmo): entediado 12 min, sono 30 min; a noite (23h-6h) fica sonolento e, parado 1 min, dorme
pub const BORED_S: f32 = 12.0 * 60.0; pub const SLEEP_S: f32 = 30.0 * 60.0;

pub struct Mascot { pub force: Option<String>, pub touch: Instant, pub target: String, from: Pose, to: Pose, t0: Instant, dur: f32, clicks: Vec<Instant>, hold_until: Instant, blink0: Option<Instant>, pub next_blink: Option<Instant>, gaze: (f32, f32), last: Instant, born: Instant }
impl Default for Mascot { fn default() -> Self { let n = Instant::now(); let p = pose_of("neutro"); Mascot { force: None, touch: n, target: "neutro".into(), from: p.clone(), to: p, t0: n, dur: 0.0, clicks: Vec::new(), hold_until: n, blink0: None, next_blink: None, gaze: (0.0, 0.0), last: n, born: n } } }

/// aleatorio 0..1 (splitmix64 semeado pelo relogio do sistema; cada chamada avanca o estado)
fn rnd(_now: Instant, salt: u32) -> f32 {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
    static S: AtomicU64 = AtomicU64::new(0);
    let mut z = S.load(Relaxed); if z == 0 { z = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0x9E37, |d| d.as_nanos() as u64) | 1; }
    z = z.wrapping_add(0x9E3779B97F4A7C15).wrapping_add(salt as u64); S.store(z, Relaxed);
    let mut x = z; x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9); x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB); x ^= x >> 31;
    ((x >> 40) & 0xFFFF) as f32 / 65535.0
}
const BLINK_MS: f32 = 110.0;

impl Mascot {
    /// a ilha foi aberta/usada agora: zera o tedio
    pub fn touched(&mut self, now: Instant) { self.touch = now; }
    pub fn on_click(&mut self, now: Instant) { self.touch = now; self.clicks.push(now); self.clicks.retain(|t| now.duration_since(*t) < Duration::from_secs(6)); }
    fn count_in(&self, now: Instant, secs: f32) -> u32 { self.clicks.iter().filter(|t| now.duration_since(**t).as_secs_f32() <= secs).count() as u32 }
    /// qual expressao agora: raiva por cliques (limiares da personalidade) > curioso (arrastando arquivo) > surpreso (1o clique) > feliz (tocando) > neutro
    pub fn decide(&self, c: &Ctx, now: Instant) -> &'static str {
        if let Some(f) = &self.force { return static_name(f); } // so testes (bench)
        let r = rig(); let p = r.personalities.get(c.personality).or_else(|| r.personalities.get("calmo")).cloned().unwrap_or_default();
        if self.count_in(now, p.furioso.1) >= p.furioso.0 * 2 { return "choro"; } // maus-tratos fortes e repetidos (calmo: 24 cliques em 5 s)
        if self.count_in(now, p.furioso.1) >= p.furioso.0 { return "furioso"; }
        if self.count_in(now, p.raiva.1) >= p.raiva.0 { return "raiva"; }
        if self.count_in(now, p.irritado.1) >= p.irritado.0 { return "irritado"; }
        if c.dragging { return "curioso"; }
        if self.clicks.last().is_some_and(|t| now.duration_since(*t).as_millis() < 700) { return "surpreso"; }
        if c.low_batt { return "preocupado"; }
        if c.playing { return "feliz"; }
        let idle = now.duration_since(self.touch).as_secs_f32();
        if idle >= SLEEP_S || (c.night && idle >= 60.0) { return "sono"; }
        if c.night { return "sonolento"; }
        if idle >= BORED_S { return "entediado"; }
        "neutro"
    }
    fn duration(&self, from: &str, to: &str, c: &Ctx) -> f32 {
        let r = rig(); let set = if c.personality == "calmo" { "calmo" } else { "normal" };
        let m = &r.dur[set]; let key = format!("{from}>{to}"); let anyn = format!("any>{to}");
        m.get(&key).or_else(|| m.get(&anyn)).copied().unwrap_or(r.default_ms * if c.personality == "calmo" { 1.3 } else { 1.0 })
    }
    /// avanca o estado e devolve a pose desenhada (morph + piscar); `gaze_out` = deslocamento do olhar em unidades do rosto
    pub fn step(&mut self, c: &Ctx, now: Instant) -> (Pose, (f32, f32)) {
        self.clicks.retain(|t| now.duration_since(*t) < Duration::from_secs(6)); // poda aqui tambem: sem cliques novos o timer de 400 ms para sozinho
        if c.mouse.is_some() { self.touch = now; }
        let want = self.decide(c, now);
        if want != self.target {
            let escalate = matches!(want, "irritado" | "raiva" | "furioso");
            let hold = rig().min_hold.get(c.personality).copied().unwrap_or(1200.0);
            if escalate || now >= self.hold_until {
                let cur = self.pose_now(now); let dur = self.duration(&self.target.clone(), want, c);
                self.from = cur; self.to = pose_of(want); self.t0 = now; self.dur = if c.calm_fx { dur } else { 0.0 }; self.target = want.to_string(); self.hold_until = now + Duration::from_millis(hold as u64);
            }
        }
        let mut pose = self.pose_now(now);
        // piscar: 110 ms, a cada 3-6 s; olho-pilula dobra ate virar linha (so se o olho nao e especial)
        if self.next_blink.is_none() { self.next_blink = Some(now + Duration::from_millis((3000.0 + 3000.0 * rnd(now, 7)) as u64)); }
        if let (Some(nb), None) = (self.next_blink, self.blink0) { if now >= nb { self.blink0 = Some(now); } }
        if let Some(b0) = self.blink0 {
            let p = now.duration_since(b0).as_secs_f32() * 1000.0 / BLINK_MS;
            if p >= 1.0 { self.blink0 = None; self.next_blink = Some(now + Duration::from_millis((3000.0 + 3000.0 * rnd(now, 13)) as u64)); }
            else { let t = 1.0 - (2.0 * p - 1.0).abs(); let ec = rig().eye_c;
                for i in 0..2 { if pose.sp[i][0] + pose.sp[i][1] < 0.5 { let (cx, cy) = ec[i]; let tgt = Stroke { p: [cx - 12.0, cy, cx - 5.0, cy + 5.0, cx + 5.0, cy + 5.0, cx + 12.0, cy], w: 5.5, a: 1.0 }; pose.eye[i] = lerp_stroke(pose.eye[i], tgt, t); } } }
        }
        // olhar: segue o mouse (so com o mouse sobre a ilha); mola critica aproximada por exponencial
        let dt = now.duration_since(self.last).as_secs_f32().min(0.1); self.last = now;
        let tgt = c.mouse.map_or((0.0, 0.0), |(mx, my)| ((mx / 40.0).clamp(-1.0, 1.0) * 6.0, (my / 30.0).clamp(-1.0, 1.0) * 6.0));
        let k = 1.0 - (-dt * 14.0).exp(); self.gaze = (self.gaze.0 + (tgt.0 - self.gaze.0) * k, self.gaze.1 + (tgt.1 - self.gaze.1) * k);
        (pose, self.gaze)
    }
    fn pose_now(&self, now: Instant) -> Pose {
        if self.dur <= 0.0 { return self.to.clone(); }
        let t = now.duration_since(self.t0).as_secs_f32() * 1000.0 / self.dur; if t >= 1.0 { return self.to.clone(); }
        lerp_pose(&self.from, &self.to, ease_out(t))
    }
    /// ha algo animando que exige quadros? (morph, piscada em curso, olhar ainda andando)
    pub fn busy(&self, now: Instant, mouse_over: bool) -> bool {
        (self.dur > 0.0 && now.duration_since(self.t0).as_secs_f32() * 1000.0 < self.dur + 20.0) || self.blink0.is_some() || self.raging() || self.to.extras.iter().any(|e| e.starts_with("tear")) || mouse_over || self.gaze.0.abs() > 0.03 || self.gaze.1.abs() > 0.03
    }
    /// proximo despertar: 400 ms enquanto ha cliques recentes ou raiva (reavaliar/esfriar), senao o proximo piscar; ocioso = um unico timer
    pub fn wake_ms(&self, now: Instant) -> Option<u32> {
        if !self.clicks.is_empty() || matches!(self.target.as_str(), "irritado" | "raiva" | "furioso" | "choro" | "surpreso") { return Some(400); }
        self.next_blink.map(|t| t.saturating_duration_since(now).as_millis() as u32 + 5)
    }
    pub fn raging(&self) -> bool { matches!(self.target.as_str(), "raiva" | "furioso") }
    pub fn age_ms(&self, now: Instant) -> u128 { now.duration_since(self.born).as_millis() }
}

fn rgba3(c: [f32; 3], a: f32) -> Rgba { (c[0], c[1], c[2], a) }
/// Mascote no peek (150x30 projeto): rosto espiando por baixo (k >= 1) ou SO OLHOS (tracos na cor da emocao, sem rosto nem boca) quando o peek fica menor que 150 px reais
pub fn draw_peek(a: &mut App, pose: &Pose, gaze: (f32, f32), w: f32, h: f32, eyes_only: bool) {
    let base = a.gfx.get_matrix();
    if eyes_only {
        // 4 tracos (2 olhos + 2 sobrancelhas) escalados para ~44 px, centrados em (w/2, h/2); mesma geometria do rig; cor = emocao
        let s = 0.5; let m = Matrix3x2::translation(-100.0, -52.0) * Matrix3x2::scale(s, s) * Matrix3x2::translation(w / 2.0, h / 2.0 + 1.0);
        a.gfx.set_matrix(m * base); let c = pose.color; let g = &a.gfx;
        for i in 0..2 { let e = pose.eye[i]; g.bezier(shift8(e.p, gaze.0, gaze.1), e.w * 1.3, rgba3(c, e.a)); let b = pose.brow[i]; g.bezier(shift8(b.p, gaze.0 * 0.4, gaze.1 * 0.4), b.w * 1.3, rgba3(c, b.a)); }
    } else {
        // rosto de 46 px de largura espiando: so a parte de cima aparece (a forma do peek recorta o resto)
        let s = 46.0 / 200.0; let m = Matrix3x2::scale(s, s) * Matrix3x2::translation(w / 2.0 - 23.0, h - 17.0);
        a.gfx.set_matrix(m * base); let g = &a.gfx;
        g.rrect(0.0, 0.0, 200.0, 120.0, 56.0, rgba3(pose.fill, 1.0)); g.rrect_stroke(1.6, 1.6, 196.8, 116.8, 54.4, 3.2, rgba3(pose.ink, 1.0));
        for i in 0..2 { let e = pose.eye[i]; g.bezier(shift8(e.p, gaze.0, gaze.1), e.w, rgba3(pose.ink, e.a)); let b = pose.brow[i]; g.bezier(shift8(b.p, gaze.0 * 0.4, gaze.1 * 0.4), b.w, rgba3(pose.ink, b.a)); }
        g.mouth(pose.up, pose.low, pose.mw, rgba3(pose.ink, pose.ma), if pose.mfill > 0.003 { Some(rgba3(pose.mcol, pose.mfill)) } else { None });
    }
    a.gfx.set_matrix(base);
}
fn shift8(p: [f32; 8], dx: f32, dy: f32) -> [f32; 8] { let mut o = p; for i in 0..4 { o[2 * i] += dx; o[2 * i + 1] += dy; } o }

/// Desenha o rosto com largura `w` px de projeto no canto (x, y). `breath` = escala vertical (1.0..1.025), `shake` = tremor em X (u). Usa a transformacao atual como base.
pub fn draw(a: &mut App, pose: &Pose, gaze: (f32, f32), x: f32, y: f32, w: f32, breath: f32, shake: f32, t: f32) {
    let base = a.gfx.get_matrix(); let s = w / 200.0;
    let anchor = Matrix3x2::translation(-100.0, -120.0) * Matrix3x2::scale(1.0, breath) * Matrix3x2::translation(100.0, 120.0);
    // translacao em px reais: o `base` ja traz escala k e DPI; as coordenadas (x, y) sao de projeto
    let local = anchor * Matrix3x2::scale(s, s) * Matrix3x2::translation(x + shake * s, y);
    a.gfx.set_matrix(local * base);
    let g = &a.gfx;
    // rosto
    g.rrect(0.0, 0.0, 200.0, 120.0, 56.0, rgba3(pose.fill, 1.0)); g.rrect_stroke(1.6, 1.6, 196.8, 116.8, 54.4, 3.2, rgba3(pose.ink, 1.0));
    let (gx, gy) = gaze; let f = 0.4;
    for i in 0..2 {
        let e = pose.eye[i]; g.bezier(shift8(e.p, gx, gy), e.w, rgba3(pose.ink, e.a));
        let (cx, cy) = (rig().eye_c[i].0 + gx, rig().eye_c[i].1 + gy);
        if pose.sp[i][0] > 0.003 { let c = rgba3(pose.ink, pose.sp[i][0]); g.line(cx - 10.0, cy - 10.0, cx + 10.0, cy + 10.0, 5.5, c); g.line(cx + 10.0, cy - 10.0, cx - 10.0, cy + 10.0, 5.5, c); }
        if pose.sp[i][1] > 0.003 { g.heart(cx, cy + 1.0, 0.8, (0.91, 0.33, 0.49, pose.sp[i][1])); }
        let b = pose.brow[i]; g.bezier(shift8(b.p, gx * f, gy * f), b.w, rgba3(pose.ink, b.a));
    }
    g.mouth(shift8(pose.up, gx * f, gy * f), shift8(pose.low, gx * f, gy * f), pose.mw, rgba3(pose.ink, pose.ma), if pose.mfill > 0.003 { Some(rgba3(pose.mcol, pose.mfill)) } else { None });
    for ex in &pose.extras {
        match ex.as_str() {
            "ques" => g.text("?", crate::gfx::Fmt::Big, 166.0, -2.0, 30.0, 38.0, rgba3(pose.ink, 0.9), false), "notes" => g.text("♪", crate::gfx::Fmt::Big, 164.0, -8.0, 30.0, 38.0, rgba3(pose.ink, 0.9), false),
            // lagrimas: caem 46 u (tear1: 30 u) a cada 900 ms, a 2a nasce na metade da 1a; alfa 1 - fase^3
            "tears" | "tear1" => { let (xs, travel): (&[f32], f32) = if ex == "tears" { (&[62.0, 138.0], 46.0) } else { (&[138.0], 30.0) };
                for (k, tx) in xs.iter().enumerate() { for ph in [0.0f32, 0.5] { let f = ((t / 0.9 + ph + k as f32 * 0.17).fract()).clamp(0.0, 1.0); let yy = 74.0 + travel * f; let al = 1.0 - f.powi(3);
                    g.circle(*tx, yy, 4.2, (0.30, 0.61, 1.0, al)); g.ring(*tx, yy, 4.2, 1.2, (0.17, 0.44, 0.84, al)); } } }
            "sweat" => { g.ellipse(176.0, 22.0, 4.6, 6.4, (0.55, 0.78, 1.0, 1.0)); g.ring(176.0, 22.0, 5.0, 1.2, (0.17, 0.44, 0.84, 1.0)); }
            // zzz: Z grande e z pequeno flutuam devagar (so quadros com a ilha aberta; a respiracao ja os pede)
            "zzz" | "z1" => { let dy = -2.5 * (std::f32::consts::TAU * t / 2.4).sin(); let c = rgba3([pose.color[0] * 0.65, pose.color[1] * 0.65, pose.color[2] * 0.65], 0.95);
                g.text("Z", crate::gfx::Fmt::Head, 164.0, 4.0 + dy, 24.0, 26.0, c, false); if ex == "zzz" { g.text("z", crate::gfx::Fmt::Title, 186.0, -14.0 - dy, 20.0, 20.0, c, false); } }
            _ => {}
        }
    }
    a.gfx.set_matrix(base);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rig_embutido_tem_as_expressoes_e_partes_normalizadas() {
        let r = rig(); assert!(r.poses.len() >= 20, "{}", r.poses.len());
        for n in ["neutro", "feliz", "curioso", "surpreso", "irritado", "raiva", "furioso", "sono"] { let p = r.poses.get(n).unwrap_or_else(|| panic!("falta {n}")); assert!(p.eye[0].w > 0.0 && p.mw > 0.0, "{n}"); }
        assert_eq!(r.eye_c[0], (66.0, 56.0));
    }
    #[test] fn morph_a_0_e_a_1_reproduz_as_pontas_e_no_meio_fica_entre() {
        let (a, b) = (pose_of("neutro"), pose_of("feliz"));
        assert_eq!(lerp_pose(&a, &b, 0.0).up, a.up); assert_eq!(lerp_pose(&a, &b, 1.0).up, b.up);
        let m = lerp_pose(&a, &b, 0.5); assert!((m.up[1] - (a.up[1] + b.up[1]) / 2.0).abs() < 1e-3); assert!((m.fill[0] - (a.fill[0] + b.fill[0]) / 2.0).abs() < 1e-3);
    }
    #[test] fn limiares_de_raiva_do_calmo() {
        let c = Ctx { playing: false, dragging: false, mouse: None, personality: "calmo", calm_fx: true, night: false, low_batt: false }; let n = Instant::now();
        let mut m = Mascot::default(); assert_eq!(m.decide(&c, n), "neutro");
        for _ in 0..3 { m.on_click(n); } assert_ne!(m.decide(&c, n), "irritado", "3 cliques ainda nao irritam o calmo");
        m.on_click(n); assert_eq!(m.decide(&c, n), "irritado", "4 em 3 s");
        for _ in 0..4 { m.on_click(n); } assert_eq!(m.decide(&c, n), "raiva", "8 em 5 s");
        for _ in 0..4 { m.on_click(n); } assert_eq!(m.decide(&c, n), "furioso", "12 em 5 s");
    }
    #[test] fn noite_bateria_e_tedio_definem_o_humor() {
        let n = Instant::now(); let mut c = Ctx { playing: false, dragging: false, mouse: None, personality: "calmo", calm_fx: true, night: false, low_batt: false };
        let m = Mascot::default(); assert_eq!(m.decide(&c, n), "neutro");
        c.low_batt = true; assert_eq!(m.decide(&c, n), "preocupado"); c.playing = true; assert_eq!(m.decide(&c, n), "preocupado", "bateria baixa vence a musica"); c.low_batt = false; c.playing = false;
        c.night = true; assert_eq!(m.decide(&c, n), "sonolento", "noite e sonolento");
        if let Some(t) = n.checked_sub(Duration::from_secs(61)) { let mut m2 = Mascot::default(); m2.touch = t; assert_eq!(m2.decide(&c, n), "sono", "noite + 1 min parado dorme"); }
        c.night = false;
        if let Some(t) = n.checked_sub(Duration::from_secs(13 * 60)) { let mut m2 = Mascot::default(); m2.touch = t; assert_eq!(m2.decide(&c, n), "entediado", "12 min parado"); }
        if let Some(t) = n.checked_sub(Duration::from_secs(31 * 60)) { let mut m2 = Mascot::default(); m2.touch = t; assert_eq!(m2.decide(&c, n), "sono", "30 min parado"); }
    }
    #[test] fn choro_so_com_maus_tratos_fortes() {
        let c = Ctx { playing: false, dragging: false, mouse: None, personality: "calmo", calm_fx: true, night: false, low_batt: false }; let n = Instant::now(); let mut m = Mascot::default();
        for _ in 0..12 { m.on_click(n); } assert_eq!(m.decide(&c, n), "furioso"); for _ in 0..12 { m.on_click(n); } assert_eq!(m.decide(&c, n), "choro", "24 cliques em 5 s");
    }
    #[test] fn extras_do_rig_para_choro_sono_e_preocupado() {
        let r = rig(); assert!(r.poses["choro"].extras.iter().any(|e| e == "tears") && r.poses["sono"].extras.iter().any(|e| e == "zzz") && r.poses["preocupado"].extras.iter().any(|e| e == "sweat"));
        assert!(r.poses["feliz"].color[0] > 0.9, "cor da emocao para os olhos soltos");
    }
    #[test] fn contexto_define_a_expressao_sem_cliques() {
        let n = Instant::now(); let m = Mascot::default(); let mut c = Ctx { playing: true, dragging: false, mouse: None, personality: "calmo", calm_fx: true, night: false, low_batt: false };
        assert_eq!(m.decide(&c, n), "feliz"); c.dragging = true; assert_eq!(m.decide(&c, n), "curioso"); c.dragging = false; c.playing = false; assert_eq!(m.decide(&c, n), "neutro");
    }
}
