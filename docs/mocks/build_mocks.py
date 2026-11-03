#!/usr/bin/env python3
"""Gera os mocks estaticos (HTML -> PNG via Edge headless). SO PARA REVISAO; nao faz parte do app.
Uso: python docs/mocks/build_mocks.py   (precisa do Microsoft Edge). Coordenadas = DESIGN-NATIVO.md (px a 100% DPI, render a 3x)."""
import os, re, subprocess, sys, html as H
ROOT = os.path.dirname(os.path.abspath(__file__))
ICONS = os.path.join(ROOT, "..", "..", "assets", "icons")
OUT = ROOT
TMP = os.path.join(os.environ.get("TEMP", "."), "eavely-mocks"); os.makedirs(TMP, exist_ok=True)
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

# tokens (DESIGN-NATIVO §1)
T2 = "rgba(255,255,255,.64)"; T3 = "rgba(255,255,255,.52)"; SURF = "rgba(255,255,255,.08)"; SURF_H = "rgba(255,255,255,.14)"; LINE = "rgba(255,255,255,.14)"
ACC = "#7CC4FF"; OKC = "#7BE0A4"; DANGER = "#FF6B6B"; AMBER = "#FFB86B"
C_MUSIC, C_POMO, C_WATER, C_SHELF, C_TODO = "#B69CFF", "#FF7A66", "#5CC8FF", "#FFC857", "#5EE0B0"
H_OPEN = 136; H_DOCK = 164; W_OPEN = 420

def icon(name, size=16, color="#fff", x=0, y=0, fill=False, sw=2):
    s = open(os.path.join(ICONS, name + ".svg"), encoding="utf8").read()
    inner = re.search(r"<svg[^>]*>(.*)</svg>", s, re.S).group(1)
    f = color if fill else "none"
    return (f'<svg class="a" style="left:{x}px;top:{y}px" width="{size}" height="{size}" viewBox="0 0 24 24" fill="{f}" stroke="{color}" '
            f'stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round">{inner}</svg>')

def box(x, y, w, h, inner="", style=""):
    return f'<div class="a" style="left:{x}px;top:{y}px;width:{w}px;height:{h}px;{style}">{inner}</div>'

def text(x, y, w, h, s, size=12, weight=400, color="#fff", align="left", extra=""):
    return box(x, y, w, h, H.escape(s) if not s.startswith("<") else s,
               f"font-size:{size}px;line-height:{h}px;font-weight:{weight};color:{color};text-align:{align};white-space:nowrap;overflow:hidden;text-overflow:ellipsis;{extra}")

def pill(x, y, w, h, bg=SURF, inner="", extra=""):
    return box(x, y, w, h, inner, f"background:{bg};border-radius:999px;{extra}")

def center(w, h, inner):
    return f'<div style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center;gap:5px;">{inner}</div>'

COVER = ("linear-gradient(135deg,#2a6fb0 0%,#0d3a5c 55%,#06223a 100%)")

def cover(x, y, s, r):
    return box(x, y, s, s, '<svg width="100%" height="100%" viewBox="0 0 64 64"><circle cx="32" cy="32" r="14" fill="none" stroke="rgba(255,255,255,.35)" stroke-width="4"/></svg>',
               f"background:{COVER};border-radius:{r}px;box-shadow:inset 0 0 0 1px {LINE};")

def island(w, h, content, rbot=28, bg="#000", flares=True, extra=""):
    left = (SCENE_W - w) // 2
    fl = ""
    if flares:  # flares concavos do topo (r=6), PROPOSTA
        fl = (f'<svg class="a" style="left:{left-6}px;top:0" width="6" height="6"><path d="M0 0H6V6A6 6 0 0 0 0 0Z" fill="{bg}"/></svg>'
              f'<svg class="a" style="left:{left+w}px;top:0" width="6" height="6"><path d="M6 0H0V6A6 6 0 0 1 6 0Z" fill="{bg}"/></svg>')
    return fl + f'<div class="a isl" style="left:{left}px;top:0;width:{w}px;height:{h}px;border-radius:0 0 {rbot}px {rbot}px;background:{bg};{extra}">{content}</div>'

SCENE_W = 560

