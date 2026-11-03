# QA visual da UI nativa

## QA 5: auditoria de CENTRALIZAÇÃO e alinhamento (feat/native-ui @ a23949a)

Método: medi **por pixel** as capturas 600×176 (ilha 420 de largura em x=90..509, centro x=300; coordenadas abaixo em px relativos à ilha, centro = 210). Cada item tem o desvio medido e a correção. A observação procede: a maioria dos botões tem o conteúdo **encostado à esquerda** e há deslocamentos de 4 a 17 px.
**Importante (autocrítica):** vários desvios nasceram no **meu** spec/mock (transporte a 218, botões com conteúdo à esquerda); o nativo copiou fielmente. A correção é no spec e no código.
Não avaliado: DPI ≠ 100%, hover/clique, animações; medidas com ±1 px.

### P1 — desvios claros (≥ 8 px)
| # | Onde | Medido | Correção |
|---|---|---|---|
| C1 | **Música: transporte** (anterior · play · próximo) | prev centro 181, play 218, next 255 → grupo centrado em **218**, ilha em **210** (+8 px); os pontos de página estão em 210 | mover o grupo −8 px: prev 173 · play **210** · next 247 (passo 37). Corrigir também o mock 01/§4.1 |
| C2 | **Pomodoro: botão "Foco"** (pílula 294..392) | conteúdo (ícone+texto) em 305..349: margem esq. 11, dir. 43 → **−16 px** do centro | centralizar o grupo ícone+rótulo (espaço 6) na pílula; ou pílula "hug" com padding 14 |
| C3 | **Tarefas: "Adicionar"** (pílula 24..128) | conteúdo 32..102: margem 8 / 26 → **−9 px** | centralizar o grupo; idem **"Editar"** (−8,5 px) |
| C4 | **Pomodoro: "Pausa curta"** (pílula 112..220) | conteúdo 123..202: margem 11 / 18 → **−3,5 px** | centralizar o grupo |

### P2 — 2 a 6 px
| # | Onde | Medido | Correção |
|---|---|---|---|
| C5 | **Texto em pílulas: vertical** | "Adicionar": pílula y 98..120 (centro 109), texto 106..115 (centro 110,5) → **1,5 px abaixo** | centrar pela altura das maiúsculas (baseline = centro + capHeight/2); vale para todos os botões e chips |
| C6 | **Música: coluna esquerda/direita** | ícone do balão em x=28..44, tempo "00:35" começa em 24 → 4 px; ícone de volume termina em 392, "−02:57" termina em 396 | alinhar bordas: balão em x=24..40 e volume em 380..396 (o centro do conjunto continua 210) |
| C7 | **Cabeçalho: título pula entre páginas** | título Pomodoro/Tarefas: centro y=22; Shelf: centro y=18 (−4 px) → pula ao trocar de aba | fixar **uma** altura: cabeçalho com título em y=8 (centro 18) em **todas** as páginas (Shelf já está; mover as outras 4 px para cima e manter o conteúdo) |
| C8 | **Ao vivo expandido (300×52)** | margem esq. 10 (capa), dir. 15 (indicador termina em 285) | indicador termina em 290 (margem 10); capa e indicador com margem igual |
| C9 | **Overlay de volume: "65%"** | dígitos y 22..30 (centro 26); trilho e ícone centro 24 → **2 px abaixo** | centrar pelo método de C5 |
| C10 | **Barra de letras 300×34** | ♪ margem 15; texto + ♪ ocupam 15..262 (margem dir. 38) | texto na caixa 32..268 (centro 150) e ♪ espelhado por um respiro de 14 + 12 na direita; checar com linha longa (hoje corta sem `…`/marquee) |
| C11 | **Setas laterais** | círculos 16×16 em x 4..20 / 400..416, centro y=75 (spec 18×18 em 3 e 399) | voltar para 18×18 ou atualizar o spec; centro y=75 está 5 px abaixo do centro da ilha (70): ok se alinhado ao conteúdo, mas documentar |

