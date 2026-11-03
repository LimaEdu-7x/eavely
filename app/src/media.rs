// Midia via SMTC: orientado a eventos (nada de polling). Estado em memoria; capa em data URL (<=256 px) + pixels 64x64 para a ilha nativa.
use serde::{Deserialize, Serialize};
use std::{sync::{atomic::{AtomicU64, Ordering::Relaxed}, Arc, Mutex}, time::{Instant, SystemTime, UNIX_EPOCH}};
use windows::{
    core::Result as WR,
    Foundation::TypedEventHandler,
    Graphics::Imaging::*,
    Media::Control::*,
    Security::Cryptography::CryptographicBuffer,
    Storage::Streams::*,
    Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED},
};

type Mgr = GlobalSystemMediaTransportControlsSessionManager;
type Sess = GlobalSystemMediaTransportControlsSession;

#[derive(Clone, Serialize, Default, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub title: String, pub artist: String, pub album: String, pub status: String,
    pub position_ms: u64, pub duration_ms: u64, pub cover_data_url: Option<String>, pub app_name: Option<String>,
}
#[derive(Default)]
struct Inner { st: MediaState, px: Option<Arc<Vec<u8>>>, at: Option<Instant> }
static COVER_GEN: AtomicU64 = AtomicU64::new(0);
pub const COVER_PX: u32 = 128; // lado da capa BGRA entregue ao renderer nativo

static STATE: Mutex<Option<Inner>> = Mutex::new(None);
static SESSION: Mutex<Option<(Sess, [i64; 3])>> = Mutex::new(None);
static BUSY: Mutex<()> = Mutex::new(()); // serializa refresh (eventos chegam em threads do pool)

fn lk<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> { m.lock().unwrap_or_else(|e| e.into_inner()) }

/// Posicao extrapolada: o SMTC so avisa em mudancas, entao somamos o tempo decorrido enquanto toca.
fn live(pos: u64, dur: u64, playing: bool, elapsed_ms: u64) -> u64 {
    let p = if playing { pos + elapsed_ms } else { pos };
    if dur > 0 { p.min(dur) } else { p }
}

/// Visao leve (sem o data URL da capa) para o renderer ler a cada quadro.
pub struct View { pub title: String, pub artist: String, pub status: String, pub pos_ms: u64, pub dur_ms: u64, pub cover_gen: u64 }
pub fn view() -> View {
    let g = lk(&STATE);
    let Some(i) = g.as_ref() else { return View { title: String::new(), artist: String::new(), status: "stopped".into(), pos_ms: 0, dur_ms: 0, cover_gen: 0 } };
    let s = &i.st;
    View { title: s.title.clone(), artist: s.artist.clone(), status: s.status.clone(), dur_ms: s.duration_ms, cover_gen: COVER_GEN.load(Relaxed),
        pos_ms: live(s.position_ms, s.duration_ms, s.status == "playing", i.at.map_or(0, |t| t.elapsed().as_millis() as u64)) }
}
pub fn cover_px() -> Option<Arc<Vec<u8>>> { lk(&STATE).as_ref().and_then(|i| i.px.clone()) }

