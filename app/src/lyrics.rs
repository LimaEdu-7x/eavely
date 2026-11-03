// Letras via LRCLIB (/api/get) com cache em disco. Parser LRC puro e testado.
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex, time::{Duration, Instant, SystemTime}};

const UA: &str = "Eavely/0.1 (github.com/LimaEdu-7x/eavely)";

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Line { pub time_ms: u64, pub text: String }
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Lyrics { pub synced: bool, pub lines: Vec<Line>, #[serde(default)] pub instrumental: bool } // instrumental: o LRCLIB marca a faixa como instrumental (sem letra de proposito)

/// "[mm:ss.xx]" (xx = 1..3 digitos) -> ms. Aceita varias marcas por linha e ignora tags de metadados ([ar:..]).
pub fn parse_lrc(src: &str) -> Vec<Line> {
    let mut out = Vec::new();
    let mut offset: i64 = 0;
    for raw in src.lines() {
        let mut rest = raw.trim();
        let mut times = Vec::new();
        while let Some(t) = rest.strip_prefix('[') {
            let Some(end) = t.find(']') else { break };
            let tag = &t[..end];
            if let Some(ms) = parse_ts(tag) { times.push(ms); }
            else if let Some(v) = tag.strip_prefix("offset:") { offset = v.trim().parse::<i64>().unwrap_or(0).clamp(-3_600_000, 3_600_000); }
            rest = t[end + 1..].trim_start();
        }
        let text: String = rest.trim_end().chars().take(MAX_LINE_CHARS).collect();
        for ms in times { if out.len() >= MAX_LINES { break; } out.push(Line { time_ms: ms, text: text.clone() }); }
        if out.len() >= MAX_LINES { break; }
    }
    for l in &mut out { l.time_ms = (l.time_ms as i64 - offset).max(0) as u64; } // offset positivo = letra adiantada
    out.sort_by_key(|l| l.time_ms); // estavel: mantem a ordem de linhas com o mesmo tempo
    out
}

fn parse_ts(tag: &str) -> Option<u64> {
    let (m, s) = tag.split_once(':')?;
    let m: u64 = m.parse().ok().filter(|m| *m < MAX_MIN)?;
    let (sec, frac) = s.split_once(['.', ':']).unwrap_or((s, ""));
    let sec: u64 = sec.parse().ok()?;
    if sec >= 60 || !frac.chars().all(|c| c.is_ascii_digit()) || frac.len() > 3 { return None; }
    let ms = if frac.is_empty() { 0 } else { frac.parse::<u64>().ok()?.checked_mul(10u64.pow(3 - frac.len() as u32))? };
    m.checked_mul(60_000)?.checked_add(sec * 1000 + ms)
}

/// Letra sem tempo: um item por paragrafo (separados por linha em branco), tempo 0.
pub fn parse_plain(src: &str) -> Vec<Line> {
    src.replace("\r\n", "\n").split("\n\n").map(str::trim).filter(|p| !p.is_empty())
        .take(MAX_LINES).map(|p| Line { time_ms: 0, text: p.chars().take(MAX_LINE_CHARS * 4).collect() }).collect()
}

fn from_json(v: &serde_json::Value) -> Option<Lyrics> {
    if v["instrumental"].as_bool() == Some(true) { return Some(Lyrics { synced: false, lines: Vec::new(), instrumental: true }); }
    if let Some(s) = v["syncedLyrics"].as_str() {
        let lines = parse_lrc(s);
        if !lines.is_empty() { return Some(Lyrics { synced: true, lines, instrumental: false }); }
    }
    let lines = parse_plain(v["plainLyrics"].as_str()?);
    (!lines.is_empty()).then_some(Lyrics { synced: false, lines, instrumental: false })
}

/// Entre os resultados do /api/search, a letra sincronizada com mais linhas (duracao dentro de 3 s quando conhecida).
fn pick_best(list: &serde_json::Value, dur_sec: u64) -> Option<Lyrics> {
    list.as_array()?.iter()
        .filter(|v| dur_sec == 0 || v["duration"].as_f64().is_none_or(|d| (d - dur_sec as f64).abs() <= 3.0))
        .filter_map(from_json).filter(|l| l.synced).max_by_key(|l| l.lines.len())
}

fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}
fn fnv(s: &str) -> u64 { s.bytes().fold(0xcbf29ce484222325, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3)) }

