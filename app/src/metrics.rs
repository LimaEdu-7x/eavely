//! Metricas do compacto de PC (CPU, memoria, GPU, rede, microfone/camera em uso). Amostradas a 1 Hz POR UM TIMER DA ILHA e SO com o compacto visivel
//! (nenhum polling com a ilha oculta, em tela cheia, aberta ou desligada). Sem thread propria, sem dados pessoais (so contadores do sistema).
use std::sync::Mutex;
use windows::Win32::{Foundation::FILETIME, System::{SystemInformation::*, Threading::GetSystemTimes}};

#[derive(Clone, Copy, Default, Debug)]
pub struct Snap { pub cpu: f32, pub ram: f32, pub gpu: Option<f32>, pub down: f64, pub up: f64, pub mic: bool, pub cam: bool }
#[derive(Clone, Copy, PartialEq, Debug)] pub enum Metric { Cpu, Ram, Gpu, Net }

static SNAP: Mutex<Option<Snap>> = Mutex::new(None);
struct St { cpu: Option<(u64, u64, u64)>, net: Option<(u64, u64, std::time::Instant)>, gpu: Gpu }
static ST: Mutex<Option<St>> = Mutex::new(None);
pub fn snap() -> Snap { SNAP.lock().unwrap_or_else(|e| e.into_inner()).unwrap_or_default() }
/// zera os acumuladores (a primeira leitura apos reabrir nao usa um intervalo antigo)
pub fn reset() { *ST.lock().unwrap_or_else(|e| e.into_inner()) = None; }

fn ft(f: FILETIME) -> u64 { ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64 }

/// Le so o que esta ligado (`want`: cpu, ram, gpu, net, privacidade mic/cam)
pub fn sample(cpu: bool, ram: bool, gpu: bool, net: bool, privacy: bool) {
    let mut g = ST.lock().unwrap_or_else(|e| e.into_inner());
    let st = g.get_or_insert_with(|| St { cpu: None, net: None, gpu: Gpu::default() });
    let mut s = SNAP.lock().unwrap_or_else(|e| e.into_inner()).unwrap_or_default();
    unsafe {
        if cpu {
            let (mut i, mut k, mut u) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
            if GetSystemTimes(Some(&mut i), Some(&mut k), Some(&mut u)).is_ok() {
                let (idle, total) = (ft(i), ft(k) + ft(u)); // kernel inclui o ocioso
                if let Some((pi, pt, _)) = st.cpu { let (di, dt) = (idle.saturating_sub(pi), total.saturating_sub(pt)); if dt > 0 { s.cpu = (100.0 * (1.0 - di as f64 / dt as f64)).clamp(0.0, 100.0) as f32; } }
                st.cpu = Some((idle, total, 0));
            }
        }
        if ram { let mut m = MEMORYSTATUSEX { dwLength: size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() }; if GlobalMemoryStatusEx(&mut m).is_ok() { s.ram = m.dwMemoryLoad as f32; } }
        if gpu { s.gpu = st.gpu.read(); }
        if net { if let Some((rx, tx)) = net_bytes() { let now = std::time::Instant::now();
            if let Some((prx, ptx, t0)) = st.net { let dt = (now - t0).as_secs_f64().max(0.2); s.down = rx.saturating_sub(prx) as f64 / dt; s.up = tx.saturating_sub(ptx) as f64 / dt; }
            st.net = Some((rx, tx, now)); } }
        if privacy { s.mic = in_use("microphone"); s.cam = in_use("webcam"); }
    }
    *SNAP.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
}

/// bytes recebidos/enviados por interfaces FISICAS ativas (evita contar adaptadores virtuais/VPN em dobro)
unsafe fn net_bytes() -> Option<(u64, u64)> {
    use windows::Win32::NetworkManagement::IpHelper::*;
    let mut t: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    if GetIfTable2(&mut t).0 != 0 || t.is_null() { return None; }
    let (mut rx, mut tx) = (0u64, 0u64);
    let n = (*t).NumEntries as usize; let rows = std::slice::from_raw_parts((*t).Table.as_ptr(), n);
    for r in rows { if r.OperStatus == windows::Win32::NetworkManagement::Ndis::IfOperStatusUp && r.Type != 24 /* loopback */ && (r.InterfaceAndOperStatusFlags._bitfield & 1) != 0 { rx += r.InOctets; tx += r.OutOctets; } }
    FreeMibTable(t as *const _); Some((rx, tx))
}

