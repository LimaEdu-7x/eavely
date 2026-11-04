// Paginas Pomodoro e Shelf (nativas). Coordenadas da ilha em px logicos, conforme docs/DESIGN-NATIVO.md §4.2 e §4.3.
use std::time::Instant;
use windows::{core::{w, HSTRING}, Win32::{UI::{Shell::ShellExecuteW, WindowsAndMessaging::*}}};
use crate::{gfx::{rgba, Fmt, Rgba}, island::App, pages::{Act, R}, shelf, timer::{self, Mode, Status}};

const TEXT: Rgba = rgba(0xFFFFFF, 1.0);
const TEXT2: Rgba = rgba(0xFFFFFF, 0.64);
const TEXT3: Rgba = rgba(0xFFFFFF, 0.52);
const SURFACE: Rgba = rgba(0xFFFFFF, 0.08);
const SURFACE_H: Rgba = rgba(0xFFFFFF, 0.14);
const CORAL: Rgba = rgba(0xFF7A66, 1.0);
const OK: Rgba = rgba(0x7BE0A4, 1.0);
const DANGER: Rgba = rgba(0xFF6B6B, 1.0);
const AMBER: Rgba = rgba(0xFFC857, 1.0);
const ACCENT: Rgba = rgba(0x7CC4FF, 1.0);

fn pill(a: &mut App, hits: &mut Vec<(R, Act)>, rc: R, act: Act, off: f32, fill: Rgba) {
    let hov = a.hover == Some(act); let pr = a.pressed == Some(act);
    let c = if pr { rgba(0xFFFFFF, 0.20) } else if hov && fill == SURFACE { SURFACE_H } else { fill };
    a.gfx.rrect(rc.x, rc.y, rc.w, rc.h, rc.h / 2.0, c);
    hits.push((R { x: rc.x + off, ..rc }, act));
}