/// Teste: "nada tocando" / volta a faixa falsa.
#[cfg(feature = "devtools")]
pub fn fake_none() { *lk(&STATE) = Some(Inner { st: MediaState { status: "stopped".into(), app_name: Some("Fake".into()), ..Default::default() }, px: None, at: Some(Instant::now()) }); COVER_GEN.fetch_add(1, Relaxed); crate::island::wake(); }
#[cfg(feature = "devtools")]
pub fn fake_again() { fake(); }
#[cfg(feature = "devtools")]
/// Teste/captura: capa falsa de uma cor so (para conferir o gradiente com capa clara e escura).
pub fn fake_cover(bgr: [u8; 3]) {
    let px: Vec<u8> = (0..COVER_PX * COVER_PX).flat_map(|_| [bgr[0], bgr[1], bgr[2], 255]).collect();
    COVER_GEN.fetch_add(1, Relaxed);
    if let Some(i) = lk(&STATE).as_mut() { i.px = Some(Arc::new(px)); }
    crate::island::wake();
}
#[cfg(feature = "devtools")]
/// Demo/benchmark (EAVELY_FAKE_MEDIA): faixa falsa tocando, capa em degrade, sem player real.
pub fn fake() {
    let mut px = vec![0u8; (COVER_PX * COVER_PX * 4) as usize];
    for y in 0..COVER_PX { for x in 0..COVER_PX { let i = ((y * COVER_PX + x) * 4) as usize;
        px[i] = (255 - x * 2) as u8; px[i + 1] = (y * 2) as u8; px[i + 2] = (80 + x) as u8; px[i + 3] = 255; } }
    let st = MediaState { title: "Never Gonna Give You Up".into(), artist: "Rick Astley".into(), album: "Whenever You Need Somebody".into(), status: "playing".into(),
        position_ms: 20_000, duration_ms: 213_000, cover_data_url: None, app_name: Some("Fake".into()) };
    COVER_GEN.fetch_add(1, Relaxed);
    *lk(&STATE) = Some(Inner { st: st.clone(), px: Some(Arc::new(px)), at: Some(Instant::now()) });
    crate::island::wake();
}

pub fn current() -> MediaState {
    let g = lk(&STATE);
    let Some(i) = g.as_ref() else { return MediaState { status: "stopped".into(), ..Default::default() } };
    let mut s = i.st.clone();
    s.position_ms = live(s.position_ms, s.duration_ms, s.status == "playing", i.at.map_or(0, |t| t.elapsed().as_millis() as u64));
    s
}

fn filetime_now() -> i64 { // 100 ns desde 1601, a mesma base de LastUpdatedTime
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| (d.as_nanos() / 100) as i64 + 116_444_736_000_000_000)
}

fn fill(s: &Sess, old: &MediaState, old_px: &Option<Arc<Vec<u8>>>, st: &mut MediaState, px: &mut Option<Arc<Vec<u8>>>) -> WR<()> {
    use windows::core::Interface;
    // titulo/artista/album primeiro (com timeout): um player que trava nao congela o estado de midia, e uma timeline que falha nao os faz sumir
    let op = s.TryGetMediaPropertiesAsync()?;
    if !crate::lyrics::wait(&op.cast()?) { return Err(windows::core::Error::from(windows::Win32::Foundation::E_ABORT)); }
    let p = op.join()?;
    st.title = p.Title().map(|t| t.to_string()).unwrap_or_default(); st.artist = p.Artist().map(|t| t.to_string()).unwrap_or_default(); st.album = p.AlbumTitle().map(|t| t.to_string()).unwrap_or_default();
    let playing = s.GetPlaybackInfo().and_then(|i| i.PlaybackStatus());
    st.status = match playing {
        Ok(x) if x == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => "playing",
        Ok(x) if x == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped || x == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed => "stopped",
        _ => "paused",
    }.into();
    st.app_name = s.SourceAppUserModelId().ok().map(|a| a.to_string()).map(|a| a.trim_end_matches(".exe").to_string()).filter(|a| !a.is_empty());
    let _ = (|| -> WR<()> { // timeline: melhor esforco (players sem timeline ficam com duracao/posicao 0)
        let tl = s.GetTimelineProperties()?;
        let (a, b, p) = (tl.StartTime()?.Duration, tl.EndTime()?.Duration, tl.Position()?.Duration);
        st.duration_ms = ((b - a).max(0) / 10_000) as u64;
        let mut pos = (p - a).max(0);
        if st.status == "playing" { pos += (filetime_now() - tl.LastUpdatedTime()?.UniversalTime).clamp(0, 86_400 * 10_000_000); }
        st.position_ms = pos as u64 / 10_000;
        Ok(())
    })();
    if st.title == old.title && st.artist == old.artist && st.album == old.album && old.cover_data_url.is_some() {
        st.cover_data_url = old.cover_data_url.clone(); *px = old_px.clone();
    } else if let Some((url, small)) = cover(&p) { st.cover_data_url = Some(url); *px = Some(Arc::new(small)); }
    Ok(())
}

