# Spike 2 — pill nativa + WebView sob demanda (branch `feat/spike-lazy-webview`)

Mesma máquina do protótipo: Windows 11 Pro 10.0.26300, WebView2 156, Tauri 2.12.5, windows-rs 0.62, build release. RAM = bytes **privados** (e working set entre parênteses) do `eavely.exe` + processos `msedgewebview2` com `app.eavely` na linha de comando (outros apps WebView2 da máquina ficam de fora).
**Ressalva de método:** o PC estava em uso durante os testes (o mouse real interferiu em algumas rodadas, e a máquina tinha carga). Tempos variam bastante; 1 monitor testado (o 2º existe, 1366x768, mas a pill só vive no primário).

## O que mudou
- `app/src/pill.rs`: pill recolhida = janela Win32 *layered* (`UpdateLayeredWindow`, alpha por pixel, cantos suavizados por cobertura), `NOACTIVATE|TOOLWINDOW|TOPMOST|TRANSPARENT` (click-through). Sem WebView, sem GDI+ (um DIB 32 bits pintado à mão, ~40 linhas). Só a forma preta; **título da música não foi desenhado**.
- `main.rs`: `tauri.conf.json` sem janelas. Hover (polling de `GetCursorPos`, igual ao protótipo) cria a janela Tauri com a UI atual; ao sair do hover emite `close`, espera a animação (320 ms) e chama `destroy()`.
- Modos de teste por env `EAVELY_MODE`: *(vazio)* = **lazy** (criar/destruir), `suspend` = WebView oculto + `TrySuspend`/`Resume`, `prewarm` = cria o WebView ao chegar perto do topo e suspende. `EAVELY_LOG=1` grava `%TEMP%\eavely-spike.log`.
- Front: título/artista entram por `initialization_script` (sem esperar evento); handshake `ready` evita perder o evento `open`.

## Antes / depois
| Métrica | Protótipo F1 (WebView fixo) | **Lazy** (padrão) | Suspend (oculto) |
|---|---|---|---|
| RAM recolhido, privada | ~130 MB (6 procs webview; WS ~327 MB) | **3,3 MB** (WS ~19 MB), 0 procs webview | ~122–178 MB (6 procs); WS cai p/ ~14–100 MB após trim |
| RAM aberto, privada | ~130 MB | ~162–169 MB (6 procs; WS ~335–342 MB) | ~170 MB |
| RAM após fechar | — | ~5,5–6 MB (procs msedgewebview2 somem) | continua ~170 MB privada |
| Hover → animação começa | 0 (já carregado) | **~250–650 ms** (6 medições limpas: 403–454; 1ª fria 776; rodadas posteriores 247–651) | **25–38 ms** (5 medições) |
| Foreground ao abrir | não muda | não muda (testado) | — |

A meta "bem abaixo de 50 MB recolhido" foi **atingida no modo lazy (3,3 MB)**. A meta "<150 ms ao abrir" **não** foi atingida no lazy: subir o WebView2 a frio custa 250–650 ms por hover (criar o ambiente + processos + carregar a página).

## Item 4 — suspender em vez de destruir
`TrySuspend` dá abertura em ~30 ms, mas **não devolve memória privada**: continua ~120–170 MB (só o working set é aparado, e varia: 52 MB aos 30 s, 14–100 MB depois). O número que o Gerenciador de Tarefas mostra cai, o commit não. Não resolve o problema de RAM; serve só se a meta for "latência" e não "RAM". **Veredito do item: não compensa.**

## Pré-aquecimento (extra, instável)
Criar o WebView quando o cursor entra numa zona de ~250 px abaixo/±150 px ao lado da pill e suspendê-lo até o hover. Medi 32/64/121/74/86 ms em 5 de 6 rodadas, e **1 rodada não abriu** (sem log). Não consegui separar bug de interferência do mouse real. **Não confiável; não tirar conclusão.** Custo conceitual: o WebView (130+ MB) sobe toda vez que o mouse passa perto do topo-centro (comum: abas do navegador).

