"""Gera assets/app/logo-sobre.bgra (96x96, BGRA pre-multiplicado) a partir de assets/app/logo-sobre-128.png. Requer Pillow. Uso: python scripts/make-about-logo.py"""
from PIL import Image
im = Image.open("assets/app/logo-sobre-128.png").convert("RGBA").resize((96, 96), Image.LANCZOS)
out = bytearray()
for r, g, b, a in im.getdata():
    out += bytes((b * a // 255, g * a // 255, r * a // 255, a))
open("assets/app/logo-sobre.bgra", "wb").write(out)
