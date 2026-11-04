# Eavely — Design nativo (Direct2D / DirectWrite)

Especificação para a implementação. Tudo em **px a 100% DPI** (96 dpi); multiplique por `dpi/96` na hora de desenhar. Origem (0,0) = canto superior esquerdo **da ilha**; a ilha é centrada no topo do monitor, colada na borda (y=0), topo reto, só os cantos de baixo são arredondados. Fontes: protótipo web (aprovado pelo autor), `COMPORTAMENTO.md` e as **medidas internas de comportamento/layout** (usadas só para medidas, ordem e fluxo; **nenhuma arte, ícone, texto ou logo copiado**, identidade visual própria). Escopo do autor: **partir de medidas internas e depois modificamos**. Legenda: **PROPOSTA** = sem referência, inventado por nós; **[AJUSTAR]** = chute a confirmar. Idioma dos textos: pt-BR.

## 1. Tokens

### Cores (sRGB, alpha não pré-multiplicado)
| Token | Valor | Uso |
|---|---|---|
| `bg` | `#000000` | corpo da ilha (igual ao protótipo e à pill atual) |
| `surface` | `#FFFFFF` @ 8% | botões, chips, trilho do progresso |
| `surface-hover` | `#FFFFFF` @ 14% | hover |
| `surface-press` | `#FFFFFF` @ 20% | pressionado |
| `line` | `#FFFFFF` @ 14% | contornos, tracejado do shelf vazio |
| `text` | `#FFFFFF` | título, linha de letra atual, números |
| `text-2` | `#FFFFFF` @ 64% | artista, rótulos (≈8:1 sobre preto) |
| `text-3` | `#FFFFFF` @ 52% | tempos, ícones inativos (≈5,2:1; **não baixar de 4,5:1**) |
| `accent` | `#7CC4FF` | anel do foco, item atual, anel de arraste |
| `accent-2` | `#FFB86B` | pausa/pausa longa |
| `ok` | `#7BE0A4` | timer concluído |
| `danger` | `#FF6B6B` | "Limpar tudo", bateria ≤ 10%, erro |
| **Cores de página** | Música `#B69CFF` (violeta) · Pomodoro `#FF7A66` (coral) · Shelf `#FFC857` (âmbar) · Tarefas `#5EE0B0` (menta) | título, ícone ativo do dock, anel/botão principal da página (tons próprios) |
| `btn-primary-bg` / `fg` | `#FFFFFF` / `#000000` | botão play (hover `#E6E6E6`, press `#CFCFCF`) |
| `trace` | ver §3.1 | traço no topo |

Foco de teclado (quando houver): anel 2 px `accent` com 2 px de folga preta (`0 0 0 2 #000, 0 0 0 4 accent`).

### Tipografia — Segoe UI Variable (fallback Segoe UI no Win10)
DirectWrite, família `"Segoe UI Variable Text"`; se ausente, `"Segoe UI"`. Antialias **GRAYSCALE** (a superfície tem alpha). Números do relógio/tempos: feature `tnum` (algarismos tabulares).
| Estilo | Tamanho/linha | Peso | Onde |
|---|---|---|---|
| `title` | 14 / 20 | 600 | título da faixa |
| `body` | 12 / 16 | 400 | artista, linha de letra, textos de botão |
| `body-strong` | 12 / 16 | 600 | botões "Iniciar", segmento ativo |
| `small` | 10.5 / 14 | 400 | tempos 0:00, "Ciclo N", legendas |
| `clock` | 16 / 20 | 600 | tempo dentro do anel |
| `clock-live` | 14 / 18 | 600 | timer no estado ao vivo |
Reticências no fim (`DWRITE_TRIMMING_GRANULARITY_CHARACTER` + ellipsis), sem quebra de linha.

### Raios
`island-open` 28 · `island-live`/overlay 22 · `island-peek` 14 (150×30) · flares côncavos do topo 6 · `trace` 2 · `cover` 14 (open) / 8 (live) · `button` 999 (círculo) · `chip` 10 · `segmented` 999 · `shelf-empty` 16.

### Espaçamento
Grade de 4 px. Margem interna da ilha aberta: 12 (topo/baixo), 16 (direita). Gap entre elementos: 8 ou 12.

### Movimento
Animação **dirigida por tempo** (não por frame), sincronizada ao refresh do monitor (60/120/144 Hz): use `DCompositionWaitForCompositorClock` ou `DwmFlush`/`IDXGIOutput::WaitForVBlank` do monitor atual; calcule `t = (agora − início)/duração` a cada vblank.
| Nome | Curva | Duração |
|---|---|---|
| `ease-out` (principal) | cubic-bezier(.20, .80, .20, 1) | — |
| `spring` (expansão da ilha, opcional) | mola amortecida: rigidez 420, amortecimento 34, massa 1 | ~280 ms |
| `island-resize` | ease-out | 220–240 ms (§5) (largura, altura, raio juntos) |
| `content-in` | ease-out | 160 ms, atraso de 110 ms (só após a forma abrir ~40%) |
| `content-out` | linear | 90 ms |
| `tab-switch` | ease-out | 220 ms: conteúdo antigo some 90 ms; novo entra com translateY(+12→0) e opacidade 0→1 |
| `lyric-change` | ease-out | 220 ms: linha antiga sobe 6 px e some; nova sobe de +6 |
| `hover/press` | ease-out | 120 ms; press = escala 0,94 |
| `ring` (timer) | linear | 1 s por tick (suave entre ticks) |
| `progress` | — | **sem animação nem atraso**: acompanha o mouse imediatamente (§5 seek otimista) |
`prefers-reduced-motion` (`SystemParametersInfo(SPI_GETCLIENTAREAANIMATION)` = FALSE): todas as durações = 0 (troca instantânea), anel do timer sem transição.