### P3 — 1 px / observações
- C12. **Altura da ilha 140** (spec 136): pontos de página em y=127..131 (centro 129), a 9 px do fundo. Atualizar o doc ou voltar a 136.
- C13. **Traço** 58×5 com flares (spec 56×3): 2 px mais alto que o spec.
- C14. **Pílula de pontos**: ativa 15×4 e inativos 4×4 (spec 14×3 e 3×3).
- C15. **Cartões da Shelf** (45 + 66·n): ícone e nome centralizados nos cards nas capturas 19/49, e os 4 cards ficam à esquerda (esperado); não medi à régua o centro interno de cada card.
- C16. **Horizontalmente OK (medido)**: pontos de página (184..236, centro 210) em todas as páginas; olhos (66 e 84, centro 75 no peek); peek (margens 14 / 14, bateria e hora com centro vertical 15); ícone e texto dos overlays de uma linha (centro y=24); relógio no cabeçalho (bases alinhadas em y=27).

### Compacto/peek: fonte e ícone da bateria diferentes (pedido 2)
No peek (02/43) e no compacto a hora aparece em **negrito largo (GDI)** e a bateria como **retângulo verde cheio, sem cantos arredondados nem bolt de carga**, enquanto o cabeçalho usa Segoe UI Variable e o ícone Lucide de traço fino com cantos arredondados. Spec único em **DESIGN-NATIVO §19** (Segoe UI Variable Text, hora 14/600 no peek, 12/600 no cabeçalho, mesmos ícones Lucide 16 px traço 2, cores por nível, DirectWrite em tons de cinza).

### Resumo de correções por arquivo 
1. Transporte da Música −8 px (C1) e colunas laterais (C6).
2. Função única `drawButtonLabel(icon, text)` que centra o grupo e usa altura de maiúsculas (C2–C5, C9).
3. Header: título em y=8 em todas as páginas (C7).
4. Unificar relógio/bateria no pipeline DirectWrite (peek e compacto, §19).

---

## QA 4: overlays, olhos, peek, configurações, tarefas com dia (feat/native-ui @ 5db3370, capturas 29–43)

Comparado com mocks 28–33, 22 e DESIGN-NATIVO §7, §8, §12. Capturas novas já vêm de janela com fundo cinza próprio (sem tela inteira). Só imagens estáticas: não vi animação de entrada/saída, olhos seguindo o cursor, piscar, clique nem hover.

**Resolvido do QA 3:** T2 (calendário agora cabe: 6 linhas + rodapé "5 nov 2026" e botão **Hoje**); T10 (botão Hoje existe); dia de hoje (outline) e dia escolhido (preenchido mint) distintos.

### Medidas
Overlay 280×48 ✓ · peek 150×30 ✓ · janela de configurações 456×639 com barra de título (spec 440×600 de área útil; ok) · ícones e raios batem.

### P1
Nenhum novo que quebre o design.

### P2
- **O1. Peek (43) com conteúdo encostado à esquerda**: bateria em x≈22 e hora logo ao lado, deixando ≈ 50 px vazios à direita da pílula. Mock 22 distribui: bateria à esquerda, hora à direita (margens 14 px). Usar `space-between` ou centralizar o grupo.
- **O2. Caps Lock em Configurações sem a nota de privacidade** (31): o mock tem "Verifica a tecla a cada 250 ms só com isto ligado." A nota é a transparência sobre o polling; sem ela o aviso ao usuário se perde. Recolocar.
- **O3. Sobre sem o ícone do app** (31): mock tem o tile 40×40 com o ícone original ao lado de "Eavely"; o nativo só tem texto. Ícone pronto em `assets/app/`. Rodapé "© 2026 Eavely · MIT" ficou alinhado à esquerda (mock: centralizado).
- **O4. "Ao vivo" (Compacto/Expandido) longe do toggle "Mostrar a ilha ao vivo"** (29): no nativo há 2 linhas entre eles ("Ocultar durante eventos", "Aparência recolhida"). Agrupar logo abaixo do toggle, como no mock.
- **O5. Texto auxiliar de ≈ 10 px nas configurações** (29): "Abre o Eavely na bandeja…" abaixo de 11 px, difícil de ler em 100% DPI. Mínimo 11/12 px e cor `text-2`.
- **O6. Atalhos sem sinal de que são editáveis** (30): botões "Ctrl+Alt+E" parecem rótulos. Falta hint "clique e pressione as teclas" ou ícone `pencil`; também o espaçamento "Ctrl + Alt + E" do mock é mais legível.
- **O7. Calendário ainda com alvos pequenos** (33): linhas ≈ 16 px de passo e células ≈ 19 px (pior que antes por causa do rodapé). Setas do mês ≈ 14 px. Mantém T3.
- **O8. Estados de overlay não capturados**: Brilho (`sun`), Wi-Fi conectado, Caps Lock **desativado**, bateria cheia/100%, volume em alto (≥ 70 com `volume-2`) e entrada/saída animadas. Não posso validar cor `text-3` do Caps desativado nem o gatilho do Wi-Fi.

