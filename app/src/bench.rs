// Roteiro de teste/benchmark (EAVELY_BENCH / EAVELY_SHOTS=pasta): percorre os estados, tira screenshots (PrintWindow, sem clicar em nada)
// e deixa as metricas de quadro no log (linhas "PERF"). So roda com essas variaveis de ambiente.
use std::{sync::atomic::{AtomicU32, Ordering::Relaxed}, time::Instant};
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
use crate::island::{App, St};

static STEP: AtomicU32 = AtomicU32::new(0);
/// EAVELY_FRAMELOG=arquivo.csv : uma linha por quadro desenhado (t_ms,dt_ms,largura,altura,estado,pagina) + marcadores "#". Prova de suavidade (60 fps) e das curvas das molas.
static FLOG: std::sync::OnceLock<std::sync::Mutex<Option<std::fs::File>>> = std::sync::OnceLock::new();
fn flog_file() -> &'static std::sync::Mutex<Option<std::fs::File>> { FLOG.get_or_init(|| std::sync::Mutex::new(std::env::var("EAVELY_FRAMELOG").ok().and_then(|p| std::fs::File::create(p).ok()))) }
pub fn flog(t: f32, dt: f32, w: f32, h: f32, st: &str, page: usize) { use std::io::Write; if let Some(f) = flog_file().lock().unwrap().as_mut() { let _ = writeln!(f, "{t:.1},{dt:.2},{w:.2},{h:.2},{st},{page}"); } }
pub fn flog_mark(s: &str) { use std::io::Write; if let Some(f) = flog_file().lock().unwrap().as_mut() { let _ = writeln!(f, "# {s}"); } }

