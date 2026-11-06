//! Autoteste interno de animacao (`Eavely.exe --selftest-anim[=arquivo.csv]`), valido no exe de RELEASE (sem devtools): abre/fecha a ilha POR CODIGO (mesmo caminho do uso real:
//! repouso GDI -> renderer sob demanda -> mola), grava um log por quadro (t, dt, largura, altura) e confere as curvas (overshoot, undershoot, passos por quadro, duracao).
//! Nao usa entrada sintetica nem toca em outros apps. Fora desse argumento nao faz nada (so um teste de atomico por quadro).
use std::sync::{atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed}, Mutex};
use crate::island::{App, St};

static ON: AtomicBool = AtomicBool::new(false);
static STEP: AtomicUsize = AtomicUsize::new(0);
static REC: Mutex<Vec<(String, f32, f32, f32, f32)>> = Mutex::new(Vec::new()); // (segmento, t_ms, dt_ms, w, h)
static SEG: Mutex<String> = Mutex::new(String::new());
pub fn on() -> bool { ON.load(Relaxed) }
pub fn path() -> Option<std::path::PathBuf> { std::env::args().find_map(|a| a.strip_prefix("--selftest-anim").map(|r| r.trim_start_matches('=').to_string())).map(|p| if p.is_empty() { std::env::temp_dir().join("eavely-selftest.csv") } else { p.into() }) }
pub fn init() { if path().is_some() { ON.store(true, Relaxed); } }
pub fn rec(t: f32, dt: f32, w: f32, h: f32, page_p: f32) { let s = SEG.lock().unwrap_or_else(|e| e.into_inner()).clone(); if s.is_empty() { return; }
    // segmentos 'aba*': registra o progresso da mola de troca de aba (0..100) no lugar da largura
    let (w, h) = if s.starts_with("aba") { (page_p * 100.0, 0.0) } else { (w, h) };
    REC.lock().unwrap_or_else(|e| e.into_inner()).push((s, t, dt, w, h)); }

#[derive(Clone, Copy, PartialEq, Debug)] pub enum Kind { Open, Close, Grow, Tab }
#[derive(Debug, Default, Clone)]
pub struct Metrics { pub n: usize, pub dur_ms: f32, pub w0: f32, pub w1: f32, pub peak_w: f32, pub min_w: f32, pub h0: f32, pub h1: f32, pub peak_h: f32, pub min_h: f32, pub max_step_w: f32, pub max_dt: f32, pub med_dt: f32 }
pub fn metrics(f: &[(f32, f32, f32, f32)]) -> Metrics { // (t, dt, w, h)
    let mut m = Metrics { n: f.len(), ..Default::default() }; if f.is_empty() { return m; }
    m.dur_ms = f[f.len() - 1].0 - f[0].0; m.w0 = f[0].2; m.w1 = f[f.len() - 1].2; m.h0 = f[0].3; m.h1 = f[f.len() - 1].3;
    m.peak_w = f.iter().map(|x| x.2).fold(f32::MIN, f32::max); m.min_w = f.iter().map(|x| x.2).fold(f32::MAX, f32::min);
    m.peak_h = f.iter().map(|x| x.3).fold(f32::MIN, f32::max); m.min_h = f.iter().map(|x| x.3).fold(f32::MAX, f32::min);
    m.max_step_w = f.windows(2).map(|p| (p[1].2 - p[0].2).abs()).fold(0.0, f32::max); m.max_dt = f.iter().skip(1).map(|x| x.1).fold(0.0, f32::max);
    let mut d: Vec<f32> = f.iter().skip(1).map(|x| x.1).collect(); d.sort_by(|a, b| a.total_cmp(b)); m.med_dt = d.get(d.len() / 2).copied().unwrap_or(0.0);
    m
}
/// minimo de quadros: depende do ritmo MEDIDO do monitor (mediana do dt), nao de um numero fixo: a 31 fps (VM) uma animacao de 188 ms tem ~6 quadros e esta certa
/// (o segmento inclui o tempo parado, entao conta-se pela duracao NOMINAL ativa da animacao: aba ~190 ms, demais >= 400 ms)
pub fn min_frames(k: Kind, m: &Metrics) -> usize { if m.med_dt > 1.0 { 4.max((0.5 * if k == Kind::Tab { 188.0 } else { 400.0 } / m.med_dt) as usize) } else { 8 } }
/// fracao maxima do curso por quadro: 30% a 60 Hz; sobe com o intervalo medido (a 31 fps cada quadro anda mais), nunca passa de 60%
pub fn max_step_frac(m: &Metrics) -> f32 { (0.30 * (m.med_dt / 16.0).max(1.0)).min(0.60) }
/// regras de regressao (abrir passa do alvo, fechar passa abaixo do alvo, nunca em um quadro so)
pub fn verdict(k: Kind, m: &Metrics) -> Result<(), String> {
    let travel = (m.w1 - m.w0).abs().max(1.0);
    if m.n < min_frames(k, m) { return Err(format!("so {} quadros (minimo {} no ritmo medido de {:.0} ms/quadro; animacao instantanea?)", m.n, min_frames(k, m), m.med_dt)); }
    if m.dur_ms < if k == Kind::Tab { 120.0 } else { 200.0 } { return Err(format!("duracao {:.0} ms < 200 ms", m.dur_ms)); }
    if m.max_step_w > max_step_frac(m) * travel { return Err(format!("salto de {:.0} px em um quadro (curso {:.0})", m.max_step_w, travel)); }
    match k {
        Kind::Open => if m.peak_w < m.w1 + 0.03 * travel { return Err(format!("sem overshoot de largura (pico {:.1}, alvo {:.1})", m.peak_w, m.w1)); },
        Kind::Close => if m.min_w > m.w1 - 0.03 * travel { return Err(format!("sem undershoot de largura (min {:.1}, alvo {:.1})", m.min_w, m.w1)); },
        Kind::Grow => {}
        Kind::Tab => { if m.peak_w > 103.0 { return Err(format!("troca de aba passa de 103% ({:.1})", m.peak_w)); } if (m.w1 - 100.0).abs() > 1.0 { return Err(format!("troca de aba nao termina em 100% ({:.1})", m.w1)); } if m.dur_ms > 700.0 { return Err(format!("troca de aba demora {:.0} ms", m.dur_ms)); } }
    }
    Ok(())
}
fn kind_of(name: &str) -> Kind { if name.starts_with("aba") { Kind::Tab } else if name.starts_with("abrir") { Kind::Open } else if name.starts_with("fechar") { Kind::Close } else { Kind::Grow } }

