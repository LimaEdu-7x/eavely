# Protótipo F1 — Tauri 2 (branch `feat/prototype`)

Medido em Windows 11 Pro 10.0.26300, monitor único 1920x1080 (100%), WebView2 156, rustc 1.99.1, Tauri 2.12.5, windows-rs 0.62. Build release (`opt-level=s`, LTO, strip).

## O que funcionou (testado rodando)
- **Janela**: sem borda, transparente, topmost, topo central, fora da barra de tarefas. Estilos `NOACTIVATE|TOOLWINDOW|TOPMOST` confirmados via `GetWindowLongPtr`. Abrir a pill **não muda a janela em foreground**.
- **Pill → aberta**: polling de `GetCursorPos` (16 ms perto do topo, 25 ms longe) e animação CSS (largura/altura/raio, ~280 ms). Recolhida usa `set_ignore_cursor_events(true)` (click-through); aberta captura o mouse.
- **SMTC** (`windows-rs`, poll 1 s): título/artista aparecem na pill aberta (testado com Spotify: "Way down We Go / KALEO").
- **Instalador NSIS**: 1,05 MiB. Exe: 3,1 MB.

## O que NÃO funcionou de primeira / pegadinhas
- **O tao reescreve o estilo da janela depois do `setup`** (some NOACTIVATE/TOOLWINDOW, volta APPWINDOW) e a janela ficou invisível. Solução: reaplicar os estilos e `SW_SHOWNOACTIVATE` a cada ~1 s na thread de polling. Funciona, mas é um remendo; ideal é achar o hook certo.
- `cargo build --release` direto gera app que carrega o `devUrl` (página de erro cinza). Sempre buildar com `npx tauri build`.
- `HWND` não é `Send`: passar como `isize` para a thread.

## Não testado
- **Esconder em tela cheia**: lógica implementada (`SHQueryUserNotificationState`, ~1 s), sem teste com app em tela cheia real (estado lido = 5, "aceita notificações").
- **DPI per-monitor / multi-monitor**: só 1 monitor a 100%. Posicionamento usa `scale_factor` do primário e reposiciona a cada ~1 s; nunca rodou com DPI misto. Notch fica só no monitor primário.
- Drag&drop, apps elevados (UIPI), Windows 10, Mica/acrylic, capa do álbum.

## Números
| Métrica | Valor |
|---|---|
| Instalador NSIS | 1,05 MiB |
| eavely.exe | 3,1 MB |
| RAM idle, processo `eavely` | ~31 MB working set / ~6 MB privado |
| RAM idle, árvore completa (7 processos: eavely + 6 msedgewebview2) | **~140–170 MB privado** (2 medições); working set somado 335–360 MB (conta páginas compartilhadas em duplicidade) |
| CPU idle (processo principal) | ~0,03 s por 10 s (~0,3%) |

A estimativa da pesquisa (30–80 MB) ficou **otimista**: o WebView2 sozinho custa ~110–130 MB privado, e é isso que domina. Apps de ilha nativos anunciam ~3 MB.

## Veredito: Tauri vs WPF
- **Tauri se mantém** para a F2: Win32 e SMTC via windows-rs foram diretos, animação CSS é trivial, instalador de ~1 MB. O custo é a RAM do WebView2 (~140+ MB) e o remendo de estilo da janela.
- **Gatilho para o plano B (WPF)**: se RAM idle for requisito de marketing (concorrer com os "3 MB" de apps nativos), WPF/.NET AOT ou nativo ganha claramente; Tauri não chega lá. Pode-se mitigar: destruir o webview quando recolhida (pill pura em Win32/DirectComposition) e só criar ao abrir — a medir.
- Recomendação: seguir com Tauri, mas **decidir com o autor** se ~150 MB idle é aceitável. Medir antes da F2: webview sob demanda e DPI misto.

## Instalado nesta máquina
Rust (rustup + toolchain stable 1.99.1 MSVC, via winget `Rustlang.Rustup`); NSIS 3.13 e nsis_tauri_utils baixados pelo `tauri build` (cache do Tauri). Já existiam: Node 24, VS Build Tools (MSVC 14.51), WebView2 156. `npm i` local: Tauri CLI/API, Vite, TypeScript.