pub fn pomodoro(a: &mut App, hits: &mut Vec<(R, Act)>, off: f32) {
    let t = timer::timer_get().unwrap();
    let (running, paused, done) = (t.status == Status::Running, t.status == Status::Paused, t.status == Status::Done);
    let col = if done { OK } else { CORAL };
    // anel: parado = cheio; rodando = arco que esvazia a cada segundo (sem interpolar)
    a.gfx.ring(60.0, 80.0, 32.0, 5.0, SURFACE_H);
    let frac = if running || paused { t.remaining_sec as f32 / t.total_sec.max(1) as f32 } else { 1.0 };
    a.gfx.circle(60.0, 80.0, 38.0, (col.0, col.1, col.2, 0.06)); // brilho suave
    a.gfx.arc(60.0, 80.0, 32.0, 5.0, frac, col);
    a.gfx.icon("timer", 46.0, 66.0, 28.0, col, false);
    // tempo: editando (digitos) ou o do timer; pausado pisca 1 Hz
    let blink = paused && (a.started.elapsed().as_millis() / 500) % 2 == 1;
    if let Some(d) = &a.p.edit {
        // editando: duas caixas azuis MM : SS e a legenda de ajuda no lugar da legenda do preset (mock 12)
        let d = format!("{:0>4}", d);
        for (i, (x, s)) in [(112.0f32, &d[..2]), (190.0, &d[2..])].into_iter().enumerate() {
            a.gfx.rrect(x, 36.0, 64.0, 38.0, 10.0, rgba(0x7CC4FF, 0.10)); a.gfx.rrect_stroke(x + 0.5, 36.5, 63.0, 37.0, 10.0, 1.5, ACCENT);
            let w = a.gfx.measure(s, Fmt::Big); a.gfx.text(s, Fmt::Big, x + (64.0 - w) / 2.0, 38.0, w + 4.0, 34.0, TEXT, false); let _ = i;
        }
        a.gfx.text(":", Fmt::Big, 178.0, 37.0, 10.0, 34.0, TEXT2, false);
        a.gfx.text("Editando · Enter confirma · Esc cancela", Fmt::Small, 112.0, 76.0, 260.0, 14.0, TEXT2, false);
    } else {
        let txt = format!("{:02}:{:02}", t.remaining_sec / 60, t.remaining_sec % 60);
        a.gfx.text(&txt, Fmt::Big, 112.0, 38.0, 150.0, 36.0, if blink { TEXT2 } else { TEXT }, false);
        if !running { hits.push((R { x: 112.0 + off, y: 40.0, w: 120.0, h: 34.0 }, Act::TimerEdit)); }
    }
    let base = match t.mode { Mode::Focus => "Foco", Mode::Short => "Pausa curta", Mode::Long => "Pausa longa" };
    let mut leg = if done { "Fim".to_string() } else { base.to_string() };
    if !done && t.custom { leg = format!("Personalizado {:02}:{:02}", t.total_sec / 60, t.total_sec % 60); }
    if t.cycle > 0 { leg += &format!(" · ciclo {}", t.cycle); }
    if a.p.edit.is_none() { a.gfx.text(&leg, Fmt::Body, 112.0, 74.0, 170.0, 14.0, TEXT2, false); }
    // reiniciar
    let rb = R { x: 268.0, y: 50.0, w: 24.0, h: 24.0 };
    pill(a, hits, rb, Act::TimerReset, off, SURFACE); a.gfx.icon("rotate-ccw", 273.0, 55.0, 14.0, TEXT, false);
    // fileira
    let short_lbl = if t.mode == Mode::Short && !running && !paused { "Pausa longa" } else { "Pausa curta" };
    let b1 = R { x: 112.0, y: 92.0, w: 108.0, h: 26.0 };
    pill(a, hits, b1, Act::TimerMode(1), off, SURFACE); a.gfx.icon_label("coffee", short_lbl, Fmt::Body, b1.x, b1.y, b1.w, b1.h, TEXT, 14.0);
    let b2 = R { x: 226.0, y: 90.0, w: 62.0, h: 30.0 };
    // Iniciar so com tempo valido (> 0): editando, o valor digitado; parado, o total do timer
    let valid = match &a.p.edit { Some(d) => d.parse::<u32>().unwrap_or(0) > 0, None => t.total_sec > 0 || running || paused || done };
    let pf = if !valid { (col.0, col.1, col.2, 0.3) } else if a.pressed == Some(Act::TimerPlay) { (col.0 * 0.8, col.1 * 0.8, col.2 * 0.8, 1.0) } else { col };
    a.gfx.rrect(b2.x, b2.y, b2.w, b2.h, 15.0, pf); if valid { hits.push((R { x: b2.x + off, ..b2 }, Act::TimerPlay)); }
    if done { a.gfx.text("Novo", Fmt::BodyStrong, b2.x + 16.0, b2.y + 7.0, 40.0, 16.0, (1.0, 1.0, 1.0, 1.0), false); }
    else { a.gfx.icon(if running { "pause" } else { "play" }, b2.x + if running { 23.0 } else { 24.0 }, b2.y + 7.0, 16.0, (1.0, 1.0, 1.0, 1.0), true); }
    let b3 = R { x: 294.0, y: 92.0, w: 100.0, h: 26.0 };
    pill(a, hits, b3, Act::TimerMode(0), off, SURFACE); a.gfx.icon_label("target", "Foco", Fmt::Body, b3.x, b3.y, b3.w, b3.h, TEXT, 14.0);
}