/// Espera uma operacao WinRT com timeout (reaproveita o `wait` do cliente HTTP); estourou = None.
macro_rules! tj { ($e:expr) => {{ let op = $e.ok()?; let info: windows_future::IAsyncInfo = windows::core::Interface::cast(&op).ok()?; if !crate::lyrics::wait(&info) { return None; } op.join().ok()? }}; }

/// Capa -> (data URL JPEG <=256 px, pixels BGRA 64x64 para a ilha nativa).
fn cover(p: &GlobalSystemMediaTransportControlsSessionMediaProperties) -> Option<(String, Vec<u8>)> {
    let st = tj!(p.Thumbnail().ok()?.OpenReadAsync());
    let dec = tj!(BitmapDecoder::CreateAsync(&st));
    let (w, h) = (dec.PixelWidth().ok()?, dec.PixelHeight().ok()?);
    if w == 0 || h == 0 { return None; }
    let pixels = |nw: u32, nh: u32| -> Option<Vec<u8>> {
        let t = BitmapTransform::new().ok()?;
        t.SetScaledWidth(nw).ok()?; t.SetScaledHeight(nh).ok()?; t.SetInterpolationMode(BitmapInterpolationMode::Fant).ok()?;
        let d = tj!(dec.GetPixelDataTransformedAsync(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Ignore, &t, ExifOrientationMode::IgnoreExifOrientation, ColorManagementMode::DoNotColorManage));
        Some(d.DetachPixelData().ok()?.to_vec())
    };
    let k = (256.0 / w.max(h) as f32).min(1.0);
    let (nw, nh) = (((w as f32 * k).round() as u32).max(1), ((h as f32 * k).round() as u32).max(1));
    let big = pixels(nw, nh)?;
    let ms = InMemoryRandomAccessStream::new().ok()?;
    let enc = tj!(BitmapEncoder::CreateAsync(BitmapEncoder::JpegEncoderId().ok()?, &ms));
    enc.SetPixelData(BitmapPixelFormat::Bgra8, BitmapAlphaMode::Ignore, nw, nh, 96.0, 96.0, &big).ok()?;
    tj!(enc.FlushAsync());
    let n = ms.Size().ok()? as u32;
    ms.Seek(0).ok()?;
    let rd = DataReader::CreateDataReader(&ms).ok()?;
    tj!(rd.LoadAsync(n));
    let b64 = CryptographicBuffer::EncodeToBase64String(&rd.ReadBuffer(n).ok()?).ok()?.to_string();
    Some((format!("data:image/jpeg;base64,{b64}"), pixels(COVER_PX, COVER_PX)?))
}

fn refresh() {
    let _g = lk(&BUSY);
    let sess = lk(&SESSION).as_ref().map(|x| x.0.clone());
    let (old, old_px) = lk(&STATE).as_ref().map(|i| (i.st.clone(), i.px.clone())).unwrap_or_default();
    let mut st = MediaState { status: "stopped".into(), ..Default::default() };
    let mut px = None;
    if let Some(s) = sess {
        if let Err(e) = fill(&s, &old, &old_px, &mut st, &mut px) { crate::log(&format!("smtc: {e}")); }
    }
    *lk(&STATE) = Some(Inner { st: st.clone(), px, at: Some(Instant::now()) });
    if old.cover_data_url != st.cover_data_url { COVER_GEN.fetch_add(1, Relaxed); }
    crate::island::wake();
}

fn bind(mgr: &Mgr) {
    let new = mgr.GetCurrentSession().ok();
    {
        let mut g = lk(&SESSION);
        if let Some((o, t)) = g.take() {
            let _ = o.RemoveMediaPropertiesChanged(t[0]); let _ = o.RemovePlaybackInfoChanged(t[1]); let _ = o.RemoveTimelinePropertiesChanged(t[2]);
        }
        if let Some(s) = new {
            
            let t = (|| -> WR<[i64; 3]> {
                Ok([s.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| { refresh(); Ok(()) }))?,
                    s.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| { refresh(); Ok(()) }))?,
                    s.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| { refresh(); Ok(()) }))?])
            })();
            match t { Ok(t) => *g = Some((s, t)), Err(e) => crate::log(&format!("smtc eventos: {e}")) }
        }
    }
    refresh();
}

