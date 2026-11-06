// Tarefas (N2): modelo + persistencia (%APPDATA%\Eavely\todo.json) + lembretes na hora + pagina nativa.
// Datas em hora local "ingenua" (YYYY-MM-DDTHH:MM): sem fuso/DST, suficiente para lembretes pessoais.
use serde::{Deserialize, Serialize};
use std::{sync::{atomic::{AtomicU64, Ordering::Relaxed}, Condvar, Mutex}, time::Duration};
use crate::{gfx::{rgba, Fmt, Rgba}, island::App, pages::{Act, R}};

const TEXT: Rgba = rgba(0xFFFFFF, 1.0);
const TEXT2: Rgba = rgba(0xFFFFFF, 0.64);
const TEXT3: Rgba = rgba(0xFFFFFF, 0.52);
const SURFACE: Rgba = rgba(0xFFFFFF, 0.08);
const SURFACE_H: Rgba = rgba(0xFFFFFF, 0.14);
const MINT: Rgba = rgba(0x5EE0B0, 1.0);
const ACCENT: Rgba = rgba(0x7CC4FF, 1.0);
const DANGER: Rgba = rgba(0xFF6B6B, 1.0);

// ---------- data/hora ----------
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Dt { pub y: i32, pub m: u32, pub d: u32, pub h: u32, pub mi: u32, pub s: u32 }
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 { // Howard Hinnant
    let y = if m <= 2 { y - 1 } else { y } as i64; let era = y.div_euclid(400); let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) as i64 + 2) / 5 + d as i64 - 1; let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097; let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}
pub fn days_in_month(y: i32, m: u32) -> u32 { match m { 1 | 3 | 5 | 7 | 8 | 10 | 12 => 31, 4 | 6 | 9 | 11 => 30, _ => if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 29 } else { 28 } } }
/// 0 = domingo
pub fn weekday(y: i32, m: u32, d: u32) -> u32 { (days_from_civil(y, m, d) + 4).rem_euclid(7) as u32 }
impl Dt {
    pub fn secs(&self) -> i64 { days_from_civil(self.y, self.m, self.d) * 86400 + self.h as i64 * 3600 + self.mi as i64 * 60 + self.s as i64 }
    pub fn fmt(&self) -> String { format!("{:04}-{:02}-{:02}T{:02}:{:02}", self.y, self.m, self.d, self.h, self.mi) }
    pub fn parse(s: &str) -> Option<Dt> {
        let (dp, tp) = s.split_once('T')?; let mut it = dp.split('-'); let mut tt = tp.split(':');
        let dt = Dt { y: it.next()?.parse().ok()?, m: it.next()?.parse().ok()?, d: it.next()?.parse().ok()?, h: tt.next()?.parse().ok()?, mi: tt.next()?.parse().ok()?, s: 0 };
        (dt.m >= 1 && dt.m <= 12 && dt.d >= 1 && dt.d <= days_in_month(dt.y, dt.m) && dt.h < 24 && dt.mi < 60).then_some(dt)
    }
    pub fn date_str(&self) -> String { format!("{:04}-{:02}-{:02}", self.y, self.m, self.d) }
    pub fn plus_days(&self, n: i64) -> Dt { let (y, m, d) = civil_from_days(days_from_civil(self.y, self.m, self.d) + n); Dt { y, m, d, ..*self } }
}
pub fn now_local() -> Dt { let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() }; Dt { y: t.wYear as i32, m: t.wMonth as u32, d: t.wDay as u32, h: t.wHour as u32, mi: t.wMinute as u32, s: t.wSecond as u32 } }

