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
    win_h = 176.0;      // 164 com dock + folga
    open_w = 420.0; open_h = 140.0; open_flare = 18.0;
    split_w = 520.0;    // aberta com letras ao lado
    dock_extra = 32.0;  // dock/calendario
    compact_h = 30.0; compact_r = 14.0; compact_flare = 6.0;
    over_w = 280.0; over_h = 48.0; over_r = 22.0;
    open_r = 28.0;
    mini_w = 56.0; mini_h = 5.0; mini_r = 2.5; mini_flare = 4.0; // MINI NOTCH (janela maximizada)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn padrao_mantem_os_valores_de_sempre_e_o_fator_escala_tudo() {
        assert_eq!((win_w(), win_h(), open_w(), open_h(), open_flare(), mini_w(), mini_h()), (600.0, 176.0, 420.0, 140.0, 18.0, 56.0, 5.0));
        set_k(0.8); assert!((win_w() - 480.0).abs() < 1e-3 && (mini_h() - 4.0).abs() < 1e-3); set_k(1.0); assert_eq!(open_h(), 140.0);
        set_k(9.0); assert_eq!(k(), 1.5); set_k(1.0);
    }
}
