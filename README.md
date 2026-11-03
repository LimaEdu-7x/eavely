<div align="center">

<img src="docs/img/logo.png" alt="Logo do Eavely" width="96" height="96">

# Eavely

**Um notch para o seu Windows: música com letras, pomodoro, shelf de arquivos e tarefas, num app minúsculo e 100% nativo.**

[![Baixar para Windows](https://img.shields.io/badge/%E2%AC%87%20Baixar-Eavely%20para%20Windows-4FA3F7?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/LimaEdu-7x/eavely/releases/latest)

![Licença MIT](https://img.shields.io/badge/licen%C3%A7a-MIT-green)
![Windows 10/11](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?logo=windows&logoColor=white)
![Tamanho](https://img.shields.io/badge/tamanho-~0%2C8%20MB-blue)
![Sem WebView](https://img.shields.io/badge/100%25%20nativo-Rust%20%2B%20Direct2D-orange)

[English](README.en.md) · [Recursos](#-recursos) · [Instalação](#-instalação) · [Atalhos](#atalhos) · [Privacidade](#-privacidade) · [Compilar](#compilar-do-código)

<br>

<img src="docs/img/musica.png" alt="Eavely tocando música" width="440">

</div>

---

## ✨ Recursos

| | |
|---|---|
| 🎵 **Música + letras sincronizadas** | Mostra a faixa do player que estiver tocando, com capa, barra de progresso, volume e letras linha a linha (tela dividida ou barra compacta). |
| 🍅 **Pomodoro** | Foco, pausa curta e tempo personalizado, com anel de progresso e aviso ao terminar. |
| 📥 **Shelf de arquivos** | Arraste arquivos para a ilha e solte onde precisar. Seleção, exclusão e "limpar tudo". |
| ✅ **Tarefas** | Lista simples com horário, calendário e edição. |
| 🔔 **Avisos do sistema** | Volume, brilho (só em notebook), bateria, Wi-Fi, fones e Caps Lock aparecem (desligados por padrão; ligue em *Configurações → Avisos do sistema*) como pequenos overlays na ilha. |
| 🖥️ **Compacto de PC** | Hora e, rotativamente, CPU, RAM, GPU e rede, sem abrir nada. |
| 👀 **Olhos / pet** | A ilha ociosa pisca e dorme. Detalhe pequeno, mas dá vida. |
| 💤 **~0% de CPU parado** | Em repouso o app não desenha nada: 0,000 s de CPU em 30 s de teste. |
| ⚡ **Desempenho ajustável** | Presets Máxima fluidez, Equilibrado e Economia, taxa de quadros, animações e efeitos. |
| 🎮 **Esconde em tela cheia** | Jogo ou vídeo em tela cheia: a ilha some sozinha. |
| 🧱 **100% nativo, sem WebView** | Rust + Direct2D/DirectComposition/DirectWrite. Sem Electron, sem navegador embutido. |
| 🔒 **Dados locais** | Tarefas, Shelf e configurações ficam no seu PC. Sem conta, sem telemetria. |

## 🖼️ Capturas

<table>
<tr>
<td align="center"><img src="docs/img/letras.png" alt="Letras sincronizadas"><br><sub>Letras sincronizadas</sub></td>
<td align="center"><img src="docs/img/pomodoro.png" alt="Pomodoro"><br><sub>Pomodoro</sub></td>
</tr>
<tr>
<td align="center"><img src="docs/img/shelf.png" alt="Shelf de arquivos"><br><sub>Shelf de arquivos</sub></td>
<td align="center"><img src="docs/img/tarefas.png" alt="Tarefas"><br><sub>Tarefas</sub></td>
</tr>
</table>

<p align="center">
<img src="docs/img/avisos-e-compacto.png" alt="Overlays do sistema, olhos e compacto de PC" width="640"><br>
<sub>Overlays (volume, brilho, bateria, fones, Caps Lock, Wi-Fi), olhos e compacto de PC</sub>
</p>

<table>
<tr>
<td align="center"><img src="docs/img/config-desempenho.png" alt="Configurações: Desempenho"><br><sub>Desempenho</sub></td>
<td align="center"><img src="docs/img/config-musica-letras.png" alt="Configurações: Música e letras"><br><sub>Música e letras</sub></td>
</tr>
<tr>
<td align="center" colspan="2"><img src="docs/img/config-ilha-aparencia.png" alt="Configurações: Ilha e aparência" width="440"><br><sub>Ilha e aparência</sub></td>
</tr>
</table>

## 📦 Instalação

1. Baixe o arquivo mais recente em **[Releases](https://github.com/LimaEdu-7x/eavely/releases/latest)**.
2. Escolha um formato: **`Eavely-Setup.exe`** (instalador NSIS, instala só para o seu usuário) ou **`Eavely.exe`** (portátil, basta dar dois cliques). `Eavely.exe` tem ~0,8 MB (821.760 bytes) e `Eavely-Setup.exe` ~417 KB (427.041 bytes).
3. O ícone aparece na bandeja. Pelo menu dele você abre as Configurações e sai do app.

### ⚠️ Aviso do Windows (SmartScreen / Smart App Control)

O Eavely **ainda não tem assinatura de código** (certificado custa caro e é um projeto pessoal). Por isso o Windows desconfia de qualquer `.exe` novo e pouco baixado:

- **SmartScreen** ("O Windows protegeu seu PC"): clique em **Mais informações → Executar assim mesmo**.
- **Smart App Control** (Windows 11, quando está ativo): pode **bloquear sem opção de executar**. Nesse caso, ou você compila o app você mesmo, ou desativa o Smart App Control em *Segurança do Windows → Controle de aplicativos e do navegador*. **Atenção:** desativar é definitivo (só volta a ligar reinstalando o Windows) e reduz a proteção do sistema. A decisão é sua.
- Quer conferir o que está rodando? O código é aberto e dá para compilar do zero (veja abaixo).

## Atalhos

| Atalho | Ação |
|---|---|
| `Ctrl` + `Alt` + `E` | Mostrar / esconder a ilha |
| `Ctrl` + `Alt` + `T` | Iniciar / pausar o timer |
| `Ctrl` + `Alt` + `F` | Abrir a Shelf |

Dá para mudar os três em *Configurações → Atalhos*. É exigido Ctrl, Alt ou Win junto da tecla (nunca uma tecla solta). Os atalhos são registrados pelo Windows; o app não monitora o teclado em segundo plano.

## ⚙️ Configurações

Janela com barra lateral e cartões agrupados: **Geral**, **Ilha e aparência** (tamanho, abrir por hover ou clique, velocidade da animação, compacto de PC), **Música e letras**, **Tarefas**, **Pomodoro**, **Shelf**, **Avisos do sistema**, **Atalhos**, **Desempenho** e **Sobre**.

A página **Desempenho** tem presets (**Máxima fluidez**, **Equilibrado**, **Economia**; mexer em um controle vira "personalizado"), taxa de quadros **Auto/60/30**, nível das animações, rolagem das letras (**suave** ou **por linha**, que poupa CPU), efeitos da capa, equalizador e a opção de respeitar as animações do Windows.

## 🔐 Privacidade

- **Sem telemetria, sem conta, sem anúncios.**
- A **única conexão de rede** é a busca de letras no [LRCLIB](https://lrclib.net): são enviados o **título, o artista e o álbum** da faixa tocando. Dá para desligar em *Configurações → Música e letras → Mostrar letras*; o cache local continua valendo.
- Métricas de CPU, RAM, GPU e rede do compacto de PC são lidas **localmente** e nunca saem do computador.
- Seus dados ficam em `%APPDATA%\Eavely` (tarefas, Shelf, configurações e cache de letras). Apagar a pasta zera tudo.

## 📊 Desempenho

Os números de RAM do projeto são **memória privada** do processo (`PrivateMemorySize64`), não o *working set*. O working set oscila porque o Windows o corta e devolve páginas (o app libera o renderer ao recolher), então ele não mostra o que o processo reserva de fato. Por isso a memória privada é a medida honesta:

| Estado | Memória privada |
|---|---|
| Parado (recolhido), memória privada | **5,6 MB** |
| Pico de memória privada, ilha aberta e animando | ~50 MB |
| Working set parado (não é a medida honesta) | ~36 MB |

CPU parado: **0,000 s em 30 s** (~0%). Medido na v0.1.9 (exe de release), em uma máquina só (Windows 11); em outras pode variar. Método e dados em [docs/perf/RESULTADOS.md](docs/perf/RESULTADOS.md).

## Compilar do código

Precisa do [Rust](https://rustup.rs) (toolchain MSVC) no Windows 10/11.

```powershell
git clone https://github.com/LimaEdu-7x/eavely.git
cd eavely\app
cargo build --release
# resultado: app\target\release\eavely.exe
```

O diretório `app/` está na branch de desenvolvimento enquanto o app nativo não é integrado à `main`.

## 📄 Licença e créditos

- Código sob licença **[MIT](LICENSE)**.
- Ícones: [Lucide](https://lucide.dev) (ISC). Detalhes em [THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md).
- Letras: [LRCLIB](https://lrclib.net), serviço aberto e gratuito. As letras pertencem aos seus autores.

## ℹ️ Projeto independente

O Eavely é um projeto **independente e feito por uma pessoa só**. **Não é afiliado, patrocinado ou endossado** por nenhum outro app de ilha/notch. Escrito do zero, inspirado no comportamento de apps de notch; arte e ícones são originais ou open source (Lucide, ISC). Nomes de marcas pertencem aos seus donos.
