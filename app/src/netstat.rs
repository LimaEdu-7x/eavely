// Estado de rede para o icone do compacto em PC sem bateria: Wi-Fi com 0-4 barras (WlanQueryInterface), cabeada ou sem rede.
// Atualizado por EVENTO (NotifyIpInterfaceChange + WlanRegisterNotification); nada de polling parado. Cache em um atomico.
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use windows::Win32::{Foundation::*, NetworkManagement::{IpHelper::*, Ndis::*, WiFi::*}};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Net { Off, Eth, Wifi(u8) } // Wifi(barras 0..=4)

static CUR: AtomicU32 = AtomicU32::new(0); // 0 sem rede, 1 cabeada, 2+n = Wi-Fi com n barras
pub fn get() -> Net { match CUR.load(Relaxed) { 0 => Net::Off, 1 => Net::Eth, v => Net::Wifi((v - 2).min(4) as u8) } }

/// qualidade 0-100 (WLAN_ASSOCIATION_ATTRIBUTES) -> barras
pub fn bars(q: u32) -> u8 { match q { 0 => 0, 1..=25 => 1, 26..=50 => 2, 51..=75 => 3, _ => 4 } }

fn wifi_quality() -> Option<u32> { unsafe {
    let (mut ver, mut h) = (0u32, HANDLE::default());
    if WlanOpenHandle(2, None, &mut ver, &mut h) != 0 { return None; } // servico WLAN ausente (PC sem Wi-Fi): sem Wi-Fi
    let mut best = None;
    let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
    if WlanEnumInterfaces(h, None, &mut list) == 0 && !list.is_null() {
        let l = &*list;
        for i in 0..l.dwNumberOfItems as usize {
            let info = &*l.InterfaceInfo.as_ptr().add(i);
            if info.isState != wlan_interface_state_connected { continue; }
            let (mut sz, mut data) = (0u32, std::ptr::null_mut::<core::ffi::c_void>());
            if WlanQueryInterface(h, &info.InterfaceGuid, wlan_intf_opcode_current_connection, None, &mut sz, &mut data, None) == 0 && !data.is_null() {
                let q = (*(data as *const WLAN_CONNECTION_ATTRIBUTES)).wlanAssociationAttributes.wlanSignalQuality;
                best = Some(best.map_or(q, |b: u32| b.max(q))); // varios adaptadores conectados: vale o de MELHOR sinal
                WlanFreeMemory(data);
            }
        }
        WlanFreeMemory(list as *const _);
    }
    WlanCloseHandle(h, None);
    best
} }

/// placa cabeada FISICA conectada (ignora virtuais: Hyper-V, VPN, WSL)
fn eth_up() -> bool { unsafe {
    let mut t: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    if GetIfTable2(&mut t).0 != 0 || t.is_null() { return false; }
    let tb = &*t; let mut up = false;
    for i in 0..tb.NumEntries as usize {
        let r = &*tb.Table.as_ptr().add(i);
        let hw = r.InterfaceAndOperStatusFlags._bitfield & 1 != 0; // HardwareInterface
        if r.Type == 6 && hw && r.OperStatus == IfOperStatusUp && r.MediaConnectState == MediaConnectStateConnected { up = true; break; }
    }
    FreeMibTable(t as *const _);
    up
} }

/// Le o estado agora. Devolve true se mudou.
pub fn refresh() -> bool {
    let n = if eth_up() { Net::Eth } else if let Some(q) = wifi_quality() { Net::Wifi(bars(q)) } else { Net::Off };
    let v = match n { Net::Off => 0, Net::Eth => 1, Net::Wifi(b) => 2 + b as u32 };
    CUR.swap(v, Relaxed) != v
}

unsafe extern "system" fn wlan_cb(_: *mut L2_NOTIFICATION_DATA, _: *mut core::ffi::c_void) { crate::island::net_changed(); } // so avisa a ilha (debounce la)
static WLAN_H: AtomicU32 = AtomicU32::new(0);
/// Registra o aviso de mudanca do Wi-Fi (sinal/conexao). Uma vez; sem Wi-Fi registra nada.
pub fn watch() { unsafe {
    if WLAN_H.swap(1, Relaxed) != 0 { return; }
    let (mut ver, mut h) = (0u32, HANDLE::default());
    if WlanOpenHandle(2, None, &mut ver, &mut h) != 0 { crate::state_log("rede: servico WLAN indisponivel (sem Wi-Fi); icone de rede so cabeado"); return; }
    let r = WlanRegisterNotification(h, WLAN_NOTIFICATION_SOURCE_ACM | WLAN_NOTIFICATION_SOURCE_MSM, false, Some(wlan_cb), None, None, None);
    crate::state_log(&format!("rede: WlanRegisterNotification = {r}")); // a alca fica aberta de proposito (vida do processo)
} }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn barras_pela_qualidade() { assert_eq!([0, 1, 25, 26, 50, 51, 75, 76, 100].map(bars), [0, 1, 1, 2, 2, 3, 3, 4, 4]); }
}