### Ícones (assets/icons/, Lucide, ISC/MIT)
Grade 24×24, traço 2, pontas e junções arredondadas, `currentColor`. Renderização recomendada: `ID2D1DeviceContext5::CreateSvgDocument` (Win10 1703+) trocando `currentColor` pela cor do token; ou converter os `<path>` para `ID2D1PathGeometry` na inicialização. **Play, pause, skip-forward, skip-back:** desenhar com `fill=currentColor` **além** do traço (ficam sólidos). Demais: só traço.
| Tamanho | Onde |
|---|---|
| 18 px | play/pause (botão grande) |
| 16 px | prev, next, dock de páginas, reset, configurações, ocultar, arquivo, pasta (chip) |
| 14 px | X do chip |
| 24 px | estado vazio do shelf |
Mapa (dock/páginas): Música=`music`, Pomodoro=`timer`, Shelf=`inbox`, Tarefas=`list-checks`, Configurações=`settings`. Música: `skip-back`/`play`/`pause`/`skip-forward`, letra=`message-square-quote`, volume=`volume-1/2/x`. Pomodoro: `rotate-ccw`, pausa=`coffee`, foco=`target`. Tarefas: `circle-plus` (adicionar), `pencil` (editar), `calendar`, `circle`/`circle-check` (item). Shelf: `file`, `folder`, `x`. Cabeçalho: `battery`, `battery-charging`, `chevron-left/right` (setas). Overlays: `volume-x/1/2`, `sun`, `battery-low`, `wifi`, `wifi-off`, `headphones`, `arrow-big-up`. Ocultar durante evento=`eye-off`/`eye`; bandeja: `power`, `log-out`; aviso=`bell`. Reserva: `chevron-up/down`, `plus`, `check`, `shuffle`, `repeat`, `trash-2`, `grip-vertical`. Ícone do app/bandeja: ver §13 (feito).

## 2. Estados e medidas da ilha (w × h, 100% DPI)
Base: medidas internas de **comportamento/layout** (só medidas e ordem, **nenhuma arte, ícone ou texto copiado**) + regras do `COMPORTAMENTO.md` (estados traço/peek/ao vivo/tela cheia/maximizada **mantidos**). Centro horizontal = centro do monitor ativo; ilha colada em y=0; topo reto, só os cantos de baixo arredondados (a referência também tem "flares" côncavos de ~6 px nos cantos de cima que fundem a ilha à borda da tela — **PROPOSTA:** desenhar os flares, raio 6, para a ilha parecer parte da tela).

| # | Estado | Tamanho | Raios de baixo | Conteúdo |
|---|---|---|---|---|
| 1 | **Traço** (ocioso) | 56 × 3 | 2 | ver §3 |
| 2 | **Peek** (mouse a ≤ 40 px do topo) | **150 × 30** | 14 | ícone de bateria 16 px à esquerda (cor por nível) + hora `HH:MM` `body-strong` 14 à direita (medido internamente). Alternativa "olhos": §8 |
| 3 | **Aberto** (mouse sobre a ilha) | **420 × 136** | 28 | página atual (§4), moldura (§3). **Mesma altura em todas as páginas** |
| 3a | **Aberto + dock** (cursor na base) | **420 × 164** | 28 | a ilha **cresce 28 px em 200 ms** e o dock ocupa os 28 px novos (§3.2) |
| 3b | **Aberto — Música com letras (tela dividida)** | **520 × 136** (164 com dock) | 28 | só na página Música, ~320 ms ease-out, ver §4.1.1 |
| 4 | **Ao vivo — compacto** (medido internamente, vídeo 2) | **168 × 30** | 14 | peek + um indicador à direita da hora: **música** = equalizador animado (original, §4.7) · **timer rodando** = ícone `timer` na cor do timer. Padrão (paridade) |
| 4b | **Ao vivo — expandido** (COMPORTAMENTO.md) | 300 × 52 | 22 | capa + título + linha de letra / anel + tempo (§4.7). Opção em Configurações "Ao vivo: Compacto · Expandido" [AJUSTAR qual é o padrão] |
| 5 | **Overlay de sistema** | 280 × 48 | 22 | §7 |
| 6 | Tela cheia no monitor | — | — | nada desenhado |
| 7 | Janela maximizada no monitor | 56 × 3 | 2 | só o traço, mesmo ao vivo, até o mouse chegar (a referência ficou sobreposto; aqui vale o COMPORTAMENTO) |
Arrastar arquivo por cima do monitor: abre sozinho em **Shelf** (§4.3) vindo de qualquer estado (exceto tela cheia).

**Transições** (todas por `island-resize`, §1): traço→peek 200 ms · peek→aberto (mouse entra na ilha, intenção 80 ms) 240 ms · aberto→anterior: atraso de saída **≤ 100 ms** após o mouse sair da ilha + 8 px de margem (a referência fecha quase de imediato), animação 220 ms · ao vivo↔aberto 240 ms. Zona de gatilho: (centro ± 140 px) × (0…40 px); aberto/ao vivo: o retângulo da ilha + 8 px.
Janela nativa: criar no tamanho máximo (**≈ 560 × 176** com folga: 520 da tela dividida + 164 do dock) e animar a forma por DirectComposition; hit-test só nos pixels da ilha (não bloquear cliques abaixo). Mesmo com ilha em 420×138 a referência usa janela 740×200 transparente.

## 3. Traço e moldura da ilha aberta

### 3.1 Traço (estado 1)
56 × 3 px, `#000000`, borda inferior 1 px `#FFFFFF` @ 22%, cantos inferiores r=2, opacidade 85%. **Visível por padrão** (decisão do autor), inclusive sem janela maximizada; opção "Mostrar traço quando ocioso" (§12) desligada = some até o mouse chegar. Nunca em tela cheia.