/// "Ligar 14:30" / "Ligar @9:05" -> (texto, hora). Token invalido (25:99) fica no texto.
pub fn split_time(s: &str) -> (String, Option<(u32, u32)>) {
    let s = s.trim(); let tok = s.rsplit(' ').next().unwrap_or("");
    let t = tok.trim_start_matches('@');
    if let Some((h, m)) = t.split_once(':') { if let (Ok(h), Ok(m)) = (h.parse::<u32>(), m.parse::<u32>()) { if h < 24 && m < 60 && t.len() <= 5 {
        return (s[..s.len() - tok.len()].trim().to_string(), Some((h, m)));
    } } }
    (s.to_string(), None)
}
/// Decide o vencimento: hora digitada e/ou data escolhida. So hora -> hoje (ou amanha se ja passou); so data -> 09:00.
pub fn due_from(date: Option<(i32, u32, u32)>, time: Option<(u32, u32)>, now: Dt) -> Option<Dt> {
    match (date, time) {
        (None, None) => None,
        (Some((y, m, d)), Some((h, mi))) => Some(Dt { y, m, d, h, mi, s: 0 }),
        (Some((y, m, d)), None) => Some(Dt { y, m, d, h: 9, mi: 0, s: 0 }),
        (None, Some((h, mi))) => { let t = Dt { h, mi, s: 0, ..now }; Some(if t <= now { t.plus_days(1) } else { t }) }
    }
}

// ---------- modelo ----------
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Task { pub id: u64, pub text: String, pub done: bool, #[serde(default)] pub done_on: Option<String>, #[serde(default)] pub due: Option<String>, #[serde(default)] pub reminded: bool }
impl Task { pub fn due_dt(&self) -> Option<Dt> { self.due.as_deref().and_then(Dt::parse) } }

/// Concluidas hoje / total (pendentes + concluidas hoje)
pub fn counts(v: &[Task], today: &str) -> (usize, usize) {
    let done = v.iter().filter(|t| t.done && t.done_on.as_deref() == Some(today)).count();
    (done, v.iter().filter(|t| !t.done).count() + done)
}
/// Concluidas em dias anteriores saem da lista (a contagem "hoje" zera a meia-noite)
pub fn prune(v: &mut Vec<Task>, today: &str) -> bool { let n = v.len(); v.retain(|t| !t.done || t.done_on.as_deref() == Some(today)); v.len() != n }
/// Tarefas com lembrete vencido (pendentes, com hora, ainda nao avisadas)
pub fn due_now(v: &[Task], now: Dt) -> Vec<u64> { v.iter().filter(|t| !t.done && !t.reminded && t.due_dt().is_some_and(|d| d <= now)).map(|t| t.id).collect() }
pub fn next_due(v: &[Task], now: Dt) -> Option<i64> { v.iter().filter(|t| !t.done && !t.reminded).filter_map(|t| t.due_dt()).map(|d| d.secs() - now.secs()).min() }

static TASKS: Mutex<Option<Vec<Task>>> = Mutex::new(None);
static CV: Condvar = Condvar::new();
pub static GEN: AtomicU64 = AtomicU64::new(1);
fn file() -> std::path::PathBuf { crate::data_dir().join("todo.json") }
fn with<T>(f: impl FnOnce(&mut Vec<Task>) -> T) -> T {
    let mut g = TASKS.lock().unwrap_or_else(|e| e.into_inner());
    let v = g.get_or_insert_with(|| { let mut v: Vec<Task> = crate::store::load_or_quarantine(&file()).unwrap_or_default(); prune(&mut v, &now_local().date_str()); v });
    f(v)
}
fn save(v: &[Task]) {
    crate::store::write_atomic(&file(), serde_json::to_string(v).unwrap_or_default().as_bytes());
}
fn changed(v: &[Task]) { save(v); GEN.fetch_add(1, Relaxed); CV.notify_all(); crate::island::wake(); }
/// Pendentes primeiro (ordem de criacao), concluidas no fim
pub fn sorted(mut v: Vec<Task>) -> Vec<Task> { let last = crate::settings::get().tasks_done_last; v.sort_by_key(|t| (last && t.done, t.id)); v }
pub fn list() -> Vec<Task> { sorted(with(|v| { prune(v, &now_local().date_str()); v.clone() })) }
/// So para capturas/testes: tarefa atrasada ja avisada
#[cfg(feature = "devtools")]
pub fn debug_overdue(text: &str, due: Dt) { with(|v| { let id = v.iter().map(|t| t.id).max().unwrap_or(0) + 1; v.push(Task { id, text: text.into(), done: false, done_on: None, due: Some(due.fmt()), reminded: true }); changed(v); }) }
pub fn add(text: &str, due: Option<Dt>) -> u64 {
    with(|v| { let id = v.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        v.push(Task { id, text: text.trim().to_string(), done: false, done_on: None, due: due.map(|d| d.fmt()), reminded: false }); changed(v); id })
}
pub fn edit(id: u64, text: &str, due: Option<Dt>) { with(|v| { if let Some(t) = v.iter_mut().find(|t| t.id == id) { t.text = text.trim().to_string(); t.due = due.map(|d| d.fmt()); t.reminded = false; changed(v); } }) }
pub fn toggle(id: u64) { with(|v| { if let Some(t) = v.iter_mut().find(|t| t.id == id) { t.done = !t.done; t.done_on = t.done.then(|| now_local().date_str()); changed(v); } }) }
pub fn remove(id: u64) { with(|v| { let n = v.len(); v.retain(|t| t.id != id); if v.len() != n { changed(v); } }) }

/// Thread dos lembretes: dorme ate o proximo vencimento (no maximo 30 s por vez, p/ acompanhar mudanca de relogio); sem lembretes pendentes nao acorda.
pub fn init() {
    std::thread::Builder::new().name("lembretes".into()).spawn(|| {
        let mut g = TASKS.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if g.is_none() { drop(g); with(|_| ()); g = TASKS.lock().unwrap_or_else(|e| e.into_inner()); continue; }
            let now = now_local();
            let ids = due_now(g.as_ref().unwrap(), now);
            if !ids.is_empty() {
                let mut msgs: Vec<String> = Vec::new();
                { let v = g.as_mut().unwrap(); for t in v.iter_mut() { if ids.contains(&t.id) { t.reminded = true; msgs.push(t.text.clone()); } } }
                { let v = g.as_ref().unwrap(); save(v); }
                GEN.fetch_add(1, Relaxed);
                drop(g);
                for m in msgs { crate::log(&format!("lembrete disparado: {m}")); crate::timer::notify("Lembrete", &m); crate::timer::beep(); }
                crate::island::wake();
                g = TASKS.lock().unwrap_or_else(|e| e.into_inner());
                continue;
            }
            g = match next_due(g.as_ref().unwrap(), now) {
                None => CV.wait(g).unwrap_or_else(|e| e.into_inner()),
                Some(secs) => CV.wait_timeout(g, Duration::from_secs((secs.max(1) as u64).min(30))).unwrap_or_else(|e| e.into_inner()).0,
            };
        }
    }).ok();
}