## Item 5 — tela cheia e DPI
- **Tela cheia (testado):** janela sem borda cobrindo o monitor primário (Windows Forms). Pill passa a invisível (pixel (960,8) vira azul do fundo em vez de preto), cursor no topo **não cria WebView** (0 processos), e a pill volta ao sair. Detecção: `SHQueryUserNotificationState`, ~1 s. Rodada 2: testado com vídeo real do YouTube (F11 e tela cheia do player). Continua sem teste: jogo D3D exclusivo.
- **DPI:** testado na rodada 2 (ver abaixo).
- **Não testado:** Windows 10, apps elevados (UIPI), flash visual na troca pill→WebView (só vi o screenshot da pill parada, cantos suaves; não gravei vídeo da transição).

## Item 6 — remendo de 1 s
**Removido.** Com `focusable(false)` + `skip_taskbar(true)` + `always_on_top(true)` no builder, `NOACTIVATE` e `TOPMOST` já vêm certos. O tao implementa `skip_taskbar` pela API da barra de tarefas (não por `TOOLWINDOW`) e deixa `APPWINDOW` ligado, então faço **um** `SetWindowLongPtr` (+`TOOLWINDOW`, −`APPWINDOW`) imediatamente antes do `ShowWindow(SW_SHOWNOACTIVATE)`. Como mostramos a janela por `ShowWindow` direto, o tao não reescreve depois. Verificado 0,7 s após abrir, 5 aberturas: `ex=0x8000198`, NOACTIVATE/TOOLWINDOW/TOPMOST ligados, APPWINDOW desligado, janela não vira foreground. **Não verificado:** estilos após minutos abertos (a janela só vive enquanto há hover).

## Rodada 2 — ilha nativa animada, texto, DPI, tela cheia real
(Os modos `suspend`/`prewarm` foram removidos do código; ficam no histórico do git. O que segue é o único caminho: lazy + ilha nativa.)

### Item 1 — expansão nativa enquanto o WebView sobe
- `pill.rs` agora é uma janela layered de 420x110 lógicos. No hover ela anima largura/altura/raio (140x28 r18 → 400x84 r28) em 280 ms (easeOutQuart), desenhada em software num DIB 32 bits + `UpdateLayeredWindow` a cada ~8 ms.
- O WebView é criado escondido no mesmo instante. Quando o front avisa `ready`, a janela web é mostrada **por cima** da ilha nativa já aberta; a nativa nunca é escondida, então um quadro transparente do WebView apenas revela a ilha igual por baixo. Ao sair do hover o WebView é destruído e a nativa anima o fechamento. Front nasce no estado aberto, sem transição (`.instant`).
- **Medido** (100%, captura da região da ilha a ~14 ms por amostra, 1,7 s por rodada): primeiro pixel da ilha aberta em **~80–100 ms**, largura ≥ 390 px em **~190 ms**, 400 px em **~255 ms**, texto visível em **~110–135 ms**. Antes (WebView puro): 250–650 ms só para *começar*. WebView web visível 570–850 ms após o hover (não importa mais: já está tudo aberto por baixo).
- **Flash:** 0 piscadas no ponto interior em todas as rodadas, incluindo o instante do handoff; o texto esteve presente em todas as amostras. Screenshots da ilha nativa (~420 ms) e da web (~1,5 s) ficaram visualmente idênticos. **Limite:** amostragem a ~14 ms pode perder uma piscada de 1 quadro (16 ms); não gravei vídeo.

### Item 2 — título/artista na pill nativa
GDI `DrawTextW` Segoe UI (título 15 px bold, artista 13 px) com reticências, fade pelo nível de cinza (fundo preto). Como o GDI não escreve alpha, a cobertura da forma é guardada à parte e reimposta depois do texto. Atualiza com o SMTC (testado: "Way down We Go / KALEO"). Nota: o texto é GDI simples (sem ClearType/DirectWrite), mas ficou igual ao do WebView a olho.