### P3
- O9. **Caps Lock ativado**: seta em contorno (`arrow-big-up` stroke); mock usa preenchida (sinal "ligado" mais forte). Preencher quando ativo, contorno quando desativado.
- O10. **Bateria fraca** sem a dica "· conecte o carregador" do mock (36): só "18%". Texto curto, adicionar se couber em 280 px.
- O11. **Olhos**: pupila parece à esquerda do olho e o contorno do olho é mais fino que o mock (41). "Dormindo" (42): pálpebra em meia-lua pequena; o mock usa traço horizontal "– –", mais legível como "dormindo". Seguir cursor/piscar não avaliados.
- O12. Segmentados: opção ativa em **azul** (nativo) vs **pílula branca** (mock 33). Coerente dentro do app, só diferença do mock; decidir e atualizar um dos dois.
- O13. Zona de ativação: nativo "Estreita / Normal / Larga" e selecionada **Larga**; mock "24/40/64 px" com 40 selecionado. Padrão deve ser Normal; mostrar px ajuda.
- O14. "Monitor preferido" com chevron `>` (parece navegação); mock usa `v` (lista). Trocar para `chevron-down`.
- O15. Ordem das linhas em **Ilha** difere do mock (Aparência/Linha de letra vs Monitor/Zona). Escolher uma ordem e travar no doc.
- O16. Linha "Mostrar traço quando ocioso" sem a legenda "Desligado: some até o mouse chegar ao topo." (mock tem).
- O17. Captura `32-config-fim` é idêntica a `29-config-topo` (mostra o topo, não o fim). O fim real aparece na 31. Peço repetir ou apagar.
- O18. Atalho 3 ficou "Abrir o Shelf" (melhor que "Abrir a aba Arquivos" do mock/doc): atualizar o doc.
- O19. Valores de teste em Timer (50/10/15) diferem dos padrões 25/5/15; ok se forem dado de teste.

### Melhor que o mock
- Overlay de **fones** com nome do dispositivo em duas linhas ("Fones conectados" / "Headphones (USB Audio)").
- Overlay de **volume mudo**: ícone `volume-x` e trilho vazio com "0%" deixam o estado claro.
- Configurações com **barra de título** própria (ícone, minimizar, fechar; maximizar desabilitado) e botão **−/+** claros nos steppers.
- Botão **Hoje** e rodapé com data no calendário.

---

## QA 3: Tarefas (feat/native-ui @ fb820a8, capturas 24–28)

Comparado com mocks 17/18 e DESIGN-NATIVO §4.4. Só imagens estáticas; não vi clique, hover, rolagem animada nem o que acontece ao escolher um dia.

**Resolvido do QA 2:** ponto azul do calendário (N8) agora aparece; painel da lista alinhado ao mock 18 (x 292–467); contador "1/5 Concluídas hoje", botões Adicionar/Editar e cabeçalho batem.

### P1
- **T1. Nome colide com o horário na lista** (25 editar): "Ligar para o Joao09:00" sem folga e sem reticências; "Enviar relatorio 3 17:30" também encosta. O botão × (modo editar) tira largura do nome e o horário não cede. Regra: nome com `…` e horário/chip com largura reservada; mínimo 8 px entre eles.
- **T2. Calendário: a 6ª linha (29, 30) encosta no fim do painel** (27): sobra ≈ 1–2 px abaixo dos números. Meses com 6 semanas sempre acontecem (nov/2026 é um). Opções: linha 20 px (hoje ≈ 21 px) com cabeçalho do mês em 14 px, ou mostrar semana seguinte rolando, ou ampliar o painel 6 px só nessa página.

