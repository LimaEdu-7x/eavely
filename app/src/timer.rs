// Pomodoro: maquina de estados pura (tempo injetado em ms) + thread que dorme ate o fim (sem polling).
use serde::{Deserialize, Serialize};
use std::{sync::{Condvar, Mutex, OnceLock}, time::{Duration, Instant}};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode { Focus, Short, Long }
#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status { Idle, Running, Paused, Done }

#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TimerState { pub mode: Mode, pub status: Status, pub remaining_sec: u32, pub total_sec: u32, pub cycle: u32, pub custom: bool }

pub struct Timer { mode: Mode, status: Status, total: u32, left_ms: u64, deadline: u64, cycle: u32 }

fn default_sec(m: Mode) -> u32 { let c = crate::settings::timer(); 60 * match m { Mode::Focus => c.focus_min, Mode::Short => c.short_min, Mode::Long => c.long_min } }

impl Timer {
    pub const fn new() -> Self { Timer { mode: Mode::Focus, status: Status::Idle, total: 25 * 60, left_ms: 25 * 60 * 1000, deadline: 0, cycle: 0 } }
    pub fn start(&mut self, mode: Mode, dur: Option<u32>, now: u64) {
        self.mode = mode;
        self.total = dur.filter(|d| *d > 0).unwrap_or_else(|| default_sec(mode));
        self.left_ms = self.total as u64 * 1000;
        self.deadline = now + self.left_ms;
        self.status = Status::Running;
    }
    pub fn pause(&mut self, now: u64) {
        if self.status == Status::Running { self.left_ms = self.deadline.saturating_sub(now); self.status = Status::Paused; }
    }
    pub fn resume(&mut self, now: u64) {
        if self.status == Status::Paused { self.deadline = now + self.left_ms; self.status = Status::Running; }
    }
    /// Troca o modo sem iniciar; so com timer parado (Idle/Done). Devolve false se estava rodando/pausado.
    pub fn set_mode(&mut self, mode: Mode) -> bool {
        if !matches!(self.status, Status::Idle | Status::Done) { return false; }
        self.mode = mode; self.total = default_sec(mode); self.left_ms = self.total as u64 * 1000; self.status = Status::Idle;
        true
    }
    /// Valor personalizado (so parado): mantem o modo, troca o total.
    pub fn set_custom(&mut self, sec: u32) -> bool {
        if !matches!(self.status, Status::Idle | Status::Done) || sec == 0 { return false; }
        self.total = sec; self.left_ms = sec as u64 * 1000; self.status = Status::Idle; true
    }
    pub fn is_default_total(&self) -> bool { self.total == default_sec(self.mode) }
    pub fn reset(&mut self) { self.status = Status::Idle; self.left_ms = self.total as u64 * 1000; }
    /// Instante absoluto do fim (so rodando).
    pub fn deadline(&self) -> Option<u64> { (self.status == Status::Running).then_some(self.deadline) }
    /// true uma unica vez, quando passa de Running para Done.
    pub fn poll(&mut self, now: u64) -> bool {
        if self.status != Status::Running || now < self.deadline { return false; }
        self.status = Status::Done; self.left_ms = 0;
        if self.mode == Mode::Focus { self.cycle += 1; } // ciclos de FOCO concluidos na sessao (Adendo 2)
        true
    }
    pub fn state(&self, now: u64) -> TimerState {
        let ms = if self.status == Status::Running { self.deadline.saturating_sub(now) } else { self.left_ms };
        TimerState { mode: self.mode, status: self.status, remaining_sec: ms.div_ceil(1000) as u32, total_sec: self.total, cycle: self.cycle, custom: !self.is_default_total() }
    }
}

static TIMER: Mutex<Timer> = Mutex::new(Timer::new());
static CV: Condvar = Condvar::new();
/// Atalho para a UI nativa: aplica a mudanca e acorda a thread do fim.
pub fn ui(f: impl FnOnce(&mut Timer, u64)) { let _ = apply(f); }
/// Iniciar/pausar/retomar (botao play, atalho global)
pub fn toggle_play() {
    let t = timer_get().unwrap();
    ui(move |tm, now| match t.status { Status::Idle => { tm.start(t.mode, Some(t.total_sec), now); } Status::Running => tm.pause(now), Status::Paused => tm.resume(now), Status::Done => { tm.start(Mode::Focus, None, now); } });
}
fn now() -> u64 { static T0: OnceLock<Instant> = OnceLock::new(); T0.get_or_init(Instant::now).elapsed().as_millis() as u64 }
fn lock() -> std::sync::MutexGuard<'static, Timer> { TIMER.lock().unwrap_or_else(|e| e.into_inner()) }