fn scenario(a: &mut App, i: u32) -> Option<&'static str> {
    let now = Instant::now();
    let setp = |a: &mut App, p: usize| { a.page = p; a.prev_page = p; a.page_t = crate::island::Spring::new(1.0); };
    let open = |a: &mut App, p: usize, split: bool, dock: bool| {
        a.force = Some(St::Open); a.evaluate_pub(); setp(a, p); a.split = split; a.split_t = crate::island::Tw::new(if split { 1.0 } else { 0.0 }); // evaluate primeiro: ao abrir, o split volta a fechado (comportamento real)
        a.dock = dock; a.dock_t = crate::island::Tw::new(if dock { 1.0 } else { 0.0 }); a.drag_over = false; a.p.edit = None; a.p.select = false; a.p.vol_t = None;
        a.retarget(Instant::now(), 0.0);
    };
    Some(match i {
        0 => { a.force = Some(St::Line); "01-traco" }
        1 => { a.force = Some(St::Peek); "02-peek" }
        2 => { a.force = Some(St::Live); "03-ao-vivo-compacto-musica" }
        3 => { crate::pages::set_live_expanded(true); a.force = Some(St::Live); a.retarget(now, 0.0); "04-ao-vivo-expandido-musica" }
        4 => { crate::pages::set_live_expanded(false); open(a, 1, false, false); "05-musica" }
        5 => { open(a, 1, false, true); "06-musica-dock" }
        6 => { open(a, 1, true, false); "07-musica-letras" }
        7 => { open(a, 1, true, true); "08-musica-letras-dock" }
        8 => { open(a, 1, false, false); a.p.vol = crate::vol::get(); a.p.vol_t = Some(now + std::time::Duration::from_secs(60)); "09-musica-volume" }
        9 => { a.p.vol_t = None; crate::media::fake_none(); "10-musica-nada-tocando" }
        10 => { crate::media::fake_again(); crate::pages::debug_lyr(3); "11-musica-sem-letra" }
        11 => { open(a, 1, true, false); crate::pages::debug_lyr(1); "12-musica-letras-carregando" }
        12 => { open(a, 3, false, false); crate::pages::debug_lyr(3); "13-pomodoro-parado" }
        13 => { crate::timer::ui(|t, n| t.start(crate::timer::Mode::Focus, Some(754), n)); "14-pomodoro-rodando-personalizado" }
        14 => { crate::timer::ui(|t, _| t.reset()); a.p.edit = Some("2000".into()); "15-pomodoro-editando" }
        15 => { a.p.edit = None; crate::timer::ui(|t, n| { t.start(crate::timer::Mode::Focus, Some(1), n); }); "16-pomodoro-fim" }
        16 => { crate::timer::ui(|t, _| t.reset()); crate::shelf::clear_native(); open(a, 2, false, false); "17-shelf-vazio" }
        17 => { a.drag_over = true; "18-shelf-arrastando" }
        18 => { a.drag_over = false;
            let d = std::env::temp_dir().join("eavely-shots"); let _ = std::fs::create_dir_all(&d);
            let mut v = Vec::new(); for (n, sz) in [("relatorio-final.pdf", 1_843_200usize), ("foto.png", 312_000), ("notas.txt", 2_100)] { let f = d.join(n); let _ = std::fs::write(&f, vec![0u8; sz]); v.push(f); }
            v.push(d.clone()); crate::shelf::add_native(v); "19-shelf-itens" }
        19 => { a.p.select = true; a.p.sel = vec![0, 2]; "20-shelf-selecionar" }
        20 => { crate::shelf::clear_native(); a.p.select = false; open(a, 4, false, false); "21-tarefas-vazio" }
        23 => { let n = crate::tasks::now_local();
            crate::tasks::add("Enviar relatorio", crate::tasks::due_from(None, Some((17, 30)), n));
            let id = crate::tasks::add("Comprar pao", None); crate::tasks::toggle(id);
            crate::tasks::add("Ligar para o João", crate::tasks::due_from(None, Some(((n.h + 3) % 24, 0)), n));
            crate::tasks::debug_overdue("Reunião de alinhamento com o time", n.plus_days(-1));
            crate::tasks::add("Revisar o relatório", None); crate::tasks::add("Pagar a conta de luz", Some(n.plus_days(2)));
            open(a, 4, false, false); "24-tarefas-itens" }
        24 => { a.p.task_edit_mode = true; "25-tarefas-editar" }
        25 => { a.p.task_edit_mode = false; a.p.task_input = Some("Reuniao de equipe 14:30".into()); a.p.task_date = Some((crate::tasks::now_local().y, crate::tasks::now_local().m, crate::tasks::now_local().d + 0)); "26-tarefas-adicionando" }
        26 => { a.p.task_input = None; a.p.task_cal = true; a.retarget(Instant::now(), 0.0); "27-tarefas-calendario" }
        27 => { a.p.task_cal = false; a.retarget(Instant::now(), 0.0); a.p.task_scroll = 1; "28-tarefas-rolada" }
        31 => { crate::settings_ui::debug_scroll(0); "skip" }
        32 => { let n = crate::tasks::now_local(); a.force = Some(St::Open); a.p.task_scroll = 0; a.p.task_cal = true; a.p.task_date = Some((n.y, n.m, if n.d < 25 { n.d + 3 } else { n.d - 3 })); a.p.task_month = (n.y, n.m); a.retarget(Instant::now(), 0.0); "33-tarefas-dia-selecionado" }
        28 => { a.force = Some(St::Line); crate::settings_ui::open(); "skip" }
        29 | 30 => "skip",
        33 => { crate::overlay::show("volume-2", "Volume", "", Some(0.65), crate::gfx::rgba(0x7CC4FF, 1.0)); a.force = Some(St::Over); "34-overlay-volume" }
        34 => { crate::overlay::show("battery-charging", "Carregando", "64%", None, crate::gfx::rgba(0x7BE0A4, 1.0)); "35-overlay-bateria" }
        35 => { crate::overlay::show("battery-low", "Bateria fraca", "18%", None, crate::gfx::rgba(0xFFB86B, 1.0)); "36-overlay-bateria-fraca" }
        36 => { crate::overlay::show("wifi-off", "Sem conexão", "", None, crate::gfx::rgba(0xFF6B6B, 1.0)); "37-overlay-wifi-sem-conexao" }
        37 => { crate::overlay::show("headphones", "Fones conectados", "Headphones (USB Audio)", None, crate::gfx::rgba(0xB69CFF, 1.0)); "38-overlay-fones" }
        38 => { crate::overlay::show("arrow-big-up", "Caps Lock ativado", "", None, crate::gfx::rgba(0xFFFFFF, 1.0)); "39-overlay-caps-lock" }
        39 => { crate::overlay::show("volume-x", "Volume", "", Some(0.0), crate::gfx::rgba(0xFFFFFF, 0.52)); "40-overlay-volume-mudo" }
        40 => { crate::settings::update(|s| s.collapsed_look = "eyes".into()); a.p.eye_move = None; a.force = Some(St::Peek); "41-olhos-ociosos" }
        41 => { a.p.eye_move = Some(Instant::now() - std::time::Duration::from_secs(61)); "42-olhos-dormindo" }
        42 => { crate::settings::update(|s| s.collapsed_look = "clock".into()); a.force = Some(St::Peek); "43-peek-bateria-hora" }
        43 => { crate::overlay::show("wifi", "Conectado", "", None, crate::gfx::rgba(0x7CC4FF, 1.0)); a.force = Some(St::Over); "44-overlay-wifi-conectado" }
        44 => { crate::overlay::show("arrow-big-up", "Caps Lock desativado", "", None, crate::gfx::rgba(0xFFFFFF, 0.52)); "45-overlay-caps-desativado" }
        45 => { crate::overlay::show("battery-charging", "Carga completa", "100%", None, crate::gfx::rgba(0x7BE0A4, 1.0)); "46-overlay-bateria-cheia" }
        46 => { crate::overlay::show("volume-2", "Volume", "", Some(0.92), crate::gfx::rgba(0x7CC4FF, 1.0)); "47-overlay-volume-alto" }
        47 => { crate::overlay::show("battery-low", "Bateria fraca", "18% · Conecte o carregador", None, crate::gfx::rgba(0xFFB86B, 1.0)); "48-overlay-bateria-fraca-carregador" }
        21 => { crate::media::fake_again(); crate::media::fake_cover([120, 230, 255]); open(a, 1, false, false); "22-musica-gradiente-capa-clara" }
        22 => { crate::media::fake_cover([90, 30, 20]); "23-musica-gradiente-capa-escura" }
        48 => { crate::shelf::clear_native();
            let d = std::env::temp_dir().join("eavely-shots"); let _ = std::fs::create_dir_all(&d);
            let mut v = Vec::new(); for (n, sz) in [("relatorio-final.pdf", 1_258_291usize), ("foto-praia.png", 859_000), ("notas.txt", 306), ("contrato-assinado-2026.pdf", 98_304)] { let f = d.join(n); let _ = std::fs::write(&f, vec![0u8; sz]); v.push(f); }
            v.push(d.clone()); crate::shelf::add_native(v); open(a, 2, false, false); a.p.items_at = None; a.p.shelf_new.clear(); "49-shelf-com-itens-hover-x" }
        49 => { a.p.select = true; a.p.sel.clear(); "50-shelf-selecao-nenhum-marcado" }
        // as capturas saem 1500 ms depois do cenario (intervalo do roteiro): o inicio da animacao e adiado para o quadro capturado cair no meio dela
        50 => { a.p.select = false; a.p.sel.clear(); a.p.shelf = crate::shelf::list_native(); let t0 = Instant::now() + std::time::Duration::from_millis(1250); if let Some(it) = a.p.shelf.get(1) { a.p.shelf_del.push((it.id.clone(), t0, true)); } "51-shelf-excluir-um-meio" }
        51 => { a.p.shelf_del.clear(); a.p.select = false; crate::shelf::clear_native(); let d = std::env::temp_dir().join("eavely-shots"); let v: Vec<_> = ["relatorio-final.pdf", "foto-praia.png", "notas.txt", "contrato-assinado-2026.pdf"].iter().map(|n| d.join(n)).collect(); crate::shelf::add_native(v); a.p.shelf = crate::shelf::list_native(); let t0 = Instant::now() + std::time::Duration::from_millis(1350); for (k, it) in a.p.shelf.clone().iter().enumerate() { a.p.shelf_del.push((it.id.clone(), t0 + std::time::Duration::from_millis(30 * k as u64), false)); } "52-shelf-limpar-tudo-voando" }
        52 => { a.p.shelf_del.clear(); crate::shelf::clear_native(); a.p.shelf = Vec::new(); a.p.shelf_n = 0; a.p.empty_at = Some(Instant::now() + std::time::Duration::from_millis(1350)); "53-shelf-vazio-fadein" }
        53 => { crate::media::fake_again(); crate::pages::debug_fake_lines(); let _ = crate::media::media_seek(3000); crate::pages::set_live_expanded(false); a.force = Some(St::Live); "54-barra-lacuna-intro" }
        54 => { crate::pages::debug_instrumental(); "55-barra-instrumental" }
        55 => { crate::media::fake_again(); crate::pages::debug_fake_lines(); let _ = crate::media::media_seek(40000); open(a, 1, true, false); "56-letras-linha-atual-destacada" }
        56 => { crate::media::fake_again(); crate::media::fake_cover([120, 60, 200]); open(a, 1, false, false); let t = Instant::now() + std::time::Duration::from_millis(1435); a.p.spin = Some(crate::pages::Spin { t, cover: a.p.cover.clone(), tint: Some((0.5, 0.2, 0.1)), title: "Faixa anterior".into(), sub: "Outro Artista — Outro album".into() }); "57-giro-capa-encolhendo" }
        57 => { let t = Instant::now() + std::time::Duration::from_millis(1170); a.p.spin = Some(crate::pages::Spin { t, cover: a.p.cover.clone(), tint: Some((0.5, 0.2, 0.1)), title: "Faixa anterior".into(), sub: "Outro Artista — Outro album".into() }); "58-giro-capa-reabrindo" }
        58 => { crate::settings_ui::debug_page(0); "60-config-geral" }
        59 => { crate::settings_ui::debug_page(1); "61-config-ilha-e-aparencia" }
        60 => { crate::settings_ui::debug_page(2); "62-config-musica-e-letras" }
        61 => { crate::settings_ui::debug_page(3); "63-config-tarefas" }
        62 => { crate::settings_ui::debug_page(4); "64-config-pomodoro" }
        63 => { crate::settings_ui::debug_page(5); "65-config-shelf" }
        64 => { crate::settings_ui::debug_page(6); "66-config-avisos" }
        65 => { crate::settings_ui::debug_page(7); "67-config-atalhos" }
        66 => { crate::settings_ui::debug_page(10); "68-config-sobre" }
        67 => { crate::settings::update(|s| { s.pc_slots = 1; s.pc_cpu = true; s.pc_ram = true; s.pc_gpu = true; s.pc_net = true; s.pc_state = true; }); crate::pages::pc_sample(); std::thread::sleep(std::time::Duration::from_millis(600)); crate::pages::pc_sample(); a.force = Some(St::Peek); "69-compacto-pc-1-dado" }
        68 => { crate::settings::update(|s| s.pc_slots = 2); crate::pages::pc_sample(); std::thread::sleep(std::time::Duration::from_millis(600)); crate::pages::pc_sample(); "70-compacto-pc-2-dados" }
        69 => { crate::settings::update(|s| { s.pc_cpu = false; s.pc_ram = false; s.pc_gpu = false; s.pc_slots = 1; }); crate::pages::pc_sample(); std::thread::sleep(std::time::Duration::from_millis(600)); crate::pages::pc_sample(); "71-compacto-pc-so-rede" }
        71 => { crate::settings::update(|s| { s.pc_cpu = false; s.pc_ram = false; s.pc_gpu = false; s.pc_net = false; s.pc_state = false; }); crate::media::fake_again(); let _ = crate::media::media_seek(1000); crate::pages::debug_long_line(); a.force = Some(St::Live); "72-barra-letras-linha-longa" }
        72 => { crate::settings::update(|s| { s.pc_cpu = true; s.pc_ram = true; s.pc_gpu = true; s.pc_net = true; s.pc_state = true; }); "skip" }
        73 => { crate::settings_ui::debug_page(8); "73-config-desempenho" }
        74 => { crate::media::fake_again(); crate::pages::debug_fake_lines(); let _ = crate::media::media_seek(40000); open(a, 1, true, false); "74-letras-rolagem-inicio" }
        75 => { let _ = crate::media::media_seek(52000); "75-letras-rolagem" }
        76 => { crate::media::fake_again(); crate::shelf::clear_native(); let d = std::env::temp_dir().join("eavely-shots"); let v: Vec<_> = ["relatorio-final.pdf", "foto-praia.png", "notas.txt", "contrato-assinado-2026.pdf"].iter().map(|n| d.join(n)).collect(); crate::shelf::add_native(v); a.p.shelf = crate::shelf::list_native(); crate::settings::update(|s| { s.slot_top = "shelf".into(); s.slot_bottom = "player".into(); }); open(a, 0, false, false); "76-mascote-feliz-musica" }
        77 => { crate::shelf::clear_native(); a.p.shelf = Vec::new(); crate::media::fake_none(); "77-mascote-neutro-slots-vazios" }
        78 => { crate::media::fake_again(); crate::droptarget::OVER.store(true, Relaxed); a.drag_over = true; "78-mascote-curioso-arrastando" }
        79 => { crate::droptarget::OVER.store(false, Relaxed); a.drag_over = false; let n = Instant::now(); for _ in 0..4 { a.p.mascot.on_click(n); } "79-mascote-irritado-4-cliques" }
        80 => { let n = Instant::now(); for _ in 0..4 { a.p.mascot.on_click(n); } "80-mascote-raiva-8-cliques" }
        81 => { let n = Instant::now(); for _ in 0..4 { a.p.mascot.on_click(n); } "81-mascote-furioso-12-cliques" }
        82 => { crate::settings::update(|s| { s.slot_top = "clock".into(); s.slot_bottom = "metrics".into(); }); "82-slots-relogio-e-metricas" }
        83 => { crate::settings::update(|s| { s.slot_top = "tasks".into(); s.slot_bottom = "timer".into(); }); "83-slots-tarefas-e-timer" }
        84 => { crate::settings::update(|s| { s.slot_top = "none".into(); s.slot_bottom = "player".into(); }); "84-slot-unico-player-alto" }
        86 => { crate::settings_ui::debug_page(9); "87-config-mascote-e-inicio" }
        87 => { crate::island::DOCK_FORCE.store(true, Relaxed); open(a, 0, false, true); "88-dock-5-abas-mascote" }
        88 => { open(a, 1, false, true); "89-dock-5-abas-musica" }
        89 => { crate::island::DOCK_FORCE.store(false, Relaxed); crate::settings::update(|s| s.size_preset = "small".into()); open(a, 0, false, false); "90-tamanho-pequeno-mascote" }
        90 => { open(a, 1, false, false); "91-tamanho-pequeno-musica" }
        91 => { open(a, 2, false, false); "92-tamanho-pequeno-arquivos" }
        92 => { crate::settings::update(|s| s.mascot_size = "small".into()); open(a, 0, false, false); "93-pequeno-mascote-pequeno" }
        93 => { crate::settings::update(|s| { s.size_preset = "large".into(); s.mascot_size = "auto".into(); }); open(a, 0, false, false); "94-tamanho-grande-mascote" }
        94 => { open(a, 1, false, false); "95-tamanho-grande-musica" }
        95 => { crate::settings::update(|s| { s.size_preset = "standard".into(); s.mascot_size = "auto".into(); }); open(a, 0, false, false); "skip" }
        85 => { crate::settings::update(|s| { s.slot_top = "shelf".into(); s.slot_bottom = "player".into(); }); "skip" }
        70 => { crate::settings::update(|s| { s.pc_cpu = true; s.pc_ram = true; s.pc_gpu = true; s.pc_state = true; }); "skip" }
        _ => return None,
    })
}