### 3.2 Moldura da ilha aberta (420 × 136, 164 com dock) — coordenadas relativas à ilha
- **Cabeçalho** (páginas Pomodoro, Shelf, Tarefas; **não** na Música — a capa ocupa o topo): linha em y=12…32 (centro 22).
  - Título da página: (24, 12, 208, 20) `title` 14/20 600, **cor da página** (§1).
  - À direita, alinhado a x=376 (os últimos 20 px são o **indicador ao vivo**, abaixo): **hora** (330, 14, 46, 16) `body-strong` 12 tabular, `text` · **data** "seg 2 nov" (234, 14, 92, 16) `body` 12 `text-2` alinhada à direita (sem ano) · **bateria** (212, 14, 18, 16): ícone `battery`/`battery-charging`, verde `ok` carregando, `text-2` normal, `accent-2` ≤ 20%, `danger` ≤ 10%.
  - **Indicador ao vivo** (canto superior direito, **em todas as páginas**, inclusive Música): caixa (382, 12, 22, 16) alinhada à direita; música tocando = **equalizador** (§4.7), timer rodando = ícone `timer` 14 na cor do timer; os dois ativos = lado a lado (timer à esquerda, 6 px de intervalo). Clique no indicador: vai à página correspondente.
- **Setas laterais:** botões circulares **18 × 18** (`surface`, hover `surface-hover`), chevron 12 `text-2`; esquerda `(3, 66)`, direita `(399, 66)` (não encostam no conteúdo, que começa em x=24 e termina em x=396). Navegação **circular** (Tarefas → Música). Somem na tela dividida de letras.
- **Área de conteúdo:** x 24…396 (372), y 40…120 (80) nas páginas com cabeçalho; Música usa y 12…120. Nada fica abaixo de y=120.
- **Paginação (pontos):** 4 pontos Ø3 `text-3`, espaçados 10 px, centrados em x=210, **y=128**; ativo = pílula 14 × 3 `text` (anima largura 160 ms). Somem na tela dividida de letras e quando o dock está visível.
- **Dock de ícones:** aparece quando o cursor chega à **base da ilha (y ≥ 122)**; a ilha **cresce 28 px em 200 ms ease-out** (136 → 164; a referência cresce ~22 px — usamos 28 para o alvo de clique) e os pontos dão lugar à faixa do dock (0, 136, 420, **28**), mesma cor `bg` (sem gradiente: nada fica por baixo). O dock **permanece** enquanto o cursor estiver na ilha abaixo de y=112 (histerese); encolhe 28 px em 200 ms, com atraso de 150 ms, quando o cursor sobe acima disso. Itens: Música, Pomodoro, Shelf, Tarefas, **separador vertical** 1×12 `line`, Configurações. Cada item: ícone **16 px** centrado em y=150; **alvo de clique 36 × 28 px** (altura inteira da faixa); larguras 36 ×4 + 12 + 36 = 192, início x=114. Inativo `text-2`; **ativo na cor da página** + pílula de fundo (cor da página @ 16%, raio 9, 32 × 22, y 139…161). Hover `text`. Troca de página imediata ao clique.
Nenhum controle de página fica abaixo de y=120, então o dock nunca cobre nada clicável. Resumo da altura: **136 px aberta** e **164 px com dock** (antes: 138 → 150 com faixa embutida).

## 4. Páginas (aberto 420 × 136)
Ordem fixa: **1 Música · 2 Pomodoro · 3 Shelf · 4 Tarefas** (a referência tem 5 com Água; **Água removida por decisão do autor**). Os textos abaixo são em **pt-BR** (a referência medida é em inglês); [AJUSTAR] se for preciso inglês. Persistência no §9.

### 4.1 Música (cor da página: violeta `#B69CFF`)
- **Capa** (24, 12, 62, 62) raio 10, `cover`, contorno interno `line`. Sem capa: `surface` + `music` 24 `text-3`.
- **Título** (98, 12, 280, 20) `title` 15/20 `text`.
- **Artista — álbum** (98, 33, 280, 14) `body` 11 `text-2`, reticências.
- **Letra (UMA linha)** (98, 52, 280, 18) `body` 12/18 `text` (**decisão do autor**; é a 1ª forma de ver a letra, a 2ª é a tela dividida §4.1.1). Troca com `lyric-change`. Sem letra/sem sync: a linha some e o artista sobe 4 px. Sem clique.
- **Progresso** (linha y=78…92): tempo decorrido (24, 78, 44, 14) `body` 12 tabular `text` esq.; barra x 72…348, centro y=85, trilho h4 raio 2 `surface-hover`, preenchido `text`, **sem bolinha**, h6 em hover/arraste; tempo **restante** "-02:46" (352, 78, 44, 14) dir. Hit da barra (72, 76, 276, 18). Seek otimista (§10).
- **Controles** (centro y=104; o botão de letra à esquerda abre a tela dividida, §4.1.1): **prev** (168, 91, 26, 26) · **play/pause** (200, 88, 36, 32; círculo branco, ícone preto 18) · **next** (242, 91, 26, 26). À esquerda `message-square-quote` (24, 92, 24, 24): abre/fecha a **tela dividida de letras** (§4.1.1, estado persiste); à direita `volume-2` (372, 92, 24, 24): abre **pop-over de volume** (barra horizontal 96×6 em (292, 98) que substitui prev/play/next por 1,5 s — [AJUSTAR]; volume do app de mídia via sessão, se a API permitir; senão, volume do sistema).
- **Vazio:** capa-ícone; "Nada tocando" `text-2`; "Abra um player de música"; controles desabilitados (35%).
- **Gradiente:** §6.