fn cache_path(key: &str) -> PathBuf { crate::data_dir().join("lyrics").join(format!("{:016x}.json", fnv(key))) }

const TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_AFTER: Duration = Duration::from_secs(300); // apos falha/503: nao martela a API
const MAX_BODY: usize = 512 * 1024; // corpo da resposta
const MAX_LINES: usize = 2000;
const MAX_LINE_CHARS: usize = 500;
const MAX_MIN: u64 = 6000; // 100 h: tag [mmm:ss] absurda e descartada
const MAX_FILES: usize = 500;
const MAX_BYTES: u64 = 50 * 1024 * 1024;
static NO_NET_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

/// Espera uma operacao WinRT ate `TIMEOUT`; cancela e devolve false se estourar. So bloqueia a thread de trabalho.
pub fn wait(op: &windows_future::IAsyncInfo) -> bool {
    use windows_future::AsyncStatus;
    let t0 = Instant::now();
    loop {
        match op.Status() { Ok(s) if s == AsyncStatus::Started => {}, Ok(s) if s == AsyncStatus::Completed => return true, _ => return false }
        if t0.elapsed() > TIMEOUT { let _ = op.Cancel(); return false; }
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// Dado (arquivo, modificado, tamanho) devolve quais apagar: mais antigos primeiro, ate caber nos limites.
fn to_prune(mut f: Vec<(PathBuf, SystemTime, u64)>, max_n: usize, max_bytes: u64) -> Vec<PathBuf> {
    f.sort_by_key(|x| x.1); // antigos primeiro
    let (mut n, mut bytes) = (f.len(), f.iter().map(|x| x.2).sum::<u64>());
    let mut out = Vec::new();
    for (p, _, sz) in f { if n <= max_n && bytes <= max_bytes { break; } out.push(p); n -= 1; bytes -= sz; }
    out
}
fn prune_cache(dir: &std::path::Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let files = rd.filter_map(|e| { let e = e.ok()?; let m = e.metadata().ok()?; Some((e.path(), m.modified().ok()?, m.len())) }).collect();
    for p in to_prune(files, MAX_FILES, MAX_BYTES) { let _ = std::fs::remove_file(p); }
}

enum Net { Found(String), NotFound, Server, Fail } // Server = HTTP 5xx: a LRCLIB responde 503 em varios "nao achei"; Fail = rede/timeout

fn http_get(url: &str) -> Net {
    use windows::{core::HSTRING, Foundation::Uri, Web::Http::{HttpClient, HttpStatusCode}};
    let r = (|| -> windows::core::Result<(HttpStatusCode, String)> {
        use windows::core::Interface;
        let c = HttpClient::new()?;
        c.DefaultRequestHeaders()?.UserAgent()?.ParseAdd(&HSTRING::from(UA))?;
        let op = c.GetAsync(&Uri::CreateUri(&HSTRING::from(url))?)?;
        if !wait(&op.cast()?) { return Err(windows::core::Error::from(windows::Win32::Foundation::E_ABORT)); }
        let resp = op.join()?;
        let st = resp.StatusCode()?;
        let body = if st == HttpStatusCode::Ok {
            if resp.Content()?.Headers()?.ContentLength().and_then(|l| l.Value()).is_ok_and(|l| l > MAX_BODY as u64) { return Ok((HttpStatusCode::NotFound, String::new())); } // grande demais: trata como "nao achei"
            let rd = resp.Content()?.ReadAsStringAsync()?;
            if !wait(&rd.cast()?) { return Err(windows::core::Error::from(windows::Win32::Foundation::E_ABORT)); }
            let s = rd.join()?.to_string();
            if s.len() > MAX_BODY { return Ok((HttpStatusCode::NotFound, String::new())); }
            s
        } else { String::new() };
        Ok((st, body))
    })();
    match r { Ok((HttpStatusCode::Ok, b)) => Net::Found(b), Ok((HttpStatusCode::NotFound, _)) => Net::NotFound,
        Ok((st, _)) if st.0 >= 500 => { crate::log(&format!("lrclib: HTTP {}", st.0)); Net::Server } _ => Net::Fail }
}

// palavras INTEIRAS (nao substrings): "(Alive)", "(Remix)" e "(With You)" sao parte do titulo
const NOISE: [&str; 25] = ["remaster", "remastered", "remasterizado", "live", "version", "edit", "mono", "stereo", "mix", "deluxe", "bonus", "feat", "ft", "featuring", "soundtrack",
    "single", "radio", "acoustic", "demo", "anniversary", "explicit", "official", "lyric", "lyrics", "video"];
fn is_noise(seg: &str) -> bool {
    let l = seg.trim().to_lowercase();
    let toks: Vec<&str> = l.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect();
    toks.iter().any(|t| NOISE.contains(t)) || toks.first() == Some(&"from") || (!toks.is_empty() && toks.iter().all(|t| t.chars().all(|c| c.is_ascii_digit()))) // "(2011)"
}
/// Tira sufixos que players (Spotify) acrescentam: "- Remastered 2011", "(feat. X)", "[Live]", "- Radio Edit"...
pub fn clean_title(t: &str) -> String {
    let mut out = String::new();
    let cs: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let close = match cs[i] { '(' => Some(')'), '[' => Some(']'), _ => None };
        if let Some(c) = close { if let Some(j) = cs[i + 1..].iter().position(|x| *x == c) {
            let inner: String = cs[i + 1..i + 1 + j].iter().collect();
            if is_noise(&inner) { i += j + 2; continue; }
        } }
        out.push(cs[i]); i += 1;
    }
    let first = out.split(" - ").next().unwrap_or("").trim().to_string(); // "Titulo - Remastered 2011" -> "Titulo"
    let r = if first.is_empty() { out.trim().to_string() } else { first };
    if r.is_empty() { t.trim().to_string() } else { r }
}
/// Spotify junta varios artistas com ", " ou "; ": a LRCLIB casa melhor com o primeiro.
pub fn first_artist(a: &str) -> String { a.split([',', ';']).next().unwrap_or(a).trim().to_string() }

fn net_state(r: Net, srv: &mut bool) -> Result<Option<String>, ()> {
    match r { Net::Found(b) => Ok(Some(b)), Net::NotFound => Ok(None), Net::Server => { *srv = true; Ok(None) } Net::Fail => Err(()) }
}

/// Busca com cache e normalizacao: tenta (titulo limpo, 1o artista) com album+duracao, sem eles, /search por campos e /search por texto livre.
/// Sem rede e sem cache -> None (nunca quebra). Falha de rede nao grava cache.
pub fn fetch(title: &str, artist: &str, album: &str, dur_sec: u64, cancel: &dyn Fn() -> bool) -> Option<Lyrics> {
    let (t, a, al) = (clean_title(title), first_artist(artist), clean_title(album));
    let key = format!("v2|{}|{}|{}", a.to_lowercase(), t.to_lowercase(), dur_sec / 5); // v2: normalizado; duracao em janelas de 5 s
    let path = cache_path(&key);
    if let Some(l) = crate::store::load_or_quarantine::<Option<Lyrics>>(&path) { return l; }
    if !crate::settings::get().online_lyrics { return None; } // privacidade: sem rede quando o usuario desligou (o cache local continua valendo)
    if NO_NET_UNTIL.lock().unwrap_or_else(|e| e.into_inner()).is_some_and(|t| Instant::now() < t) { crate::log("lyrics: em espera apos falha de rede"); return None; }
    crate::log("lyrics: buscando"); // sem titulo/artista no log
    let base = format!("https://lrclib.net/api/get?artist_name={}&track_name={}", enc(&a), enc(&t));
    let mut urls = Vec::new();
    if !al.is_empty() && dur_sec > 0 { urls.push(format!("{base}&album_name={}&duration={dur_sec}", enc(&al))); }
    if dur_sec > 0 { urls.push(format!("{base}&duration={dur_sec}")); }
    urls.push(base);
    if artist.trim() != a { urls.push(format!("https://lrclib.net/api/get?artist_name={}&track_name={}", enc(artist.trim()), enc(&t))); } // "Tyler, The Creator": tenta o nome completo
    let (mut best, mut srv): (Option<Lyrics>, bool) = (None, false);
    let good = |l: &Lyrics| l.synced && l.lines.len() >= 4; // <4 linhas = suspeito (ex.: texto de teste publicado por terceiros)
    for u in &urls {
        if cancel() { return None; }
        match net_state(http_get(u), &mut srv) {
            Err(()) => return fail(),
            Ok(Some(b)) => if let Some(l) = serde_json::from_str::<serde_json::Value>(&b).ok().and_then(|v| from_json(&v)) {
                crate::log(&format!("lyrics: /get ok sincronizada={} linhas={}", l.synced, l.lines.len()));
                if l.instrumental { best = Some(l); break; }
                if good(&l) { best = Some(l); break; } else if best.is_none() { best = Some(l); } },
            Ok(None) => crate::log("lyrics: /get 404"),
        }
    }
    if !best.as_ref().is_some_and(|b| good(b) || b.instrumental) {
        for u in [format!("https://lrclib.net/api/search?artist_name={}&track_name={}", enc(&a), enc(&t)),
                  format!("https://lrclib.net/api/search?q={}", enc(&format!("{a} {t}")))] {
            if cancel() { return None; }
            match net_state(http_get(&u), &mut srv) {
                Err(()) => return fail(),
                Ok(Some(b)) => if let Some(l) = serde_json::from_str::<serde_json::Value>(&b).ok().and_then(|v| pick_best(&v, dur_sec)) {
                    crate::log(&format!("lyrics: /search ok linhas={}", l.lines.len()));
                    if good(&l) { best = Some(l); break; } },
                Ok(None) => {}
            }
        }
    }
    if best.is_none() { crate::log("lyrics: nao encontrada"); }
    if best.is_none() && srv { return None; } // 5xx no meio: nao grava "nao existe" no cache
    if let Some(d) = path.parent() { let _ = std::fs::create_dir_all(d); }
    crate::store::write_atomic(&path, serde_json::to_string(&best).unwrap_or_default().as_bytes());
    if let Some(d) = path.parent() { prune_cache(d); }
    best
}
// ---- worker unico: a ultima faixa vence; requisicoes antigas sao canceladas entre uma e outra ----
pub struct Job { pub key: String, pub title: String, pub artist: String, pub album: String, pub dur_sec: u64 }
static JOB: Mutex<Option<Job>> = Mutex::new(None);
static JOB_CV: std::sync::Condvar = std::sync::Condvar::new();
static LATEST: Mutex<String> = Mutex::new(String::new());
static STARTED: std::sync::Once = std::sync::Once::new();
/// Pede a letra; `done(chave, resultado)` so e chamado se esta ainda for a faixa mais recente.
pub fn request(job: Job, done: fn(&str, Option<Lyrics>)) {
    *LATEST.lock().unwrap_or_else(|e| e.into_inner()) = job.key.clone();
    *JOB.lock().unwrap_or_else(|e| e.into_inner()) = Some(job);
    JOB_CV.notify_one();
    STARTED.call_once(move || { std::thread::Builder::new().name("letras".into()).spawn(move || {
        unsafe { let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED); }
        loop {
            let job = { let mut g = JOB.lock().unwrap_or_else(|e| e.into_inner()); loop { if let Some(j) = g.take() { break j; } g = JOB_CV.wait(g).unwrap_or_else(|e| e.into_inner()); } };
            let cancelled = || *LATEST.lock().unwrap_or_else(|e| e.into_inner()) != job.key;
            let res = fetch(&job.title, &job.artist, &job.album, job.dur_sec, &cancelled);
            if !cancelled() { done(&job.key, res); }
        }
    }).ok(); });
}