// ---------- UI ----------
const MONTHS: [&str; 12] = ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"];
const MON3: [&str; 12] = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];

pub fn draw(a: &mut App, hits: &mut Vec<(R, Act)>, off: f32) {
    let g = GEN.load(Relaxed);
    if a.p.task_gen != g { a.p.tasks = list(); a.p.task_gen = g; }
    let now = now_local(); let today = now.date_str();
    let (done, total) = counts(&a.p.tasks, &today);
    a.gfx.text(&format!("{done}/{total}"), Fmt::Big, 24.0, 40.0, 84.0, 36.0, TEXT, false);
    a.gfx.text("Concluídas hoje", Fmt::Small, 24.0, 78.0, 120.0, 14.0, TEXT2, false);
    // calendario (clique: escolher a data do proximo lembrete); ponto azul = ha tarefa pendente para hoje
    let cal = Act::TaskCal; let hov = a.hover == Some(cal) || a.p.task_cal;
    a.gfx.circle(125.0, 59.0, 13.0, if hov { SURFACE_H } else { SURFACE }); a.gfx.icon("calendar", 118.0, 52.0, 14.0, if a.p.task_cal { MINT } else { TEXT2 }, false);
    hits.push((R { x: 112.0 + off, y: 46.0, w: 26.0, h: 26.0 }, cal));
    if a.p.tasks.iter().any(|t| !t.done && t.due_dt().is_some_and(|d| d.date_str() == today)) { a.gfx.circle(135.0, 49.0, 3.0, ACCENT); }
    // botoes
    let adding = a.p.task_input.is_some();
    let r1 = R { x: 24.0, y: 98.0, w: 104.0, h: 22.0 }; let hv = a.hover == Some(Act::TaskAdd);
    a.gfx.rrect(r1.x, r1.y, r1.w, r1.h, 11.0, if adding { rgba(0x5EE0B0, 0.2) } else if hv { SURFACE_H } else { SURFACE });
    a.gfx.icon_label("circle-plus", "Adicionar", Fmt::Body, r1.x, r1.y, r1.w, r1.h, if adding { MINT } else { TEXT }, 14.0);
    hits.push((R { x: r1.x + off, ..r1 }, Act::TaskAdd));
    let has = !a.p.tasks.is_empty(); let r2 = R { x: 134.0, y: 98.0, w: 84.0, h: 22.0 };
    let em = a.p.task_edit_mode; let ec = if has { TEXT } else { rgba(0xFFFFFF, 0.35) };
    a.gfx.rrect(r2.x, r2.y, r2.w, r2.h, 11.0, if em { rgba(0x5EE0B0, 0.2) } else if a.hover == Some(Act::TaskEdit) && has { SURFACE_H } else { SURFACE });
    a.gfx.icon_label(if em { "check" } else { "pencil" }, if em { "Pronto" } else { "Editar" }, Fmt::Body, r2.x, r2.y, r2.w, r2.h, if em { MINT } else { ec }, 14.0);
    if has { hits.push((R { x: r2.x + off, ..r2 }, Act::TaskEdit)); }
    // painel
    a.gfx.rrect(222.0, 40.0, 174.0, 78.0, 14.0, rgba(0xFFFFFF, 0.05));
    if a.p.task_cal { draw_cal(a, hits, off, &now); }
    else if a.p.task_input.is_some() { draw_input(a, hits, off, &now); }
    else if a.p.tasks.is_empty() {
        a.gfx.icon("list-checks", 299.0, 60.0, 20.0, TEXT3, false);
        let tw = a.gfx.measure("Sem tarefas.", Fmt::Body); a.gfx.text("Sem tarefas.", Fmt::Body, 222.0 + (174.0 - tw) / 2.0, 86.0, tw + 4.0, 16.0, TEXT2, false);
    } else { draw_list(a, hits, off); }
}

