//! Espacamento central (um unico modulo), padrao do Master (design/REGRAS-UI.md §12). Grade de 4 px; tokens 4/8/12/16/24.
//! Todo layout novo usa estas constantes. Quando o padrao mudar, muda so aqui.

/// margem do conteudo ate a borda do cartao, slot ou janela
pub const EDGE: f32 = 16.0;
/// minimo absoluto do texto a borda (nunca menos)
pub const MIN_EDGE: f32 = 12.0;
/// entre cartoes e slots
pub const CARD_GAP: f32 = 12.0;
/// icone -> texto; botao -> botao (entre botoes vizinhos)
pub const ITEM_GAP: f32 = 8.0;
/// titulo -> subtitulo
pub const TITLE_GAP: f32 = 4.0;
/// entre linhas de uma lista
pub const LINE_GAP: f32 = 8.0;
/// entre grupos
pub const GROUP_GAP: f32 = 16.0;
/// alvo clicavel minimo
pub const TARGET: f32 = 32.0;
/// linha de configuracao: altura e margem lateral
pub const CFG_ROW: f32 = 44.0; pub const CFG_SIDE: f32 = 16.0;
/// margem do conteudo dentro do slot (>= MIN_EDGE)
pub const SLOT_PAD: f32 = 12.0;
/// barrinha vertical de Arquivos (slot grande): largura
pub const BAR_W: f32 = 4.0;
/// bolinha (thumb) do progresso e do volume: diametro
pub const THUMB: f32 = 11.0;

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn tokens_no_padrao() { assert_eq!(EDGE, 16.0); assert!(MIN_EDGE >= 12.0 && SLOT_PAD >= MIN_EDGE); assert_eq!(CARD_GAP, 12.0); assert_eq!(TARGET, 32.0); assert_eq!(CFG_ROW, 44.0); }
    #[test] fn tokens_na_grade_de_4() { for v in [EDGE, MIN_EDGE, CARD_GAP, ITEM_GAP, TITLE_GAP, LINE_GAP, GROUP_GAP, SLOT_PAD] { assert_eq!(v % 4.0, 0.0, "{v} fora da grade de 4 px"); } }
}