### P2
- **T3. Alvos do calendário pequenos**: células ≈ 21×19 px e setas do mês ≈ 14 px, abaixo dos 28 px que o projeto adotou (dock). Para 7×6 em 175 px não cabe 28; mínimo viável: setas 24×24 e células 24×20 com hit-area maior que o desenho.
- **T4. Marcador de "dia com tarefa" quase invisível** (27, dias 3 e 4): ponto ≈ 2 px, contraste baixo. Usar 4 px na cor mint e alinhado ao centro do número.
- **T5. Data ambígua no chip** (24): "3 17:30" não diz que 3 é o dia (nov). Sugestão: "3 nov 17:30", "Amanhã 09:00", "Hoje 14:30" e vermelho para atrasada. Hoje só o horário de hoje aparece em mint.
- **T6. Adicionando sem feedback do que foi entendido** (26): digitei "Reuniao de equipe 14:30" e o chip continua "Hoje". O horário 14:30 reconhecido deveria aparecer no chip ("Hoje 14:30") e o texto "14:30" sair do título (ou ficar destacado). Dica "Ex.: Ligar 14:30 · Enter salva" não cita Esc para cancelar.
- **T7. Botão "Editar" ativo continua escrito "Editar"** (25): o estado mint não diz como sair. Trocar por "Pronto" (com ícone `check`) enquanto edita.
- **T8. Rolagem pouco descoberta** (24, 28): barra ≈ 2 px, 3 linhas exatas, sem sinal de que há mais (5 tarefas). Mostrar meia linha/fade de 8 px no rodapé e no topo quando houver conteúdo; a barra encosta no horário "09:00" (≈ 0 px) — dar 4 px.
- **T9. Linha concluída tachada com texto cinza escuro** (28): "Comprar pao" ≈ 3:1; ok como estado desativado, mas conferir ≥ 3:1 medido.

### P3
- T10. Calendário e lista: sem botão "Hoje" para voltar ao mês atual depois de navegar.
- T11. Calendário sobrepõe a lista inteira (27): o contador "1/5" fica visível, bom; fechar com Esc/clique fora não foi testado.
- T12. Textos de exemplo sem acento ("pao", "Joao", "Reuniao") nas capturas: dados de teste ou fonte sem acento? Se for dado de teste, ok; se for código, corrigir.

### Melhorias de UX (sugestões, não bugs)
1. **Edição só por clique em "Editar"**: permitir duplo clique no texto para renomear, e Delete/Backspace com a linha focada para apagar. Hoje apagar exige entrar no modo, mirar um × de ≈ 14 px.
2. **Desfazer** após excluir (toast de 4 s com "Desfazer") em vez de apagar direto.
3. **Teclado**: ↑/↓ entre tarefas, Espaço marca, Enter edita, `N` adiciona. Foco visível (anel mint 2 px) não aparece em nenhuma captura.
4. **Rolagem com roda do mouse** conferir que a roda não troca de página quando a lista rola (ou só troca no limite).
5. **Atrasadas** (data < hoje, não concluída) com chip vermelho e contagem no contador (ex.: "1/5 · 1 atrasada").
6. **Calendário**: clicar num dia filtra a lista e o chip do botão mostra o dia ("3 nov"); segundo clique limpa o filtro.
7. **Concluídas ao final** da lista ou ocultas após 1 dia; hoje "Comprar pao" concluída ocupa a 1ª linha e empurra as pendentes para fora da vista (28).
8. **Persistir rolagem** ao voltar à página (Tarefas ↔ Pomodoro).

### Medidas (por olho)
| Item | Mock | Nativo |
|---|---|---|
| Painel da lista | 292–467 | ✓ |
| Linha da lista | 24 px | ≈ 25 ✓ |
| Células do calendário | n/a | ≈ 21×19 |
| Setas do mês | n/a | ≈ 14 |

---

## QA 2 (feat/native-ui @ 456012b, 21 capturas)