fn draw_list(a: &mut App, hits: &mut Vec<(R, Act)>, off: f32) {
    let n = a.p.tasks.len(); a.p.task_scroll = a.p.task_scroll.min(n.saturating_sub(3));
    a.gfx.clip(222.0, 40.0, 174.0, 78.0);
    for k in 0..3 {
        let i = a.p.task_scroll + k; let Some(t) = a.p.tasks.get(i).cloned() else { break };
        let y = 42.0 + k as f32 * 25.0;
        let tg = Act::TaskToggle(i as u32);
        if a.hover == Some(tg) { a.gfx.circle(236.0, y + 12.0, 10.0, SURFACE); }
        if t.done { a.gfx.icon("circle-check", 229.0, y + 5.0, 14.0, MINT, false); } else { a.gfx.ring(236.0, y + 12.0, 6.5, 1.5, TEXT3); }
        hits.push((R { x: 224.0 + off, y, w: 24.0, h: 24.0 }, tg));
        let col = if t.done { TEXT3 } else { TEXT };
        let due = t.due_dt().map(|d| if d.date_str() == now_local().date_str() { format!("{:02}:{:02}", d.h, d.mi) } else { format!("{} {:02}:{:02}", d.d, d.h, d.mi) });
        let right = if a.p.task_edit_mode { 24.0 } else { 0.0 };
        let (tw, dw) = (190.0 - 34.0 - right, due.as_ref().map_or(0.0, |s| a.gfx.measure(s, Fmt::Small) + 6.0));
        let wtxt = (394.0 - right - dw - 8.0 - 254.0).max(30.0); // o nome termina 8 px antes do horario (e do X no modo editar)
        a.gfx.text(&t.text, Fmt::Body, 254.0, y + 4.0, wtxt, 16.0, col, false);
        if t.done { let w = a.gfx.measure(&t.text, Fmt::Body).min(wtxt); a.gfx.line(254.0, y + 12.0, 254.0 + w, y + 12.0, 1.0, TEXT3); }
        let overdue = !t.done && t.due_dt().is_some_and(|d| d < now_local());
        if let Some(d) = due { a.gfx.text(&d, Fmt::Small, 394.0 - right - dw, y + 5.0, dw, 14.0, if overdue && crate::settings::get().tasks_overdue_red { DANGER } else if t.reminded { MINT } else { TEXT3 }, true); }
        if a.p.task_edit_mode {
            let pk = Act::TaskPick(i as u32); hits.push((R { x: 254.0 + off, y, w: 100.0, h: 24.0 }, pk));
            let rm = Act::TaskRemove(i as u32); if a.hover == Some(rm) { a.gfx.circle(383.0, y + 12.0, 9.0, SURFACE_H); }
            a.gfx.icon("x", 376.0, y + 5.0, 14.0, rgba(0xFF6B6B, 1.0), false); hits.push((R { x: 372.0 + off, y, w: 22.0, h: 24.0 }, rm));
        }
    }
    a.gfx.unclip();
    if n > 3 { // trilho de rolagem
        let h = 78.0 * 3.0 / n as f32; let y = 40.0 + (78.0 - h) * (a.p.task_scroll as f32 / (n - 3) as f32);
        let hov = a.hover.is_some_and(|h| matches!(h, Act::TaskToggle(_) | Act::TaskPick(_) | Act::TaskRemove(_)));
        let bw = if hov { 5.0 } else { 3.0 }; a.gfx.rrect(394.0 - bw, y, bw, h, bw / 2.0, rgba(0xFFFFFF, if hov { 0.4 } else { 0.28 }));
        if a.p.task_scroll > 0 { a.gfx.vgrad(222.0, 40.0, 170.0, 10.0, (0.02, 0.02, 0.02, 1.0), 0.0, 0.0); } // dica visual de que ha mais acima/abaixo (barra + bordas)
    }
}

