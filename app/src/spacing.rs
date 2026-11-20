//! Espacamento central (um unico modulo): margens, folgas e tamanhos dos elementos de slot, grade e slider.
//! Todo layout novo usa estas constantes. Quando o padrao do Master mudar, muda so aqui.

/// margem minima do conteudo ate a borda do cartao (vertical e horizontal), em px
pub const EDGE: f32 = 12.0;
/// margem dos slots de 52 px (validada na captura da beta.5; 52 - 2 x 10 = faixa de 32 px)
pub const SLOT_PAD: f32 = 10.0;
/// espaco entre itens lado a lado (grade de Arquivos, colunas)
pub const GAP: f32 = 6.0;
/// barrinha vertical de Arquivos (slot 110 px): largura
pub const BAR_W: f32 = 4.0;
/// bolinha (thumb) do progresso e do volume: diametro (aparece so ao passar o mouse)
pub const THUMB: f32 = 11.0;

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn margens_e_folgas_coerentes() { assert!(EDGE >= 12.0); assert!(SLOT_PAD >= 10.0); assert!(THUMB >= 10.0 && THUMB <= 12.0 + 1.0); assert!(GAP < EDGE); }
}