pub fn step(a: &mut App) {
    let i = STEP.fetch_add(1, Relaxed);
    let dir = std::env::var("EAVELY_SHOTS").ok();
    if i < 220 {
        if i % 2 == 0 { let k = i / 2; match scenario(a, k) { Some(n) => { crate::log(&format!("BENCH cenario {n}")); flog_mark(n); NAME.with(|c| *c.borrow_mut() = n); } None => { STEP.store(220, Relaxed); } } a.evaluate_pub(); }
        else if let Some(d) = dir { let n = NAME.with(|c| *c.borrow()); if n == "skip" { } else if a.fs.iter().any(|f| *f) { crate::log("screenshot ignorado: ha janela em tela cheia (nao capturo outros apps)"); } else { unsafe { shot(a, &format!("{d}/{n}.bmp")); } } }
        return;
    }
    // fase de benchmark: abre/fecha repetidamente (animacao de forma + conteudo)
    let k = i - 220;
    if k == 0 { unsafe { SetTimer(Some(a.hwnd), 4, 450, None); } crate::timer::ui(|t, _| t.reset()); a.page = 1; a.prev_page = 1; a.split = false; a.dock = false; a.dock_t = crate::island::Tw::new(0.0); }
    if k < 40 {
        flog_mark(&format!("k={k} {}", ["abrir", "dock", "fechar", "ao-vivo"][(k % 4) as usize])); match k % 4 { 0 => a.force = Some(St::Open), 1 => { a.dock = true; a.dock_t.go(Instant::now(), 1.0, 200.0); a.retarget(Instant::now(), 200.0); } 2 => { a.dock = false; a.dock_t.go(Instant::now(), 0.0, 200.0); a.force = Some(St::Peek); } _ => a.force = Some(St::Live) }
        a.evaluate_pub();
    } else {
        crate::log("BENCH FIM");
        if std::env::var_os("EAVELY_BENCH_EXIT").is_some() { crate::fast_exit(0); }
        unsafe { let _ = KillTimer(Some(a.hwnd), 4); }
    }
}
thread_local! { static NAME: std::cell::RefCell<&'static str> = const { std::cell::RefCell::new("x") }; }

unsafe fn shot(a: &App, path: &str) {
    let cfg = path.contains("config");
    let target = if cfg { HWND(crate::settings_ui::hwnd() as *mut _) } else { a.hwnd };
    let mut wr0 = RECT::default(); let _ = GetWindowRect(target, &mut wr0);
    let (w, h) = if cfg { (wr0.right - wr0.left, wr0.bottom - wr0.top) } else { (a.gfx.size.0 as i32, a.gfx.size.1 as i32) };
    let hdc = GetDC(None); let mem = CreateCompatibleDC(Some(hdc));
    let bmi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, ..Default::default() }, ..Default::default() };
    let mut bits = std::ptr::null_mut();
    let Ok(bmp) = CreateDIBSection(Some(mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else { return };
    let old = SelectObject(mem, bmp.into());
    let px = std::slice::from_raw_parts_mut(bits as *mut u32, (w * h) as usize);
    // PrintWindow nao preserva o alfa de janelas DirectComposition (sai preto): copiamos a area da tela onde a janela esta (so leitura)
    let wr = wr0;
    // Fundo neutro: uma janela cinza propria fica atras da ilha/janela durante a captura, entao NENHUM conteudo de outros apps entra na imagem.
    let hi: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap().into();
    let back = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, windows::core::w!("STATIC"), windows::core::w!(""), WS_POPUP | WINDOW_STYLE(0x4 /* SS_GRAYRECT */), wr.left, wr.top, w, h, None, None, Some(hi), None).unwrap_or_default();
    let _ = SetWindowPos(back, Some(HWND_TOPMOST), wr.left, wr.top, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    let _ = SetWindowPos(target, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); // acima do fundo
    if IsWindowVisible(a.trace).as_bool() { let _ = SetWindowPos(a.trace, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); } // forma de repouso em GDI (janela propria) tambem acima do fundo
    let t0 = std::time::Instant::now(); let mut m = MSG::default();
    while t0.elapsed() < std::time::Duration::from_millis(250) { while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() { let _ = TranslateMessage(&m); DispatchMessageW(&m); } std::thread::sleep(std::time::Duration::from_millis(10)); }
    { let rg = CreateRectRgn(0, 0, 0, 0); let kind = GetWindowRgn(a.hwnd, rg); let mut bx = RECT::default(); let _ = GetRgnBox(rg, &mut bx); let _ = DeleteObject(rg.into());
      flog_mark(&format!("REGIAO {path} tipo={} caixa={}x{} (janela {}x{} px, forma {:.0}x{:.0} logicos)", kind.0, bx.right - bx.left, bx.bottom - bx.top, w, h, a.tw.pos, a.th.pos)); }
    let mut ok = BitBlt(mem, 0, 0, w, h, Some(hdc), wr.left, wr.top, SRCCOPY | CAPTUREBLT).is_ok();
    // Configuracoes (janela DirectComposition sem bitmap de redirecionamento): se a copia da tela veio preta, pede o conteudo composto da propria janela (PW_RENDERFULLCONTENT)
    if cfg && px.iter().all(|p| p & 0xFFFFFF == 0) { ok = windows::Win32::Storage::Xps::PrintWindow(target, mem, windows::Win32::Storage::Xps::PRINT_WINDOW_FLAGS(2)).as_bool(); crate::log(&format!("config: captura da tela preta, PrintWindow={ok} ({w}x{h})")); }
    let mut out = Vec::with_capacity((w * h * 3) as usize + 54);
    out.extend_from_slice(b"BM"); out.extend_from_slice(&((w * h * 3 + 54) as u32).to_le_bytes()); out.extend_from_slice(&[0; 4]); out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes()); out.extend_from_slice(&w.to_le_bytes()); out.extend_from_slice(&h.to_le_bytes()); out.extend_from_slice(&1u16.to_le_bytes()); out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in (0..h).rev() { for x in 0..w { let p = px[(y * w + x) as usize]; out.extend_from_slice(&[p as u8, (p >> 8) as u8, (p >> 16) as u8]); } } // linhas de baixo p/ cima, sem padding (w*3 multiplo de 4? ajustado abaixo)
    // BMP exige linhas alinhadas a 4 bytes
    if (w * 3) % 4 != 0 { let pad = (4 - (w * 3) % 4) as usize; let mut fixed = out[..54].to_vec(); for row in out[54..].chunks((w * 3) as usize) { fixed.extend_from_slice(row); fixed.extend(std::iter::repeat_n(0u8, pad)); } out = fixed; }
    let _ = std::fs::write(path, out);
    let _ = DestroyWindow(back);
    crate::log(&format!("screenshot {path} bitblt={ok}"));
    SelectObject(mem, old); let _ = DeleteObject(bmp.into()); let _ = DeleteDC(mem); ReleaseDC(None, hdc);
    let _ = HWND::default();
}