### Item 3 — DPI e 2º monitor
Mudei a escala do monitor primário ao vivo (`DisplayConfigSetDeviceInfo`) e **restaurei 100% nos dois monitores no fim**.
| Cenário | Largura da ilha aberta (esperado) | Início | ≥ 390·s px | Piscadas |
|---|---|---|---|---|
| Primário 100% | 400 px | ~100 ms | ~190 ms | 0 |
| Primário 125% | 500 px → **500 medido** | 82 ms | 186 ms | 0 |
| Primário 150% | 600 px → **600 medido** | 79 ms | 177 ms | 0 |
| Primário 150% + 2º monitor 125% (DPI misto) | 600 px → medido 630* | 27 ms | 126 ms | 0 |
| Primário 100% + 2º monitor 125% (DPI misto) | 400 px → **400 medido** | 103 ms | 188 ms | 0 |
\* 630 = 420·1,5, a janela inteira: provavelmente fundo escuro ao lado da ilha somado ao preto (o conteúdo da tela mudou ao trocar a escala); não investiguei mais.
- O app seguiu a mudança de escala **ao vivo** (log "geometria mudou ... escala 1.5" em ≤ 1 s, sem reiniciar). Posição, tamanho, texto e WebView escalaram juntos.
- 2º monitor (1366x768): nesta rodada a ilha vivia só no primário; a rodada 3 (abaixo) a faz seguir o cursor. Coloquei o 2º monitor em 125% para testar DPI misto e a ilha do primário não foi afetada. Win10 não testado.
- Antes de chegar aqui um teste com escala "falsa" (script sem aplicar a mudança) deu números inválidos; foram descartados e refeitos com o DPI realmente aplicado (confirmado por `GetDpiForMonitor` = 120 e 144).

### Item 4 — tela cheia real
Edge em `--start-fullscreen` com um vídeo do YouTube tocando (perfil temporário, mudo, apagado depois):
- F11 (página): `SHQueryUserNotificationState` = 2 (BUSY) → pill oculta; com o cursor no topo **nenhum WebView é criado** (0 processos).
- Tecla `f` (vídeo do player em tela cheia): idem; screenshot de tela inteira sem a pill.
- Ao fechar o Edge: estado volta a 5 e a pill reaparece (pixel preto no topo). Houve um flap `ocupado→livre→ocupado` durante a carga da página (o Edge sai e volta da tela cheia sozinho); a pill acompanhou. **Não testado:** jogo D3D de verdade em tela cheia exclusiva.

### Mouse real
O PC estava em uso. Em todas as rodadas de timeline o mouse real me tirou do lugar (8 a 75 vezes por rodada de 1,7 s); o medidor devolve o cursor ao topo a cada amostra, então as medições valem, mas há ruído e janelas do Windows mudaram de lugar na troca de escala.

## Rodada 3 — ilha segue o cursor entre monitores
Código: `main.rs` (`monitors`, `mon_at`, `geo_of`, `fullscreen_flags`). Máquina: DISPLAY1 1920x1080 em (0,0), DISPLAY6 1366x768 em (1920,0); escalas de teste 100/125/150% por monitor.

**Como funciona**
- Lista de monitores via `available_monitors()` (posição/tamanho físicos + escala de cada um), relida a cada ~1 s. O monitor ativo é o que contém o cursor.
- **Pill fechada:** ao cursor entrar em outro monitor, a janela layered é movida/redimensionada com **uma única** `UpdateLayeredWindow` (nova posição + novo DIB na escala do monitor de destino): nunca fica sem janela nem com duas. **Pill aberta:** fica no monitor dela; sair da janela fecha (a nativa anima o fechamento, ~280 ms) e só depois de 330 ms a ilha pode trocar de monitor (evita ver a animação de fechar "pulando" de monitor).
- **WebView:** posição e tamanho em pixels físicos direto por `SetWindowPos` no momento de mostrar (e ao mudar a geometria), para o tao não reescalar a janela por `WM_DPICHANGED` ao cruzar monitores de escalas diferentes.
- **Tela cheia por monitor:** `SHQueryUserNotificationState` é global (e nem acusa tela cheia num monitor secundário sem foco: leu 5 com Edge em F11 no 2º monitor). Troquei por varredura `EnumWindows` a cada ~1 s: janela visível, não minimizada, não *cloaked*, **sem barra de título**, sem `WS_EX_TRANSPARENT`, fora `Progman`/`WorkerW`/a própria pill, cuja caixa cobre o monitor inteiro. A ilha só some se o **monitor ativo** tiver tela cheia.