def page_html(body, scene_h=190, title=""):
    return f"""<!doctype html><meta charset=utf-8><title>{title}</title>
<style>
html,body{{margin:0;background:#2b3548;overflow:hidden}}
body{{width:{SCENE_W}px;height:{scene_h}px;position:relative;font:12px/16px "Segoe UI Variable Text","Segoe UI",sans-serif;color:#fff;-webkit-font-smoothing:antialiased;
 background:radial-gradient(120% 140% at 20% 0%,#51617f 0%,#2b3548 55%,#1c2230 100%)}}
.a{{position:absolute;box-sizing:border-box}}
.isl{{overflow:hidden}}
.tab{{font-variant-numeric:tabular-nums}}
</style><body>{body}</body>"""


def eq_icon(x, y, color=C_MUSIC, phase=0.0, paused=False):
    """Equalizador original: 3 barras de 3 px, intervalo 2 px, alturas 4-14 px."""
    import math
    out = ""
    for i, per in enumerate((0.9, 1.15, 1.4)):
        h = 4 if paused else 4 + 10 * (0.5 + 0.5 * math.sin(2 * math.pi * (phase / per + i * 0.33)))
        out += box(x + i * 5, y + 14 - h, 3, h, "", f"background:{T3 if paused else color};border-radius:2px")
    return out

def live_slot(music=True, timer=False):
    out = ""
    if music: out += eq_icon(387, 14, phase=0.37)
    if timer: out += icon("timer", 14, C_POMO, 365 if music else 387, 14)
    return out

# ------------------------------------------------------------------ moldura
def clock_group(color_batt=OKC, charging=True):
    return (icon("battery-charging" if charging else "battery", 16, color_batt, 212, 14) +
            text(234, 14, 92, 16, "seg 2 nov", 12, 400, T2, "right") +
            text(330, 14, 46, 16, "19:41", 12, 700, "#fff", "right", "font-variant-numeric:tabular-nums"))

def arrows():
    return (pill(3, 66, 18, 18, SURF, icon("chevron-left", 12, T2, 3, 3)) + pill(399, 66, 18, 18, SURF, icon("chevron-right", 12, T2, 3, 3)))

def pager(active, dock=False, colors=(C_MUSIC, C_POMO, C_SHELF, C_TODO), cx=210):
    if not dock:
        out = ""
        x = cx - (3 * 10 + 14) / 2
        for i in range(4):
            if i == active:
                out += box(x, 126.5, 14, 3, "", "background:#fff;border-radius:2px"); x += 24
            else:
                out += box(x, 126.5, 3, 3, "", f"background:{T3};border-radius:2px"); x += 13
        return out
    names = ["music", "timer", "inbox", "list-checks", None, "settings"]
    out = ''
    x = cx - 96 + 0
    for i, n in enumerate(names):
        if n is None:
            out += box(x + 6, 144, 1, 12, "", f"background:{LINE}"); x += 12; continue
        act = (i == active)
        col = colors[i] if act else T2
        if act:
            out += box(x + 2, 139, 32, 22, "", f"background:{col}29;border-radius:9px")
        out += icon(n, 16, col, x + 10, 142)
        x += 36
    return out

def frame(title, color, active, dock=False, header_right=True, extra_header=""):
    hdr = text(24, 12, 208, 20, title, 14, 600, color)
    if header_right: hdr += clock_group()
    return hdr + extra_header + arrows() + pager(active, dock)

def wrap_open(content, bg="#000", extra="", scene_h=190, title="", dock=False, live=None):
    live = live_slot(**live) if live else ""
    return page_html(island(W_OPEN, H_DOCK if dock else H_OPEN, content + live, 28, bg, extra=extra), scene_h, title)