Método igual ao QA 1: capturas estáticas 560×176 (1×) contra mocks e spec; ampliei e medi alguns pixels.
**Ainda não avaliado:** animação, hover, clique, DPI ≠ 100%, tema claro, overlays do sistema (volume/bateria/Caps), olhos, configurações, vivo expandido com timer, tarefas com itens.

### Status dos itens do QA 1

| # | Item | Status |
|---|---|---|
| 1 | Gradiente vaza do canto arredondado | **Resolvido** (pixel do canto = fundo da tela) |
| 2 | "Limpar tudo" colide com indicador | **Resolvido** (termina antes do slot) |
| 3 | Shelf sem "Selecionar" | **Resolvido** (e modo Cancelar/Remover com checks) |
| 4 | Tarefas placeholder | **Parcial**: estado vazio feito; falta lista com itens (sem captura) |
| 5 | Chips "20 KB" fixo | **Resolvido** (1,8 MB / 305 KB / 2 KB / —) |
| 6 | Descendentes cortados no artista | **Resolvido** (descendentes de ~3 px visíveis) |
| 7 | Artista sem reticências | **Parcial**: texto atual cabe; o corte com "…" só aparece no vivo expandido ("thin…"). Falta teste com nome longo na aberta |
| 8 | Split sem pontos | **Aberto** (decisão pendente; nativo sem pontos, mock 04 com pontos). Recomendo tirar do mock |
| 9 | Split: título truncado | **Aberto** ("Never Gonna Give Yo…", artista sem álbum) |
| 10 | Balão desalinhado do tempo (split) | **Aberto** (pill do balão ~8 px à direita do tempo) |
| 11 | Letras distantes quase invisíveis | **Aberto** (1ª/última linha ainda ≈ 15%) |
| 12 | Bateria solta no cabeçalho | **Resolvido** (bateria · data · hora agrupadas) |
| 13 | Indicador ao vivo na hora | **Resolvido** (slot separado; no Pomodoro rodando ícone do timer + equalizador lado a lado) |
| 14 | Legenda "Personalizado 12:34" | **Aberto**: ver N1 |
| 15 | Play do Pomodoro sem compensação | **Parcial**: parece centrado agora, não medi |
| 19 | Gradiente escuro demais | **Parcial**: mais saturado e visível (≈ RGB 36,20,82 embaixo), mas só no terço inferior; L continua baixo |
| 20 | Peek | **Resolvido/ok** |
| 22 | Traço | **Resolvido/ok** |
| 23 | Capturas que faltavam | **Quase tudo entregue** (faltam overlays, olhos, config, tarefas com itens, vivo expandido com timer) |

### Novos achados

**P1**
- **N1. Pomodoro editando mantém a legenda antiga** (15): enquanto digita "20:00" a legenda ainda diz "Personalizado 12:34". O mock 12 mostra "Editando · Enter confirma · Esc cancela" no lugar da legenda. Também falta a caixa azul de foco nos dois campos MM e SS (nativo só tem texto coral com cursor).
- **N2. "Nada tocando" mostra capa colorida e controles ativos** (10): a capa continua o arco-íris da faixa anterior. O mock 08 usa capa placeholder escura com ícone `music`. Anterior/próximo e barra devem aparecer desabilitados (nativo: só o play está cinza). Sem tempo "00:00" = ok.

**P2**
- **N3. Volume substitui os controles** (09): ao abrir o slider (65 px, "73%") os botões anterior/play/próximo desaparecem. No mock 03 o slider fica ao lado do alto-falante e os controles continuam. Mover o slider para a direita (x≈ 340–395) sem esconder o transporte. O rótulo "73%" é um extra bom, mas ocupa espaço.
- **N4. Estado "sem letra" não foi capturado** (11): a captura mostra uma linha de letra normal ("We're no strangers to love"). Não consigo validar o mock 09 (linha some, título sozinho) nem "Sem letra" na tela dividida.
- **N5. Shelf arrastando: título fora do centro vertical** (18): "Solte para adicionar" fica no topo, com ícone mais baixo. Com 1 linha o texto deve centrar com o ícone. Glow azul bem contido, ok.
- **N6. Pílulas "Selecionar"/"Cancelar" com padding assimétrico** (19, 20): texto colado à esquerda e sobra à direita (~12 px). Centrar.
- **N7. Seleção no Shelf: indicador ao vivo e "Remover" a 20 px** (20): ok, só confirmar que o texto longo ("Remover 3") não cruza o slot.
- **N8. Tarefas: ponto azul do calendário ausente** (21): mock 17 tem um ponto de "hoje" no botão de calendário. Detalhe.
- **N9. Vivo expandido: sem botão de ocultar** (04): mock 26 tem ícone `eye-off` ao lado do equalizador. Também a capa é 36 px e o texto fica 2 px mais alto.