fn fail() -> Option<Lyrics> {
    crate::log("lrclib: falha/timeout, nova tentativa em 5 min");
    *NO_NET_UNTIL.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now() + RETRY_AFTER);
    None
}


#[cfg(test)]
mod tests {
    use super::*;
    fn l(t: u64, s: &str) -> Line { Line { time_ms: t, text: s.into() } }
    #[test] fn lrc_basico() {
        let v = parse_lrc("[ar:X]\n[00:01.50] ola\n[01:02.5]mundo\n[00:03.123]fim");
        assert_eq!(v, vec![l(1500, "ola"), l(3123, "fim"), l(62_500, "mundo")]);
    }
    #[test] fn lrc_varias_marcas_e_ordem() {
        let v = parse_lrc("[00:20.00][00:10.00]refrao\n[00:15.00]verso");
        assert_eq!(v, vec![l(10_000, "refrao"), l(15_000, "verso"), l(20_000, "refrao")]);
    }
    #[test] fn lrc_linha_vazia_mantida_e_lixo_ignorado() {
        let v = parse_lrc("[00:05.00]\nsem marca\n[xx:yy]nada\n[00:70.00]seg invalido\r\n[00:06.00]a");
        assert_eq!(v, vec![l(5000, ""), l(6000, "a")]);
    }
    #[test] fn lrc_offset() {
        assert_eq!(parse_lrc("[offset:500]\n[00:01.00]a\n[00:00.20]b"), vec![l(0, "b"), l(500, "a")]);
    }
    #[test] fn lrc_vazio() { assert!(parse_lrc("").is_empty()); assert!(parse_lrc("so texto").is_empty()); }
    #[test] fn plain_por_paragrafo() {
        assert_eq!(parse_plain("a\nb\r\n\r\nc\n\n\n\nd "), vec![l(0, "a\nb"), l(0, "c"), l(0, "d")]);
    }
    #[test] fn json_prefere_sincronizada_e_trata_instrumental() {
        let v: serde_json::Value = serde_json::json!({"syncedLyrics":"[00:01.00]x","plainLyrics":"x"});
        assert_eq!(from_json(&v), Some(Lyrics { synced: true, lines: vec![l(1000, "x")], instrumental: false }));
        let v = serde_json::json!({"syncedLyrics":null,"plainLyrics":"p1\n\np2"});
        assert_eq!(from_json(&v).map(|x| (x.synced, x.lines.len())), Some((false, 2)));
        assert_eq!(from_json(&serde_json::json!({"instrumental":true,"plainLyrics":"x"})).map(|x| (x.instrumental, x.lines.len())), Some((true, 0)));
        assert_eq!(from_json(&serde_json::json!({"syncedLyrics":null,"plainLyrics":null})), None);
    }
    #[test] fn prune_apaga_mais_antigos_por_contagem_e_por_bytes() {
        let t = |s| SystemTime::UNIX_EPOCH + Duration::from_secs(s);
        let f = || vec![(PathBuf::from("c"), t(3), 10), (PathBuf::from("a"), t(1), 10), (PathBuf::from("b"), t(2), 10)];
        assert!(to_prune(f(), 3, 100).is_empty());
        assert_eq!(to_prune(f(), 2, 100), vec![PathBuf::from("a")]);
        assert_eq!(to_prune(f(), 10, 15), vec![PathBuf::from("a"), PathBuf::from("b")]);
    }
    #[test] fn pick_best_escolhe_mais_linhas_e_respeita_duracao() {
        let v = serde_json::json!([
            {"duration": 213.0, "syncedLyrics": "[00:00.00]probe"},
            {"duration": 213.0, "syncedLyrics": "[00:01.00]a
[00:02.00]b
[00:03.00]c
[00:04.00]d"},
            {"duration": 300.0, "syncedLyrics": "[00:01.00]a
[00:02.00]b
[00:03.00]c
[00:04.00]d
[00:05.00]e"},
            {"duration": 213.0, "syncedLyrics": null, "plainLyrics": "x

y"}]);
        assert_eq!(pick_best(&v, 214).unwrap().lines.len(), 4);
        assert_eq!(pick_best(&v, 0).unwrap().lines.len(), 5);
        assert!(pick_best(&serde_json::json!([]), 1).is_none());
    }
    #[test] fn normaliza_titulos_do_spotify() {
        assert_eq!(clean_title("Never Gonna Give You Up - Remastered 2022"), "Never Gonna Give You Up");
        assert_eq!(clean_title("Song (feat. Someone) [Live]"), "Song");
        assert_eq!(clean_title("Bohemian Rhapsody - 2011 Mix"), "Bohemian Rhapsody");
        assert_eq!(clean_title("Hello (Radio Edit)"), "Hello");
        assert_eq!(clean_title("Song (2011)"), "Song");
        assert_eq!(clean_title("Heroes (Part 2)"), "Heroes (Part 2)"); // parenteses legitimos ficam
        assert_eq!(clean_title("Where Is My Mind?"), "Where Is My Mind?");
        assert_eq!(clean_title("(Live)"), "(Live)"); // nunca devolve vazio
        assert_eq!(clean_title("Nothing Else Matters - Remastered"), "Nothing Else Matters");
        assert_eq!(first_artist("Daft Punk, Pharrell Williams"), "Daft Punk");
        assert_eq!(first_artist("Simon & Garfunkel"), "Simon & Garfunkel");
        assert_eq!(first_artist("AC/DC"), "AC/DC");
    }
    #[test] fn limites_de_entrada() {
        assert!(parse_lrc("[99999999999999999999:00.00]x").is_empty());       // estouro de u64 no parse
        assert!(parse_lrc("[7000:00.00]x").is_empty());                        // minutos absurdos
        assert_eq!(parse_lrc("[offset:99999999999999999]\n[00:01.00]a")[0].time_ms, 0); // offset clampado
        let many = (0..5000).map(|i| format!("[00:{:02}.00]l", i % 60)).collect::<Vec<_>>().join("\n");
        assert_eq!(parse_lrc(&many).len(), MAX_LINES);
        let long = format!("[00:01.00]{}", "x".repeat(2000));
        assert_eq!(parse_lrc(&long)[0].text.chars().count(), MAX_LINE_CHARS);
    }
    #[test] fn ruido_so_por_palavra_inteira() {
        assert_eq!(clean_title("Alive (Alive)"), "Alive (Alive)");
        assert_eq!(clean_title("Song (Remix)"), "Song (Remix)");
        assert_eq!(clean_title("Stay (With You)"), "Stay (With You)");
        assert_eq!(clean_title("Song (feat. X)"), "Song");
        assert_eq!(clean_title("Song (From \"Movie\")"), "Song");
        assert_eq!(clean_title("Song (Remastered 2011)"), "Song");
    }
    #[test] fn url_encode_e_hash_estavel() {
        assert_eq!(enc("Beyoncé & Co/1"), "Beyonc%C3%A9%20%26%20Co%2F1");
        assert_eq!(fnv("a"), fnv("a")); assert_ne!(fnv("a"), fnv("b"));
    }
    /// Rede real (cache em pasta temporaria, nunca no APPDATA): `cargo test -- --ignored`
    #[test] #[ignore] fn lrclib_real() {
        let d = std::env::temp_dir().join(format!("eavely-lyrics-test-{}", std::process::id()));
        std::env::set_var("EAVELY_DATA_DIR", &d);
        let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED) };
        // entradas no estilo Spotify (sufixos, varios artistas, album "Deluxe", duracao levemente errada ou 0)
        for (t, a, al, dur) in [("Never Gonna Give You Up - Remastered 2022", "Rick Astley", "Whenever You Need Somebody (2022 Remaster)", 214),
                                ("Shape of You", "Ed Sheeran, Someone", "÷ (Deluxe)", 0), ("Blinding Lights", "The Weeknd", "After Hours (Deluxe)", 250)] {
            let l = fetch(t, a, al, dur, &|| false).unwrap_or_else(|| panic!("sem letra para {t}"));
            assert!(l.synced && l.lines.len() > 10 && l.lines.iter().any(|x| x.time_ms > 0), "{t}: {} linhas", l.lines.len());
            println!("{t}: sincronizada, {} linhas, 1a: {:?}", l.lines.len(), l.lines[0]);
            assert!(fetch(t, a, al, dur, &|| false).is_some()); // 2a vez: do cache
        }
        assert_eq!(std::fs::read_dir(d.join("lyrics")).unwrap().count(), 3);
        let _ = std::fs::remove_dir_all(&d);
    }
}