#### 4.1.1 Tela dividida de letras (botão `message-square-quote`)
Segunda forma de ver a letra (a primeira é a **linha única**, §4.1). O botão de **balão de citação** (`message-square-quote` 16, caixa 24 × 24) fica à esquerda dos controles em (24, 92) e **abre/fecha** a tela dividida; **ativo = ícone e pílula de fundo brancos** (fundo `#FFFFFF` @ 16%, ícone `#FFFFFF`). Medido internamente (vídeo 2): a ilha alarga, some tudo que não é player/letra, painel direito só com letras. Layout e cores próprios. Estado persiste (`lyricsSplit`); vale só na página Música (ao sair, a ilha volta a 420; ao voltar, reabre em 520).
- **Tamanho:** **520 × 136** (mesma altura; 164 com dock), raio 28, centrada. Janela nativa ≈ **560 × 176**. Divisória vertical 1 px `line` em x=250 (y 16…120).
- **Some na tela dividida:** data, bateria, setas laterais, pontos de paginação e **volume**. O dock continua aparecendo na base (cresce como no §3.2, re-centrado em x=260).
- **Painel esquerdo (0…250, ~240 úteis):** capa (20, 14, 56, 56) raio 9 · título (88, 14, 148, 18) `title` 14 · artista (88, 34, 148, 14) `body` 11 `text-2` · progresso y=76…90: tempo (18, 76, 34, 14) 12, barra x 56…196 (centro y=83, h4), restante (200, 76, 40, 14) dir. · controles (centro y=104): balão **ativo** (20, 92, 24, 24), prev (72, 91, 26, 26), play (104, 88, 36, 32), next (146, 91, 26, 26). Sem linha de letra única (o painel direito a substitui).
- **Painel direito (251…520, ~230 úteis) — só letras:** viewport (268, 14, 236, 108), **5 linhas visíveis**, passo **22 px**, linha atual centrada em y=54 do viewport (68 da ilha). Atual: 14/18 **600 `text`** (branca semibold); vizinhas: 13/18 400 `text` **@ 40%**; 1 linha com reticências (a atual pode quebrar em 2; então passo 36 px). **Fade no topo e na base** (máscara de 28 px). Linha atual centraliza e o conteúdo **rola em 350 ms ease-out** (300–400) a cada troca; clique numa linha **não** pula; a roda do mouse rola manualmente por 3 s e volta a seguir a faixa. Não sincronizada: texto estático rolável, sem destaque.
- **Estados do painel direito:** "**Carregando letras…**" (`body` 12 `text-2`, centralizado, três pontos animados em 1,2 s) · "**Sem letra**" (`message-square-quote` 20 `text-3` + "Sem letra para esta faixa" 12 `text-2`) · sem faixa: "Toque uma música para ver a letra".
- **Fundo:** o gradiente da capa (§6) cobre as duas metades.
- **Animação (ease-out):** abrir **320 ms** (300–350): largura 420→520 com a ilha centrada; conteúdo antigo (data/setas/pontos/volume) some em 100 ms; o painel esquerdo desliza à posição compacta com crossfade de 160 ms; o painel direito dá fade-in de 160 ms a partir de t=120 ms com as linhas subindo 8 px; o balão fica branco em 120 ms. **Fechar ≈ 300 ms**: painel direito some em 100 ms, a largura encolhe em 260 ms, o layout normal reaparece. Reduced-motion: instantâneo.

### 4.2 Pomodoro (cor: coral `#FF7A66`)
- **Anel** (24, 44, 72, 72): círculo raio 32, traço 5, `coral`; trilho `surface-hover`; glow suave (sombra da mesma cor @ 35%, raio 10); **ícone `timer` 28 px** no centro. **Parado:** anel completo. **Rodando:** o anel vira um **arco de progresso** (esvazia no sentido horário a partir do topo) e **a contagem anda de segundo em segundo, sem animação** (nem no arco nem nos dígitos: o arco salta 1 passo por tick; não interpolar). O botão **play vira pause** (e volta a play ao pausar).
- **Tempo** `MM:SS` (112, 40, 150, 34) 30/34 700 tabular `text`; **legenda** "Foco" / "Pausa curta" / "Pausa longa" / "Personalizado 20:00" (+ " · ciclo N" se N>0) (112, 74, 170, 14) `body` 12 `text-2`.
- **Reiniciar** botão circular 24 (`rotate-ccw` 14, fundo `surface`) em (268, 50).
- **Fileira de botões** (y 90…118): **Pausa curta** pílula (112, 92, 108, 26) `surface` com `coffee` 14 + texto `body` 11; **play/pausa** pílula **coral** (226, 90, 62, 30) com ícone ▶/❚❚ 16 branco; **Foco** pílula (294, 92, 100, 26) `surface` com `target` 14. Pausa longa: segunda pressão em "Pausa curta" alterna Curta → Longa (**PROPOSTA**); ver custom abaixo.
- **Valor personalizado (MM:SS):** clique no tempo (parado) → campos `MM : SS` editáveis (teclado, ↑/↓, roda), Enter confirma, Esc cancela; limites 1 s…99:59. Presets e padrões em §12.
- **Estados:** parado ("Foco 25:00"), rodando, pausado (tempo pisca `text-2`↔`text` a 1 Hz), **fim** (anel cheio `ok`, "Fim" + toast do Windows + bipe; botão play vira "Novo ciclo"). [AJUSTAR: a referência não mostrou o timer rodando.]

