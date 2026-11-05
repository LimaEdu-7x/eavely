// Slots (v0.2.0): widgets pequenos 202x52 da pagina Mascote. Usam o MESMO estado, os MESMOS Act e os mesmos helpers das paginas cheias (card_art, draw_cover,
// button_bg, ui_pos): nada e duplicado. Coordenadas de projeto (a escala k vem da transformacao); os hits saem ja deslocados por `off`.
use std::time::Instant;
use crate::{gfx::{rgba, Fmt, Rgba}, island::App, media, pages::{self, hov, button_bg, ui_pos, draw_cover, timer_running, r, Act, R, R2}, pages2, shelf, tasks, timer};

const TEXT: Rgba = rgba(0xFFFFFF, 1.0);
const TEXT2: Rgba = rgba(0xFFFFFF, 0.64);
const TEXT3: Rgba = rgba(0xFFFFFF, 0.52);
const SURFACE: Rgba = rgba(0xFFFFFF, 0.08);
const SURFACE_H: Rgba = rgba(0xFFFFFF, 0.14);
const BLUE: Rgba = rgba(0x7CC4FF, 1.0);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)] pub enum Widget { #[default] Empty, Shelf, Player, Tasks, Timer, Clock, Metrics }
impl Widget {
    pub const ALL: [Widget; 7] = [Widget::Shelf, Widget::Player, Widget::Tasks, Widget::Timer, Widget::Clock, Widget::Metrics, Widget::Empty];
    pub fn parse(s: &str) -> Widget { match s { "shelf" => Widget::Shelf, "player" => Widget::Player, "tasks" => Widget::Tasks, "timer" => Widget::Timer, "clock" => Widget::Clock, "metrics" => Widget::Metrics, _ => Widget::Empty } }
    pub fn key(self) -> &'static str { match self { Widget::Shelf => "shelf", Widget::Player => "player", Widget::Tasks => "tasks", Widget::Timer => "timer", Widget::Clock => "clock", Widget::Metrics => "metrics", Widget::Empty => "none" } }
    pub fn label(self) -> &'static str { match self { Widget::Shelf => "Arquivos", Widget::Player => "Player mini", Widget::Tasks => "Tarefas", Widget::Timer => "Timer", Widget::Clock => "Relógio / Data", Widget::Metrics => "Métricas do PC", Widget::Empty => "Nenhum" } }
}
pub const SLOT_W: f32 = 190.0; pub const SLOT_H: f32 = 52.0; // termina em 396: as setas de pagina (402..414) ficam fora do slot
/// slots efetivos (retangulos) a partir das opcoes: a mesma opcao nao pode estar nos dois (o 2o vira Nenhum); com um Nenhum o outro cresce para 202x110
pub fn layout(top: &str, bottom: &str) -> [(Widget, R); 2] {
    let (t, mut b) = (Widget::parse(top), Widget::parse(bottom)); if t != Widget::Empty && t == b { b = Widget::Empty; }
    match (t, b) {
        (Widget::Empty, Widget::Empty) => [(Widget::Empty, r(206.0, 10.0, SLOT_W, SLOT_H)), (Widget::Empty, r(206.0, 68.0, SLOT_W, SLOT_H))],
        (t, Widget::Empty) => [(t, r(206.0, 10.0, SLOT_W, 110.0)), (Widget::Empty, r(206.0, 68.0, SLOT_W, SLOT_H))],
        (Widget::Empty, b) => [(Widget::Empty, r(206.0, 10.0, SLOT_W, SLOT_H)), (b, r(206.0, 10.0, SLOT_W, 110.0))],
        (t, b) => [(t, r(206.0, 10.0, SLOT_W, SLOT_H)), (b, r(206.0, 68.0, SLOT_W, SLOT_H))],
    }
}
pub fn draw_slot(a: &mut App, w: Widget, s: R, hits: &mut Vec<(R, Act)>, off: f32, now: Instant) {
    if w == Widget::Empty { return; }
    // slot alto (202x110): o widget de 52 px fica centrado no cartao
    let mut sr = s; if s.h > SLOT_H + 1.0 { sr = r(s.x, s.y + (s.h - SLOT_H) / 2.0, s.w, SLOT_H); a.gfx.rrect(s.x, s.y, s.w, s.h, 14.0, SURFACE); }
    a.gfx.clip(s.x, s.y, s.w, s.h); // nada vaza do slot, em nenhum tamanho
    match w {
        Widget::Shelf => shelf_mini(a, sr, hits, off, now), Widget::Player => player_mini(a, sr, hits, off, now), Widget::Tasks => tasks_mini(a, sr, hits, off),
        Widget::Timer => timer_mini(a, sr, hits, off), Widget::Clock => clock_mini(a, sr), Widget::Metrics => metrics_mini(a, sr), Widget::Empty => {}
    }
    a.gfx.unclip();
}
fn card(a: &mut App, s: R) { a.gfx.rrect(s.x, s.y, s.w, s.h, 14.0, SURFACE); }

/// Arquivos: 2 cards 56x40 (arte do shell + inicio do nome e extensao em UMA linha) + alvo de soltar (ou +N); vazio/arrastando = caixa tracejada.
/// Mesma selecao (Ctrl/Shift), Delete/botao do meio e saida animada da pagina Arquivos; nome completo na legenda.
pub fn shelf_mini(a: &mut App, s: R, hits: &mut Vec<(R, Act)>, off: f32, now: Instant) {
    pages2::shelf_tick(a, now);
    let items = a.p.shelf.clone(); let drag = a.drag_over;
    card(a, s);
    if items.is_empty() || drag {
        let col = if drag { BLUE } else { rgba(0xFFFFFF, 0.28) };
        a.gfx.rrect_dashed(s.x + 6.0, s.y + 6.0, s.w - 12.0, s.h - 12.0, 10.0, 1.5, col, 3.0, 2.0);
        a.gfx.icon("inbox", s.x + 16.0, s.y + s.h / 2.0 - 10.0, 20.0, if drag { BLUE } else { TEXT2 }, false);
        a.gfx.text(if drag { "Solte para adicionar" } else { "Solte arquivos aqui" }, Fmt::BodyStrong, s.x + 44.0, s.y + 11.0, s.w - 52.0, 16.0, if drag { BLUE } else { TEXT }, false);
        a.gfx.text(if drag { "vai para a Shelf" } else { "ou arraste direto na ilha" }, Fmt::Small, s.x + 44.0, s.y + 28.0, s.w - 52.0, 14.0, TEXT3, false);
        return;
    }
    hits.push((R { x: s.x + off, ..s }, Act::ShelfPan)); // clicar no vazio do slot limpa a selecao
    let (tw, th, step) = (56.0f32, 40.0f32, 60.0f32); let max = ((s.w - 6.0 - step) / step) as usize; // 2 cards + alvo de soltar (ou +N): o nome cabe em uma linha (Relatori….pdf)
    for (i, it) in items.iter().take(max).enumerate() {
        let (x, y) = (s.x + 6.0 + i as f32 * step, s.y + 6.0); let act = Act::ShelfOpen(i as u32);
        if let Some(p) = pages2::del_progress(a, &it.id, now) { // saindo (lixeira): encolhe e some
            let e = p * p; let k = 1.0 - 0.6 * e; let (w, h) = (tw * k, th * k); a.gfx.layer(1.0 - e);
            a.gfx.rrect(x + (tw - w) / 2.0, y + (th - h) / 2.0, w, h, 10.0 * k, SURFACE_H); pages2::card_art(a, it, x + tw / 2.0, y + th / 2.0 + 1.0, 0.62 * k); a.gfx.unlayer(); continue;
        }
        let hv = a.hover == Some(act); let on = a.p.sel.contains(&i);
        a.gfx.rrect(x, y, tw, th, 10.0, if hv || on { SURFACE_H } else { SURFACE });
        pages2::card_art(a, it, x + tw / 2.0, y + 19.0, 0.62);
        let (nm, cut) = pages2::fit_name(a, &it.name, Fmt::Tiny, tw - 4.0); let nw = a.gfx.measure(&nm, Fmt::Tiny);
        a.gfx.text(&nm, Fmt::Tiny, x + tw / 2.0 - nw / 2.0, y + 24.5, nw + 4.0, 11.0, TEXT2, false);
        if on { a.gfx.rrect_stroke(x + 0.75, y + 0.75, tw - 1.5, th - 1.5, 9.25, 1.5, BLUE); }
        if cut && hv { a.p.slot_tip = Some((it.name.clone(), x + tw / 2.0, if s.y > 40.0 { y - 20.0 } else { y + th + 3.0 })); }
        hits.push((R { x: x + off, y, w: tw, h: th }, act));
    }
    let dx = s.x + 6.0 + items.len().min(max) as f32 * step;
    if items.len() > max { // sobra: "+N" abre a pagina Arquivos
        let rc = R { x: dx, y: s.y + 6.0, w: tw, h: th }; let hv = a.hover == Some(Act::Page(pages::P_SHELF as u8));
        a.gfx.rrect(rc.x, rc.y, rc.w, rc.h, 10.0, if hv { SURFACE_H } else { SURFACE }); let t = format!("+{}", items.len() - max); let w = a.gfx.measure(&t, Fmt::BodyStrong);
        a.gfx.text(&t, Fmt::BodyStrong, rc.x + (rc.w - w) / 2.0, rc.y + 12.0, w + 4.0, 16.0, TEXT2, false); hits.push((R { x: rc.x + off, ..rc }, Act::Page(pages::P_SHELF as u8)));
    } else if dx + tw <= s.x + s.w {
        a.gfx.rrect_dashed(dx + 0.75, s.y + 6.75, tw - 1.5, th - 1.5, 10.0, 1.5, rgba(0xFFFFFF, 0.28), 3.0, 2.0); a.gfx.icon("plus", dx + tw / 2.0 - 6.0, s.y + 12.0, 12.0, TEXT2, false);
        let t = "soltar"; let tw2 = a.gfx.measure(t, Fmt::Tiny); a.gfx.text(t, Fmt::Tiny, dx + tw / 2.0 - tw2 / 2.0, s.y + 28.0, tw2 + 4.0, 11.0, TEXT3, false);
    }
}

/// Player mini (sem letras nem volume): capa 40, titulo/artista, anterior/tocar/proximo e barra fina (mesmos Act e estado da Musica)
pub fn player_mini(a: &mut App, s: R, hits: &mut Vec<(R, Act)>, off: f32, now: Instant) {
    let v = media::view(); let empty = v.title.is_empty() && v.status == "stopped";
    card(a, s);
    let cs = s.h - 12.0; draw_cover(a, now, s.x + 6.0, s.y + 6.0, cs, 8.0, false);
    let tx = s.x + cs + 12.0; let ctl = s.x + s.w - 74.0; let tw = (ctl - tx - 2.0).max(20.0);
    let dis = if empty { 0.4 } else { 1.0 };
    a.gfx.text(if empty { "Sem música" } else { &v.title }, Fmt::BodyStrong, tx, s.y + 8.0, tw, 15.0, if empty { TEXT2 } else { TEXT }, false);
    a.gfx.text(if empty { "Abra um player" } else { &v.artist }, Fmt::Small, tx, s.y + 24.0, tw, 13.0, TEXT3, false);
    let (p0, pp, n0) = (r(ctl, s.y + 9.0, 20.0, 20.0), r(ctl + 22.0, s.y + 6.0, 26.0, 26.0), r(ctl + 50.0, s.y + 9.0, 20.0, 20.0));
    if !empty { button_bg(a, hits, p0, Act::Prev, off, 11.0); } a.gfx.icon("skip-back", p0.x + 4.0, p0.y + 4.0, 12.0, (1.0, 1.0, 1.0, dis), true);
    let pc = if a.pressed == Some(Act::Play) { rgba(0xCFCFCF, 1.0) } else if !empty && hov(a, Act::Play) { rgba(0xE6E6E6, 1.0) } else { rgba(0xFFFFFF, dis) };
    a.gfx.rrect(pp.x, pp.y, pp.w, pp.h, 13.0, pc); a.gfx.icon(if v.status == "playing" { "pause" } else { "play" }, pp.x + 6.0, pp.y + 6.0, 14.0, (0.0, 0.0, 0.0, 1.0), true);
    if !empty { hits.push((R { x: pp.x + off, ..pp }, Act::Play)); button_bg(a, hits, n0, Act::Next, off, 11.0); }
    a.gfx.icon("skip-forward", n0.x + 4.0, n0.y + 4.0, 12.0, (1.0, 1.0, 1.0, dis), true);
    // progresso fino (seek otimista, igual a Musica)
    let pos = ui_pos(a, &v, now); let frac = if empty { 0.0 } else { (pos as f32 / v.dur_ms.max(1) as f32).clamp(0.0, 1.0) };
    let big = hov(a, Act::Seek(R2(720, 2760))) || a.seek_drag; let bh = if big { 4.0 } else { 3.0 };
    let (sx, sw, sy) = (tx, s.x + s.w - 10.0 - tx, s.y + s.h - 10.0);
    a.gfx.rrect(sx, sy - bh / 2.0, sw, bh, 1.5, SURFACE_H); a.gfx.rrect(sx, sy - bh / 2.0, (sw * frac).max(0.0), bh, 1.5, TEXT);
    if !empty { hits.push((r(sx + off, sy - 7.0, sw, 14.0), Act::Seek(R2(720, 2760)))); }
}

/// Tarefas: proxima pendente + concluidas hoje/total (clique abre a aba Tarefas)
pub fn tasks_mini(a: &mut App, s: R, hits: &mut Vec<(R, Act)>, off: f32) {
    card(a, s); let list = tasks::list(); let today = tasks::now_local().date_str(); let (done, total) = tasks::counts(&list, &today);
    a.gfx.icon("list-checks", s.x + 12.0, s.y + 8.0, 18.0, rgba(0x5EE0B0, 1.0), false);
    let next = list.iter().find(|t| !t.done);
    a.gfx.text(next.map_or("Sem tarefas pendentes", |t| t.text.as_str()), Fmt::BodyStrong, s.x + 40.0, s.y + 8.0, s.w - 40.0 - 52.0, 16.0, TEXT, false);
    a.gfx.text(&format!("{done} de {total} hoje"), Fmt::Small, s.x + 40.0, s.y + 25.0, s.w - 92.0, 13.0, TEXT3, false);
    if total > 0 { a.gfx.rrect(s.x + 12.0, s.y + s.h - 9.0, s.w - 24.0, 3.0, 1.5, SURFACE_H); a.gfx.rrect(s.x + 12.0, s.y + s.h - 9.0, (s.w - 24.0) * done as f32 / total as f32, 3.0, 1.5, rgba(0x5EE0B0, 1.0)); }
    a.gfx.text(&format!("{done}/{total}"), Fmt::Small, s.x + s.w - 50.0, s.y + 9.0, 40.0, 13.0, rgba(0x5EE0B0, 1.0), true);
    hits.push((R { x: s.x + off, ..s }, Act::Page(pages::P_TASKS as u8)));
}
/// Timer: tempo restante + modo (clique abre o Pomodoro)
pub fn timer_mini(a: &mut App, s: R, hits: &mut Vec<(R, Act)>, off: f32) {
    card(a, s); let t = timer::timer_get().ok(); let running = timer_running();
    let (rem, label) = t.map_or((0, "Foco".to_string()), |t| (t.remaining_sec, format!("{} · ciclo {}", match t.mode { timer::Mode::Focus => "Foco", _ => "Pausa" }, t.cycle.max(1))));
    a.gfx.icon("timer", s.x + 12.0, s.y + 8.0, 18.0, rgba(0xFF7A66, 1.0), false);
    a.gfx.text(&format!("{:02}:{:02}", rem / 60, rem % 60), Fmt::Clock, s.x + 40.0, s.y + 5.0, 90.0, 22.0, TEXT, false);
    a.gfx.text(&label, Fmt::Small, s.x + 40.0, s.y + 28.0, s.w - 90.0, 13.0, TEXT3, false);
    a.gfx.circle(s.x + s.w - 26.0, s.y + s.h / 2.0, 14.0, rgba(0xFF7A66, if running { 0.35 } else { 1.0 })); a.gfx.icon(if running { "pause" } else { "play" }, s.x + s.w - 33.0, s.y + s.h / 2.0 - 7.0, 14.0, TEXT, true);
    hits.push((R { x: s.x + off, ..s }, Act::Page(pages::P_POMO as u8)));
}
/// Relogio / data
pub fn clock_mini(a: &mut App, s: R) {
    card(a, s); let (hm, date) = pages::rest_clock();
    a.gfx.text(&hm, Fmt::Clock, s.x + 14.0, s.y + 6.0, 100.0, 22.0, TEXT, false); a.gfx.text(&date, Fmt::Small, s.x + 14.0, s.y + 29.0, s.w - 60.0, 13.0, TEXT3, false);
    a.gfx.icon("clock", s.x + s.w - 34.0, s.y + s.h / 2.0 - 10.0, 20.0, TEXT2, false);
}
/// Metricas do PC (CPU/RAM/GPU) lidas por evento/1 Hz so com o compacto/ilha visivel
pub fn metrics_mini(a: &mut App, s: R) {
    card(a, s); let sn = crate::metrics::snap();
    a.gfx.icon("cpu", s.x + 10.0, s.y + 8.0, 16.0, BLUE, false);
    for (i, (lbl, v, col)) in [("CPU", Some(sn.cpu), BLUE), ("RAM", Some(sn.ram), rgba(0x5EE0B0, 1.0)), ("GPU", sn.gpu, rgba(0xB69CFF, 1.0))].into_iter().enumerate() {
        let x = s.x + 34.0 + i as f32 * 56.0;
        a.gfx.text(lbl, Fmt::Tiny, x, s.y + 8.0, 40.0, 11.0, TEXT3, false); a.gfx.text(&v.map_or("—".into(), |v| format!("{v:.0}%")), Fmt::BodyStrong, x, s.y + 20.0, 48.0, 16.0, TEXT, false);
        a.gfx.rrect(x, s.y + s.h - 9.0, 46.0, 3.0, 1.5, SURFACE_H); a.gfx.rrect(x, s.y + s.h - 9.0, 46.0 * (v.unwrap_or(0.0) / 100.0).clamp(0.0, 1.0), 3.0, 1.5, col);
    }
}