fn date_label(d: Option<(i32, u32, u32)>, now: &Dt) -> String {
    match d { None => "Sem data".into(), Some((y, m, dd)) => { let t = Dt { y, m, d: dd, h: 0, mi: 0, s: 0 };
        if t.date_str() == now.date_str() { "Hoje".into() } else if t.date_str() == now.plus_days(1).date_str() { "Amanhã".into() } else { format!("{} {}", dd, MON3[m as usize - 1]) } } }
}
fn draw_input(a: &mut App, hits: &mut Vec<(R, Act)>, off: f32, now: &Dt) {
    let txt = a.p.task_input.clone().unwrap_or_default();
    a.gfx.rrect(222.0, 40.0, 174.0, 28.0, 10.0, SURFACE_H); a.gfx.rrect_stroke(222.5, 40.5, 173.0, 27.0, 10.0, 1.5, MINT);
    if txt.is_empty() { a.gfx.text("Nova tarefa…", Fmt::Body, 232.0, 47.0, 150.0, 16.0, TEXT3, false); a.gfx.line(232.0, 46.0, 232.0, 62.0, 1.0, TEXT); }
    else { let w = a.gfx.measure(&txt, Fmt::Body); let shown = if w > 148.0 { 148.0 } else { w }; a.gfx.clip(228.0, 40.0, 164.0, 28.0);
        let x = 232.0 - (w - shown); a.gfx.text(&txt, Fmt::Body, x, 47.0, w + 8.0, 16.0, TEXT, false); a.gfx.unclip(); a.gfx.line(232.0 + shown + 1.0, 46.0, 232.0 + shown + 1.0, 62.0, 1.0, TEXT); }
    let chip = Act::TaskCal; let (_, tm) = split_time(&txt); let due = due_from(a.p.task_date, tm, *now);
    let (lab, late) = match due { Some(d) => (format!("{} {:02}:{:02}", date_label(Some((d.y, d.m, d.d)), now), d.h, d.mi), d < *now), None => ("Sem data".to_string(), false) };
    let cw = (a.gfx.measure(&lab, Fmt::Body) + 38.0).clamp(80.0, 172.0);
    a.gfx.rrect(222.0, 74.0, cw, 22.0, 11.0, if late { rgba(0xFF6B6B, 0.18) } else if a.hover == Some(chip) { SURFACE_H } else { SURFACE });
    a.gfx.icon("calendar", 230.0, 78.0, 14.0, if late { DANGER } else { TEXT2 }, false); a.gfx.text(&lab, Fmt::Body, 250.0, 78.0, cw - 30.0, 16.0, if late { DANGER } else { TEXT }, false);
    hits.push((R { x: 222.0 + off, y: 74.0, w: cw, h: 22.0 }, chip));
    a.gfx.text("Ex.: Ligar 14:30 · Enter salva", Fmt::Small, 224.0, 100.0, 172.0, 14.0, TEXT3, false);
}
fn draw_cal(a: &mut App, hits: &mut Vec<(R, Act)>, off: f32, now: &Dt) {
    let (y, m) = if a.p.task_month.1 == 0 { (now.y, now.m) } else { a.p.task_month }; a.p.task_month = (y, m);
    a.gfx.rrect(222.0, 40.0, 174.0, 106.0, 14.0, rgba(0xFFFFFF, 0.05)); // a ilha cresce 28 px enquanto o calendario esta aberto: o painel ocupa 40..146
    for (x, ic, d) in [(224.0f32, "chevron-left", -1i8), (372.0, "chevron-right", 1)] {
        let act = Act::TaskMonth(d); if a.hover == Some(act) { a.gfx.circle(x + 12.0, 50.0, 11.0, SURFACE_H); }
        a.gfx.icon(ic, x + 5.0, 43.0, 14.0, TEXT2, false); hits.push((R { x: x + off, y: 38.0, w: 26.0, h: 24.0 }, act)); }
    let title = format!("{} {}", MONTHS[m as usize - 1], y); let tw = a.gfx.measure(&title, Fmt::BodyStrong);
    a.gfx.text(&title, Fmt::BodyStrong, 222.0 + (174.0 - tw) / 2.0, 43.0, tw + 6.0, 16.0, TEXT, false);
    let cw = 174.0 / 7.0;
    for (i, c) in ["D", "S", "T", "Q", "Q", "S", "S"].iter().enumerate() { a.gfx.text(c, Fmt::Small, 222.0 + i as f32 * cw + cw / 2.0 - 3.0, 61.0, 12.0, 12.0, TEXT3, false); }
    let first = weekday(y, m, 1) as usize; let nd = days_in_month(y, m);
    let sel = a.p.task_date; let due_days: Vec<u32> = a.p.tasks.iter().filter(|t| !t.done).filter_map(|t| t.due_dt()).filter(|d| d.y == y && d.m == m).map(|d| d.d).collect();
    let rh = 10.6; // 6 semanas cabem (grade 73..136) com a linha do botao Hoje logo abaixo
    for d in 1..=nd {
        let idx = first + d as usize - 1; let (c, r) = (idx % 7, idx / 7);
        let (cx, cy) = (222.0 + c as f32 * cw + cw / 2.0, 78.0 + r as f32 * rh);
        let is_sel = sel == Some((y, m, d)); let is_today = (y, m, d) == (now.y, now.m, now.d);
        if is_sel { a.gfx.circle(cx, cy, 6.0, MINT); } else if is_today { a.gfx.ring(cx, cy, 5.6, 1.0, MINT); }
        let s = d.to_string(); let w = a.gfx.measure(&s, Fmt::Small);
        a.gfx.text(&s, Fmt::Small, cx - w / 2.0, cy - 7.0, w + 4.0, 12.0, if is_sel { rgba(0x000000, 1.0) } else { TEXT }, false);
        if due_days.contains(&d) && !is_sel { a.gfx.circle(cx + 6.0, cy + 4.5, 2.0, ACCENT); } // marcador de dia com tarefa (4 px)
        hits.push((R { x: cx - cw / 2.0 + off, y: cy - rh / 2.0, w: cw, h: rh }, Act::TaskDay(d)));
    }
    // botao "Hoje" (leva ao mes atual e seleciona o dia)
    let hb = Act::TaskToday; a.gfx.rrect(344.0, 134.0, 48.0, 12.0, 6.0, if a.hover == Some(hb) { SURFACE_H } else { SURFACE });
    let hw = a.gfx.measure("Hoje", Fmt::Small); a.gfx.text("Hoje", Fmt::Small, 344.0 + (48.0 - hw) / 2.0, 132.5, hw + 4.0, 13.0, TEXT, false);
    hits.push((R { x: 344.0 + off, y: 132.0, w: 48.0, h: 16.0 }, hb));
    if let Some((sy, sm, sd)) = sel { a.gfx.text(&format!("{} {} {}", sd, MON3[sm as usize - 1], sy), Fmt::Small, 228.0, 132.5, 110.0, 13.0, TEXT2, false); }
}