**P3**
- N10. Tarefas: painel "Sem tarefas." encosta a 8 px da seta direita; mock tem 10 px.
- N11. Fim do Pomodoro: legenda "Fim · ciclo 1" (spec: "Ciclo concluído"). Aceitável, só alinhar com o doc ou atualizar o doc.
- N12. Equalizador capturado em quadros diferentes (3 pontos, 3 barras) = animação ok; não avaliei fluidez.

### Melhor que o mock
- Header agrupado (bateria · data · hora) e indicador ao vivo coerente em todas as páginas.
- Anel verde + botão "Novo" no fim do Pomodoro: leitura imediata.
- Modo seleção do Shelf com checks amber e "Remover" em vermelho.
- Gradiente da capa agora respeita o raio e dá profundidade sem perder contraste do texto.

---

# QA 1 (histórico, feat/native-ui @ 965344d)

Base: 12 capturas `docs/native-shots/*.png` (560×176, 1×, 100% DPI) contra `docs/mocks/` (aprovados) e `DESIGN-NATIVO.md`.
Método: olhei cada captura ampliada (2×–6×) e medi pixels. Só vi imagens estáticas.
**Não avaliado:** animações, hover, clique, arrasto, DPI ≠ 100%, tema claro, fullscreen, overlays, olhos, configurações (sem captura).

Legenda: **P1** quebra o design · **P2** polimento · **P3** detalhe.

## P1

1. **Gradiente da capa vaza do canto arredondado** (04, 05, 06, 07). Os cantos inferiores da ilha mostram um bloco quadrado roxo (≈ RGB 23,14,49) fora do arco, sobre o fundo. O fundo do gradiente não está recortado pelo raio (22 px). Recortar o gradiente pela mesma forma da ilha (ou desenhar o gradiente dentro do layer/clip arredondado).
2. **Shelf com itens: "Limpar tudo" colide com o indicador ao vivo** (11). O texto vermelho termina em x≈466 e o equalizador ocupa (382–404+70…); os dois se sobrepõem. O slot do indicador é (382,12,22,16) e "Limpar tudo" tem de terminar em x ≤ 376 (alinhar à direita com 8 px de folga), ou o botão sobe de lugar quando houver indicador.
3. **Shelf com itens perdeu o cabeçalho do mock** (11 vs mock 16). Faltam o botão "Selecionar" ao lado do título; bateria, data e hora ficam ausentes (no mock 16 também não há, então só o "Selecionar" falta). Também não aparece o "×" de remover no hover (não dá para validar sem captura de hover).
4. **Tarefas é só placeholder** (12): "0/0 Concluídas hoje" e o texto de desenvolvedor "Em breve (Fase N2)". Faltam estado vazio (mock 17), lista com itens, checkbox, campo de adicionar (mock 18). Se for sair assim, trocar o texto por algo de produto ("Nenhuma tarefa ainda") e esconder a fase.
5. **Dados falsos nos chips da Shelf** (11): todos os arquivos mostram "20 KB" e nenhum tem tamanho real ou formato de pasta correto ("pasta" no lugar de "—" do mock). Se for dado de teste, tudo bem. Se for código, o tamanho está fixo.

## P2