# ------------------------------------------------------------------ paginas
def p_music(lyric="Seu nome no letreiro azul", playing=False, dock=False, empty=False, tint=True, vol=False):
    c = ""
    if empty:
        c += box(24, 12, 62, 62, icon("music", 24, T3, 19, 19), f"background:{SURF};border-radius:10px;box-shadow:inset 0 0 0 1px {LINE}")
        c += text(98, 12, 296, 20, "Nada tocando", 15, 600, T2) + text(98, 33, 296, 14, "Abra um player de música", 11, 400, T2)
        elapsed, rest, frac = "00:00", "-00:00", 0
    else:
        c += cover(24, 12, 62, 10)
        c += text(98, 12, 280, 20, "Luz de Neon", 15, 600) + text(98, 33, 280, 14, "Maré Alta — Noites (Edição Deluxe)", 11, 400, T2)
        c += text(98, 52, 280, 18, lyric, 12, 500, "#fff") if lyric else ""
        elapsed, rest, frac = "01:26", "-02:46", 0.34
    c += text(24, 78, 44, 14, elapsed, 12, 400, "#fff", "left", "font-variant-numeric:tabular-nums")
    c += box(72, 83, 276, 4, "", f"background:{SURF_H};border-radius:2px") + box(72, 83, 276 * frac, 4, "", "background:#fff;border-radius:2px")
    c += text(352, 78, 44, 14, rest, 12, 400, "#fff", "right", "font-variant-numeric:tabular-nums")
    dis = "opacity:.35;" if empty else ""
    c += box(0, 0, 0, 0, "", "") + icon("message-square-quote", 16, T2 if not empty else T3, 28, 96)
    c += box(168, 91, 26, 26, icon("skip-back", 16, "#fff", 5, 5, True), dis)
    c += box(200, 88, 36, 32, icon("pause" if playing else "play", 18, "#000", 9, 7, True), f"background:#fff;border-radius:999px;{dis}")
    c += box(242, 91, 26, 26, icon("skip-forward", 16, "#fff", 5, 5, True), dis)
    if vol:
        c += box(300, 102, 65, 4, "", f"background:{SURF_H};border-radius:2px") + box(300, 102, 65 * .7, 4, "", "background:#fff;border-radius:2px")
    c += icon("volume-2", 16, "#fff" if vol else T2, 376, 96)
    c += pager(0, dock) + arrows()
    return c

def p_pomodoro(state="idle", dock=False, edit=False):
    c = frame("Pomodoro", C_POMO, 1, dock)
    frac = {"idle": 1.0, "run": 0.62, "done": 1.0}[state]
    col = OKC if state == "done" else C_POMO
    circ = 2 * 3.14159 * 32
    c += box(24, 44, 72, 72, f'<svg width="72" height="72" viewBox="0 0 72 72" style="transform:rotate(-90deg);filter:drop-shadow(0 0 8px {col}59)">'
             f'<circle cx="36" cy="36" r="32" fill="none" stroke="{SURF_H}" stroke-width="5"/>'
             f'<circle cx="36" cy="36" r="32" fill="none" stroke="{col}" stroke-width="5" stroke-linecap="round" stroke-dasharray="{circ:.2f}" stroke-dashoffset="{circ*(1-frac):.2f}"/></svg>'
             + icon("timer", 28, col, 22, 22))
    if edit:
        fld = lambda x, s: box(x, 46, 52, 34, text(0, 0, 52, 34, s, 26, 700, "#fff", "center", "font-variant-numeric:tabular-nums"), f"background:{SURF};border-radius:10px;box-shadow:0 0 0 1.5px {ACC}")
        c += fld(112, "20") + text(166, 46, 10, 34, ":", 26, 700, T2, "center") + fld(178, "00")
        c += text(112, 74, 200, 14, "Editando · Enter confirma · Esc cancela", 11, 400, T2)
    else:
        tm = {"idle": "25:00", "run": "18:30", "done": "00:00"}[state]
        c += text(112, 40, 150, 34, "Fim" if state == "done" else tm, 30, 700, OKC if state == "done" else "#fff", "left", "font-variant-numeric:tabular-nums;line-height:34px")
        leg = {"idle": "Foco", "run": "Personalizado 20:00 · ciclo 2", "done": "Ciclo concluído"}[state]
        c += text(112, 74, 170, 14, leg, 12, 400, T2)
    c += pill(268, 50, 24, 24, SURF, icon("rotate-ccw", 14, "#fff", 5, 5))
    c += pill(112, 92, 108, 26, SURF, icon("coffee", 14, "#fff", 12, 6) + text(32, 5, 72, 16, "Pausa curta", 11, 400))
    play = icon("play" if state != "run" else "pause", 16, "#fff", 23, 7, True)
    c += pill(226, 90, 62, 30, C_POMO, play) if state != "done" else pill(226, 90, 62, 30, C_POMO, text(0, 7, 62, 16, "Novo ciclo", 10.5, 600, "#fff", "center"))
    c += pill(294, 92, 100, 26, SURF, icon("target", 14, "#fff", 12, 6) + text(32, 5, 64, 16, "Foco", 11, 400))
    return c