/// SHELF nativa (DESIGN-NATIVO 14, estrutura desenhada internamente). Animacoes (14.4): arrasto 150 ms, degrade decai 1 s, card entra 300 ms (0,8->1),
/// excluir: encolhe 250 ms ease-in + vizinhos deslizam 250 ms (a partir de 50%), limpar tudo: voa ao botao 300 ms (cascata 30 ms), vazio entra 300 ms.
const SHRINK_MS: f32 = 250.0; const SLIDE_MS: f32 = 250.0; const FLY_MS: f32 = 300.0; const FLY_STAGGER_MS: u64 = 30; const GRAD_DECAY_MS: f32 = 1000.0;
const SEL_BLUE: Rgba = rgba(0x7CC4FF, 1.0);
fn since(t: Instant, now: Instant, ms: f32) -> f32 { if now < t { 0.0 } else { ((now - t).as_secs_f32() * 1000.0 / ms).min(1.0) } }
fn ease_in(x: f32) -> f32 { x * x }
/// "Selecionar"/"Limpar tudo" (cabecalho com itens) so aparecem com a lista nao vazia; saem em fade (200 ms) durante a cascata de limpar tudo
fn ellip(a: &App, s: &str, f: Fmt, maxw: f32) -> String {
    if a.gfx.measure(s, f) <= maxw { return s.to_string(); }
    let mut v: Vec<char> = s.chars().collect();
    while v.len() > 1 { v.pop(); let t: String = v.iter().collect::<String>() + "…"; if a.gfx.measure(&t, f) <= maxw { return t; } }
    "…".into()
}
fn human_pt(b: u64) -> String { if b < 1024 { format!("{b} B") } else if b < 1 << 20 { format!("{:.0} KB", b as f64 / 1024.0) } else if b < 1 << 30 { format!("{:.1} MB", b as f64 / 1048576.0).replace('.', ",") } else { format!("{:.1} GB", b as f64 / 1073741824.0).replace('.', ",") } }
impl crate::pages::PageState {
    /// ha animacao da Shelf em andamento? (pede quadros no ritmo do monitor)
    pub fn shelf_busy(&self) -> bool {
        let t = |o: Option<Instant>, ms: u128| o.is_some_and(|t| t.elapsed().as_millis() < ms);
        !self.shelf_del.is_empty() || t(self.items_at, 400) || t(self.empty_at, 400) || t(self.drag_end, 1100) || self.drag_t.is_some() || self.shelf_new.iter().any(|(_, s)| s.elapsed().as_millis() < 350)
    }
}
/// opacidade do cabecalho direito (bateria/data/hora): cheio com a lista vazia; some quando ha itens (e volta quando esvazia)
pub fn shelf_header_alpha(a: &App, now: Instant) -> f32 {
    if a.p.shelf.is_empty() && a.p.shelf_del.is_empty() { a.p.empty_at.map_or(1.0, |t| since(t, now, 300.0)) }
    else { 1.0 - a.p.items_at.map_or(1.0, |t| since(t, now, 200.0)) }
}
/// fundo: degrade azul-marinho -> roxo ao segurar o arquivo (entra em 200 ms) e decai em 1 s depois (soltar ou sair)
pub fn shelf_bg(a: &mut App, now: Instant) {
    let g = if a.drag_over { a.p.drag_t.map_or(1.0, |t| crate::island::ease(since(t, now, 200.0))) } else if let Some(e) = a.p.drag_end { 1.0 - crate::island::ease(since(e, now, GRAD_DECAY_MS)) } else { 0.0 };
    // brilho ATRAS DO ARQUIVO: halo radial que segue o cursor (nao ilumina o notch inteiro); desligavel (Economia)
    if g > 0.01 && crate::settings::flag(crate::settings::F_SHELF_GLOW) {
        let (lx, ly) = a.cursor_local(); let (x, y) = (lx.clamp(40.0, 380.0), ly.clamp(40.0, 112.0));
        a.gfx.vignette(x, y, 96.0, (0.36, 0.42, 1.0, 0.55 * g)); a.gfx.vignette(x, y, 48.0, (0.62, 0.55, 1.0, 0.40 * g));
    }
}