fn open_input(a: &mut App, text: String) {
    a.p.task_input = Some(text); a.p.task_cal = false;
    unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(a.hwnd); let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(a.hwnd)); }
}
pub fn click(a: &mut App, act: Act) {
    match act {
        Act::TaskAdd => { if a.p.task_input.is_some() { a.p.task_input = None; } else { a.p.task_edit_id = None; a.p.task_edit_mode = false; a.p.task_date = None; open_input(a, String::new()); } }
        Act::TaskEdit => { a.p.task_edit_mode = !a.p.task_edit_mode; a.p.task_input = None; a.p.task_cal = false; }
        Act::TaskCal => { a.p.task_cal = !a.p.task_cal; a.retarget(std::time::Instant::now(), 200.0); if a.p.task_cal && a.p.task_month.1 == 0 { let n = now_local(); a.p.task_month = (a.p.task_date.map_or(n.y, |d| d.0), a.p.task_date.map_or(n.m, |d| d.1)); } }
        Act::TaskToday => { let n = now_local(); a.p.task_month = (n.y, n.m); a.p.task_date = Some((n.y, n.m, n.d)); a.p.task_cal = false; a.retarget(std::time::Instant::now(), 200.0); if a.p.task_input.is_none() { open_input(a, String::new()); } }
        Act::TaskMonth(d) => { let (mut y, mut m) = a.p.task_month; let mi = m as i32 - 1 + d as i32; y += mi.div_euclid(12); m = mi.rem_euclid(12) as u32 + 1; a.p.task_month = (y, m); }
        Act::TaskDay(d) => { let (y, m) = a.p.task_month; a.p.task_date = if a.p.task_date == Some((y, m, d)) { None } else { Some((y, m, d)) }; a.p.task_cal = false; a.retarget(std::time::Instant::now(), 200.0);
            if a.p.task_input.is_none() { open_input(a, String::new()); } }
        Act::TaskToggle(i) => { if let Some(t) = a.p.tasks.get(i as usize) { a.p.task_focus = Some(t.id); toggle(t.id); } focus_window(a); }
        Act::TaskRemove(i) => { if let Some(t) = a.p.tasks.get(i as usize) { remove(t.id); } }
        Act::TaskPick(i) => { if let Some(t) = a.p.tasks.get(i as usize).cloned() {
            a.p.task_focus = Some(t.id);
            a.p.task_edit_id = Some(t.id); a.p.task_date = t.due_dt().map(|d| (d.y, d.m, d.d));
            let txt = match t.due_dt() { Some(d) => format!("{} {:02}:{:02}", t.text, d.h, d.mi), None => t.text }; open_input(a, txt); } }
        _ => {}
    }
}

