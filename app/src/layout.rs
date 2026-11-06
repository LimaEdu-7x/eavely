// Metricas de layout da ilha em px logicos de projeto. UM fator `k` (preset de tamanho do notch: 1.0 = Padrao) multiplica todas as constantes de forma/janela;
// nada de numero magico espalhado: forma, regiao da janela, hit-test do mini notch e flares leem daqui. Com k = 1.0 os valores sao exatamente os de antes.
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
static K: AtomicU32 = AtomicU32::new(0x3F80_0000); // 1.0f32
pub fn k() -> f32 { f32::from_bits(K.load(Relaxed)) }
/// preset de tamanho (0.6..1.5); chamado ao ler as configuracoes
pub fn set_k(v: f32) { K.store(v.clamp(0.6, 1.5).to_bits(), Relaxed); }
macro_rules! m { ($($n:ident = $v:expr;)*) => { $(#[inline] pub fn $n() -> f32 { $v * k() })* } }
m! {
    win_w = 600.0;      // janela maxima (520 das letras + flares de 18 + folga para o overshoot)
    open_w = 420.0; open_flare = 18.0;
    split_w = 520.0;    // aberta com letras ao lado
    dock_extra = 32.0;  // dock/calendario
    compact_h = 30.0; compact_r = 14.0; compact_flare = 6.0;
    over_w = 280.0; over_h = 48.0; over_r = 22.0;
    open_r = 28.0;
    mini_w = 56.0; mini_h = 5.0; mini_r = 2.5; mini_flare = 4.0; // MINI NOTCH (janela maximizada)
}
/// altura extra (unidades de projeto) que o preset PEQUENO ganha: a tipografia tem piso em px reais, entao o conteudo ocupa mais que 80 % e o rodape (pontos, dock) nao pode ser cortado.
/// 0 em k >= 1; +10 em 0,8 (a ilha fica com ~120 px reais em vez de 112).
#[inline] pub fn pad() -> f32 { (1.0 - k()).max(0.0) * 50.0 }
#[inline] pub fn open_h() -> f32 { (140.0 + pad()) * k() }
#[inline] pub fn win_h() -> f32 { (176.0 + pad()) * k() } // 164 com dock + folga
/// tamanho de fonte (px logicos) no preset atual: escala com k, mas nunca abaixo do piso legivel (9 px). Preset pequeno ganha tipografia propria sobre este piso.
#[inline] pub fn font(base: f32) -> f32 { (base * k()).max(FONT_FLOOR.min(base)) }
pub const FONT_FLOOR: f32 = 9.0;
/// tamanho de fonte/icone em unidades de PROJETO para que o tamanho REAL (px) nao fique abaixo do piso: real = base*k, piso em px reais -> projeto = max(base, piso/k).
/// Pisos (rig-v2 'tamanhos'): titulo/valor 11, secundario 10, icone 12.
#[inline] pub fn design_px(base: f32, floor_px: f32, k: f32) -> f32 { base.max(floor_px / k.max(0.1)) }
pub const FLOOR_TITLE: f32 = 11.0; pub const FLOOR_SECOND: f32 = 10.0; pub const FLOOR_ICON: f32 = 12.0;
#[cfg(test)]
mod tests {
    use super::*;
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(()); // k e global: os testes que o mudam rodam um de cada vez
    fn lock() -> std::sync::MutexGuard<'static, ()> { SERIAL.lock().unwrap_or_else(|e| e.into_inner()) }
    /// presets de tamanho previstos: a geometria precisa continuar coerente em todos (ordem das formas, janela comporta a maior forma + flares + folga do overshoot)
    #[test] fn invariantes_da_geometria_em_todos_os_presets() {
        let _g = lock();
        for k10 in [6, 7, 8, 9, 10, 11, 12, 13, 15] {
            set_k(k10 as f32 / 10.0);
            assert!(mini_h() < compact_h() && compact_h() < over_h() && over_h() < open_h(), "ordem das alturas em k={}", k());
            assert!(mini_w() < over_w() && over_w() < open_w() && open_w() < split_w(), "ordem das larguras em k={}", k());
            assert!(compact_flare() <= open_flare() && mini_flare() <= compact_flare(), "flares em k={}", k());
            assert!(win_w() - 4.0 - 2.0 * open_flare() >= split_w(), "janela comporta as letras ao lado em k={}", k());
            assert!(win_h() >= open_h() + dock_extra() + 2.0, "janela comporta o dock em k={}", k());
            assert!(mini_r() <= mini_h() && compact_r() <= compact_h(), "raios nao passam da altura em k={}", k());
        }
        set_k(1.0);
    }
    #[test] fn pequeno_ganha_altura_para_o_rodape_e_padrao_nao_muda() {
        let _g = lock(); set_k(1.0); assert_eq!((pad(), open_h(), win_h()), (0.0, 140.0, 176.0)); set_k(1.2); assert_eq!(pad(), 0.0);
        set_k(0.8); assert!((pad() - 10.0).abs() < 1e-4 && (open_h() - 120.0).abs() < 1e-3 && win_h() >= open_h() + dock_extra() + 2.0, "pad {} open {} win {}", pad(), open_h(), win_h()); set_k(1.0);
    }
    #[test] fn pisos_de_tipografia_do_rig() {
        // 80 %: titulo 12 -> 9,6 ingenuo -> piso 11; secundario 10 -> 8 -> piso 10; icone 14 -> 11,2 -> piso 12
        let real = |b: f32, f: f32, k: f32| design_px(b, f, k) * k;
        assert!((real(12.0, FLOOR_TITLE, 0.8) - 11.0).abs() < 1e-3 && (real(10.0, FLOOR_SECOND, 0.8) - 10.0).abs() < 1e-3 && (real(14.0, FLOOR_ICON, 0.8) - 12.0).abs() < 1e-3);
        // 100 % e 120 %: a base ja passa do piso, nada muda
        for k in [1.0f32, 1.2] { assert_eq!(design_px(12.0, FLOOR_TITLE, k), 12.0); assert_eq!(design_px(10.0, FLOOR_SECOND, k), 10.0); assert_eq!(design_px(14.0, FLOOR_ICON, k), 14.0); }
    }
    #[test] fn fonte_respeita_o_piso_e_no_padrao_e_identidade() {
        let _g = lock(); set_k(1.0);
        assert_eq!([10.0, 11.0, 12.0, 14.0, 16.0].map(font), [10.0, 11.0, 12.0, 14.0, 16.0]);
        set_k(0.6); assert_eq!(font(10.0), 9.0); assert_eq!(font(16.0), 9.6); assert_eq!(font(8.0), 8.0); // base ja abaixo do piso nao sobe
        set_k(1.5); assert_eq!(font(10.0), 15.0); set_k(1.0);
    }
    #[test] fn padrao_mantem_os_valores_de_sempre_e_o_fator_escala_tudo() {
        let _g = lock(); set_k(1.0);
        assert_eq!((win_w(), win_h(), open_w(), open_h(), open_flare(), mini_w(), mini_h()), (600.0, 176.0, 420.0, 140.0, 18.0, 56.0, 5.0));
        assert_eq!((split_w(), dock_extra(), compact_h(), compact_r(), compact_flare(), over_w(), over_h(), over_r(), open_r(), mini_r(), mini_flare()), (520.0, 32.0, 30.0, 14.0, 6.0, 280.0, 48.0, 22.0, 28.0, 2.5, 4.0));
        set_k(0.8); assert!((win_w() - 480.0).abs() < 1e-3 && (mini_h() - 4.0).abs() < 1e-3); set_k(1.0); assert_eq!(open_h(), 140.0);
        set_k(9.0); assert_eq!(k(), 1.5); set_k(1.0);
    }
}
