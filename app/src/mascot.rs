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
    pub fill: [f32; 3], pub ink: [f32; 3],
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
    Pose { fill: hex(e["face_fill"].as_str().unwrap_or("#f6f9ff")), ink, eye: [l, r], sp: [sl, sr], brow: [stroke(&e["brow_L"]), stroke(&e["brow_R"])],
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
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] { [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)] }
fn lerp8(a: [f32; 8], b: [f32; 8], t: f32) -> [f32; 8] { let mut o = [0.0; 8]; for i in 0..8 { o[i] = lerp(a[i], b[i], t); } o }
fn lerp_stroke(a: Stroke, b: Stroke, t: f32) -> Stroke { Stroke { p: lerp8(a.p, b.p, t), w: lerp(a.w, b.w, t), a: lerp(a.a, b.a, t) } }
/// morph: lerp de TODOS os numeros (pontos, larguras, alfas, cor do rosto); extras = os da pose de destino
pub fn lerp_pose(a: &Pose, b: &Pose, t: f32) -> Pose {
    Pose { fill: lerp3(a.fill, b.fill, t), ink: lerp3(a.ink, b.ink, t), eye: [lerp_stroke(a.eye[0], b.eye[0], t), lerp_stroke(a.eye[1], b.eye[1], t)],
        sp: [[lerp(a.sp[0][0], b.sp[0][0], t), lerp(a.sp[0][1], b.sp[0][1], t)], [lerp(a.sp[1][0], b.sp[1][0], t), lerp(a.sp[1][1], b.sp[1][1], t)]],
        brow: [lerp_stroke(a.brow[0], b.brow[0], t), lerp_stroke(a.brow[1], b.brow[1], t)], up: lerp8(a.up, b.up, t), low: lerp8(a.low, b.low, t), mw: lerp(a.mw, b.mw, t), ma: lerp(a.ma, b.ma, t), mfill: lerp(a.mfill, b.mfill, t), mcol: lerp3(a.mcol, b.mcol, t),
        extras: if t < 0.5 { a.extras.clone() } else { b.extras.clone() } }
}
fn ease_out(t: f32) -> f32 { 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3) }
fn pose_of(name: &str) -> Pose { let r = rig(); r.poses.get(name).or_else(|| r.poses.get("neutro")).cloned().unwrap_or_default() }

/// Entradas de contexto (lidas pela pagina a cada quadro)
pub struct Ctx<'a> { pub playing: bool, pub dragging: bool, pub mouse: Option<(f32, f32)>, pub personality: &'a str, pub calm_fx: bool }

pub struct Mascot { pub target: String, from: Pose, to: Pose, t0: Instant, dur: f32, clicks: Vec<Instant>, hold_until: Instant, blink0: Option<Instant>, pub next_blink: Option<Instant>, gaze: (f32, f32), last: Instant, born: Instant }
impl Default for Mascot { fn default() -> Self { let n = Instant::now(); let p = pose_of("neutro"); Mascot { target: "neutro".into(), from: p.clone(), to: p, t0: n, dur: 0.0, clicks: Vec::new(), hold_until: n, blink0: None, next_blink: None, gaze: (0.0, 0.0), last: n, born: n } } }

fn rnd(now: Instant, salt: u32) -> f32 { let n = now.elapsed().subsec_nanos() ^ (salt.wrapping_mul(2654435761)); ((n >> 4) & 0xFFFF) as f32 / 65535.0 }
const BLINK_MS: f32 = 110.0;