/// Thread que dorme ate o fim do timer; acorda por start/pause/reset.
pub fn init() {
    std::thread::spawn(move || {
        let mut g = lock();
        loop {
            let t = now();
            if g.poll(t) {
                let s = g.state(t);
                drop(g);
                crate::island::wake();
                let cfg = crate::settings::timer();
                if cfg.notify { toast(s.mode); }
                if cfg.auto_next { // proximo ciclo sozinho: foco -> pausa curta (longa a cada N focos) -> foco
                    let next = if s.mode == Mode::Focus { if s.cycle > 0 && s.cycle % cfg.cycles_to_long == 0 { Mode::Long } else { Mode::Short } } else { Mode::Focus };
                    let _ = apply(|t, n| { t.start(next, None, n); });
                }
                if cfg.notify { beep(); }
                g = lock();
                continue;
            }
            g = match g.deadline() {
                None => CV.wait(g).unwrap_or_else(|e| e.into_inner()),
                Some(d) => CV.wait_timeout(g, Duration::from_millis(d.saturating_sub(t))).unwrap_or_else(|e| e.into_inner()).0,
            };
        }
    });
}

/// Toast nativo (WinRT). App sem pacote precisa de AUMID registrado em HKCU; sem isso o Windows descarta o toast. Falha = so beep+ponto.
fn toast(mode: Mode) {
    let (t, b) = if mode == Mode::Focus { ("Foco concluído", "Hora de uma pausa.") } else { ("Pausa concluída", "Hora de voltar ao foco.") };
    notify(t, b);
}
/// Beep triplo (aviso sonoro)
pub fn beep() { std::thread::spawn(|| for _ in 0..3 { unsafe { let _ = windows::Win32::System::Diagnostics::Debug::MessageBeep(windows::Win32::UI::WindowsAndMessaging::MB_ICONEXCLAMATION); } std::thread::sleep(std::time::Duration::from_millis(500)); }); }
/// Toast nativo com titulo e texto (usado pelo timer e pelos lembretes de tarefas)
pub fn notify(title: &str, body: &str) {
    use windows::{core::{w, HSTRING}, Data::Xml::Dom::XmlDocument, UI::Notifications::*, Win32::System::{Com::{CoInitializeEx, COINIT_MULTITHREADED}, Registry::*}};
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let (t, b) = (esc(title), esc(body));
    const ID: &str = "app.eavely";
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let name: Vec<u16> = "Eavely\0".encode_utf16().collect();
        let _ = RegSetKeyValueW(HKEY_CURRENT_USER, w!(r"Software\Classes\AppUserModelId\app.eavely"), w!("DisplayName"), REG_SZ.0, Some(name.as_ptr() as *const _), (name.len() * 2) as u32);
        let r = (|| -> windows::core::Result<()> {
            let d = XmlDocument::new()?;
            d.LoadXml(&HSTRING::from(format!("<toast><visual><binding template='ToastGeneric'><text>{t}</text><text>{b}</text></binding></visual></toast>")))?;
            ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(ID))?.Show(&ToastNotification::CreateToastNotification(&d)?)
        })();
        if let Err(e) = r { crate::log(&format!("toast: {e}")); }
    });
}

fn apply(f: impl FnOnce(&mut Timer, u64)) -> Result<TimerState, String> {
    let t = now();
    let s = { let mut g = lock(); f(&mut g, t); g.state(t) };
    CV.notify_all();
    crate::island::wake();
    Ok(s)
}

