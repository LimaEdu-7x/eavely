//! Canal de build: estavel (padrao) ou dev (`--features dev`). Mesmo codigo; so nomes, pastas, mutex, chave Run, AUMID, icone e classes de janela mudam.
//! Assim a instalacao dev convive com a estavel (nada compartilhado, exceto a leitura inicial das configuracoes da estavel).

pub const DEV: bool = cfg!(feature = "dev");
/// nome mostrado (bandeja, janelas, Run, instalador)
pub const NAME: &str = if DEV { "Eavely Dev" } else { "Eavely" };
/// pasta de dados: %APPDATA%\Eavely ou %APPDATA%\EavelyDev
pub const DATA_DIR: &str = if DEV { "EavelyDev" } else { "Eavely" };
/// mutex de instancia unica (Local\...)
pub const MUTEX: &str = if DEV { "Local\\EavelyDevNotchSingleInstance" } else { "Local\\EavelyNotchSingleInstance" };
/// AppUserModelID (notificacoes e atalhos do Windows)
pub const AUMID: &str = if DEV { "app.eavely.dev" } else { "app.eavely" };
/// classes de janela (a ilha, a tela de configuracoes e o rastro); distintas para as duas instalacoes coexistirem
pub const CLASS_ISLAND: &str = if DEV { "EavelyDevIsland" } else { "EavelyIsland" };
pub const CLASS_SETTINGS: &str = if DEV { "EavelyDevSettings" } else { "EavelySettings" };
pub const CLASS_TRACE: &str = if DEV { "EavelyDevTrace" } else { "EavelyTrace" };
/// icone da bandeja (16/20/24/32 px) e do exe (ver build.rs)
pub const TRAY_ICON_PREFIX: &str = if DEV { "eavely-dev-tray" } else { "eavely-tray" };

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn canal_estavel_mantem_os_nomes_antigos() { if !DEV { assert_eq!((NAME, DATA_DIR, AUMID), ("Eavely", "Eavely", "app.eavely")); assert_eq!(CLASS_ISLAND, "EavelyIsland"); } }
    #[test] fn canal_dev_nao_colide_com_a_estavel() { if DEV { assert_ne!(MUTEX, "Local\\EavelyNotchSingleInstance"); assert_ne!(AUMID, "app.eavely"); assert_ne!(DATA_DIR, "Eavely"); assert_ne!(CLASS_ISLAND, "EavelyIsland"); } }
}

/// 1a execucao do canal dev: copia as configuracoes da estavel para a pasta dev. So LE a estavel; nunca escreve nela.
/// Roda uma vez (se a pasta dev ja tem settings.json, nao faz nada).
pub fn seed_settings() {
    if !DEV { return; }
    let Some(appdata) = std::env::var_os("APPDATA") else { return };
    let base = std::path::PathBuf::from(appdata);
    let (stable, dev_dir) = (base.join("Eavely").join("settings.json"), base.join(DATA_DIR));
    let dst = dev_dir.join("settings.json");
    if dst.exists() || !stable.exists() { return; }
    let _ = std::fs::create_dir_all(&dev_dir);
    let _ = std::fs::copy(&stable, &dst);
}

/// string UTF-16 com NUL, viva para sempre (nomes de classe/janela do canal; criada uma vez)
pub fn wide(s: &str) -> &'static [u16] { Box::leak(s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>().into_boxed_slice()) }
