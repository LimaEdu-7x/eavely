// Slots (v0.2.0): widgets da pagina Mascote. Usam o MESMO estado, os MESMOS Act e os mesmos helpers das paginas cheias (card_art, draw_cover,
// button_bg, ui_pos): nada e duplicado. Coordenadas de projeto (a escala k vem da transformacao); os hits saem ja deslocados por `off`.
// Cada widget recebe o retangulo INTEIRO do slot (52 px ou 110 px de altura). O conteudo fica numa faixa de BAND px com PAD px de folga
// em cima e embaixo (em 52 px a faixa e o slot menos 10 px de cada lado); em 110 px a faixa fica centralizada.
use std::time::Instant;
use crate::spacing::{EDGE, CARD_GAP, ITEM_GAP, BAR_W};
use crate::{gfx::{rgba, Fmt, Rgba}, island::App, media, pages::{self, hov, button_bg, ui_pos, draw_cover, timer_running, r, Act, R, R2}, pages2, shelf, tasks, timer};

const TEXT: Rgba = rgba(0xFFFFFF, 1.0);
const TEXT2: Rgba = rgba(0xFFFFFF, 0.64);
const TEXT3: Rgba = rgba(0xFFFFFF, 0.52);
const SURFACE: Rgba = rgba(0xFFFFFF, 0.08);
const SURFACE_H: Rgba = rgba(0xFFFFFF, 0.14);
const BLUE: Rgba = rgba(0x7CC4FF, 1.0);
const PAD: f32 = crate::spacing::SLOT_PAD; // margem do conteudo no slot de 52 px (modulo spacing)
const BAND: f32 = SLOT_H - 2.0 * PAD; // altura util do conteudo (slot pequeno - 2 x PAD)
const TALL_MIN: f32 = 100.0; // acima disso o slot e grande (144 px): variante grande dos widgets

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)] pub enum Widget { #[default] Empty, Shelf, Player, Tasks, Timer, Clock, Metrics }
impl Widget {
    pub const ALL: [Widget; 7] = [Widget::Shelf, Widget::Player, Widget::Tasks, Widget::Timer, Widget::Clock, Widget::Metrics, Widget::Empty];
    pub fn parse(s: &str) -> Widget { match s { "shelf" => Widget::Shelf, "player" => Widget::Player, "tasks" => Widget::Tasks, "timer" => Widget::Timer, "clock" => Widget::Clock, "metrics" => Widget::Metrics, _ => Widget::Empty } }
    pub fn key(self) -> &'static str { match self { Widget::Shelf => "shelf", Widget::Player => "player", Widget::Tasks => "tasks", Widget::Timer => "timer", Widget::Clock => "clock", Widget::Metrics => "metrics", Widget::Empty => "none" } }
    pub fn label(self) -> &'static str { match self { Widget::Shelf => "Arquivos", Widget::Player => "Player mini", Widget::Tasks => "Tarefas", Widget::Timer => "Timer", Widget::Clock => "Relógio / Data", Widget::Metrics => "Métricas do PC", Widget::Empty => "Nenhum" } }
}
pub const SLOT_X: f32 = 252.0; pub const SLOT_W: f32 = 212.0; pub const SLOT_H: f32 = 64.0; pub const SLOT_BIG: f32 = 132.0; pub const SLOT_TOP: f32 = 12.0; pub const SLOT_TOP2: f32 = 84.0; // termina em 396: as setas de pagina (402..414) ficam fora do slot
/// slots efetivos (retangulos) a partir das opcoes: a mesma opcao nao pode estar nos dois (o 2o vira Nenhum); com um Nenhum o outro cresce para 190x110
pub fn layout(top: &str, bottom: &str) -> [(Widget, R); 2] {
    let (t, mut b) = (Widget::parse(top), Widget::parse(bottom)); if t != Widget::Empty && t == b { b = Widget::Empty; }
    match (t, b) {
        (Widget::Empty, Widget::Empty) => [(Widget::Empty, r(SLOT_X, SLOT_TOP, SLOT_W, SLOT_H)), (Widget::Empty, r(SLOT_X, SLOT_TOP2, SLOT_W, SLOT_H))],
        (t, Widget::Empty) => [(t, r(SLOT_X, SLOT_TOP, SLOT_W, SLOT_BIG)), (Widget::Empty, r(SLOT_X, SLOT_TOP2, SLOT_W, SLOT_H))],
        (Widget::Empty, b) => [(Widget::Empty, r(SLOT_X, SLOT_TOP, SLOT_W, SLOT_H)), (b, r(SLOT_X, SLOT_TOP, SLOT_W, SLOT_BIG))],
        (t, b) => [(t, r(SLOT_X, SLOT_TOP, SLOT_W, SLOT_H)), (b, r(SLOT_X, SLOT_TOP2, SLOT_W, SLOT_H))],
    }
}
/// Topo de um bloco de altura `h` centrado no slot (funciona em 52 e em 110 px)
fn mid(s: R, h: f32) -> f32 { s.y + (s.h - h) / 2.0 }
/// Topo da faixa de conteudo: PAD px abaixo do topo em 52 px; centralizada em 110 px
fn band(s: R) -> f32 { mid(s, BAND) }
pub fn draw_slot(a: &mut App, w: Widget, s: R, hits: &mut Vec<(R, Act)>, off: f32, now: Instant) {
    if w == Widget::Empty { return; }
    a.gfx.clip(s.x, s.y, s.w, s.h); // nada vaza do slot, em nenhum tamanho
    match w {
        Widget::Shelf => shelf_mini(a, s, hits, off, now), Widget::Player => player_mini(a, s, hits, off, now), Widget::Tasks => tasks_mini(a, s, hits, off),
        Widget::Timer => timer_mini(a, s, hits, off), Widget::Clock => clock_mini(a, s), Widget::Metrics => metrics_mini(a, s), Widget::Empty => {}
    }
    a.gfx.unclip();
}
fn card(a: &mut App, s: R) { a.gfx.rrect(s.x, s.y, s.w, s.h, 14.0, SURFACE); }

/// Arquivos: cards 52 x (h - 2 PAD) (arte do shell + inicio do nome e extensao em UMA linha) + alvo de soltar (ou +N); vazio/arrastando = caixa tracejada.
/// Mesma selecao (Ctrl/Shift), Delete/botao do meio e saida animada da pagina Arquivos; nome completo na legenda.
pub fn shelf_mini(a: &mut App, s: R, hits: &mut Vec<(R, Act)>, off: f32, now: Instant) {
    pages2::shelf_tick(a, now);
    let items = a.p.shelf.clone(); let drag = a.drag_over;
    card(a, s);
    if items.is_empty() || drag {
        let col = if drag { BLUE } else { rgba(0xFFFFFF, 0.28) };
        let pd = if s.h > TALL_MIN { EDGE } else { PAD }; // 110 px: caixa com a margem de 12 px
        a.gfx.rrect_dashed(s.x + pd, s.y + pd, s.w - 2.0 * pd, s.h - 2.0 * pd, 10.0, 1.5, col, 3.0, 2.0);
        a.gfx.icon("inbox", s.x + 20.0, mid(s, 20.0), 20.0, if drag { BLUE } else { TEXT2 }, false);
        let ty = band(s);
        a.gfx.text(if drag { "Solte para adicionar" } else { "Solte arquivos aqui" }, Fmt::BodyStrong, s.x + 48.0, ty, s.w - 58.0, 16.0, if drag { BLUE } else { TEXT }, false);
        a.gfx.text(if drag { "vai para a Shelf" } else { "ou arraste direto na ilha" }, Fmt::Small, s.x + 48.0, ty + 15.0, s.w - 58.0, 14.0, TEXT3, false);
        return;
    }
    hits.push((R { x: s.x + off, ..s }, Act::ShelfPan)); // clicar no vazio do slot limpa a selecao
    if s.h > TALL_MIN { return shelf_grid(a, s, hits, off, &items); }
    let (tw, step) = (52.0f32, 56.0f32); let th = s.h - 2.0 * PAD; let y = s.y + PAD; let oy = (th - BAND) / 2.0; // cards ocupam a altura do slot; o conteudo do card fica na faixa
    let max = ((s.w - 2.0 * PAD - step) / step) as usize; // 2 cards + alvo de soltar (ou +N) dentro da margem de 10 px
    for (i, it) in items.iter().take(max).enumerate() {
        let x = s.x + PAD + i as f32 * step; let act = Act::ShelfOpen(i as u32);
        if let Some(p) = pages2::del_progress(a, &it.id, now) { // saindo (lixeira): encolhe e some
            let e = p * p; let k = 1.0 - 0.6 * e; let (w, h) = (tw * k, th * k); a.gfx.layer(1.0 - e);
            a.gfx.rrect(x + (tw - w) / 2.0, y + (th - h) / 2.0, w, h, 10.0 * k, SURFACE_H); pages2::card_art(a, it, x + tw / 2.0, y + th / 2.0 + 1.0, 0.62 * k); a.gfx.unlayer(); continue;
        }
        let hv = a.hover == Some(act); let on = a.p.sel.contains(&i);
        a.gfx.rrect(x, y, tw, th, 10.0, if hv || on { SURFACE_H } else { SURFACE });
        pages2::card_art(a, it, x + tw / 2.0, y + 12.0 + oy, 0.34); // icone desce 4 px para dentro do card (o topo do glifo fica a ~4 px da borda)
        let (nm, cut) = pages2::fit_name(a, &it.name, Fmt::Tiny, tw - 4.0); let nw = a.gfx.measure(&nm, Fmt::Tiny);
        a.gfx.text(&nm, Fmt::Tiny, x + tw / 2.0 - nw / 2.0, y + 17.0 + oy, nw + 4.0, 11.0, TEXT2, false);
        if on { a.gfx.rrect_stroke(x + 0.75, y + 0.75, tw - 1.5, th - 1.5, 9.25, 1.5, BLUE); }
        if cut && hv { a.p.slot_tip = Some((it.name.clone(), x + tw / 2.0, if s.y > 40.0 { y - 20.0 } else { y + th + 3.0 })); }
        hits.push((R { x: x + off, y, w: tw, h: th }, act));
    }
    let dx = s.x + PAD + items.len().min(max) as f32 * step;
    if items.len() > max { // sobra: "+N" abre a pagina Arquivos
        let rc = R { x: dx, y, w: tw, h: th }; let hv = a.hover == Some(Act::Page(pages::P_SHELF as u8));
        a.gfx.rrect(rc.x, rc.y, rc.w, rc.h, 10.0, if hv { SURFACE_H } else { SURFACE }); let t = format!("+{}", items.len() - max); let w = a.gfx.measure(&t, Fmt::BodyStrong);
        a.gfx.text(&t, Fmt::BodyStrong, rc.x + (rc.w - w) / 2.0, rc.y + th / 2.0 - 8.0, w + 4.0, 16.0, TEXT2, false); hits.push((R { x: rc.x + off, ..rc }, Act::Page(pages::P_SHELF as u8)));
    } else if dx + tw <= s.x + s.w - PAD {
        a.gfx.rrect_dashed(dx + 0.75, y + 0.75, tw - 1.5, th - 1.5, 10.0, 1.5, rgba(0xFFFFFF, 0.28), 3.0, 2.0); a.gfx.icon("plus", dx + tw / 2.0 - 6.0, y + 5.0 + oy, 12.0, TEXT2, false);
        let t = "soltar"; let tw2 = a.gfx.measure(t, Fmt::Tiny); a.gfx.text(t, Fmt::Tiny, dx + tw / 2.0 - tw2 / 2.0, y + 17.0 + oy, tw2 + 4.0, 11.0, TEXT3, false);
    }
}

/// Arquivos no slot de 110 px: grade 3 x 2 (ate 6 itens) e barrinha vertical a esquerda, de cima a baixo, que abre a pagina Arquivos.
/// Medidas do modulo spacing (margem EDGE 16, CARD_GAP 12, BAR_W 4): 48x50 por card no slot grande.
/// geometria da grade: (x do 1o card, y do 1o card, largura, altura) e a faixa da barrinha
fn grid_geom(s: R) -> (f32, f32, f32, f32, f32) {
    let gx = s.x + EDGE + BAR_W + ITEM_GAP; let gw = s.w - EDGE - (gx - s.x); let (gy, gh) = (s.y + EDGE, s.h - 2.0 * EDGE);
    (gx, gy, (gw - 2.0 * CARD_GAP) / 3.0, (gh - CARD_GAP) / 2.0, gh)
}
fn shelf_grid(a: &mut App, s: R, hits: &mut Vec<(R, Act)>, off: f32, items: &[shelf::ShelfItem]) {
    let (gx, gy, tw, th, gh) = grid_geom(s);
    let bar = R { x: s.x + EDGE, y: gy, w: BAR_W, h: gh }; let shelf_act = Act::Page(pages::P_SHELF as u8);
    let hv_bar = a.hover == Some(shelf_act);
    a.gfx.rrect(bar.x, bar.y, bar.w, bar.h, BAR_W / 2.0, if hv_bar { rgba(0xFFFFFF, 0.42) } else { rgba(0xFFFFFF, 0.22) });
    hits.push((R { x: bar.x + off - 4.0, y: bar.y, w: bar.w + 8.0, h: bar.h }, shelf_act)); // a barrinha (com folga de 4 px) abre a pagina Arquivos
    for (i, it) in items.iter().take(6).enumerate() {
        let (x, y) = (gx + (i % 3) as f32 * (tw + CARD_GAP), gy + (i / 3) as f32 * (th + CARD_GAP)); let act = Act::ShelfOpen(i as u32);
        let hv = a.hover == Some(act); let on = a.p.sel.contains(&i);
        a.gfx.rrect(x, y, tw, th, 10.0, if hv || on { SURFACE_H } else { SURFACE });
        pages2::card_art(a, it, x + tw / 2.0, y + 12.0, 0.4); // icone pequeno no topo do card; nome numa linha embaixo
        let (nm, cut) = pages2::fit_name(a, &it.name, Fmt::Tiny, tw - 6.0); let nw = a.gfx.measure(&nm, Fmt::Tiny);
        a.gfx.text(&nm, Fmt::Tiny, x + tw / 2.0 - nw / 2.0, y + 24.0, nw + 4.0, 11.0, TEXT2, false);
        if on { a.gfx.rrect_stroke(x + 0.75, y + 0.75, tw - 1.5, th - 1.5, 9.25, 1.5, BLUE); }
        if cut && hv { a.p.slot_tip = Some((it.name.clone(), x + tw / 2.0, if s.y > 40.0 { y - 20.0 } else { y + th + 3.0 })); }
        hits.push((R { x: x + off, y, w: tw, h: th }, act));
    }
}

/// Player mini (sem letras nem volume): 52 px = capa 32 + titulo/artista + controles pequenos a direita (play 22, anterior/proximo 16);
/// 110 px = capa 72 em cima, texto ao lado e controles embaixo. Mesmos Act e estado da Musica.
pub fn player_mini(a: &mut App, s: R, hits: &mut Vec<(R, Act)>, off: f32, now: Instant) {
    let v = media::view(); let empty = v.title.is_empty() && v.status == "stopped";
    card(a, s);
    let tall = s.h > TALL_MIN;
    let cs = if tall { 72.0 } else { s.h - 2.0 * PAD }; let (cx, cy) = (s.x + PAD, if tall { s.y + PAD } else { s.y + PAD });
    let tx = cx + cs + PAD; let ctl = s.x + s.w - 68.0; // 52 px: controles (16+2+22+2+16 = 58 px) terminam a 10 px da borda
    let (ty, ay) = if tall { (s.y + 16.0, s.y + 33.0) } else { (s.y + 8.0, s.y + 22.0) }; // 52 px: titulo + artista terminam antes da barra (y 39)
    let tw = if tall { s.x + s.w - PAD - tx } else { ctl - tx - 4.0 };
    let (p0, pp, n0) = if tall { let (x, y) = (tx, s.y + 56.0); (r(x, y, 20.0, 20.0), r(x + 24.0, y - 3.0, 26.0, 26.0), r(x + 54.0, y, 20.0, 20.0)) }
        else { (r(ctl, s.y + 18.0, 16.0, 16.0), r(ctl + 18.0, s.y + 15.0, 22.0, 22.0), r(ctl + 42.0, s.y + 18.0, 16.0, 16.0)) };
    draw_cover(a, now, cx, cy, cs, 8.0, false);
    let dis = if empty { 0.4 } else { 1.0 };
    a.gfx.text(if empty { "Sem música" } else { &v.title }, Fmt::BodyStrong, tx, ty, tw, 15.0, if empty { TEXT2 } else { TEXT }, false);
    a.gfx.text(if empty { "Abra um player" } else { &v.artist }, Fmt::Small, tx, ay, tw, 13.0, TEXT3, false);
    let (ri, si) = if tall { (11.0, 12.0) } else { (8.0, 10.0) }; // raio e icone dos botoes laterais
    let ico = if tall { 4.0 } else { 3.0 };
    if !empty { button_bg(a, hits, p0, Act::Prev, off, ri); } a.gfx.icon("skip-back", p0.x + ico, p0.y + ico, si, (1.0, 1.0, 1.0, dis), true);
    let pc = if a.pressed == Some(Act::Play) { rgba(0xCFCFCF, 1.0) } else if !empty && hov(a, Act::Play) { rgba(0xE6E6E6, 1.0) } else { rgba(0xFFFFFF, dis) };
    a.gfx.rrect(pp.x, pp.y, pp.w, pp.h, pp.w / 2.0, pc); a.gfx.icon(if v.status == "playing" { "pause" } else { "play" }, pp.x + 6.0, pp.y + 6.0, 14.0, (0.0, 0.0, 0.0, 1.0), true);
    if !empty { hits.push((R { x: pp.x + off, ..pp }, Act::Play)); button_bg(a, hits, n0, Act::Next, off, ri); }
    a.gfx.icon("skip-forward", n0.x + ico, n0.y + ico, si, (1.0, 1.0, 1.0, dis), true);
    // progresso fino (seek otimista, igual a Musica): faixa com 10 px da base e das laterais
    let pos = ui_pos(a, &v, now); let frac = if empty { 0.0 } else { (pos as f32 / v.dur_ms.max(1) as f32).clamp(0.0, 1.0) };
    let big = hov(a, Act::Seek(R2(720, 2760))) || a.seek_drag; let bh = if big { 4.0 } else { 3.0 };
    let (sx, sw, sy) = if tall { (s.x + PAD, s.w - 2.0 * PAD, s.y + s.h - PAD - bh / 2.0) } else { (tx, s.x + s.w - PAD - tx, s.y + s.h - PAD - bh / 2.0) };
    a.gfx.rrect(sx, sy - bh / 2.0, sw, bh, 1.5, SURFACE_H); a.gfx.rrect(sx, sy - bh / 2.0, (sw * frac).max(0.0), bh, 1.5, TEXT);
    crate::pages::thumb(a, 0, sx + (sw * frac).max(0.0), sy, big, now);
    if !empty { hits.push((r(sx + off, sy - 7.0, sw, 14.0), Act::Seek(R2(720, 2760)))); }
}

/// area util do slot grande: margem de 16 px (EDGE) em todos os lados
fn inner(s: R) -> (f32, f32, f32, f32) { (s.x + EDGE, s.y + EDGE, s.w - 2.0 * EDGE, s.h - 2.0 * EDGE) }
/// Tarefas: proxima pendente + contagem (clique abre a aba Tarefas). Slot grande: ate 3 pendentes com marcador, contagem e barra ocupando a altura toda
pub fn tasks_mini(a: &mut App, s: R, hits: &mut Vec<(R, Act)>, off: f32) {
    card(a, s); hits.push((R { x: s.x + off, ..s }, Act::Page(pages::P_TASKS as u8)));
    let list = tasks::list(); let today = tasks::now_local().date_str(); let (done, total) = tasks::counts(&list, &today);
    let green = rgba(0x5EE0B0, 1.0);
    if s.h > TALL_MIN {
        let (ix, iy, iw, ih) = inner(s);
        let pend: Vec<&tasks::Task> = list.iter().filter(|t| !t.done).collect();
        if pend.is_empty() { a.gfx.text("Sem tarefas pendentes", Fmt::Head, ix + 12.0, iy, iw - 12.0, 22.0, TEXT2, false); }
        for (i, t) in pend.iter().take(3).enumerate() { // linhas de 16 px com 8 px entre elas
            let y = iy + i as f32 * 24.0;
            a.gfx.circle(ix + 4.0, y + 8.0, 4.0, green);
            a.gfx.text(&t.text, Fmt::Row, ix + 16.0, y, iw - 16.0, 16.0, TEXT, false);
        }
        a.gfx.text(&format!("{done} de {total} hoje"), Fmt::Small, ix, iy + ih - 34.0, iw, 13.0, TEXT3, false);
        a.gfx.rrect(ix, iy + ih - 4.0, iw, 4.0, 2.0, SURFACE_H);
        if total > 0 { a.gfx.rrect(ix, iy + ih - 4.0, iw * done as f32 / total as f32, 4.0, 2.0, green); }
        return;
    }
    let top = band(s);
    a.gfx.icon("list-checks", s.x + 12.0, top, 18.0, rgba(0x5EE0B0, 1.0), false);
    let next = list.iter().find(|t| !t.done);
    a.gfx.text(next.map_or("Sem tarefas pendentes", |t| t.text.as_str()), Fmt::BodyStrong, s.x + 40.0, top, s.w - 40.0 - 52.0, 16.0, TEXT, false);
    a.gfx.text(&format!("{done} de {total} hoje"), Fmt::Small, s.x + 40.0, top + 14.0, s.w - 92.0, 13.0, TEXT3, false);
    if total > 0 { a.gfx.rrect(s.x + 12.0, s.y + s.h - PAD - 3.0, s.w - 24.0, 3.0, 1.5, SURFACE_H); a.gfx.rrect(s.x + 12.0, s.y + s.h - PAD - 3.0, (s.w - 24.0) * done as f32 / total as f32, 3.0, 1.5, rgba(0x5EE0B0, 1.0)); }
    a.gfx.text(&format!("{done}/{total}"), Fmt::Small, s.x + s.w - 50.0, top + 1.0, 40.0, 13.0, rgba(0x5EE0B0, 1.0), true);
}
/// Timer: tempo restante + modo (clique abre o Pomodoro). Slot grande: digitos, play/pause e reset, rotulo e barra ocupando a altura toda. Clique do slot empilhado antes; botoes depois (vencem)
pub fn timer_mini(a: &mut App, s: R, hits: &mut Vec<(R, Act)>, off: f32) {
    card(a, s); hits.push((R { x: s.x + off, ..s }, Act::Page(pages::P_POMO as u8)));
    let t = timer::timer_get().ok(); let running = timer_running();
    let (rem, total, label) = t.map_or((0, 0, "Foco".to_string()), |t| (t.remaining_sec, t.total_sec, format!("{} · ciclo {}", match t.mode { timer::Mode::Focus => "Foco", _ => "Pausa" }, t.cycle.max(1))));
    let coral = rgba(0xFF7A66, 1.0);
    if s.h > TALL_MIN {
        let (ix, iy, iw, ih) = inner(s);
        a.gfx.text(&format!("{:02}:{:02}", rem / 60, rem % 60), Fmt::Big, ix, iy, iw, 36.0, TEXT, false);
        let (pl, rs) = (r(ix, iy + 44.0, 64.0, 32.0), r(ix + 72.0, iy + 44.0, 32.0, 32.0)); // alvo de 32 px, 8 px entre botoes
        a.gfx.rrect(pl.x, pl.y, pl.w, pl.h, 16.0, rgba(0xFF7A66, if running { 0.35 } else { 1.0 }));
        a.gfx.icon(if running { "pause" } else { "play" }, pl.x + 25.0, pl.y + 9.0, 14.0, TEXT, true); hits.push((R { x: pl.x + off, ..pl }, Act::TimerPlay));
        a.gfx.rrect(rs.x, rs.y, rs.w, rs.h, 16.0, SURFACE_H); a.gfx.icon("rotate-ccw", rs.x + 9.0, rs.y + 9.0, 14.0, TEXT, false); hits.push((R { x: rs.x + off, ..rs }, Act::TimerReset));
        let lab = pages2::ellip(a, &label, Fmt::Small, iw); a.gfx.text(&lab, Fmt::Small, ix, iy + 84.0, iw, 13.0, TEXT3, false);
        let frac = if total > 0 { 1.0 - rem as f32 / total as f32 } else { 0.0 }.clamp(0.0, 1.0);
        a.gfx.rrect(ix, iy + ih - 4.0, iw, 4.0, 2.0, SURFACE_H); a.gfx.rrect(ix, iy + ih - 4.0, iw * frac, 4.0, 2.0, coral);
        return;
    }
    let top = mid(s, 36.0);
    a.gfx.icon("timer", s.x + 12.0, mid(s, 18.0), 18.0, rgba(0xFF7A66, 1.0), false);
    a.gfx.text(&format!("{:02}:{:02}", rem / 60, rem % 60), Fmt::Clock, s.x + 40.0, top - 3.0, 90.0, 22.0, TEXT, false);
    a.gfx.text(&label, Fmt::Small, s.x + 40.0, top + 20.0, s.w - 90.0, 13.0, TEXT3, false);
    a.gfx.circle(s.x + s.w - 26.0, s.y + s.h / 2.0, 14.0, rgba(0xFF7A66, if running { 0.35 } else { 1.0 })); a.gfx.icon(if running { "pause" } else { "play" }, s.x + s.w - 33.0, s.y + s.h / 2.0 - 7.0, 14.0, TEXT, true);
}
/// Relogio / data: 64 px = hora + data; slot grande = hora em Hero (44 px), data e icone de 40 px a direita, bloco centrado na area util
pub fn clock_mini(a: &mut App, s: R) {
    card(a, s); let (hm, date) = pages::rest_clock();
    if s.h > TALL_MIN {
        let (ix, iy, iw, ih) = inner(s); let top = iy + (ih - 72.0) / 2.0;
        a.gfx.text(&hm, Fmt::Hero, ix, top, iw - 56.0, 52.0, TEXT, false); a.gfx.text(&date, Fmt::Row, ix, top + 56.0, iw - 56.0, 16.0, TEXT3, false);
        a.gfx.icon("clock", ix + iw - 40.0, iy + (ih - 40.0) / 2.0, 40.0, TEXT2, false);
        return;
    }
    let t0 = band(s);
    a.gfx.text(&hm, Fmt::Clock, s.x + 14.0, t0, 100.0, 22.0, TEXT, false); a.gfx.text(&date, Fmt::Small, s.x + 14.0, t0 + 19.0, s.w - 60.0, 13.0, TEXT3, false);
    a.gfx.icon("clock", s.x + s.w - 34.0, s.y + s.h / 2.0 - 10.0, 20.0, TEXT2, false);
}
/// Metricas do PC (CPU/RAM/GPU). Slot grande: 3 colunas (rotulo, valor Big ou Head, barra de 8 px) centradas na area util
pub fn metrics_mini(a: &mut App, s: R) {
    card(a, s); let sn = crate::metrics::snap();
    let cols = [("CPU", Some(sn.cpu), BLUE), ("RAM", Some(sn.ram), rgba(0x5EE0B0, 1.0)), ("GPU", sn.gpu, rgba(0xB69CFF, 1.0))];
    if s.h > TALL_MIN {
        let (ix, iy, iw, ih) = inner(s); let cw = (iw - 2.0 * 12.0) / 3.0; let top = iy + (ih - 77.0) / 2.0;
        for (i, (lbl, v, col)) in cols.into_iter().enumerate() {
            let x = ix + i as f32 * (cw + 12.0); let txt = v.map_or("—".into(), |v| format!("{v:.0}%"));
            let vf = if a.gfx.measure(&txt, Fmt::Big) <= cw { Fmt::Big } else { Fmt::Head };
            a.gfx.text(lbl, Fmt::Small, x, top, cw, 13.0, TEXT3, false); a.gfx.text(&txt, vf, x, top + 17.0, cw, 36.0, TEXT, false);
            a.gfx.rrect(x, top + 69.0, cw, 8.0, 4.0, SURFACE_H); a.gfx.rrect(x, top + 69.0, cw * (v.unwrap_or(0.0) / 100.0).clamp(0.0, 1.0), 8.0, 4.0, col);
        }
        return;
    }
    let top = band(s);
    for (i, (lbl, v, col)) in cols.into_iter().enumerate() {
        let x = s.x + 12.0 + i as f32 * 58.0;
        a.gfx.text(lbl, Fmt::Tiny, x, top, 40.0, 11.0, TEXT3, false); a.gfx.text(&v.map_or("—".into(), |v| format!("{v:.0}%")), Fmt::BodyStrong, x, top + 11.0, 48.0, 16.0, TEXT, false);
        a.gfx.rrect(x, s.y + s.h - PAD - 3.0, 50.0, 3.0, 1.5, SURFACE_H); a.gfx.rrect(x, s.y + s.h - PAD - 3.0, 50.0 * (v.unwrap_or(0.0) / 100.0).clamp(0.0, 1.0), 3.0, 1.5, col);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slot_unico_ocupa_altura_toda() { // com um Nenhum, o outro cresce para 110 px; com dois, cada um tem 52
        assert_eq!(layout("shelf", "none")[0].1.h, SLOT_BIG);
        assert_eq!(layout("shelf", "player")[0].1.h, SLOT_H);
        assert_eq!(layout("shelf", "player")[1].1.h, SLOT_H);
    }
    #[test]
    fn grade_de_arquivos_cabe_com_margem_de_12_px() { // 110 px: 3 x 2 cards dentro de EDGE 12 e barrinha de BAR_W antes da grade
        let s = r(SLOT_X, SLOT_TOP, SLOT_W, SLOT_BIG); let (gx, gy, tw, th, _) = grid_geom(s);
        assert_eq!((tw, th), (48.0, 44.0)); // slot 212x132: gh 100 - 12 entre linhas, 2 linhas
        assert!(gx + 3.0 * tw + 2.0 * CARD_GAP <= s.x + s.w - EDGE + 0.01, "colunas passam da margem direita");
        assert!(gy + 2.0 * th + CARD_GAP <= s.y + s.h - EDGE + 0.01, "linhas passam da margem de baixo");
        assert!(gx - s.x >= EDGE + BAR_W + ITEM_GAP - 0.01, "grade colide com a barrinha");
    }
    #[test]
    fn bloco_centrado_com_margem_de_10_px() { // a faixa de conteudo tem >= 10 px em cima e embaixo, em 52 e em 110 px
        assert!(BAND <= SLOT_H - 2.0 * PAD, "faixa de {BAND} px nao cabe com margem de {PAD} px em 52 px");
        for h in [SLOT_H, SLOT_BIG] {
            let s = r(SLOT_X, SLOT_TOP, SLOT_W, h);
            let top = band(s);
            let (above, below) = (top - s.y, s.y + s.h - (top + BAND));
            assert!(above >= PAD && below >= PAD, "h={h}: margem {above} em cima, {below} embaixo (minimo {PAD})");
            assert!((above - below).abs() < 0.01, "h={h}: nao centralizado ({above} x {below})");
        }
    }
}