def p_shelf(mode="empty", dock=False):
    right = clock_group()
    title = text(24, 12, 60, 20, "Shelf", 14, 600, C_SHELF)
    if mode == "items":
        right = text(320, 14, 76, 16, "Limpar tudo", 11, 400, DANGER, "right")
        title += pill(72, 11, 78, 22, SURF, text(0, 3, 78, 16, "Selecionar", 11, 400, "#fff", "center"))
    c = title + right + arrows() + pager(2, dock and mode != "drag")
    if mode in ("empty", "drag"):
        drag = mode == "drag"
        bcol = ACC if drag else "rgba(255,255,255,.35)"
        c += box(40, 38, 340, 80, "", f"border:1.5px dashed {bcol};border-radius:16px")
        c += icon("inbox", 24, ACC if drag else T3, 60, 66)
        c += text(96, 62, 270, 16, "Solte para adicionar" if drag else "Solte arquivos aqui", 13, 600, ACC if drag else "#fff")
        if not drag: c += text(96, 80, 270, 28, "Arraste para fora quando quiser · clique para abrir ou copiar", 10.5, 400, T2, "left", "white-space:normal;line-height:14px")
    else:
        items = [("PDF", "Relatorio-Q3.pdf", "1,2 MB"), ("TXT", "notas.txt", "306 B"), ("PNG", "foto-praia.png", "840 KB"), ("DIR", "Projetos", "—"), ("DOC", "contrato.docx", "96 KB")]
        for n, (k, nm, sz) in enumerate(items):
            x = 24 + n * 72
            if x > 380: break
            bg = SURF_H if n == 1 else SURF
            card = (icon("folder" if k == "DIR" else "file", 22, C_SHELF, 21, 8) + text(6, 44, 52, 14, nm, 10.5, 400) + text(6, 59, 52, 12, sz, 10, 400, T3))
            if n == 1: card += pill(46, 4, 14, 14, "rgba(255,255,255,.2)", icon("x", 9, "#fff", 2.5, 2.5))
            c += box(x, 40, 64, 78, card, f"background:{bg};border-radius:10px")
        c += box(370, 40, 30, 78, "", "background:linear-gradient(90deg,transparent,#000)")
    return c

def p_todo(items=False, dock=False):
    c = frame("Tarefas", C_TODO, 3, dock)
    c += text(24, 38, 90, 38, "1/4" if items else "0/0", 30, 700, "#fff", "left", "line-height:38px")
    c += text(24, 78, 120, 14, "Concluídas hoje", 11, 400, T2)
    c += pill(112, 46, 26, 26, SURF, icon("calendar", 14, "#fff", 6, 6) + box(18, 1, 6, 6, "", f"background:{ACC};border-radius:50%;box-shadow:0 0 0 2px #000"))
    c += pill(24, 98, 104, 22, SURF, icon("circle-plus", 14, "#fff", 10, 4) + text(30, 3, 70, 16, "Adicionar", 11, 400))
    c += pill(134, 98, 84, 22, SURF, icon("pencil", 14, "#fff", 10, 4) + text(30, 3, 50, 16, "Editar", 11, 400), "" if items else "opacity:.4;")
    panel = ""
    if items:
        rows = [("Responder e-mails", True), ("Revisar o relatório", False), ("Comprar pão", False)]
        for i, (t, dn) in enumerate(rows):
            y = 4 + i * 24
            panel += icon("circle-check" if dn else "circle", 14, C_TODO if dn else T2, 10, y + 5)
            panel += text(32, y + 4, 134, 16, t, 12, 400, T3 if dn else "#fff", "left", "text-decoration:line-through;" if dn else "")
        panel += text(10, 76, 150, 0, "", 1)
    else:
        panel += icon("list-checks", 20, T3, 77, 18) + text(0, 44, 174, 16, "Sem tarefas.", 11, 400, T2, "center")
    c += box(222, 40, 174, 78, panel, "background:rgba(255,255,255,.05);border-radius:14px")
    return c


LYR = ["Acendo a cidade devagar", "Cada rua guarda um verso", "Seu nome no letreiro azul", "A noite não pede licença", "Eu danço sem olhar pra trás", "O mundo gira em câmera lenta", "Deixa a luz me atravessar"]

