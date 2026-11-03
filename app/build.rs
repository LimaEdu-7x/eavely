// Manifesto (DPI por monitor v2, Windows 10/11) + icone do app embutidos no exe.
fn main() {
    // numero de compilacao (data.hora UTC): muda a cada build, para as Configuracoes mostrarem a versao real do exe
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, tod) = ((secs / 86400) as i64, secs % 86400);
    let z = days + 719468; let era = z.div_euclid(146097); let doe = z.rem_euclid(146097); let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; let y = yoe + era * 400; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 }; let y = if m <= 2 { y + 1 } else { y };
    println!("cargo:rustc-env=EAVELY_BUILD={:04}{:02}{:02}.{:02}{:02}", y, m, d, tod / 3600, (tod % 3600) / 60);
    println!("cargo:rustc-link-arg-bins=/PDBALTPATH:eavely.pdb"); // nao grava o caminho local do PDB no exe
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=../assets/app/eavely-icon.ico");
    let mut res = winres::WindowsResource::new();
    res.set_icon("../assets/app/eavely-icon.ico");
    res.set_manifest(r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3"><security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security></trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1"><application>
    <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
  </application></compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3"><windowsSettings>
    <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
    <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
  </windowsSettings></application>
</assembly>"#);
    res.set("ProductName", "Eavely");
    res.set("FileDescription", "Eavely - ilha de notificacoes e midia");
    res.set("LegalCopyright", "MIT");
    if let Err(e) = res.compile() { println!("cargo:warning=winres: {e}"); }
}
