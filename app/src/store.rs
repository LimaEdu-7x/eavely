// Persistencia de JSON: escrita atomica (tmp unico + rename) e leitura que poe em quarentena um arquivo corrompido (.bad) em vez de perde-lo.
use serde::de::DeserializeOwned;
use std::{fs, path::Path, sync::atomic::{AtomicU64, Ordering::Relaxed}};

static SEQ: AtomicU64 = AtomicU64::new(0);

/// tmp com nome unico (pid + contador) no mesmo diretorio, depois rename: nunca deixa o arquivo pela metade nem disputa o tmp entre threads.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> bool {
    if let Some(d) = path.parent() { let _ = fs::create_dir_all(d); }
    let mut tmp = path.as_os_str().to_owned(); tmp.push(format!(".tmp{}-{}", std::process::id(), SEQ.fetch_add(1, Relaxed)));
    let tmp = std::path::PathBuf::from(tmp);
    if fs::write(&tmp, bytes).is_err() { return false; }
    if fs::rename(&tmp, path).is_err() { let _ = fs::remove_file(&tmp); return false; }
    true
}

/// None se o arquivo nao existe OU esta corrompido; no segundo caso ele e renomeado para <nome>.bad (o proximo save nao o sobrescreve).
pub fn load_or_quarantine<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let s = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { if e.kind() != std::io::ErrorKind::NotFound { crate::log(&format!("leitura de {}: {e}", path.display())); } return None; }
    };
    match serde_json::from_str(s.trim_start_matches('\u{feff}')) {
        Ok(v) => Some(v),
        Err(e) => {
            let mut bad = path.as_os_str().to_owned(); bad.push(".bad");
            let bad = std::path::PathBuf::from(bad);
            let _ = fs::remove_file(&bad);
            let moved = fs::rename(path, &bad).is_ok();
            crate::log(&format!("{} corrompido ({e}); {}", path.display(), if moved { "guardado como .bad" } else { "nao consegui renomear" }));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tmpdir(n: &str) -> std::path::PathBuf { let d = std::env::temp_dir().join(format!("eavely-store-{n}-{}", std::process::id())); let _ = fs::remove_dir_all(&d); fs::create_dir_all(&d).unwrap(); d }
    #[test] fn escrita_atomica_ida_e_volta_sem_sobras() {
        let d = tmpdir("w"); let f = d.join("a.json");
        assert!(write_atomic(&f, b"[1,2]")); assert!(write_atomic(&f, b"[3]"));
        assert_eq!(load_or_quarantine::<Vec<u32>>(&f), Some(vec![3]));
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1); // nenhum .tmp restou
        let _ = fs::remove_dir_all(&d);
    }
    #[test] fn json_corrompido_vai_para_bad_e_nao_e_sobrescrito() {
        let d = tmpdir("q"); let f = d.join("todo.json");
        fs::write(&f, b"[{\"id\":1,\"te").unwrap(); // truncado
        assert_eq!(load_or_quarantine::<Vec<u32>>(&f), None);
        assert!(!f.exists()); assert_eq!(fs::read(d.join("todo.json.bad")).unwrap(), b"[{\"id\":1,\"te");
        assert!(write_atomic(&f, b"[]")); assert!(d.join("todo.json.bad").exists()); // salvar o padrao nao apaga o .bad
        let _ = fs::remove_dir_all(&d);
    }
    #[test] fn ausente_e_bom_utf8() {
        let d = tmpdir("m"); assert_eq!(load_or_quarantine::<Vec<u32>>(&d.join("x.json")), None); assert!(!d.join("x.json.bad").exists());
        fs::write(d.join("b.json"), b"\xEF\xBB\xBF[7]").unwrap(); assert_eq!(load_or_quarantine::<Vec<u32>>(&d.join("b.json")), Some(vec![7]));
        let _ = fs::remove_dir_all(&d);
    }
}