pub fn shelf(a: &mut App, hits: &mut Vec<(R, Act)>, off: f32, now: Instant) {
    // exclusoes concluidas: remove so a REFERENCIA do Shelf (shelf.json); nunca toca nos arquivos em disco
    let slide_end = |s: &Instant| *s + std::time::Duration::from_millis((SHRINK_MS * 0.5 + SLIDE_MS) as u64);
    let fend = |s: &Instant| *s + std::time::Duration::from_millis(FLY_MS as u64);
    let single_done: Vec<String> = a.p.shelf_del.iter().filter(|e| e.2 && now >= slide_end(&e.1)).map(|e| e.0.clone()).collect();
    for id in single_done { shelf::remove_native(&id); a.p.shelf_del.retain(|e| e.0 != id); }
    if a.p.shelf_del.iter().any(|e| !e.2) && a.p.shelf_del.iter().filter(|e| !e.2).all(|e| now >= fend(&e.1)) {
        let ids: Vec<String> = a.p.shelf_del.iter().filter(|e| !e.2).map(|e| e.0.clone()).collect();
        if ids.len() == a.p.shelf.len() { shelf::clear_native(); } else { for id in &ids { shelf::remove_native(id); } }
        a.p.shelf_del.retain(|e| e.2);
    }
    let g = shelf::GEN.load(std::sync::atomic::Ordering::Relaxed);
    if a.p.shelf_gen != g {
        a.p.shelf = shelf::list_native(); a.p.shelf_gen = g;
        for it in &a.p.shelf { if !a.p.shelf_ids.contains(&it.id) && a.p.shelf_n_known() { a.p.shelf_new.push((it.id.clone(), now)); } }
        a.p.shelf_ids = a.p.shelf.iter().map(|i| i.id.clone()).collect(); a.p.shelf_new.retain(|(_, s)| s.elapsed().as_millis() < 400);
    }
    let n = a.p.shelf.len();
    if n == 0 && a.p.shelf_n > 0 { a.p.empty_at = Some(now); a.p.select = false; a.p.sel.clear(); a.p.shelf_sx = 0.0; }
    if n > 0 && a.p.shelf_n == 0 { a.p.items_at = Some(now); }
    a.p.shelf_n = n;
    let max_sx = (n as f32 * 66.0 - 330.0).max(0.0); a.p.shelf_sx = a.p.shelf_sx.clamp(0.0, max_sx);
    // arrasto
    if a.drag_over { if a.p.drag_t.is_none() { a.p.drag_t = Some(now); a.p.drag_end = None; } } else if a.p.drag_t.take().is_some() { a.p.drag_end = Some(now); }
    let enter = a.p.drag_t.map_or(0.0, |t| crate::island::ease(since(t, now, 150.0)));      // caixa -> modo "solte" (150 ms)
    let empty_a = a.p.empty_at.map_or(1.0, |t| since(t, now, 300.0));                   // vazio entra apos limpar (300 ms)
    let clearing = a.p.shelf_del.iter().filter(|e| !e.2).map(|e| e.1).min();            // inicio da cascata de limpar tudo
    let busy_a = clearing.map_or(1.0, |t| 1.0 - since(t, now, 200.0));                  // botoes saem em 200 ms
    let in_a = a.p.items_at.map_or(1.0, |t| since(t, now, 300.0));                      // botoes entram em 300 ms
    let btn_a = (busy_a * in_a).clamp(0.0, 1.0);
    if n > 0 && btn_a > 0.01 {
        a.gfx.layer(btn_a);
        let sel_on = a.p.select; let sel = Act::ShelfSelect;
        let rb = if sel_on { R { x: 74.0, y: 7.0, w: 74.0, h: 20.0 } } else { R { x: 68.0, y: 7.0, w: 68.0, h: 20.0 } };
        let hv = a.hover == Some(sel); a.gfx.rrect(rb.x, rb.y, rb.w, rb.h, 10.0, if sel_on { rgba(0x7CC4FF, 0.20) } else if hv { SURFACE_H } else { SURFACE });
        let lb = if sel_on { "Concluído" } else { "Selecionar" }; let lw = a.gfx.measure(lb, Fmt::Small);
        a.gfx.text(lb, Fmt::Small, rb.x + (rb.w - lw) / 2.0, rb.y + 0.5, lw + 4.0, 16.0, if sel_on { SEL_BLUE } else { TEXT }, false); hits.push((R { x: rb.x + off, ..rb }, sel));
        let act = Act::ShelfClear; let nsel = a.p.sel.len();
        let (lbl, al) = if sel_on { ("Remover", if nsel == 0 { 0.4 } else { 1.0 }) } else { ("Limpar tudo", 1.0) };
        let lw2 = a.gfx.measure(lbl, Fmt::Small);
        let col = if a.hover == Some(act) && al >= 1.0 { rgba(0xFF9090, 1.0) } else { DANGER };
        a.gfx.layer(al); a.gfx.text(lbl, Fmt::Small, 372.0 - lw2 - 2.0, 9.0, lw2 + 4.0, 16.0, col, true); a.gfx.unlayer();
        if al >= 1.0 { hits.push((R { x: 372.0 - lw2 - 8.0 + off, y: 5.0, w: lw2 + 14.0, h: 24.0 }, act)); }
        a.gfx.unlayer();
    }
    // ---- vazio / segurando ----
    let show_empty = n == 0 && a.p.shelf_del.is_empty();
    if show_empty || (a.drag_over && n == 0) {
        a.gfx.layer(if a.drag_over { 1.0 } else { empty_a });
        let k = enter; let bc = rgba(0xFFFFFF, 0.28 + (0.85 - 0.28) * k);
        if k > 0.0 { a.gfx.rrect(39.0, 29.0, 342.0, 96.0, 16.0, rgba(0xFFFFFF, 0.05 * k)); }
        a.gfx.rrect_dashed(39.75, 29.75, 340.5, 94.5, 15.25, 1.5, bc, 3.0, 2.0);
        let ic = if k > 0.5 { TEXT } else { TEXT2 };
        a.gfx.icon("inbox", 39.0 + 38.0, 29.0 + 37.0, 22.0, ic, false);
        // texto "Solte arquivos aqui" (+ subtitulo) <-> "Solte para adicionar"
        a.gfx.layer(1.0 - k); a.gfx.text("Solte arquivos aqui", Fmt::BodyStrong, 39.0 + 76.0, 29.0 + 28.0, 250.0, 18.0, TEXT, false);
        a.gfx.text("Arraste para fora quando quiser ·", Fmt::Small, 39.0 + 76.0, 29.0 + 47.0, 250.0, 15.0, TEXT2, false); a.gfx.text("clique para abrir ou copiar", Fmt::Small, 39.0 + 76.0, 29.0 + 62.0, 250.0, 15.0, TEXT2, false); a.gfx.unlayer();
        if k > 0.0 { a.gfx.layer(k); a.gfx.text("Solte para adicionar", Fmt::BodyStrong, 39.0 + 76.0, 29.0 + 39.0, 250.0, 18.0, TEXT, false); a.gfx.unlayer(); }
        a.gfx.unlayer();
        if n == 0 { return; }
    }
    // ---- cards ----
    a.gfx.clip(16.0, 30.0, 388.0, 86.0);
    let items = a.p.shelf.clone(); let sx = a.p.shelf_sx;
    let mut flying: Vec<(f32, f32, f32, bool)> = Vec::new();
    // posicao deslizada: cada exclusao (shrink) anterior empurra os vizinhos 66 px
    let slide = |a: &App, upto: usize, items: &[shelf::ShelfItem]| -> f32 {
        let mut s = 0.0; for p in items.iter().take(upto) { if let Some(d) = a.p.shelf_del.iter().find(|e| e.2 && e.0 == p.id) { let t = since(d.1 + std::time::Duration::from_millis((SHRINK_MS * 0.5) as u64), now, SLIDE_MS); s -= 66.0 * crate::island::ease(t); } } s };
    for (i, it) in items.iter().enumerate() {
        let x = 45.0 + i as f32 * 66.0 + slide(a, i, &items) - sx; let y = 36.0;
        if x > 404.0 || x + 58.0 < 16.0 { continue; }
        let del = a.p.shelf_del.iter().find(|e| e.0 == it.id).cloned();
        if let Some((_, st0, single)) = del {
            if !single { flying.push((x, y, since(st0, now, FLY_MS), it.is_dir)); continue; }
            let t = ease_in(since(st0, now, SHRINK_MS)); let k = 1.0 - 0.65 * t; let (w, h) = (58.0 * k, 74.0 * k);
            a.gfx.layer(1.0 - t); a.gfx.rrect(x + (58.0 - w) / 2.0, y + (74.0 - h) / 2.0, w, h, 10.0 * k, SURFACE);
            a.gfx.icon(if it.is_dir { "folder" } else { "file" }, x + 29.0 - 13.0 * k, y + 13.0 + 37.0 * (1.0 - k) - 0.0, 26.0 * k, AMBER, false);
            a.gfx.unlayer(); continue;
        }
        // entrada do card recem-solto: escala 0,8 -> 1 e fade (300 ms)
        let born = a.p.shelf_new.iter().find(|(id, _)| *id == it.id).map(|(_, s)| crate::island::ease(since(*s, now, 300.0))).unwrap_or(1.0);
        let (cx, cy) = (x + 29.0, y + 37.0); let sc = 0.8 + 0.2 * born;
        let mat = if born < 1.0 { a.gfx.layer(born); Some(a.gfx.scale_about(cx, cy, sc)) } else { None };
        let act = Act::ShelfOpen(i as u32); let rm = Act::ShelfRemove(i as u32);
        let hov = a.hover == Some(act) || a.hover == Some(rm); let on = a.p.select && a.p.sel.contains(&i);
        if hov && crate::settings::flag(crate::settings::F_SHELF_GLOW) { a.gfx.vignette(x + 29.0, y + 37.0, 52.0, (0.45, 0.52, 1.0, 0.30)); } // brilho atras do item sob o mouse
        a.gfx.rrect(x, y, 58.0, 74.0, 10.0, if hov || on { SURFACE_H } else { SURFACE });
        let th = if crate::settings::flag(crate::settings::F_THUMBS) { shelf::thumb(&it.id, &it.path) } else { None };
        if let Some(t) = th { let key = { use std::hash::{Hash, Hasher}; let mut h = std::collections::hash_map::DefaultHasher::new(); it.id.hash(&mut h); h.finish() };
            if t.icon { a.gfx.thumb(key, &t.px, t.w, t.h, x + 13.0, y + 6.0, 32.0, 32.0, 0.0, 1.0); } // icone real do shell (tipo do arquivo/pasta)
            else { a.gfx.thumb(key, &t.px, t.w, t.h, x + 11.0, y + 12.0, 36.0, 26.0, 6.0, 1.0); a.gfx.rrect_stroke(x + 11.5, y + 12.5, 35.0, 25.0, 5.5, 1.0, rgba(0xFFFFFF, 0.18)); }
        } else { a.gfx.icon(if it.is_dir { "folder" } else { "file" }, x + 16.0, y + 13.0, 26.0, AMBER, false); }
        let nm = ellip(a, &it.name, Fmt::BodyStrong, 50.0); let nw = a.gfx.measure(&nm, Fmt::BodyStrong);
        a.gfx.text(&nm, Fmt::BodyStrong, x + 4.0 + (50.0 - nw) / 2.0, y + 42.0, nw + 4.0, 14.0, TEXT, false);
        let sz = if it.is_dir { "pasta".to_string() } else { it.size_bytes.map(human_pt).unwrap_or_else(|| "—".into()) }; let sw = a.gfx.measure(&sz, Fmt::Small);
        a.gfx.text(&sz, Fmt::Small, x + 4.0 + (50.0 - sw) / 2.0, y + 57.0, sw + 4.0, 12.0, TEXT3, false);
        hits.push((R { x: x + off, y, w: 58.0, h: 74.0 }, act));
        if a.p.select {
            if on { a.gfx.rrect_stroke(x + 0.75, y + 0.75, 56.5, 72.5, 9.25, 1.5, SEL_BLUE); a.gfx.circle(x + 13.0, y + 13.0, 8.0, SEL_BLUE); a.gfx.icon("check", x + 7.5, y + 7.5, 11.0, rgba(0x06223A, 1.0), false); }
            else { a.gfx.ring(x + 13.0, y + 13.0, 7.25, 1.5, TEXT3); }
        } else if hov {
            a.gfx.circle(x + 50.5, y + 11.5, 7.5, if a.hover == Some(rm) { rgba(0xFFFFFF, 0.34) } else { rgba(0xFFFFFF, 0.22) }); a.gfx.icon("x", x + 46.0, y + 7.0, 9.0, TEXT, false);
            hits.push((R { x: x + 39.0 + off, y: y - 2.0, w: 24.0, h: 24.0 }, rm));
        }
        if let Some(m) = mat { a.gfx.set_matrix(m); a.gfx.unlayer(); }
    }
    // degrade na borda direita: indica rolagem horizontal
    if max_sx - sx > 1.0 { a.gfx.grad(352.0, 36.0, 30.0, 74.0, true, &[(0.0, rgba(0x000000, 0.0)), (1.0, rgba(0x000000, 1.0))], 1.0); }
    a.gfx.unclip();
    // limpar tudo: cards encolhem e voam ao botao (destino ~ 320,4), escalonados de 30 ms (sem recorte)
    for (x, y, t, dir) in flying {
        let e = ease_in(t); let (cx, cy) = (x + 29.0 + (320.0 - x - 29.0) * e, y + 37.0 + (4.0 - y - 37.0) * e);
        let k = 1.0 - 0.78 * e; let (w, h) = (58.0 * k, 74.0 * k);
        a.gfx.layer(1.0 - 0.6 * e); a.gfx.rrect(cx - w / 2.0, cy - h / 2.0, w, h, 10.0 * k, SURFACE_H);
        a.gfx.icon(if dir { "folder" } else { "file" }, cx - 13.0 * k, cy - 24.0 * k, 26.0 * k, AMBER, false); a.gfx.unlayer();
    }
}
impl crate::pages::PageState { fn shelf_n_known(&self) -> bool { !self.shelf_ids.is_empty() || self.shelf_n > 0 || self.shelf_gen > 0 } }
fn human(b: u64) -> String { if b < 1024 { format!("{b} B") } else if b < 1 << 20 { format!("{:.0} KB", b as f64 / 1024.0) } else if b < 1 << 30 { format!("{:.1} MB", b as f64 / 1048576.0) } else { format!("{:.1} GB", b as f64 / 1073741824.0) } }