**Medido** (captura de tela; fundo branco atrás da ilha para o preto ser inequívoco; scripts PMv2, ver "Mouse real")
| Cenário | Resultado |
|---|---|
| Ambos 100%: cursor no topo do 2º monitor | ilha abre lá: 1º pixel 78 ms, ≥390 px em 186 ms, fica em 400 px depois do handoff do WebView (WebView visível ~460 ms) |
| Primário 100% + 2º 125% | ilha no 2º = 500 px (esperado 500), ≥487 px em 181 ms |
| Primário 150% + 2º 125% | primário 600 px, 2º 500 px; início 92/111 ms |
| Travessia cursor mon.1→mon.2 com a pill fechada (100→100, 100→125, 150→125; 3+3+2 travessias, ~25 ms por quadro) | **0 quadros sem pill** em nenhum monitor; 1 quadro "pill nos dois" em algumas, que é artefato da captura sequencial (duas leituras separadas por alguns ms), não duas janelas |
| Tela cheia só no mon.2 (form azul) | pill visível no mon.1; some no mon.2 (inclusive com cursor no topo: nenhum WebView criado); volta ao mon.1 |
| Tela cheia só no mon.1 | pill aparece no mon.2 (preto sobre fundo branco); no mon.1 some |
| Edge F11 no mon.2 (perfil temporário) | `fullscreen=[false,true]`, ilha oculta com o cursor no mon.2; visível no mon.1 |

**Bug achado no caminho (e corrigido):** `DrawTextW` com slice vazio (artista vazio quando nada toca) dava *access violation* em `user32` e **derrubava o app ao abrir a ilha**; já estava no commit anterior e só não apareceu porque o Spotify estava tocando. Corrigido (`08a6231`). Também adicionei um *vectored exception handler* que grava endereço/módulo no log em caso de AV.

**DPI do teste:** a escala dos dois monitores foi alterada ao vivo (`DisplayConfigSetDeviceInfo`) e **restaurada para 100% em ambos, confirmada** por `DisplayConfigGetDeviceInfo` (rel=0) e por `GetDpiForMonitor`=96.

**Mouse real:** nesta rodada o mouse do usuário não interferiu (0 reposicionamentos nas medições de timeline). Cuidado de método: scripts em `SetProcessDPIAware` (por sistema) *virtualizam* coordenadas do 2º monitor quando as escalas diferem, o que invalidou uma primeira rodada de DPI misto; refiz tudo com `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`.

**Não testado / limitações**
- Heurística de tela cheia: um **overlay opaco sem barra de título que cubra o monitor** (alguns overlays de GPU/launchers) vira falso positivo; o oposto, jogo D3D exclusivo, não foi testado. Atraso de até ~1 s para reagir a entrar/sair de tela cheia. Windows 10, apps elevados (UIPI) e mais de 2 monitores também não.
- O Edge do teste de tela cheia mostrou um aviso de sincronização/captcha em cima da página (o vídeo não ficou visível); vale como "janela Edge F11 sem título cobrindo o monitor", não como vídeo em player fullscreen (esse foi testado na rodada 2, no primário). O perfil temporário foi apagado.
- Monitores empilhados na vertical/negativos e a ilha quando o cursor está num "vão" entre monitores (a ilha fica onde estava) não foram testados.
- Sem vídeo da travessia: ~25 ms por quadro pode perder uma piscada de 1 quadro.

## Veredito (atualizado)
- **Tauri + ilha nativa + WebView lazy serve.** RAM recolhido 3,3 MB; a ilha abre "visualmente" em ~100–250 ms (nativa) e o WebView carrega por trás sem flash medido. A latência que reprovava o modo lazy puro (250–650 ms) fica escondida.
- **Funciona em 100/125/150%, DPI misto (primário), tela cheia real (vídeo) e acompanha mudança de escala ao vivo.**
- **Custo novo:** a ilha nativa (forma, animação, texto) é código nosso em Win32/GDI e tem de espelhar o visual da UI web (hoje só o estado aberto: texto e forma). Cada novo estado da ilha (timer, shelf, notificação...) precisa de um desenho nativo *ou* aceitar esperar ~0,6 s pelo WebView. Estratégia sugerida: a nativa só cobre a abertura/estado "música"; qualquer conteúdo extra aparece quando o WebView chega.
- **Migrar para WPF/nativo continua desnecessário.** Reavaliar só se a UI nativa precisar crescer além de forma + texto simples.
- **Pendente:** jogo D3D exclusivo, Win10, UIPI, ClearType/DirectWrite, e um teste em vídeo de flash. (Mover a ilha entre monitores: feito na rodada 3.)