### 4.3 Shelf (cor: âmbar `#FFC857`)
- Cabeçalho: título "Shelf" `title`. **Com itens:** pílula **Selecionar** (72, 11, 78, 22) `surface` 11 ao lado do título e **Limpar tudo** (320, 14, 76, 16) `body` 11 `danger` (`#FF6B6B`), alinhado à direita **em vez** de data/hora/bateria.
- **Vazio:** caixa tracejada (40, 38, 340, 80) raio 16, traço 1,5 `line`, tracejado 6/4; `inbox` 24 `text-3` em (60, 66); título "Solte arquivos aqui" `body-strong` 13 (96, 62, 270, 16); legenda "Arraste para fora quando quiser · clique para abrir ou copiar" `small` `text-2` (96, 80, 270, 28).
- **Com itens:** cards **64 × 78**, raio 10, `surface`, x = 24 + n·72, y=40; rolagem horizontal (roda/arrasto), esmaecer 16 px nas bordas. Card: ícone do tipo 22 px (miniatura da imagem se for imagem [AJUSTAR]) em (21, 8), nome (6, 44, 52, 14) `small` `text` reticências, tamanho (6, 59, 52, 12) 10 `text-3`; `x` 14 em (46, 4) visível no hover; hover `surface-hover`. Clique: abre o arquivo; Ctrl+clique/menu: copia (Selecionar ativa multi-seleção com checkbox).
- **Arrastar arquivo por cima do monitor:** abre sozinho (qualquer estado) em Shelf; fundo ganha **vinhetas azul-marinho** (`#0A2A5C` @ 55%, radial 90 px nos cantos inferiores), caixa vazia vira "**Solte para adicionar**" `text`, borda `accent`; pontos de paginação visíveis (dock oculto). 220 ms. Solta → card entra com escala 0,9→1 (160 ms). Arrastar **para fora**: a ilha recolhe durante o arraste e reabre se o arraste voltar (como medido); `shelf_start_drag` (já provado).

### 4.4 Tarefas (cor: menta `#5EE0B0`) — **PROPOSTA**
- Título "Tarefas" inteiro em menta.
- **Contador** "0/0" (24, 40, 80, 36) 30/36 700 `text`; legenda "Concluídas hoje" (24, 78, 120, 14) `small` `text-2`; ícone `calendar` em círculo 26 `surface` (112, 46) com ponto Ø6 `accent` quando há tarefa com prazo hoje [AJUSTAR].
- **Botões**: **Adicionar** pílula (24, 98, 104, 22) `surface` com `circle-plus` 14 · **Editar** (134, 98, 84, 22) com `pencil` 14, desabilitada (40%) sem tarefas.
- **Painel de lista** (222, 40, 174, 78) raio 14, fundo `#FFFFFF` @ 5%: até 3 tarefas visíveis (altura 24): círculo Ø14 (vazio / `circle-check` menta), texto `body` 12 reticências; concluída = `text-3` riscada; clique alterna; rola com roda. **Vazio:** `list-checks` 20 `text-3` + "Sem tarefas." `body` 11 `text-2`.
- **Adicionar:** o painel vira um campo de texto de 1 linha (autofoco, "Nova tarefa…"), Enter adiciona e mantém o campo, Esc fecha. **Editar:** mostra `x` por item; duplo clique edita o texto. Dados em `%APPDATA%\Eavely\todo.json`; "concluídas hoje" zera à meia-noite.

### 4.5 Configurações (engrenagem do dock)
Abre a **janela de Configurações** (§12), não uma página da ilha.

### 4.6 Atalhos de interação
Roda do mouse sobre a ilha: próxima/anterior página (travar 380 ms); ←/→ quando focada; clique nas setas/pontos/dock. Duplo clique no traço/peek: nada.

### 4.7 Ao vivo
**Compacto — padrão, medido internamente (168 × 30, raios 14):** é o peek com um indicador depois da hora. Layout: bateria (14, 7, 16, 16) · hora `HH:MM` `body-strong` 14 (40, 5, 56, 20) · **indicador** (112, 7, 44, 16), alinhado à direita em x=154: **música** = **equalizador** · **timer rodando** = ícone `timer` 16 na cor do timer (`coral`, `ok` no fim). Música + timer: os dois lado a lado (pílula cresce para ~190 px) [AJUSTAR]. Mouse sobre → Aberto; o **mesmo indicador** aparece no canto superior direito da ilha aberta (§3.2).
**Equalizador (arte original):** 3 barras verticais de **3 px** com **2 px** de intervalo (caixa 13 × 14), cantos redondos, cor da página (violeta `#B69CFF`), alturas entre **4 e 14 px** oscilando por seno a **60 fps** com períodos **0,9 s / 1,15 s / 1,4 s** e fases defasadas (0, 0,33, 0,66 do ciclo); **pausado** = 3 barras de 4 px paradas (cor `text-3`); reduced-motion = barras fixas em 8/12/6 px. Só anima com música tocando e ilha visível (zero custo oculta).
**Expandido — opção (COMPORTAMENTO.md, 300 × 52, raios 22):** **Música:** capa (8, 8, 36, 36) raio 8; título (54, 7, 190, 18) `body-strong`; **linha de letra** (54, 26, 190, 16) `body` `text-2` (sem letra: artista); equalizador (254, 18, 13, 14); `eye-off` (274, 14, 24, 24) no hover. **Timer:** arco 32 (10, 10) traço 3 `coral`; `MM:SS` `clock-live` (52, 8, 120, 18); legenda "Foco · ciclo N" (52, 28). Música + timer: mostra a música e um ponto 6 px coral em (286, 8). Mouse sobre → Aberto. Configurações: "Ao vivo: Compacto (padrão) · Expandido".

