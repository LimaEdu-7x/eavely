// Shelf: lista de arquivos/pastas persistida em %APPDATA%\Eavely\shelf.json; arrastar para fora via OLE DoDragDrop.
use serde::{Deserialize, Serialize};
use std::{path::{Path, PathBuf}, sync::{atomic::{AtomicBool, Ordering::Relaxed}, Mutex}};
use windows::{
    core::{implement, BOOL, HRESULT, HSTRING},
    Win32::{Foundation::{DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, S_OK},
        System::{Com::IDataObject, Ole::*, SystemServices::MK_LBUTTON},
        UI::Shell::{BHID_DataObject, IShellItem, SHCreateItemFromParsingName}},
};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem { pub id: String, pub name: String, pub path: String, pub is_dir: bool, pub size_bytes: Option<u64> }

/// Verdadeiro enquanto um arrasto para fora esta em andamento (a ilha nao pode fechar no meio).
pub static DRAGGING: AtomicBool = AtomicBool::new(false);
/// Incrementa a cada mudanca da lista (a UI nativa recarrega quando difere).
pub static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn bump() { GEN.fetch_add(1, Relaxed); crate::island::wake(); }
static ITEMS: Mutex<Option<Vec<ShelfItem>>> = Mutex::new(None);

fn file() -> PathBuf { crate::data_dir().join("shelf.json") }
fn id_of(path: &str) -> String { format!("{:016x}", path.to_lowercase().bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))) }

fn item_for(p: &Path) -> Option<ShelfItem> {
    let md = std::fs::metadata(p).ok()?;
    let path = p.to_string_lossy().replace('/', "\\"); // barras do Windows: a shell nao aceita "C:/x" de forma confiavel
    Some(ShelfItem { id: id_of(&path), name: p.file_name().map_or(path.clone(), |n| n.to_string_lossy().to_string()), path,
        is_dir: md.is_dir(), size_bytes: md.is_file().then(|| md.len()) })
}

/// Junta novos itens (sem duplicar por id), novos no topo. Devolve true se mudou algo.
fn merge(list: &mut Vec<ShelfItem>, new: Vec<ShelfItem>) -> bool {
    let mut add: Vec<ShelfItem> = Vec::new();
    for n in new { if !list.iter().any(|i| i.id == n.id) && !add.iter().any(|i| i.id == n.id) { add.push(n); } }
    let changed = !add.is_empty();
    list.splice(0..0, add);
    changed
}

fn with_items<T>(f: impl FnOnce(&mut Vec<ShelfItem>) -> T) -> T {
    let mut g = ITEMS.lock().unwrap_or_else(|e| e.into_inner());
    let v = g.get_or_insert_with(|| crate::store::load_or_quarantine(&file()).unwrap_or_default());
    f(v)
}
fn save(v: &[ShelfItem]) { crate::store::write_atomic(&file(), serde_json::to_string(v).unwrap_or_default().as_bytes()); }

/// Adiciona sem AppHandle (UI nativa: drop no OLE).
pub fn add_native(paths: Vec<PathBuf>) {
    let new: Vec<ShelfItem> = paths.iter().filter_map(|p| item_for(p)).collect();
    if with_items(|v| merge(v, new).then(|| save(v)).is_some()) { bump(); }
}
pub fn clear_native() { with_items(|v| { v.clear(); save(v); }); bump(); }
pub fn remove_native(id: &str) { if with_items(|v| { let n = v.len(); v.retain(|i| i.id != id); (v.len() != n).then(|| save(v)).is_some() }) { bump(); } }
/// Sem tocar no disco (a UI nao pode travar em share de rede/disco removivel) e sem podar itens offline: item ausente so falha ao abrir.
pub fn list_native() -> Vec<ShelfItem> { with_items(|v| v.clone()) }

#[implement(IDropSource)]
struct Src;
impl IDropSource_Impl for Src_Impl {
    fn QueryContinueDrag(&self, esc: BOOL, keys: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS) -> HRESULT {
        let r = if esc.as_bool() { DRAGDROP_S_CANCEL } else if keys.0 & MK_LBUTTON.0 == 0 { DRAGDROP_S_DROP } else { S_OK };
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if N.fetch_add(1, Relaxed) < 3 || r != S_OK { dbg_log(&format!("QueryContinueDrag esc={} keys=0x{:X} -> {r:?}", esc.as_bool(), keys.0)); }
        r
    }
    fn GiveFeedback(&self, _: DROPEFFECT) -> HRESULT { DRAGDROP_S_USEDEFAULTCURSORS }
}

/// Diagnostico: so em build debug (e com EAVELY_LOG, como todo log).
fn dbg_log(s: &str) { if cfg!(debug_assertions) { crate::log(s); } }
fn lbutton_down() -> bool { unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(0x01) < 0 } }