6. **Descendentes cortados na linha do artista** (04, 05, 06): `y`, `g`, `d` perdem a parte de baixo em "Rick Astley — Whenever You Need Somebody". A caixa de texto está com altura menor que a linha (spec: 14 px de corpo em caixa de 14). Aumentar a caixa para ≥ 17 px ou não recortar vertical.
7. **Subtítulo sem reticências** (04): "Rick Astley — Whenever You Need Somebody" termina em x≈412 e encosta no equalizador (x≈462). Aplicar `…` com largura máxima 270 px (mock 01: artista — álbum limitado até o indicador).
8. **Tela dividida sem os pontos de paginação** (06, 07). O mock 04 tem os 4 pontos em y=128 também no split; o nativo não os desenha. Decidir: o spec §4.1.1 diz que somem, o mock mostra. Alinhar um dos dois (recomendo remover do mock, o nativo está limpo).
9. **Split: título truncado e artista sem álbum** (06): "Never Gonna Give Yo…" com 150 px útil. Aceitável, mas o mock dava 188 px. Conferir largura do painel esquerdo (spec ~250 com a capa).
10. **Split: coluna esquerda desalinhada**: tempo "00:38" em x≈38+… e o botão do balão (pill 24×24) ficam deslocados à direita do tempo (≈ 10 px). No mock, o botão fica alinhado ao tempo (x=38). Alinhar pela margem de 38.
11. **Linhas de letra distantes quase invisíveis** (06): a 1ª e a última linha têm ≈ 15% de opacidade. Spec: vizinhas 40%, fade progressivo. Contraste real da linha 2 (vizinha) ≈ 3,5:1 — ok; as de 2 posições estão abaixo de 3:1 (tolerado como decoração, mas o fade está mais forte que o mock).
12. **Cabeçalho Pomodoro: bateria no meio, não junto da data/hora** (08, 09). Mock 10: bateria colada à data. Nativo: ícone da bateria flutuando em x≈290 e data/hora à direita, com ≈ 100 px de vazio. Agrupar bateria · data · hora à direita com 8 px entre eles, e deixar a folga do indicador ao vivo.
13. **Indicador ao vivo toca o horário** (09): o ícone do timer (coral) e o equalizador ficam a 4 px da hora. Spec: slot próprio (382,12,22,16) e a hora termina em x≤372. Folga entre ícones de 6 px.
14. **Pomodoro rodando: legenda "Personalizado 12:34"** (09) com tempo 12:33 — conferir se é o valor inicial (spec: "Personalizado 20:00"). Está coerente se 12:34 for o total.
15. **Botão play do Pomodoro** (08): o ícone está deslocado ~1 px à esquerda dentro da pílula coral; o ícone play precisa de compensação óptica de +1 px à direita (já aplicado na música, não aqui).
## P3

19. **Gradiente da capa só no terço inferior** (04): em mock 02 o degradê começa em y≈40; nativo começa ≈ y 85. Está mais sutil; aceitável. Verificar a clamp de L (0,18–0,32): o roxo medido tem L≈0,12 (RGB 23,14,49) — escuro demais.
20. **Peek** (02): 150×30 ✓, bateria verde carregando + "20:54" ✓. Raio inferior 12 ✓. Sem diferença do mock 22.
22. **Traço** (01): 56×3 ✓, quase invisível sobre fundo escuro (esperado).
23. **Nenhuma captura de**: tarefas com itens, pomodoro fim/editando, shelf arrastando, volume slider, "nada tocando", sem letra, carregando letras, overlays, olhos, configurações, ao vivo expandido.

## O que está melhor que o mock

- **Texto do relógio e tempo da música** com `tnum`: dígitos não "dançam".
- **Linha atual da letra** na tela dividida tem mais contraste e peso 600 que o mock.
- **Ícones Lucide** nítidos em 1×.
- **Dock** com pílula ativa e separador antes da engrenagem leem melhor que a faixa do mock 19.
- **Pomodoro**: anel com arco de progresso e botão pause lado a lado leem com mais clareza.

## Medidas conferidas por inspeção visual (±2 px; não medi com régua em todas)

| Item | Spec | Nativo |
|---|---|---|
| Traço | 56×3 | 56×3 ✓ |
| Peek | 150×30 | 150×30 ✓ |
| Ao vivo compacto | 168×30 | 168×30 ✓ |
| Aberta | 420×136 | 420×136 ✓ |
| Aberta + dock | 420×164 | 420×164 ✓ |
| Split | 520×136 | 520×136 ✓ |
| Setas | 18×18 @ (3,66)/(399,66) | ✓ |
| Pontos | y=128 | ✓ (exceto split) |
| Cover | 56×56 | ≈ 56 ✓ |