def p_music_split(lyrics=True, dock=False, mode="synced"):
    """Tela dividida 520x136 (DESIGN-NATIVO 4.1.1)."""
    c = box(250, 16, 1, 104, "", f"background:{LINE}")
    c += cover(20, 14, 56, 9)
    c += text(88, 14, 148, 18, "Luz de Neon", 14, 600) + text(88, 34, 148, 14, "Maré Alta", 11, 400, T2)
    c += text(18, 76, 34, 14, "01:26", 12, 400, "#fff", "left", "font-variant-numeric:tabular-nums")
    c += box(56, 81, 140, 4, "", f"background:{SURF_H};border-radius:2px") + box(56, 81, 140 * .34, 4, "", "background:#fff;border-radius:2px")
    c += text(200, 76, 40, 14, "-02:46", 12, 400, "#fff", "right", "font-variant-numeric:tabular-nums")
    c += box(20, 92, 24, 24, icon("message-square-quote", 16, "#fff", 4, 4), "background:rgba(255,255,255,.16);border-radius:8px")
    c += box(72, 91, 26, 26, icon("skip-back", 16, "#fff", 5, 5, True))
    c += box(104, 88, 36, 32, icon("play", 18, "#000", 9, 7, True), "background:#fff;border-radius:999px")
    c += box(146, 91, 26, 26, icon("skip-forward", 16, "#fff", 5, 5, True))
    if lyrics and mode == "synced":
        inner = ""
        for k, d in enumerate([-2, -1, 0, 1, 2]):
            y = 45 + 22 * d
            cur = d == 0
            inner += text(0, y, 236, 18, LYR[k + 1], 14 if cur else 13, 600 if cur else 400, "#fff" if cur else "rgba(255,255,255,.4)")
        inner = box(0, 0, 236, 108, inner, "-webkit-mask-image:linear-gradient(transparent,#000 28px,#000 80px,transparent);mask-image:linear-gradient(transparent,#000 28px,#000 80px,transparent)")
        c += box(268, 14, 236, 108, inner)
    elif mode == "loading":
        c += box(268, 14, 236, 108, text(0, 46, 236, 16, "Carregando letras…", 12, 400, T2, "center"))
    else:
        c += box(268, 14, 236, 108, icon("message-square-quote", 20, T3, 108, 34) + text(0, 60, 236, 16, "Sem letra para esta faixa", 12, 400, T2, "center"))
    c += pager(0, dock, cx=260)
    return c

def scene_split(c, tint=True, dock=False):
    bg = "linear-gradient(180deg,#000 0%,#000 35%,rgba(16,70,110,.8) 100%)" if tint else "#000"
    return page_html(island(520, H_DOCK if dock else H_OPEN, c, 28, bg), 210, "split")

# ------------------------------------------------------------------ estados
def scene_music(**k): return wrap_open(p_music(**k), "#000")

def tinted(content):  # gradiente da capa (DESIGN §6)
    return wrap_open(content, "linear-gradient(180deg,#000 0%,#000 35%,rgba(16,70,110,.8) 100%)", live=dict(music=True))