/// O botao esquerdo foi apertado numa janela de OUTRA thread (o WebView2): ela detem a captura do mouse e o DoDragDrop da nossa thread
/// nao recebe nada. Ligamos as filas de entrada (AttachThreadInput) para compartilhar a captura. Devolve a thread ligada.
fn attach_input_owner() -> Option<u32> {
    use windows::Win32::{Foundation::POINT, System::Threading::*, UI::WindowsAndMessaging::*};
    unsafe {
        let mut gi = GUITHREADINFO { cbSize: size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        let mut cap = if GetGUIThreadInfo(0, &mut gi).is_ok() { gi.hwndCapture } else { Default::default() };
        if cap.0.is_null() { let mut p = POINT::default(); let _ = GetCursorPos(&mut p); cap = WindowFromPoint(p); }
        if cap.0.is_null() { return None; }
        let t = GetWindowThreadProcessId(cap, None);
        let ok = t != 0 && t != GetCurrentThreadId() && AttachThreadInput(GetCurrentThreadId(), t, true).as_bool();
        dbg_log(&format!("attach_input_owner: janela {:?} thread {t} -> {ok}", cap.0));
        ok.then_some(t)
    }
}

fn drag_path(path: &str) -> windows::core::Result<()> {
    unsafe {
        let item: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(path.replace('/', "\\")), None)?;
        let data: IDataObject = item.BindToHandler(None, &BHID_DataObject)?;
        dbg_log("IDataObject criado");
        let src: IDropSource = Src.into();
        let mut eff = DROPEFFECT_NONE;
        // so COPY: arrastar para outro app nunca move/apaga o original
        let hr = DoDragDrop(&data, &src, DROPEFFECT_COPY, &mut eff);
        dbg_log(&format!("DoDragDrop -> {hr:?} efeito={}", eff.0)); // DRAGDROP_S_DROP=0x40100 solto, S_CANCEL=0x40101
        if hr == DRAGDROP_S_DROP && eff != DROPEFFECT_NONE {
            // o alvo (Explorer) le o IDataObject DEPOIS do drop, chamando de volta esta thread STA: precisa continuar bombeando mensagens
            use windows::Win32::UI::WindowsAndMessaging::*;
            let t0 = std::time::Instant::now();
            let mut m = MSG::default();
            while t0.elapsed() < std::time::Duration::from_secs(4) {
                while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() { let _ = TranslateMessage(&m); DispatchMessageW(&m); }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
    Ok(())
}

pub fn start_drag(id: &str) -> Result<(), String> {
    let path = with_items(|v| v.iter().find(|i| i.id == id).map(|i| i.path.clone())).ok_or("item nao encontrado")?;
    if !Path::new(&path).exists() { return Err("arquivo nao existe mais".into()); }
    dbg_log(&format!("shelf_start_drag: id={id} path={path} thread={:?} botao_esq_apertado={}", std::thread::current().id(), lbutton_down()));
    DRAGGING.store(true, Relaxed);
    // DoDragDrop e modal e precisa de uma thread STA com fila de mensagens: thread propria, nunca a principal do Tauri
    std::thread::Builder::new().name("shelf-drag".into()).spawn(move || {
        let ole = unsafe { OleInitialize(None) };
        let att = attach_input_owner();
        dbg_log(&format!("shelf drag na thread {:?}: OleInitialize={ole:?} botao_esq_apertado={}", std::thread::current().id(), lbutton_down()));
        if let Err(e) = drag_path(&path) { dbg_log(&format!("shelf drag erro: {e}")); }
        DRAGGING.store(false, Relaxed);
        if let Some(t) = att { unsafe { let _ = windows::Win32::System::Threading::AttachThreadInput(windows::Win32::System::Threading::GetCurrentThreadId(), t, false); } }
        unsafe { OleUninitialize(); }
    }).map(|_| ()).map_err(|e| { DRAGGING.store(false, Relaxed); e.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn it(id: &str) -> ShelfItem { ShelfItem { id: id.into(), name: id.into(), path: id.into(), is_dir: false, size_bytes: None } }
    #[test] fn merge_sem_duplicar_e_novos_no_topo() {
        let mut v = vec![it("a")];
        assert!(merge(&mut v, vec![it("b"), it("a"), it("b"), it("c")]));
        assert_eq!(v.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), ["b", "c", "a"]);
        assert!(!merge(&mut v, vec![it("a")]));
    }
    #[test] fn id_ignora_caixa_e_item_for_le_arquivo_e_pasta() {
        assert_eq!(id_of("C:\\X\\A.txt"), id_of("c:\\x\\a.TXT"));
        let d = std::env::temp_dir().join(format!("eavely-shelf-test-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("a.txt"); std::fs::write(&f, b"abc").unwrap();
        let fi = item_for(&f).unwrap(); assert_eq!((fi.name.as_str(), fi.is_dir, fi.size_bytes), ("a.txt", false, Some(3)));
        let di = item_for(&d).unwrap(); assert_eq!((di.is_dir, di.size_bytes), (true, None));
        assert!(item_for(&d.join("nao-existe")).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }
    #[test] fn json_roundtrip_camel_case() {
        let s = serde_json::to_string(&it("a")).unwrap();
        assert!(s.contains("\"isDir\":false") && s.contains("\"sizeBytes\":null"));
        assert_eq!(serde_json::from_str::<ShelfItem>(&s).unwrap(), it("a"));
    }
}

// ---- miniaturas de imagem (pre-visualizacao real do shell; thread propria para a UI nunca esperar) ----
pub struct Thumb { pub w: u32, pub h: u32, pub px: Vec<u8>, pub icon: bool }
static THUMBS: Mutex<Option<std::collections::HashMap<String, Option<std::sync::Arc<Thumb>>>>> = Mutex::new(None);
/// Miniatura pronta do item (None enquanto carrega, para nao-imagens ou se o shell nao tiver). Pede a geracao na primeira chamada.
pub fn thumb(id: &str, path: &str) -> Option<std::sync::Arc<Thumb>> {
    let mut g = THUMBS.lock().unwrap_or_else(|e| e.into_inner());
    let m = g.get_or_insert_with(Default::default);
    if let Some(v) = m.get(id) { return v.clone(); }
    m.insert(id.to_string(), None);
    let (id, path) = (id.to_string(), path.to_string());
    std::thread::spawn(move || {
        let mut got = None; for k in 0..4 { got = unsafe { load_thumb(&path) }; if got.is_some() { break; } std::thread::sleep(std::time::Duration::from_millis(120 * (k + 1))); } // o shell as vezes falha na 1a tentativa
        if let Some(t) = got { THUMBS.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(Default::default).insert(id, Some(std::sync::Arc::new(t))); bump(); }
    });
    None
}
unsafe fn load_thumb(path: &str) -> Option<Thumb> {
    use windows::Win32::{Graphics::Gdi::*, System::Com::*, UI::Shell::{IShellItemImageFactory, SIIGBF_THUMBNAILONLY, SIIGBF_BIGGERSIZEOK}, Foundation::SIZE};
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    let item: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(path.replace('/', "\\")), None).ok()?;
    let f: IShellItemImageFactory = windows::core::Interface::cast(&item).ok()?;
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let img = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "tif" | "tiff" | "ico" | "heic" | "avif");
    // imagem: miniatura (pre-visualizacao real); senao (ou se falhar) o ICONE real do shell para o tipo de arquivo/pasta
    let (bmp, icon) = match if img { f.GetImage(SIZE { cx: 108, cy: 78 }, SIIGBF_THUMBNAILONLY | SIIGBF_BIGGERSIZEOK).ok() } else { None } { Some(b) => (b, false), None => (f.GetImage(SIZE { cx: 64, cy: 64 }, windows::Win32::UI::Shell::SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK).ok()?, true) };
    let mut bm = BITMAP::default();
    if GetObjectW(bmp.into(), size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut _)) == 0 { let _ = DeleteObject(bmp.into()); return None; }
    let (w, h) = (bm.bmWidth as u32, bm.bmHeight as u32);
    if w == 0 || h == 0 || w > 1024 || h > 1024 { let _ = DeleteObject(bmp.into()); return None; }
    let bmi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: w as i32, biHeight: -(h as i32), biPlanes: 1, biBitCount: 32, ..Default::default() }, ..Default::default() };
    let mut px = vec![0u8; (w * h * 4) as usize];
    let dc = GetDC(None);
    let ok = GetDIBits(dc, bmp, 0, h, Some(px.as_mut_ptr() as *mut _), &bmi as *const _ as *mut _, DIB_RGB_COLORS);
    ReleaseDC(None, dc); let _ = DeleteObject(bmp.into());
    if ok == 0 { return None; }
    // sem nenhum alfa (miniatura de foto): opaca; com alfa (icones): pre-multiplica para o Direct2D
    if px.chunks_exact(4).all(|p| p[3] == 0) { for p in px.chunks_exact_mut(4) { p[3] = 255; } } else { for p in px.chunks_exact_mut(4) { let a = p[3] as u32; p[0] = (p[0] as u32 * a / 255) as u8; p[1] = (p[1] as u32 * a / 255) as u8; p[2] = (p[2] as u32 * a / 255) as u8; } }
    Some(Thumb { w, h, px, icon })
}
