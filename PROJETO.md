# Eavely — visão e roteiro

## Objetivo
Um notch para o Windows: uma ilha no topo da tela que mostra música, timer, arquivos e tarefas, e some quando não é necessária. 100% nativo (Rust, Direct2D/DirectComposition), leve e sem uso de CPU quando parado. Código aberto (MIT).

## Estado atual (v0.2.0, candidata a release)
- Núcleo nativo concluído: janela da ilha, estados (mini notch, compacto, aberta, ao vivo), animações por vblank.
- Abas: **Mascote**, Música (com letras sincronizadas), Arquivos (File Shelf), Pomodoro e Tarefas.
- **Mascote** (v0.2.0): rosto em curvas Bézier com morph entre expressões, piscar, olhar que segue o mouse, humores por contexto (música, arrasto de arquivo, cliques, noite, bateria baixa, inatividade) e personalidade *Calmo* (também *Travesso* e *Dramático*). Dois slots configuráveis ao lado (Arquivos, Player mini, Tarefas, Timer, Relógio, Métricas do PC). Também no compacto (só olhos no tamanho Pequeno).
- **Tamanho do notch** Pequeno 80 % / Padrão 100 % / Grande 120 %, aplicado na hora, com pisos de tipografia (títulos 11 px, secundário 10 px, ícones 12 px).
- Configurações em janela própria: ilha e aparência, mascote e início, desempenho (presets, limite de quadros, efeitos), atalhos, monitor.
- Uso de CPU parado: medido na v0.1.9 (ver `docs/perf/RESULTADOS.md`); a v0.2.0 foi medida só em VM (renderização por software), então não há número novo de PC real.
- Distribuição: `Eavely.exe` (portátil) e `Eavely-Setup.exe` (instalador por usuário). **Binários ainda sem assinatura**: o Windows pode avisar (SmartScreen / Smart App Control).

## Roteiro
1. **Núcleo nativo** ✔
2. **Polimento e desempenho** ✔
3. **Distribuição e assinatura**: assinatura de código em avaliação (MSIX/Microsoft Store e outros caminhos), atualizações.
4. **Mascote interativo** ✔ (v0.2.0)
5. **Clipboard, monitor do sistema e ações rápidas**
6. **Internacionalização, Windows 10 e DPI misto**

### Fica para a v0.2.1
- Tamanho **Avançado** (fator livre) e **prévia** do notch na página de configuração.
- Humores menores sem gatilho definido (animado, orgulhoso, tímido, apaixonado etc.), lágrima/zzz em hardware real e ajustes finos do mascote no compacto.
- Troca de conteúdo por texto que não cabe nos tamanhos pequenos além do tile de Arquivos.

## Como contribuir e testar
- Requisitos: Windows 10/11, Rust estável (MSVC).
- Testes: `cd app && cargo test --release` (testes unitários) e `eavely.exe --selftest-anim` (autoteste de animação do app).
- Build de release sem vazar caminhos locais: `scripts/build-release.ps1`.
- Issues e pull requests são bem-vindos. Mudanças visuais devem vir com captura de tela; evite incluir dados pessoais em capturas e logs.
- Diagnóstico: `%APPDATA%\Eavely\state.log` registra início, saída e motivo de cada encerramento; não contém títulos, artistas nem arquivos.