/// microfone/camera em uso: o Windows grava LastUsedTimeStart/Stop por app (Stop = 0 enquanto usa)
unsafe fn in_use(cap: &str) -> bool {
    use windows::{core::{w, HSTRING, PCWSTR}, Win32::System::Registry::*};
    let root = format!(r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\{cap}");
    let _ = w!("");
    let scan = |path: &str| -> bool {
        let mut h = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, KEY_READ, &mut h).is_err() { return false; }
        let mut used = false; let mut i = 0u32;
        loop {
            let mut name = [0u16; 260]; let mut n = name.len() as u32;
            if RegEnumKeyExW(h, i, Some(windows::core::PWSTR(name.as_mut_ptr())), &mut n, None, None, None, None).is_err() { break; }
            i += 1; if &name[..n as usize] == "NonPackaged".encode_utf16().collect::<Vec<_>>().as_slice() { continue; }
            let mut sub = HKEY::default();
            if RegOpenKeyExW(h, PCWSTR(name.as_ptr()), None, KEY_READ, &mut sub).is_ok() {
                let rd = |v: &str| -> u64 { let mut b = [0u8; 8]; let mut sz = 8u32; let mut ty = REG_VALUE_TYPE(0);
                    if RegQueryValueExW(sub, &HSTRING::from(v), None, Some(&mut ty), Some(b.as_mut_ptr()), Some(&mut sz)).is_ok() && sz == 8 { u64::from_le_bytes(b) } else { 0 } };
                let (start, stop) = (rd("LastUsedTimeStart"), rd("LastUsedTimeStop")); if start != 0 && stop == 0 { used = true; }
                let _ = RegCloseKey(sub);
            }
            if used { break; }
        }
        let _ = RegCloseKey(h); used
    };
    scan(&root) || scan(&format!(r"{root}\NonPackaged"))
}

/// GPU: contadores PDH "GPU Engine" (% por engine; usamos o maior total por tipo de engine, como o Gerenciador de Tarefas)
#[derive(Default)] struct Gpu { q: isize, c: isize, tried: bool, ok: bool }
impl Gpu {
    unsafe fn read(&mut self) -> Option<f32> {
        use windows::{core::{w, PCWSTR}, Win32::System::Performance::*};
        if !self.tried {
            self.tried = true; let mut q = PDH_HQUERY::default();
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut q) == 0 {
                let mut c = PDH_HCOUNTER::default();
                if PdhAddEnglishCounterW(q, w!(r"\GPU Engine(*)\Utilization Percentage"), 0, &mut c) == 0 { self.q = q.0 as isize; self.c = c.0 as isize; self.ok = true; let _ = PdhCollectQueryData(q); return None; }
                let _ = PdhCloseQuery(q);
            }
            return None;
        }
        if !self.ok { return None; }
        let (hq, hc) = (PDH_HQUERY(self.q as *mut _), PDH_HCOUNTER(self.c as *mut _));
        if PdhCollectQueryData(hq) != 0 { return None; }
        let (mut size, mut count) = (0u32, 0u32);
        let _ = PdhGetFormattedCounterArrayW(hc, PDH_FMT_DOUBLE, &mut size, &mut count, None); // PDH_MORE_DATA: tamanho necessario
        if size == 0 || count == 0 { return None; }
        let mut buf = vec![0u8; size as usize];
        let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
        if PdhGetFormattedCounterArrayW(hc, PDH_FMT_DOUBLE, &mut size, &mut count, Some(items)) != 0 { return None; }
        let mut per: std::collections::HashMap<String, f64> = Default::default();
        for i in 0..count as usize { let it = &*items.add(i); let name = it.szName.to_string().unwrap_or_default();
            let ty = name.rsplit("engtype_").next().unwrap_or("").to_string(); if it.FmtValue.CStatus == 0 { *per.entry(ty).or_default() += it.FmtValue.Anonymous.doubleValue; } }
        Some(per.values().cloned().fold(0.0, f64::max).clamp(0.0, 100.0) as f32)
    }
}

/// "1,2 MB/s", "340 KB/s", "0 B/s"
pub fn fmt_rate(b: f64) -> String {
    if b >= 1_048_576.0 { format!("{:.1} MB/s", b / 1_048_576.0).replace('.', ",") } else if b >= 1024.0 { format!("{:.0} KB/s", b / 1024.0) } else { format!("{:.0} B/s", b) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn formata_velocidade() { assert_eq!(fmt_rate(0.0), "0 B/s"); assert_eq!(fmt_rate(2048.0), "2 KB/s"); assert_eq!(fmt_rate(1_572_864.0), "1,5 MB/s"); }
    #[test] fn amostra_real_nao_quebra_e_fica_em_faixa() {
        sample(true, true, true, true, true); std::thread::sleep(std::time::Duration::from_millis(400)); sample(true, true, true, true, true);
        let s = snap(); println!("SNAP {s:?}"); assert!((0.0..=100.0).contains(&s.cpu) && (0.0..=100.0).contains(&s.ram) && s.down >= 0.0 && s.up >= 0.0, "{s:?}");
    }
}