impl crate::pages::PageState { pub fn shelf_scroll(&self) -> f32 { 0.0 } }

pub fn click(a: &mut App, act: Act) {
    match act {
        Act::TimerMode(m) => { let mode = if m == 0 { Mode::Focus } else { Mode::Short };
            timer::ui(|t, _| { let st = t.state(0).status; if matches!(st, Status::Idle | Status::Done) {
                // segunda pressao em "Pausa curta" alterna curta -> longa
                let m2 = if mode == Mode::Short && t.state(0).mode == Mode::Short { Mode::Long } else { mode }; t.set_mode(m2); } }); }
        Act::TimerPlay => { // editando: confirma o tempo digitado (so se > 0) e inicia
            if let Some(d) = a.p.edit.clone() { let d = format!("{:0>4}", d); let (m, s): (u32, u32) = (d[..2].parse().unwrap_or(0), d[2..].parse().unwrap_or(0)); if m * 60 + s == 0 { return; } a.p.edit = None; let sec = (m * 60 + s).clamp(1, 99 * 60 + 59); timer::ui(move |t, _| { t.set_custom(sec); }); }
            timer::toggle_play(); }
        Act::TimerReset => timer::ui(|t, _| t.reset()),
        Act::TimerEdit => { a.p.edit = Some(String::new()); unsafe { let _ = SetForegroundWindow(a.hwnd); let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(a.hwnd)); } }
        Act::ShelfSelect => { a.p.select = !a.p.select; a.p.sel.clear(); } // entra com NENHUM marcado
        Act::ShelfClear => { // so referencias do Shelf (shelf.json); arquivos em disco nunca sao tocados
            let now = Instant::now();
            if a.p.select { // "Remover": so os marcados (nenhum marcado = nada); encolhem juntos
                let sel = a.p.sel.clone(); for i in sel { if let Some(it) = a.p.shelf.get(i) { if !a.p.shelf_del.iter().any(|e| e.0 == it.id) { a.p.shelf_del.push((it.id.clone(), now, true)); } } }
                a.p.sel.clear(); a.p.select = false;
            } else { // "Limpar tudo": voam ao botao, 30 ms entre cards; a remocao real e em lote ao fim
                for (k, it) in a.p.shelf.clone().iter().enumerate() { if !a.p.shelf_del.iter().any(|e| e.0 == it.id) { a.p.shelf_del.push((it.id.clone(), now + std::time::Duration::from_millis(FLY_STAGGER_MS * k as u64), false)); } }
            } }
        Act::ShelfRemove(i) => { if let Some(it) = a.p.shelf.get(i as usize) { if !a.p.shelf_del.iter().any(|e| e.0 == it.id) { a.p.shelf_del.push((it.id.clone(), Instant::now(), true)); } } }
        Act::ShelfOpen(i) if a.p.select => { let i = i as usize; if let Some(p) = a.p.sel.iter().position(|x| *x == i) { a.p.sel.remove(p); } else { a.p.sel.push(i); } }
        Act::ShelfOpen(i) => { if let Some(it) = a.p.shelf.get(i as usize) { unsafe { ShellExecuteW(None, w!("open"), &HSTRING::from(it.path.as_str()), None, None, SW_SHOWNORMAL); } } }
        _ => {}
    }
}

/// Teclado da edicao MM:SS (digitos entram da direita, como relogio de cozinha).
pub fn key(a: &mut App, vk: u32, ch: Option<char>) {
    if crate::tasks::key(a, vk, ch) { return; }
    let Some(buf) = a.p.edit.as_mut() else { return };
    match (vk, ch) {
        (0x1B, _) => a.p.edit = None, // Esc
        (0x0D, _) => { let d = format!("{:0>4}", buf); let (m, s): (u32, u32) = (d[..2].parse().unwrap_or(0), d[2..].parse().unwrap_or(0)); if m * 60 + s == 0 { return; } a.p.edit = None; // 00:00 nao e aceito
            let sec = (m * 60 + s).clamp(1, 99 * 60 + 59); timer::ui(move |t, _| { t.set_custom(sec); }); }
        (0x08, _) => { buf.pop(); }
        (_, Some(c)) if c.is_ascii_digit() && buf.len() < 4 => buf.push(c),
        _ => {}
    }
    a.dirty = true;
}