## 5. Animações (tempo, vblank; reduced-motion = 0)
| Evento | Duração | Curva | Detalhe |
|---|---|---|---|
| Abrir (peek/traço/ao vivo → aberto) | **240 ms** | ease-out | largura+altura+raio juntos; conteúdo fade-in 0→1 em 100 ms começando em t=100 ms (aos 100 ms a forma já está ~280×80) |
| Fechar | **220 ms** | ease-in-out | conteúdo some em 100 ms **primeiro**, depois a forma encolhe; atraso de saída ≤ 100 ms |
| Troca de página | **280 ms** | ease-out | **crossfade** (A 1→0 em 140 ms, B 0→1 de 60 a 280 ms), itens deslizam ±8 px na direção da navegação; **mesma altura** (sem salto) |
| Dock (ilha cresce/encolhe 28 px) | **200 ms** | ease-out | pontos↔dock em crossfade 120 ms |
| Tela dividida de letras | **320 ms** abrir / **300 ms** fechar | ease-out | §4.1.1; rolagem das letras 350 ms |
| Drag → Shelf | 220 ms | ease-out | abrir + vinhetas azuis fade 200 ms |
| Letra | 220 ms | ease-out | sobe 6 px + crossfade |
| Gradiente da capa | 400 ms | linear | crossfade ao mudar de faixa |
| Hover/press, toggles | 120 / 160 ms | ease-out | |
| Overlay entra/sai | 200 / 220 ms | ease-out / ease-in | §7 |
Nada de overshoot (a referência não tem). Mouse sai durante a abertura: inverte a partir do ponto atual (sem saltos).