fn mark(name: &str) { *SEG.lock().unwrap_or_else(|e| e.into_inner()) = name.to_string(); }
/// chamado a cada 100 ms pelo timer da ilha
pub fn tick(a: &mut App) {
    let ms = a.started.elapsed().as_millis() as u64;
    #[derive(Clone, Copy)] enum Do { Force(St), Page(usize), Finish }
    const PLAN: &[(u64, &str, Do)] = &[
        (4500, "abrir-do-repouso", Do::Force(St::Open)), (6800, "fechar", Do::Force(St::Peek)),
        (9000, "abrir-renderer-quente", Do::Force(St::Open)), (11200, "fechar-2", Do::Force(St::Peek)),
        (16000, "abrir-do-repouso-2", Do::Force(St::Open)), (18300, "fechar-3", Do::Force(St::Peek)),
        (20500, "mini-notch", Do::Force(St::Line)), (22800, "mini-para-compacto", Do::Force(St::Peek)),
        (25000, "abrir-para-abas", Do::Force(St::Open)), (27500, "aba-lateral-ida", Do::Page(1)), (28800, "aba-lateral-volta", Do::Page(0)),
        (30500, "fim", Do::Finish),
    ];
    let i = STEP.load(Relaxed); let Some((at, name, st)) = PLAN.get(i) else { return };
    if ms < *at { return; }
    STEP.store(i + 1, Relaxed);
    match st {
        Do::Force(s) => { mark(name); a.force = Some(*s); a.evaluate_pub(); }
        Do::Page(p) => { mark(name); let t = if *p == a.page { (*p + 1) % crate::pages::N_PAGES } else { *p }; a.go_page(t); } // a pagina inicial depende do contexto (musica tocando abre no player): nunca 'trocar' para a mesma
        Do::Finish => finish(),
    }
}
fn finish() -> ! {
    let rec = REC.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let mut csv = String::from("segmento,t_ms,dt_ms,largura,altura\n"); for r in &rec { csv += &format!("{},{:.1},{:.2},{:.2},{:.2}\n", r.0, r.1, r.2, r.3, r.4); }
    let p = path().unwrap_or_default(); let _ = std::fs::write(&p, csv);
    let mut rep = format!("reduced_motion={} versao={} build={}
", crate::island::reduced_motion(), env!("CARGO_PKG_VERSION"), env!("EAVELY_BUILD")); let mut fail = 0;
    let mut names: Vec<String> = Vec::new(); for r in &rec { if !names.contains(&r.0) { names.push(r.0.clone()); } }
    for n in names {
        let f: Vec<(f32, f32, f32, f32)> = rec.iter().filter(|r| r.0 == n).map(|r| (r.1, r.2, r.3, r.4)).collect(); let f = if kind_of(&n) == Kind::Tab { let end = f.iter().position(|x| x.2 >= 99.9).map_or(f.len(), |i| i + 1); f[..end].to_vec() } else { f }; // aba: so ate chegar em 100% (depois so ha quadros de outros motivos)
        let m = metrics(&f);
        let v = verdict(kind_of(&n), &m);
        rep += &format!("{:24} quadros={:3} dur={:4.0}ms w {:6.1}->{:6.1} pico {:6.1} min {:6.1} | h {:5.1}->{:5.1} pico {:5.1} min {:5.1} | passo_max_w={:5.1} dt_max={:4.1}  {}\n", n, m.n, m.dur_ms, m.w0, m.w1, m.peak_w, m.min_w, m.h0, m.h1, m.peak_h, m.min_h, m.max_step_w, m.max_dt,
            match &v { Ok(()) => "PASS".to_string(), Err(e) => { fail += 1; format!("FAIL: {e}") } });
    }
    let _ = std::fs::write(p.with_extension("report.txt"), rep);
    crate::fast_exit(if fail == 0 { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::island::{Spring, SPRING_OPEN, SPRING_CLOSE};
    fn run(from: f32, to: f32, p: crate::island::SpringParams) -> Vec<(f32, f32, f32, f32)> {
        let mut s = Spring::new(from); s.go_p(to, p); let mut t = 0.0; let mut v = vec![(0.0, 13.3, from, 30.0)];
        while !s.settled() && t < 2000.0 { s.step(0.0133); t += 13.3; v.push((t, 13.3, s.pos, 30.0)); } v
    }
    #[test] fn molas_reais_passam_na_regressao() {
        let o = metrics(&run(150.0, 420.0, SPRING_OPEN)); assert!(verdict(Kind::Open, &o).is_ok(), "{o:?}");
        let c = metrics(&run(420.0, 150.0, SPRING_CLOSE)); assert!(verdict(Kind::Close, &c).is_ok(), "{c:?}");
    }
    #[test] fn aba_a_31_fps_com_7_quadros_passa_e_instantanea_reprova() {
        let w = [2.3f32, 28.0, 55.0, 80.0, 96.0, 101.0, 100.0];
        let tab: Vec<_> = w.iter().enumerate().map(|(i, v)| (i as f32 * 32.0, 32.0, *v, 0.0)).collect();
        assert_eq!(verdict(Kind::Tab, &metrics(&tab)), Ok(()), "7 quadros a 31 fps e uma troca de aba valida");
        let inst = [(0.0, 32.0, 2.3f32, 0.0), (32.0, 32.0, 100.0, 0.0), (64.0, 32.0, 100.0, 0.0), (96.0, 32.0, 100.0, 0.0), (128.0, 32.0, 100.0, 0.0), (160.0, 32.0, 100.0, 0.0), (192.0, 32.0, 100.0, 0.0)];
        assert!(verdict(Kind::Tab, &metrics(&inst)).is_err(), "salto de um quadro continua reprovando");
        let poucos = [(0.0, 16.0, 2.3f32, 0.0), (190.0, 190.0, 100.0, 0.0)]; assert!(verdict(Kind::Tab, &metrics(&poucos)).is_err());
    }
    #[test] fn animacao_instantanea_ou_linear_reprova() {
        let inst = metrics(&[(0.0, 13.0, 150.0, 30.0), (13.0, 13.0, 420.0, 140.0)]); assert!(verdict(Kind::Open, &inst).is_err());
        let lin: Vec<_> = (0..30).map(|i| (i as f32 * 13.0, 13.0, 150.0 + 9.0 * i as f32, 30.0)).collect(); assert!(verdict(Kind::Open, &metrics(&lin)).is_err(), "monotonica sem overshoot deve reprovar");
        let jump: Vec<_> = (0..30).map(|i| (i as f32 * 13.0, 13.0, if i < 10 { 150.0 } else { 440.0 - (i as f32 - 10.0) }, 30.0)).collect(); assert!(verdict(Kind::Open, &metrics(&jump)).is_err(), "salto de um quadro deve reprovar");
    }
}