pub fn timer_get() -> Result<TimerState, String> { Ok(lock().state(now())) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn defaults_e_contagem() {
        let mut t = Timer::new();
        assert_eq!(t.state(0).status, Status::Idle);
        t.start(Mode::Focus, None, 1000);
        let s = t.state(1000); assert_eq!((s.remaining_sec, s.total_sec, s.status), (1500, 1500, Status::Running));
        assert_eq!(t.state(1000 + 1).remaining_sec, 1500); // arredonda para cima
        assert_eq!(t.state(1000 + 61_000).remaining_sec, 1439);
    }
    #[test] fn duracao_customizada_e_zero_cai_no_padrao() {
        let mut t = Timer::new();
        t.start(Mode::Short, Some(90), 0); assert_eq!(t.state(0).total_sec, 90);
        t.start(Mode::Short, Some(0), 0); assert_eq!(t.state(0).total_sec, 300);
        t.start(Mode::Long, None, 0); assert_eq!(t.state(0).total_sec, 900);
    }
    #[test] fn pausa_congela_e_retoma() {
        let mut t = Timer::new();
        t.start(Mode::Focus, Some(100), 0);
        t.pause(30_000);
        assert_eq!((t.state(99_000).status, t.state(99_000).remaining_sec), (Status::Paused, 70));
        assert_eq!(t.deadline(), None);
        t.resume(100_000);
        assert_eq!(t.state(110_000).remaining_sec, 60);
        assert_eq!(t.deadline(), Some(170_000));
    }
    #[test] fn pause_resume_em_estado_errado_nao_faz_nada() {
        let mut t = Timer::new();
        t.pause(0); t.resume(0); assert_eq!(t.state(0).status, Status::Idle);
        t.start(Mode::Focus, Some(10), 0); t.resume(5000); assert_eq!(t.state(5000).remaining_sec, 5);
    }
    #[test] fn fim_dispara_uma_vez() {
        let mut t = Timer::new();
        t.start(Mode::Focus, Some(10), 0);
        assert!(!t.poll(9_999));
        assert!(t.poll(10_000));
        assert!(!t.poll(20_000));
        let s = t.state(20_000); assert_eq!((s.status, s.remaining_sec, s.cycle), (Status::Done, 0, 1));
    }
    #[test] fn ciclo_conta_so_focos_concluidos_e_nao_zera() {
        let mut t = Timer::new();
        for i in 1..=4 { t.start(Mode::Focus, Some(1), 0); t.poll(1000); assert_eq!(t.state(0).cycle, i);
            if i < 4 { t.start(Mode::Short, Some(1), 0); t.poll(1000); assert_eq!(t.state(0).cycle, i); } }
        t.start(Mode::Long, Some(1), 0); t.poll(1000);
        assert_eq!(t.state(0).cycle, 4);
        t.start(Mode::Focus, Some(10), 0); t.reset(); // reset/abandono nao conta
        assert_eq!(t.state(0).cycle, 4);
    }
    #[test] fn set_mode_so_parado_e_nao_inicia() {
        let mut t = Timer::new();
        assert!(t.set_mode(Mode::Long));
        let s = t.state(0); assert_eq!((s.mode, s.status, s.total_sec, s.remaining_sec), (Mode::Long, Status::Idle, 900, 900));
        assert_eq!(t.deadline(), None);
        t.start(Mode::Short, Some(5), 0);
        assert!(!t.set_mode(Mode::Focus)); // rodando
        t.pause(1000); assert!(!t.set_mode(Mode::Focus)); // pausado
        t.resume(1000); t.poll(10_000); // done
        assert!(t.set_mode(Mode::Focus)); assert_eq!(t.state(0).status, Status::Idle);
    }
    #[test] fn reset_volta_ao_total_sem_perder_ciclo() {
        let mut t = Timer::new();
        t.start(Mode::Focus, Some(1), 0); t.poll(1000);
        t.start(Mode::Focus, Some(60), 0); t.reset();
        let s = t.state(5000); assert_eq!((s.status, s.remaining_sec, s.cycle), (Status::Idle, 60, 1));
        assert_eq!(t.deadline(), None);
    }
    #[test] fn pausa_apos_prazo_nao_estoura() {
        let mut t = Timer::new();
        t.start(Mode::Focus, Some(1), 0); t.pause(5000);
        assert_eq!(t.state(5000).remaining_sec, 0);
    }
}