## 6. Letra de uma linha e gradiente da capa
- **Letra:** só **uma linha**, a atual (última com `timeMs ≤ posição`); linha vazia ("♪") mostra "♪". Troca com crossfade 220 ms. Letra não sincronizada: nenhuma linha. Spotify precisa funcionar (LRCLIB por título/artista).
- **Gradiente:** fundo da ilha = `bg` (#000) no topo → `tint` na parte de baixo: `linear-gradient(180°, #000 0%, #000 35%, tint 100%)`, alpha de `tint` 0,55. `tint` = **cor dominante da capa**: reduzir a 24×24, descartar pixels com saturação < 0,15 ou luminância < 0,08/> 0,92, agrupar em 12 baldes de matiz ponderados por saturação, pegar o mais pesado, e **fixar L entre 0,18 e 0,32 (HSL)** para o texto `text-2` (64%) manter ≥ 4,5:1. Sem capa/sem cor útil: sem gradiente (preto). Só na página Música (e opcionalmente ao vivo [AJUSTAR]). Calcular na troca de faixa em thread de fundo.

## 7. Overlays de sistema (**PROPOSTA** — a referência medida não tem nenhum)
Pílula **280 × 48** (raio inferior 22) no topo central, mesmo fundo preto, sobre qualquer estado exceto tela cheia; se a ilha está **aberta**, o overlay não aparece (a ilha já está em foco). Entra 200 ms (crescendo de 150×30 ou da forma atual), fica **1,8 s** após o último evento (volume/brilho: cada evento reinicia o prazo), sai 220 ms. Cada overlay pode ser desligado em Configurações.
Layout comum: ícone 20 px em (16, 14), cor própria.
| Overlay | Conteúdo | Cor | Gatilho (Win32) |
|---|---|---|---|
| **Volume** | ícone `volume-x`/`volume-1`/`volume-2` + barra x 48…216 (h6, centro y=24) + "NN%" (224, 17, 40, 14) dir. 12 tabular | `accent` (mudo: `text-3`) | `IAudioEndpointVolume::RegisterControlChangeNotify` (mudou volume/mudo) |
| **Brilho** | `sun` + barra + "NN%" | `accent-2` | WMI `WmiMonitorBrightnessEvent` (só notebook) |
| **Bateria** | `battery-charging`/`battery`/`battery-low` + título "Carregando"/"Bateria fraca"/"Carga completa" (48, 9, 200, 16) `body-strong` 13 + "64%" `small` (48, 27) | verde / `accent-2` ≤ 20% / vermelho ≤ 10% | `WM_POWERBROADCAST` (PBT_APMPOWERSTATUSCHANGE) e `GetSystemPowerStatus`; mostra ao ligar/desligar o carregador, 20%, 10%, 100% |
| **Wi‑Fi** | `wifi`/`wifi-off` + "Conectado" (13) + SSID (11 `text-2`) ou "Sem conexão" | `accent` / `danger` | `NotifyIpInterfaceChange` + `WlanRegisterNotification` |
| **Fones** | `headphones` + "Fones conectados"/"Fones desconectados" + nome do dispositivo (11) | violeta `#B69CFF` | `IMMNotificationClient` (OnDeviceStateChanged/OnDefaultDeviceChanged, formfactor Headphones/Headset) |
| **Caps Lock** | `arrow-big-up` + "Caps Lock ativado/desativado" (13) | `text` (ativado) / `text-3` | **sem hook de teclado**: ver §7.1 |
Prioridade se vários: o mais novo substitui o atual (sem fila). Com mídia/timer ao vivo, o overlay cobre a ilha ao vivo e depois ela volta.

### 7.1 Caps Lock sem hook global (antivírus e privacidade)
`WH_KEYBOARD_LL` fica **proibido** (é o padrão de keylogger e aciona antivírus). Alternativas, em ordem de preferência:
1. **(Recomendada) Polling leve de `GetKeyState(VK_CAPITAL) & 1`** a cada **250 ms**, só enquanto o overlay de Caps Lock estiver ligado em Configurações (padrão: desligado) e a ilha não estiver em tela cheia. Não vê nenhuma tecla, não instala nada global, sem risco de antivírus; custo < 0,1% de CPU (1 chamada leve de `user32`), latência ≤ 250 ms (aceitável para um aviso). Verificado: um processo de console sem janela, em segundo plano, lê o estado do Caps Lock corretamente após o toggle (teste feito no design: antes=0, depois=1, restaurado=0). **Abre exceção à regra "sem polling"** (só quando o overlay está ligado; documentar no README); pausar o timer quando o PC está bloqueado/tela desligada (`WTS_SESSION_LOCK`/`PBT_POWERSETTINGCHANGE`).
2. **Raw Input** (`RegisterRawInputDevices`, teclado `UsagePage=1, Usage=6`, `RIDEV_INPUTSINK`, janela de mensagens própria): orientado a evento (sem polling), mas recebe **todas** as teclas de todos os apps; o código deve ler apenas `RAWKEYBOARD.VKey == VK_CAPITAL`, descartar o resto no mesmo instante e nunca armazenar/registrar. Menos suspeito que o hook, mas ainda "vê" a digitação — **só se a latência do polling não bastar**.
3. Descartadas: `WH_KEYBOARD_LL`, `GetAsyncKeyState` em laço (varre teclado), `SetWindowsHookEx` em geral.
**Validação pendente:** confirmar o toggle com o Caps pressionado em outro app em foco e em sessão remota; trocar para a opção 2 se algum caso falhar.

## 8. Olhos ociosos (**PROPOSTA**)
Opção (Configurações → "Aparência recolhida"): *Bateria e hora* (padrão, a referência) ou *Olhos*. Em *Olhos*, o **peek** (150×30, §2) mostra dois olhos no lugar de bateria/hora: elipses brancas 8 × 12 (centros em x=−9 e +9 do centro, y=15), pupilas pretas Ø5 que **seguem o cursor** (deslocamento máx. 2 px, suavizado 80 ms), **piscam** (escala Y → 0,1 em 120 ms) a cada 3–7 s aleatórios, e fazem "dormir" (pálpebra a meio) após 60 s sem o mouse por perto. Reaproveita o olho do ícone do app. Só desenha com a pill visível (nunca em tela cheia nem no traço); zero custo quando oculta (timer parado).

## 9. Persistência
- **Página selecionada persiste** ao fechar/reabrir (e entre execuções): `selectedPage`. Só muda se o usuário navegar, **ou** ao arrastar arquivo (vai para Shelf e **fica** em Shelf) — [AJUSTAR: voltar à anterior?].
- Persistem entre execuções: itens do Shelf (`shelf.json`), tarefas (`todo.json`), config geral (`settings.json`), tela dividida de letras (`lyricsSplit`), linha de letra (`showLyricLine`), meta/padrões do timer.
- **Não** persistem: timer em andamento (reinicia ocioso ao abrir o app; durante a execução continua rodando mesmo com a ilha fechada), posição do popover de volume.
- Ordem das páginas é **fixa** (não reordenável no MVP).

## 10. Interação e menu da bandeja
- **Seek otimista:** ao clicar/arrastar a barra, preenchimento e tempo mudam **no mesmo frame**; chama `media_seek`; **ignora posições do player por 400 ms**; depois só corrige se `|real − UI| > 1 s`. Interpolação entre eventos de posição (~500 ms) com relógio monotônico.
- **Acessibilidade:** UI Automation básica (nome/estado dos botões); foco com Tab quando a ilha tem foco (opcional no MVP). Tooltips após 600 ms (`small` em pílula `surface-press`).
- **Bandeja** (nativo): `Desligar notch`/`Ativar notch` · `Configurações` · `Sair`. App inicia escondido na bandeja.

## 11. Pendências e perguntas
1. **[AJUSTAR]** Tarefas é inferida de um único quadro estático; Adicionar/Editar tarefa e timer rodando **sem referência**.
2. Overlays de sistema (§7), olhos (§8), peek, pop-over de volume: **PROPOSTA** pura.
3. Idioma: textos em pt-BR (referência em inglês) — confirmar.
4. Janela maximizada/tela cheia: seguimos o COMPORTAMENTO.md (a referência não recuou, tela cheia não observada).
5. Win10: `Segoe UI Variable` ausente → Segoe UI; D2D SVG/DirectComposition exigem 1703+.
6. Configurações acrescentam: overlays (6 toggles), "Aparência recolhida", "Linha de letra", ver §12.


## 12. Tela de Configurações
Janela normal do Windows (não é a ilha): **440 × 600**, redimensionável só na vertical (mín. 440×420), centrada no monitor do cursor, título "Configurações — Eavely", ícone do app. Abre pelo `settings` (dock da ilha, §3.2) ou pelo menu da bandeja. Fundo `#121316`, cartões `#1A1C20` (raio 14), mesmos tokens de texto/accent. Barra de título nativa escura (`DWMWA_USE_IMMERSIVE_DARK_MODE`). Conteúdo rola na vertical (roda do mouse, barra fina 6 px `surface-hover` que aparece ao rolar), margem 20 px nos lados.

### 12.1 Componentes (px)
| Componente | Medida | Detalhe |
|---|---|---|
| Linha | altura 48 (56 com descrição), padding 0 16 | rótulo `body` 13/18 `text`; descrição `small` `text-2`; controle alinhado à direita; divisória 1 px `line` entre linhas |
| Cartão de seção | largura 400 (x=20), raio 14 | título da seção acima: `small` caixa-alta, espaçamento +0,6 px, `text-3`, em (24, topo−22) |
| **Toggle** | 40 × 22, raio 11 | desligado: trilho `surface-hover`, botão 16 `text`; ligado: trilho `accent`, botão `#000`; botão desliza 160 ms `ease-out` |
| **Menu suspenso** | 168 × 28, raio 8 | fundo `surface`, texto `body`, `chevron-down` 14 à direita; lista em cartão `#24262B` sobre a janela, itens 32 px |
| **Stepper** (minutos) | 104 × 28 | `[−] 25 min [+]`, botões 28×28 `surface`, valor `body-strong` tabular; segurar repete; limites por campo |
| **Campo de atalho** | 148 × 28, raio 8 | mostra "Ctrl + Alt + E"; clique → "Pressione as teclas…" (`accent`); Esc cancela, Backspace limpa; exige ao menos Ctrl/Alt/Win |
| Segmentado | altura 28, raio 999 | igual ao do timer (§4.2) |
| Botão | 30 de altura, raio 999 | secundário: `surface-hover` + texto `body-strong` |
Foco de teclado em todos (anel §1); Tab percorre na ordem visual.

### 12.2 Seções e valores padrão
Layout vertical (y relativo ao início do conteúdo, após o cabeçalho de 56 px com "Configurações" `16/24 600` em (20,16)):

**Geral** (cartão y=24, 1 linha)
1. *Iniciar com o Windows* — toggle, padrão **desligado** (liga = entrada em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`). Descrição: "Abre o Eavely na bandeja ao entrar no Windows."

**Ilha** (cartão, 5 linhas)
2. *Mostrar traço quando ocioso* — toggle, padrão **ligado**. Desligado: some por completo até o mouse chegar à zona (decisão do autor).
3. *Mostrar a ilha ao vivo* — toggle, padrão **ligado** (música/timer em 300×52).
4. *Ocultar durante eventos* — toggle, padrão **desligado**. Ligado: durante música/timer só o traço aparece (mesmo efeito do botão `eye-off` do estado ao vivo, mas permanente). Dependência: com (3) desligado fica desabilitado.
5. *Monitor preferido* — menu: **Seguir o cursor** (padrão) · Monitor principal · cada monitor listado ("Monitor 1 — 1920×1080").
6. *Zona de ativação do topo* — segmentado **Estreita 24 px · Normal 40 px · Larga 64 px**, padrão Normal.

**Timer** (cartão, 6 linhas)
7. *Foco* — stepper 1–180 min, padrão **25**.
8. *Pausa curta* — stepper 1–60 min, padrão **5**.
9. *Pausa longa* — stepper 1–120 min, padrão **15**.
10. *Focos até a pausa longa* — stepper 2–8, padrão **4**.
11. *Iniciar o próximo ciclo automaticamente* — toggle, padrão **desligado**.
12. *Aviso e som ao terminar* — toggle, padrão **ligado**.
Os presets alimentam os segmentos Foco/Pausa/Longa da aba Timer (§4.2); o valor personalizado MM:SS é por uso e não altera estes padrões.

**Atalhos** (cartão, 3 linhas; globais, `RegisterHotKey`)
13. *Mostrar/ocultar a ilha* — padrão **Ctrl + Alt + E**.
14. *Iniciar/pausar o timer* — padrão **Ctrl + Alt + T**.
15. *Abrir a aba Arquivos* — padrão **Ctrl + Alt + F**.
Conflito (atalho já registrado): campo fica com contorno `accent-2` e texto "Em uso por outro app".

**Sobre** (cartão, altura ~150)
- Ícone do app 48×48 em (16, 16); "Eavely" `title` e "Versão 0.0.1" `small` `text-2` em (76, 18) e (76, 40).
- Texto `small` `text-2` (16, 76, 368, 32): "Notch leve para Windows. Sem telemetria: a rede é usada só para buscar letras (LRCLIB)."
- Botões (16, 112): **Licenças de terceiros** (abre `THIRD-PARTY-LICENSES.md` embutido em uma janela de texto rolável 480×520), **Código-fonte (GitHub)** (abre `github.com/LimaEdu-7x/eavely`), **Licença MIT**.
- Rodapé: "© 2026 Eavely · MIT" `small` `text-3`.

### 12.3 Persistência
`%APPDATA%\Eavely\settings.json`, aplicado imediatamente (sem botão Salvar). Chaves: `startWithWindows`, `showTrace`, `showLive`, `hideDuringEvent`, `monitor` (`"cursor"`, `"primary"` ou id do monitor), `triggerZonePx`, `timer.{focusMin,shortMin,longMin,cyclesToLong,autoNext,notify}`, `hotkeys.{toggleIsland,toggleTimer,openShelf}`, `selectedTab`. Valores fora do intervalo são corrigidos ao ler; arquivo ausente/corrompido = padrões.


### 12.4 Acréscimos da paridade (todas **PROPOSTA**)
Na seção **Ilha**: *Aparência recolhida* — segmentado **Bateria e hora (padrão) · Olhos** (§8); *Mostrar linha de letra na Música* — toggle, padrão ligado; *Ao vivo* — segmentado **Compacto (padrão) · Expandido** (§4.7). Nova seção **Avisos do sistema** (cartão de 6 linhas, toggles, padrão **desligados** até o autor aprovar): Volume, Brilho, Bateria, Wi‑Fi, Fones, Caps Lock (§7). Chave extra em `settings.json`: `collapsedLook`, `liveStyle`, `showLyricLine`, `overlays.{volume,brightness,battery,wifi,headphones,caps}`, `lyricsSplit`.

## 13. Ícone do app e da bandeja (originais)
Conceito: uma **ilha pendurada no topo** (o beiral) com um **olho** dentro — "Eavely" ≈ *eave* (beiral) + *eye*. Arte própria, sem relação com outros apps. Arquivos em `assets/app/`:
| Arquivo | Uso |
|---|---|
| `eavely-icon.ico` | 16/20/24/32 (simplificadas, sem estrelas) + 40/48/64/128/256 (completas), PNG em todas — exe, instalador, atalhos |
| `eavely-tray-{16,20,24,32}.png` | bandeja: um desenho que le em barra clara e escura; o tamanho segue o DPI (`SM_CXSMICON`) |
| `logo-sobre-128.png` / `logo-sobre.bgra` | logo da pagina Sobre (o `.bgra` 96x96 pre-multiplicado e gerado por `scripts/make-about-logo.py`) |
| `docs/img/banner.png` | banner dos READMEs |
