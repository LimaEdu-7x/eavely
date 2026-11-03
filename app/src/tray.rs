// Bandeja nativa (Shell_NotifyIcon): inicia escondido; menu "Desligar notch"/"Ligar notch" e "Sair".
use windows::{core::{w, PCWSTR}, Win32::{Foundation::*, System::Registry::*, UI::{Shell::*, WindowsAndMessaging::*}}};
use crate::island::{self, App};

const ID_TOGGLE: u32 = 1;
const ID_QUIT: u32 = 2;
const ID_SETTINGS: u32 = 3;

/// Icone claro/escuro conforme o tema da barra de tarefas (assets/app).
pub fn app_icon() -> HICON { static I: std::sync::OnceLock<isize> = std::sync::OnceLock::new(); HICON(*I.get_or_init(|| make_icon().0 as isize) as *mut _) } // criado uma vez (vive ate o fim do processo)
fn make_icon() -> HICON {
    let light = unsafe {
        let mut v = 0u32; let mut n = 4u32;
        RegGetValueW(HKEY_CURRENT_USER, w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"), w!("SystemUsesLightTheme"), RRF_RT_REG_DWORD, None, Some(&mut v as *mut _ as *mut _), Some(&mut n)).is_ok() && v == 1
    };
    let png: &[u8] = if light { include_bytes!("../../assets/app/eavely-tray-black-32.png") } else { include_bytes!("../../assets/app/eavely-tray-white-32.png") };
    unsafe { CreateIconFromResourceEx(png, true, 0x00030000, 32, 32, LR_DEFAULTCOLOR).or_else(|_| LoadIconW(None, IDI_APPLICATION)).unwrap_or_default() }
}

fn data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut d = NOTIFYICONDATAW { cbSize: size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP, uCallbackMessage: island::WM_TRAY, hIcon: app_icon(), ..Default::default() };
    for (i, c) in "Eavely".encode_utf16().enumerate() { d.szTip[i] = c; }
    d
}
pub fn add(hwnd: HWND) { unsafe { let d = data(hwnd); let _ = Shell_NotifyIconW(NIM_ADD, &d); } }
pub fn remove(hwnd: HWND) { unsafe { let d = data(hwnd); let _ = Shell_NotifyIconW(NIM_DELETE, &d); } }

/// Chamado FORA do with_app: TrackPopupMenu e modal e despacha mensagens da ilha; com APP emprestado elas seriam descartadas.
pub fn on_msg(hwnd: HWND, enabled: bool, lp: LPARAM) {
    let m = (lp.0 & 0xFFFF) as u32;
    if m == WM_RBUTTONUP || m == WM_LBUTTONUP || m == WM_CONTEXTMENU {
        unsafe {
            let Ok(menu) = CreatePopupMenu() else { return };
            let lbl: Vec<u16> = (if enabled { "Desligar notch" } else { "Ligar notch" }).encode_utf16().chain([0]).collect();
            let _ = AppendMenuW(menu, MF_STRING, ID_TOGGLE as usize, PCWSTR(lbl.as_ptr()));
            let _ = AppendMenuW(menu, MF_STRING, ID_SETTINGS as usize, w!("Configurações"));
            let _ = AppendMenuW(menu, MF_STRING, ID_QUIT as usize, w!("Sair"));
            let mut p = POINT::default(); let _ = GetCursorPos(&mut p);
            let _ = SetForegroundWindow(hwnd); // exigido para o menu fechar ao clicar fora
            let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, p.x, p.y, Some(0), hwnd, None);
            let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            let _ = DestroyMenu(menu);
        }
    }
}
pub fn on_cmd(a: &mut App, id: u32) {
    match id { ID_TOGGLE => { let on = !a.enabled; island::set_enabled(a, on, "menu da bandeja"); } ID_QUIT => island::quit(a), ID_SETTINGS => crate::settings_ui::open(), _ => {} }
}