def trace():
    return page_html(box((SCENE_W - 56) // 2, 0, 56, 3, "", "background:#000;border-radius:0 0 2px 2px;box-shadow:inset 0 -1px 0 rgba(255,255,255,.22);opacity:.85"), 40, "traco")

def peek(eyes=False, blink=False, look=0):
    w, h = 150, 30
    left = (SCENE_W - w) // 2
    if eyes:
        ry = 1.2 if blink else 6
        inner = ""
        for cx in (w / 2 - 9, w / 2 + 9):
            inner += f'<ellipse cx="{cx}" cy="15" rx="4" ry="{ry}" fill="#fff"/>'
            if not blink: inner += f'<circle cx="{cx+look}" cy="{15+1}" r="2.5" fill="#000"/>'
        inner = f'<svg class="a" width="150" height="30">{inner}</svg>'
    else:
        inner = icon("battery-charging", 16, OKC, 16, 7) + text(88, 5, 46, 20, "19:41", 14, 600, "#fff", "right", "font-variant-numeric:tabular-nums;line-height:20px")
    return page_html(island(w, h, inner, 14), 60, "peek")


def live_compact(kind="music", both=False):
    w = 190 if both else 168
    c = icon("battery-charging", 16, OKC, 14, 7) + text(40, 5, 56, 20, "19:41", 14, 600, "#fff", "left", "font-variant-numeric:tabular-nums;line-height:20px")
    if kind == "music" or both: c += eq_icon(w - 14 - 13 - (22 if both else 0), 8, phase=0.37)
    if kind == "timer" or both: c += icon("timer", 16, C_POMO, w - 14 - 16, 7)
    return page_html(island(w, 30, c, 14), 60, "ao vivo compacto")

def live_music():
    c = cover(8, 8, 36, 8) + text(54, 7, 190, 18, "Luz de Neon", 12, 600) + text(54, 26, 190, 16, "Seu nome no letreiro azul", 12, 400, T2)
    c += eq_icon(254, 18)
    c += icon("eye-off", 16, T3, 274, 18)
    return page_html(island(300, 52, c, 22), 80, "ao vivo musica")

def live_timer():
    circ = 2 * 3.14159 * 14
    c = box(10, 10, 32, 32, f'<svg width="32" height="32" viewBox="0 0 32 32" style="transform:rotate(-90deg)"><circle cx="16" cy="16" r="14" fill="none" stroke="{SURF_H}" stroke-width="3"/>'
            f'<circle cx="16" cy="16" r="14" fill="none" stroke="{C_POMO}" stroke-width="3" stroke-linecap="round" stroke-dasharray="{circ:.1f}" stroke-dashoffset="{circ*.38:.1f}"/></svg>')
    c += text(52, 8, 120, 18, "18:30", 14, 600, "#fff", "left", "font-variant-numeric:tabular-nums") + text(52, 28, 160, 14, "Foco · ciclo 2", 11, 400, T2)
    c += icon("eye-off", 16, T3, 274, 18)
    return page_html(island(300, 52, c, 22), 80, "ao vivo timer")

def overlay(kind):
    c = ""
    if kind == "volume":
        c = icon("volume-2", 20, ACC, 16, 14) + box(48, 21, 168, 6, "", f"background:{SURF_H};border-radius:3px") + box(48, 21, 168 * .64, 6, "", f"background:{ACC};border-radius:3px")
        c += text(224, 17, 40, 14, "64%", 12, 400, "#fff", "right", "font-variant-numeric:tabular-nums")
    elif kind == "battery":
        c = icon("battery-low", 20, AMBER, 16, 14) + text(48, 8, 200, 16, "Bateria fraca", 13, 600) + text(48, 26, 200, 14, "19% · conecte o carregador", 11, 400, T2)
    elif kind == "caps":
        c = icon("arrow-big-up", 20, "#fff", 16, 14, True) + text(48, 15, 220, 18, "Caps Lock ativado", 13, 600)
    return page_html(island(280, 48, c, 22), 80, "overlay " + kind)

def dock_scene(active=0):
    return wrap_open(p_music(dock=True) if active == 0 else p_pomodoro(dock=True), "#000")

# ------------------------------------------------------------------ configuracoes
def settings():
    W = 440
    css = f"""<!doctype html><meta charset=utf-8><style>
html,body{{margin:0;background:#121316;color:#fff;font:13px/18px "Segoe UI Variable Text","Segoe UI",sans-serif;-webkit-font-smoothing:antialiased}}
.wrap{{width:{W}px;padding:0 20px 24px;box-sizing:border-box}}
h1{{font-size:16px;line-height:24px;margin:0;padding:16px 0 12px;font-weight:600}}
h2{{font-size:10.5px;letter-spacing:.6px;text-transform:uppercase;color:{T3};margin:20px 4px 6px;font-weight:600}}
.card{{background:#1A1C20;border-radius:14px}}
.row{{display:flex;align-items:center;justify-content:space-between;min-height:48px;padding:0 16px;box-sizing:border-box;border-top:1px solid {LINE}}}
.row:first-child{{border-top:0}}
.row small{{display:block;font-size:11px;color:{T2};line-height:14px}}
.l{{padding:7px 0}}
.tg{{width:40px;height:22px;border-radius:11px;background:{SURF_H};position:relative;flex:none}}
.tg::after{{content:"";position:absolute;top:3px;left:3px;width:16px;height:16px;border-radius:50%;background:#fff}}
.tg.on{{background:{ACC}}}.tg.on::after{{left:21px;background:#000}}
.sel{{width:168px;height:28px;border-radius:8px;background:{SURF};display:flex;align-items:center;padding:0 10px;box-sizing:border-box;justify-content:space-between;font-size:12px}}
.st{{width:104px;height:28px;display:flex;align-items:center;justify-content:space-between;font-size:12px;font-weight:600}}
.st b{{width:28px;height:28px;border-radius:8px;background:{SURF};display:grid;place-items:center;font-weight:400;font-size:15px}}
.key{{width:148px;height:28px;border-radius:8px;background:{SURF};display:flex;align-items:center;justify-content:center;font-size:12px}}
.seg{{display:flex;background:{SURF};border-radius:999px;padding:2px;font-size:11px}}.seg span{{padding:4px 10px;border-radius:999px;color:{T2}}}.seg .a1{{background:#fff;color:#000;font-weight:600}}
.btn{{height:30px;border-radius:999px;background:{SURF_H};padding:0 14px;display:inline-flex;align-items:center;font-size:12px;font-weight:600;margin-right:8px}}
.num{{font-variant-numeric:tabular-nums}}
</style>"""
    tg = lambda on: f'<div class="tg{" on" if on else ""}"></div>'
    row = lambda label, ctl, desc="": f'<div class="row"><div class="l">{label}{"<small>"+desc+"</small>" if desc else ""}</div>{ctl}</div>'
    st = lambda v: f'<div class="st"><b>−</b><span class="num">{v}</span><b>+</b></div>'
    sel = lambda v: f'<div class="sel">{v}<span style="opacity:.6">⌄</span></div>'
    key = lambda v: f'<div class="key">{v}</div>'
    body = f"""<div class="wrap"><h1>Configurações</h1>
<h2>Geral</h2><div class="card">{row("Iniciar com o Windows", tg(False), "Abre o Eavely na bandeja ao entrar no Windows.")}</div>
<h2>Ilha</h2><div class="card">
{row("Mostrar traço quando ocioso", tg(True), "Desligado: some até o mouse chegar ao topo.")}
{row("Mostrar a ilha ao vivo", tg(True))}{row("Ao vivo", '<div class="seg"><span class="a1">Compacto</span><span>Expandido</span></div>')}{row("Ocultar durante eventos", tg(False))}
{row("Monitor preferido", sel("Seguir o cursor"))}
{row("Zona de ativação do topo", '<div class="seg"><span>24 px</span><span class="a1">40 px</span><span>64 px</span></div>')}
{row("Aparência recolhida", '<div class="seg"><span class="a1">Bateria e hora</span><span>Olhos</span></div>')}
{row("Linha de letra na Música", tg(True))}</div>
<h2>Timer</h2><div class="card">{row("Foco", st("25 min"))}{row("Pausa curta", st("5 min"))}{row("Pausa longa", st("15 min"))}{row("Focos até a pausa longa", st("4"))}{row("Iniciar o próximo ciclo automaticamente", tg(False))}{row("Aviso e som ao terminar", tg(True))}</div>
<h2>Atalhos</h2><div class="card">{row("Mostrar/ocultar a ilha", key("Ctrl + Alt + E"))}{row("Iniciar/pausar o timer", key("Ctrl + Alt + T"))}{row("Abrir a aba Arquivos", key("Ctrl + Alt + F"))}</div>
<h2>Avisos do sistema</h2><div class="card">{row("Volume", tg(False))}{row("Brilho", tg(False))}{row("Bateria", tg(False))}{row("Wi‑Fi", tg(False))}{row("Fones", tg(False))}{row("Caps Lock", tg(False), "Verifica a tecla a cada 250 ms só com isto ligado.")}</div>
<h2>Sobre</h2><div class="card" style="padding:16px;height:150px;box-sizing:border-box;position:relative">
<div style="position:absolute;left:16px;top:16px;width:48px;height:48px;border-radius:12px;background:linear-gradient(#232733,#0B0C10);overflow:hidden"><div style="margin:0 auto;width:30px;height:24px;background:#fff;border-radius:0 0 12px 12px"></div><div style="margin:-14px auto 0;width:10px;height:10px;border-radius:50%;background:{ACC};position:relative"></div></div>
<div style="position:absolute;left:76px;top:16px"><div style="font-size:14px;font-weight:600;line-height:20px">Eavely</div><div style="font-size:11px;color:{T2}">Versão 0.0.1</div></div>
<div style="position:absolute;left:16px;top:76px;width:368px;font-size:11px;color:{T2};line-height:15px">Notch leve para Windows. Sem telemetria: a rede é usada só para buscar letras (LRCLIB).</div>
<div style="position:absolute;left:16px;top:112px"><span class="btn">Licenças de terceiros</span><span class="btn">Código-fonte (GitHub)</span><span class="btn">Licença MIT</span></div></div>
<div style="text-align:center;color:{T3};font-size:11px;margin-top:14px">© 2026 Eavely · MIT</div></div>"""
    return css + body

# ------------------------------------------------------------------ build
MOCKS = [
    ("01-musica-letra-1-linha", wrap_open(p_music(), "#000", live=dict(music=True)), None),
    ("02-musica-gradiente-capa", tinted(p_music()), None),
    ("02b-musica-volume-slider", wrap_open(p_music(vol=True), live=dict(music=True)), None),
    ("02c-musica-tela-dividida-letras", scene_split(p_music_split()), None),
    ("02d-musica-tela-dividida-carregando", scene_split(p_music_split(mode="loading")), None),
    ("02e-musica-tela-dividida-sem-letra", scene_split(p_music_split(lyrics=False, mode="none")), None),
    ("02f-musica-tela-dividida-dock", scene_split(p_music_split(dock=True), dock=True), None),
    ("03-musica-vazio-nada-tocando", wrap_open(p_music(empty=True)), None),
    ("04-musica-sem-letra", wrap_open(p_music(lyric="")), None),
    ("05-pomodoro-parado", wrap_open(p_pomodoro("idle")), None),
    ("06-pomodoro-rodando-personalizado", wrap_open(p_pomodoro("run"), live=dict(music=False, timer=True)), None),
    ("07-pomodoro-editando-valor", wrap_open(p_pomodoro("idle", edit=True)), None),
    ("08-pomodoro-fim", wrap_open(p_pomodoro("done")), None),
    ("10-shelf-vazio", wrap_open(p_shelf("empty")), None),
    ("11-shelf-arrastando", wrap_open(p_shelf("drag"), "radial-gradient(90px 70px at 0% 100%,rgba(10,42,92,.9),transparent),radial-gradient(90px 70px at 100% 100%,rgba(10,42,92,.9),transparent),#000"), None),
    ("12-shelf-com-itens", wrap_open(p_shelf("items")), None),
    ("13-tarefas-vazio", wrap_open(p_todo(False)), None),
    ("14-tarefas-com-itens", wrap_open(p_todo(True)), None),
    ("15-dock-visivel-musica", wrap_open(p_music(dock=True), dock=True, scene_h=210), (560, 210)),
    ("16-dock-visivel-pomodoro", wrap_open(p_pomodoro("idle", dock=True), dock=True, scene_h=210), (560, 210)),
    ("17-traco", trace(), (560, 40)),
    ("18-peek-bateria-hora", peek(), (560, 60)),
    ("19a-ao-vivo-compacto-musica", live_compact("music"), (560, 60)),
    ("19b-ao-vivo-compacto-timer", live_compact("timer"), (560, 60)),
    ("19c-ao-vivo-compacto-musica-e-timer", live_compact(both=True), (560, 60)),
    ("19d-ao-vivo-expandido-musica", live_music(), (560, 80)),
    ("19e-ao-vivo-expandido-timer", live_timer(), (560, 80)),
    ("21-overlay-volume", overlay("volume"), (560, 80)),
    ("22-overlay-bateria", overlay("battery"), (560, 80)),
    ("23-overlay-caps-lock", overlay("caps"), (560, 80)),
    ("24-olhos-ociosos", peek(eyes=True, look=2), (560, 60)),
    ("25-olhos-piscando", peek(eyes=True, blink=True), (560, 60)),
    ("26-configuracoes", settings(), (440, 1560)),
]

def render(name, htmltext, size):
    w, h = size or (SCENE_W, 190)
    hp = os.path.join(TMP, name + ".html"); open(hp, "w", encoding="utf8").write(htmltext)
    png = os.path.join(OUT, name + ".png")
    subprocess.run([EDGE, "--headless=new", "--disable-gpu", "--hide-scrollbars", f"--window-size={max(w,500)},{h+130}", "--force-device-scale-factor=3",
                    "--virtual-time-budget=1500", f"--screenshot={png}", "file:///" + hp.replace("\\", "/")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    return png

if __name__ == "__main__":
    for i, (n, h, s) in enumerate(MOCKS):
        print(render(f"{i+1:02d}-" + n.split("-", 1)[1], h, s))
