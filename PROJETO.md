# Eavely — visão e roteiro

## Objetivo
Um notch para o Windows: uma ilha no topo da tela que mostra música, timer, arquivos e tarefas, e some quando não é necessária. 100% nativo (Rust, Direct2D/DirectComposition), leve e sem uso de CPU quando parado. Código aberto (MIT).

## Estado atual (v0.1.10)
- Núcleo nativo concluído: janela da ilha, estados (mini notch, compacto, aberta, ao vivo), animações por vblank.
- Páginas: Música (com letras sincronizadas), Pomodoro/timer, File Shelf (arrastar e soltar), Tarefas.
- Configurações em janela própria: ilha e aparência, desempenho (presets, limite de quadros, efeitos), atalhos, monitor.
- Parado: 0% de CPU e cerca de 6 MB de memória privada.
- Distribuição: `Eavely.exe` (portátil) e `Eavely-Setup.exe` (instalador por usuário). **Binários ainda sem assinatura**: o Windows pode avisar (SmartScreen / Smart App Control).

## Roteiro
1. **Núcleo nativo** ✔
2. **Polimento e desempenho** ✔
3. **Distribuição e assinatura**: assinatura de código (SignPath Foundation), MSIX/Microsoft Store, atualizações.
4. **Mascote interativo**
5. **Clipboard, monitor do sistema e ações rápidas**
6. **Internacionalização, Windows 10 e DPI misto**

## Como contribuir e testar
- Requisitos: Windows 10/11, Rust estável (MSVC).
- Testes: `cd app && cargo test --release` (testes unitários) e `eavely.exe --selftest-anim` (autoteste de animação do app).
- Build de release sem vazar caminhos locais: `scripts/build-release.ps1`.
- Issues e pull requests são bem-vindos. Mudanças visuais devem vir com captura de tela; evite incluir dados pessoais em capturas e logs.
- Diagnóstico: `%APPDATA%\Eavely\state.log` registra início, saída e motivo de cada encerramento; não contém títulos, artistas nem arquivos.
