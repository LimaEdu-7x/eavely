"""Intervalo de quadros por cenario a partir do EAVELY_FRAMELOG (t,dt,w,h,estado,pagina; linhas '# nome' marcam o cenario).
uso: python analyze.py arquivo.csv [trecho-do-nome ...]"""
import sys, statistics as st
def seg(path):
    cur, out = None, {}
    for l in open(path, encoding='utf8'):
        l = l.strip()
        if l.startswith('#'): cur = l[2:]; out.setdefault(cur, []); continue
        p = l.split(',')
        if cur is not None and len(p) >= 6: out[cur].append(float(p[1]))
    return out
def stats(v):
    v = sorted(x for x in v if x < 100)  # ignora o 1o quadro depois de ociosidade
    if len(v) < 3: return None
    q = lambda f: v[min(len(v) - 1, int(len(v) * f))]
    return len(v), sum(v) / len(v), q(.5), q(.95), v[-1], 1000 / (sum(v) / len(v))
if __name__ == '__main__':
    d = seg(sys.argv[1]); want = sys.argv[2:]
    for k, v in d.items():
        if want and not any(w in k for w in want): continue
        s = stats(v)
        print(f"{k:45s}", "sem quadros" if not s else f"n={s[0]:4d} med={s[1]:6.2f}ms p50={s[2]:6.2f} p95={s[3]:6.2f} max={s[4]:6.2f} fps~{s[5]:5.1f}")