/// Teclado do campo de tarefa (WM_CHAR/WM_KEYDOWN encaminhados pela ilha). true = tratou.
fn focus_window(a: &App) { unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(a.hwnd); let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(a.hwnd)); } }
pub fn key(a: &mut App, vk: u32, ch: Option<char>) -> bool {
    if a.p.task_input.is_none() { // Delete exclui a linha focada (a ultima clicada)
        if vk == 0x2E && a.page == crate::pages::P_TASKS { if let Some(id) = a.p.task_focus.take() { remove(id); a.dirty = true; return true; } }
        return false;
    }
    let Some(buf) = a.p.task_input.as_mut() else { return false };
    match (vk, ch) {
        (0x1B, _) => { if a.p.task_cal { a.p.task_cal = false; a.retarget(std::time::Instant::now(), 200.0); } else { a.p.task_input = None; a.p.task_edit_id = None; } }
        (0x0D, _) => {
            let raw = std::mem::take(buf); let (text, time) = split_time(&raw);
            if !text.is_empty() {
                let due = due_from(a.p.task_date, time, now_local());
                match a.p.task_edit_id.take() { Some(id) => { edit(id, &text, due); a.p.task_input = None; } None => { add(&text, due); a.p.task_input = Some(String::new()); a.p.task_date = None; } } // adicionar mantem o campo aberto
            } else { a.p.task_input = Some(String::new()); }
        }
        (0x08, _) => { buf.pop(); }
        (_, Some(c)) if !c.is_control() && buf.chars().count() < 80 => buf.push(c),
        _ => {}
    }
    a.dirty = true; true
}


