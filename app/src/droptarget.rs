// Alvo de drop OLE na janela da ilha: arquivos soltos entram no Shelf. Roda na thread "island" (STA).
use std::{path::PathBuf, sync::atomic::{AtomicBool, Ordering::Relaxed}};
use windows::{
    core::implement,
    Win32::{Foundation::{HWND, POINTL}, System::{Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL}, Ole::*, SystemServices::MODIFIERKEYS_FLAGS},
        UI::Shell::{DragQueryFileW, HDROP}},
};

pub static OVER: AtomicBool = AtomicBool::new(false);
/// true quando o arrasto terminou com um drop (a ilha fica em Arquivos); false = saiu sem soltar (volta a pagina anterior)
pub static DROPPED: AtomicBool = AtomicBool::new(false);

#[implement(IDropTarget)]
struct Dt;
impl IDropTarget_Impl for Dt_Impl {
    fn DragEnter(&self, _: windows_core::Ref<IDataObject>, _: MODIFIERKEYS_FLAGS, _: &POINTL, eff: *mut DROPEFFECT) -> windows_core::Result<()> {
        crate::log("droptarget: DragEnter"); OVER.store(true, Relaxed); crate::island::wake(); unsafe { *eff = DROPEFFECT_COPY; } Ok(())
    }
    fn DragOver(&self, _: MODIFIERKEYS_FLAGS, _: &POINTL, eff: *mut DROPEFFECT) -> windows_core::Result<()> { unsafe { *eff = DROPEFFECT_COPY; } Ok(()) }
    fn DragLeave(&self) -> windows_core::Result<()> { crate::log("droptarget: DragLeave"); OVER.store(false, Relaxed); crate::island::wake(); Ok(()) }
    fn Drop(&self, data: windows_core::Ref<IDataObject>, _: MODIFIERKEYS_FLAGS, _: &POINTL, eff: *mut DROPEFFECT) -> windows_core::Result<()> {
        OVER.store(false, Relaxed); DROPPED.store(true, Relaxed);
        unsafe { *eff = DROPEFFECT_COPY; }
        if let Some(d) = data.as_ref() { let p = paths(d); crate::log(&format!("drop: {} itens", p.len())); std::thread::spawn(move || crate::shelf::add_native(p)); } // metadata dos arquivos fora da thread da ilha
        crate::island::wake(); Ok(())
    }
}

fn paths(d: &IDataObject) -> Vec<PathBuf> { unsafe {
    let fmt = FORMATETC { cfFormat: 15, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 }; // CF_HDROP
    let Ok(mut m) = d.GetData(&fmt) else { return Vec::new() };
    let h = HDROP(m.u.hGlobal.0);
    let n = DragQueryFileW(h, u32::MAX, None);
    let mut v = Vec::new();
    for i in 0..n { let len = DragQueryFileW(h, i, None); let mut b = vec![0u16; len as usize + 1]; DragQueryFileW(h, i, Some(&mut b)); v.push(PathBuf::from(String::from_utf16_lossy(&b[..len as usize]))); }
    windows::Win32::System::Ole::ReleaseStgMedium(&mut m); // libera o HGLOBAL do GetData
    v
} }

pub fn register(hwnd: HWND) { unsafe {
    let _ = OleInitialize(None);
    let t: IDropTarget = Dt.into();
    crate::log(&format!("RegisterDragDrop: {:?}", RegisterDragDrop(hwnd, &t)));
} }