impl Mascot {
    pub fn on_click(&mut self, now: Instant) { self.clicks.push(now); self.clicks.retain(|t| now.duration_since(*t) < Duration::from_secs(6)); }
    fn count_in(&self, now: Instant, secs: f32) -> u32 { self.clicks.iter().filter(|t| now.duration_since(**t).as_secs_f32() <= secs).count() as u32 }
    /// qual expressao agora: raiva por cliques (limiares da personalidade) > curioso (arrastando arquivo) > surpreso (1o clique) > feliz (tocando) > neutro
    pub fn decide(&self, c: &Ctx, now: Instant) -> &'static str {
        let r = rig(); let p = r.personalities.get(c.personality).or_else(|| r.personalities.get("calmo")).cloned().unwrap_or_default();
        if self.count_in(now, p.furioso.1) >= p.furioso.0 { return "furioso"; }
        if self.count_in(now, p.raiva.1) >= p.raiva.0 { return "raiva"; }
        if self.count_in(now, p.irritado.1) >= p.irritado.0 { return "irritado"; }
        if c.dragging { return "curioso"; }
        if self.clicks.last().is_some_and(|t| now.duration_since(*t).as_millis() < 700) { return "surpreso"; }
        if c.playing { "feliz" } else { "neutro" }
    }
    fn duration(&self, from: &str, to: &str, c: &Ctx) -> f32 {
        let r = rig(); let set = if c.personality == "calmo" { "calmo" } else { "normal" };
        let m = &r.dur[set]; let key = format!("{from}>{to}"); let anyn = format!("any>{to}");
        m.get(&key).or_else(|| m.get(&anyn)).copied().unwrap_or(r.default_ms * if c.personality == "calmo" { 1.3 } else { 1.0 })
    }
    /// avanca o estado e devolve a pose desenhada (morph + piscar); `gaze_out` = deslocamento do olhar em unidades do rosto
    pub fn step(&mut self, c: &Ctx, now: Instant) -> (Pose, (f32, f32)) {
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
        (self.dur > 0.0 && now.duration_since(self.t0).as_secs_f32() * 1000.0 < self.dur + 20.0) || self.blink0.is_some() || self.raging() || mouse_over || self.gaze.0.abs() > 0.03 || self.gaze.1.abs() > 0.03
    }
    /// proximo despertar: 400 ms enquanto ha cliques recentes ou raiva (reavaliar/esfriar), senao o proximo piscar; ocioso = um unico timer
    pub fn wake_ms(&self, now: Instant) -> Option<u32> {
        if !self.clicks.is_empty() || !matches!(self.target.as_str(), "neutro" | "feliz" | "curioso") { return Some(400); }
        self.next_blink.map(|t| t.saturating_duration_since(now).as_millis() as u32 + 5)
    }
    pub fn raging(&self) -> bool { matches!(self.target.as_str(), "raiva" | "furioso") }
    pub fn age_ms(&self, now: Instant) -> u128 { now.duration_since(self.born).as_millis() }
}

fn rgba3(c: [f32; 3], a: f32) -> Rgba { (c[0], c[1], c[2], a) }
fn shift8(p: [f32; 8], dx: f32, dy: f32) -> [f32; 8] { let mut o = p; for i in 0..4 { o[2 * i] += dx; o[2 * i + 1] += dy; } o }

/// Desenha o rosto com largura `w` px de projeto no canto (x, y). `breath` = escala vertical (1.0..1.025), `shake` = tremor em X (u). Usa a transformacao atual como base.
pub fn draw(a: &mut App, pose: &Pose, gaze: (f32, f32), x: f32, y: f32, w: f32, breath: f32, shake: f32) {
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
        match ex.as_str() { "ques" => g.text("?", crate::gfx::Fmt::Big, 166.0, -2.0, 30.0, 38.0, rgba3(pose.ink, 0.9), false), "notes" => g.text("♪", crate::gfx::Fmt::Big, 164.0, -8.0, 30.0, 38.0, rgba3(pose.ink, 0.9), false), _ => {} }
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
        let c = Ctx { playing: false, dragging: false, mouse: None, personality: "calmo", calm_fx: true }; let n = Instant::now();
        let mut m = Mascot::default(); assert_eq!(m.decide(&c, n), "neutro");
        for _ in 0..3 { m.on_click(n); } assert_ne!(m.decide(&c, n), "irritado", "3 cliques ainda nao irritam o calmo");
        m.on_click(n); assert_eq!(m.decide(&c, n), "irritado", "4 em 3 s");
        for _ in 0..4 { m.on_click(n); } assert_eq!(m.decide(&c, n), "raiva", "8 em 5 s");
        for _ in 0..4 { m.on_click(n); } assert_eq!(m.decide(&c, n), "furioso", "12 em 5 s");
    }
    #[test] fn contexto_define_a_expressao_sem_cliques() {
        let n = Instant::now(); let m = Mascot::default(); let mut c = Ctx { playing: true, dragging: false, mouse: None, personality: "calmo", calm_fx: true };
        assert_eq!(m.decide(&c, n), "feliz"); c.dragging = true; assert_eq!(m.decide(&c, n), "curioso"); c.dragging = false; c.playing = false; assert_eq!(m.decide(&c, n), "neutro");
    }
}