#[cfg(test)]
mod tests {
    use super::*;
    fn t(id: u64, due: Option<&str>, done: bool) -> Task { Task { id, text: format!("t{id}"), done, done_on: done.then(|| "2026-11-02".into()), due: due.map(Into::into), reminded: false } }
    #[test] fn calendario() {
        assert_eq!(days_in_month(2024, 2), 29); assert_eq!(days_in_month(2026, 2), 28); assert_eq!(days_in_month(2100, 2), 28); assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(weekday(2026, 11, 2), 1); // segunda
        assert_eq!(weekday(2024, 2, 29), 4); // quinta
        assert_eq!(Dt { y: 2026, m: 12, d: 31, h: 10, mi: 0, s: 0 }.plus_days(1).date_str(), "2027-01-01");
        assert_eq!(Dt { y: 2026, m: 3, d: 1, h: 0, mi: 0, s: 0 }.plus_days(-1).date_str(), "2026-02-28");
    }
    #[test] fn dt_parse_e_ordem() {
        let a = Dt::parse("2026-11-02T14:30").unwrap(); assert_eq!((a.y, a.h, a.mi), (2026, 14, 30)); assert_eq!(a.fmt(), "2026-11-02T14:30");
        assert!(Dt::parse("2026-02-30T10:00").is_none() && Dt::parse("2026-11-02T24:00").is_none() && Dt::parse("lixo").is_none());
        assert!(Dt::parse("2026-11-02T14:30").unwrap() < Dt::parse("2026-11-02T14:31").unwrap());
        assert_eq!(Dt::parse("2026-11-03T00:00").unwrap().secs() - a.secs(), 9 * 3600 + 30 * 60);
    }
    #[test] fn hora_no_fim_do_texto() {
        assert_eq!(split_time("Ligar 14:30"), ("Ligar".into(), Some((14, 30))));
        assert_eq!(split_time("Remedio @9:05"), ("Remedio".into(), Some((9, 5))));
        assert_eq!(split_time("Reuniao 25:99"), ("Reuniao 25:99".into(), None));
        assert_eq!(split_time("Comprar pao"), ("Comprar pao".into(), None));
        assert_eq!(split_time("14:30"), ("".into(), Some((14, 30))));
    }
    #[test] fn concluidas_vao_para_o_fim_em_ordem_estavel() {
        let v = vec![t(1, None, true), t(2, None, false), t(3, None, true), t(4, None, false)];
        assert_eq!(sorted(v).iter().map(|x| x.id).collect::<Vec<_>>(), vec![2, 4, 1, 3]);
    }
    #[test] fn vencimento() {
        let now = Dt { y: 2026, m: 11, d: 2, h: 15, mi: 0, s: 0 };
        assert_eq!(due_from(None, None, now), None);
        assert_eq!(due_from(None, Some((16, 0)), now).unwrap().fmt(), "2026-11-02T16:00");
        assert_eq!(due_from(None, Some((9, 0)), now).unwrap().fmt(), "2026-11-03T09:00"); // ja passou: amanha
        assert_eq!(due_from(Some((2026, 12, 25)), None, now).unwrap().fmt(), "2026-12-25T09:00");
        assert_eq!(due_from(Some((2026, 12, 25)), Some((18, 30)), now).unwrap().fmt(), "2026-12-25T18:30");
    }
    #[test] fn contagem_e_poda() {
        let mut v = vec![t(1, None, false), t(2, None, true), { let mut x = t(3, None, true); x.done_on = Some("2026-11-01".into()); x }];
        assert_eq!(counts(&v, "2026-11-02"), (1, 2));
        assert!(prune(&mut v, "2026-11-02")); assert_eq!(v.len(), 2); // a concluida de ontem some
        assert_eq!(counts(&v, "2026-11-03"), (0, 1)); // meia-noite zera
    }
    #[test] fn lembretes() {
        let now = Dt { y: 2026, m: 11, d: 2, h: 15, mi: 0, s: 0 };
        let mut v = vec![t(1, Some("2026-11-02T14:59"), false), t(2, Some("2026-11-02T15:00"), false), t(3, Some("2026-11-02T15:01"), false), t(4, Some("2026-11-02T10:00"), true), t(5, None, false)];
        assert_eq!(due_now(&v, now), vec![1, 2]);
        v[0].reminded = true; assert_eq!(due_now(&v, now), vec![2]);
        assert_eq!(next_due(&v[2..], now), Some(60));
        assert_eq!(next_due(&[t(5, None, false)], now), None);
    }
}