/// Thread dona do SMTC (MTA). Sem sessao/SMTC indisponivel: estado "stopped", sem erro.
pub fn init() {
    std::thread::spawn(move || {
        unsafe { let _ = CoInitializeEx(None, COINIT_MULTITHREADED); }
        let Ok(mgr) = Mgr::RequestAsync().and_then(|o| o.join()) else { crate::log("smtc indisponivel"); return };
        let m2 = mgr.clone();
        let _ = mgr.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| { bind(&m2); Ok(()) }));
        bind(&mgr);
        loop { std::thread::park(); } // mantem mgr e handlers vivos
    });
}


#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Action { Play, Pause, Toggle, Next, Prev }

fn with_session(f: impl FnOnce(&Sess) -> WR<()>) -> Result<(), String> {
    let s = lk(&SESSION).as_ref().map(|x| x.0.clone()).ok_or("nenhuma sessao de midia")?;
    f(&s).map_err(|e| e.to_string())
}

#[cfg(feature = "devtools")]
/// Demo/benchmark (EAVELY_FAKE_MEDIA): o "player" falso aceita seek e play/pause.
fn fake_apply(f: impl FnOnce(&mut Inner)) -> bool {
    let mut g = lk(&STATE);
    match g.as_mut() { Some(i) if i.st.app_name.as_deref() == Some("Fake") => { f(i); true } _ => false }
}
pub fn media_control(action: Action) -> Result<(), String> {
    #[cfg(feature = "devtools")]
    if fake_apply(|i| { let now = i.at.map_or(0, |t| t.elapsed().as_millis() as u64); i.st.position_ms = live(i.st.position_ms, i.st.duration_ms, i.st.status == "playing", now); i.at = Some(Instant::now());
        i.st.status = match action { Action::Pause => "paused", Action::Play => "playing", Action::Toggle => if i.st.status == "playing" { "paused" } else { "playing" }, _ => "playing" }.into(); }) { crate::island::wake(); return Ok(()); }
    // so dispara a operacao (nao bloqueia a thread principal); o resultado chega pelos eventos do SMTC
    with_session(|s| { match action {
        Action::Play => drop(s.TryPlayAsync()?), Action::Pause => drop(s.TryPauseAsync()?), Action::Toggle => drop(s.TryTogglePlayPauseAsync()?),
        Action::Next => drop(s.TrySkipNextAsync()?), Action::Prev => drop(s.TrySkipPreviousAsync()?),
    } Ok(()) })
}
pub fn media_seek(position_ms: u64) -> Result<(), String> {
    #[cfg(feature = "devtools")]
    if fake_apply(|i| { i.st.position_ms = position_ms; i.at = Some(Instant::now()); }) { return Ok(()); }
    with_session(|s| {
        let op = s.TryChangePlaybackPositionAsync((position_ms as i64).saturating_mul(10_000))?;
        // o player pode recusar (false): avisa a UI para um feedback discreto
        std::thread::spawn(move || { if !matches!(op.join(), Ok(true)) { SEEK_FAIL.store(start_ms() + 1, std::sync::atomic::Ordering::Relaxed); crate::island::wake(); } });
        Ok(())
    })
}
static SEEK_FAIL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
fn start_ms() -> u64 { static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new(); T0.get_or_init(Instant::now).elapsed().as_millis() as u64 }
/// o player recusou um seek ha menos de `ms`?
pub fn seek_failed_within(ms: u64) -> bool { let f = SEEK_FAIL.load(std::sync::atomic::Ordering::Relaxed); f != 0 && start_ms().saturating_sub(f - 1) < ms }

#[cfg(test)]
mod tests {
    use super::live;
    #[test] fn posicao_extrapolada() {
        assert_eq!(live(1000, 10_000, true, 500), 1500);
        assert_eq!(live(1000, 10_000, false, 500), 1000);
        assert_eq!(live(9900, 10_000, true, 500), 10_000); // limita na duracao
        assert_eq!(live(1000, 0, true, 500), 1500);        // duracao desconhecida
    }
}
