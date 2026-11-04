# Mocks para revisão (não são o app)

Imagens **estáticas** geradas de HTML local (`build_mocks.py`) só para revisão do design de `docs/DESIGN-NATIVO.md`. O app final é nativo (Direct2D); nada aqui será usado em produção. Coordenadas = px a 100% DPI, renderizadas a 3×, sobre um fundo cinza-azulado só para mostrar a silhueta. Estado: **PROPOSTA** salvo onde o `DESIGN-NATIVO.md` diz que vem da referência (só referência de comportamento; arte própria). **Água foi removida** (decisão do projeto): 4 páginas — Música, Pomodoro, Shelf, Tarefas.

Para regenerar: `python docs/mocks/build_mocks.py` (precisa do Microsoft Edge; usa `assets/icons`).

## Medidas usadas
Traço 56×3 · Peek 150×30 · Ao vivo compacto 168×30 (expandido 300×52) · Aberto **420×136** (**164** com dock) · Tela dividida **520×136** (164 com dock) · Overlay 280×48 · Configurações 440 de largura. Dock: alvo de clique ≥ **28 px** (36×28), a ilha cresce 28 px em 200 ms.

## Índice
| Arquivo | O que mostra |
|---|---|
| `01-musica-letra-1-linha.png` | Música: capa, título, **uma linha de letra** sob o título, progresso, controles, botão de balão (letras) e volume; indicador ao vivo (equalizador) no canto |
| `02-musica-gradiente-capa.png` | Música com **gradiente da cor da capa** |
| `03-musica-volume-slider.png` | Música com o **slider de volume** (~65 px) aberto ao passar no alto-falante |
| `04-musica-tela-dividida-letras.png` | **Tela dividida de letras** (520×136): player à esquerda, letras à direita (atual branca semibold, resto 40%, fade) |
| `05-musica-tela-dividida-carregando.png` | Tela dividida: estado "Carregando letras…" |
| `06-musica-tela-dividida-sem-letra.png` | Tela dividida: estado "Sem letra" |
| `07-musica-tela-dividida-dock.png` | Tela dividida com o **dock** visível (ilha cresce 28 px: 164) |
| `08-musica-vazio-nada-tocando.png` | Música vazia ("Nada tocando") |
| `09-musica-sem-letra.png` | Música sem letra (linha única ausente) |
| `10-pomodoro-parado.png` | Pomodoro parado (25:00) |
| `11-pomodoro-rodando-personalizado.png` | Pomodoro **rodando com valor personalizado** (arco de progresso, play vira pause, ícone ao vivo no canto) |
| `12-pomodoro-editando-valor.png` | Pomodoro **editando** o valor MM:SS |
| `13-pomodoro-fim.png` | Pomodoro **fim** (anel cheio verde) |
| `14-shelf-vazio.png` | Shelf vazio |
| `15-shelf-arrastando.png` | Shelf **arrastando** ("Solte para adicionar", vinhetas azuis) |
| `16-shelf-com-itens.png` | Shelf com itens (Selecionar / Limpar tudo) |
| `17-tarefas-vazio.png` | Tarefas vazio |
| `18-tarefas-com-itens.png` | Tarefas com itens |
| `19-dock-visivel-musica.png` | **Dock** visível na Música (ilha 420×164, alvo de clique 36×28) |
| `20-dock-visivel-pomodoro.png` | Dock visível no Pomodoro |
| `21-traco.png` | **Traço** (56×3) |
| `22-peek-bateria-hora.png` | **Peek** 150×30 (bateria + hora) |
| `23-ao-vivo-compacto-musica.png` | **Ao vivo compacto** 168×30 — música (equalizador) |
| `24-ao-vivo-compacto-timer.png` | Ao vivo compacto — timer |
| `25-ao-vivo-compacto-musica-e-timer.png` | Ao vivo compacto — música + timer |
| `26-ao-vivo-expandido-musica.png` | Ao vivo **expandido** (opção) — música |
| `27-ao-vivo-expandido-timer.png` | Ao vivo expandido — timer |
| `28-overlay-volume.png` | **Overlay** de volume (280×48) |
| `29-overlay-bateria.png` | Overlay de bateria |
| `30-overlay-caps-lock.png` | Overlay de Caps Lock |
| `31-olhos-ociosos.png` | **Olhos ociosos** (peek) |
| `32-olhos-piscando.png` | Olhos piscando |
| `33-configuracoes.png` | **Configurações** (janela 440, página inteira rolável) |
