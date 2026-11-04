// Renderer nativo: D3D11 + DXGI (swap chain de composicao, espera por quadro) + DirectComposition + Direct2D + DirectWrite.
// Coordenadas em px logicos (100% DPI); `scale` do monitor entra como transformacao.
use std::{collections::HashMap, mem::ManuallyDrop};
use windows_numerics::{Matrix3x2, Vector2};
use windows::{
    core::{w, Interface, Result},
    Win32::{Foundation::{HANDLE, HMODULE, HWND},
        Graphics::{Direct2D::{Common::*, *}, Direct3D::*, Direct3D11::*, DirectComposition::*, DirectWrite::*, Dxgi::{Common::*, *}},
        UI::Shell::SHCreateMemStream},
};

pub type Rgba = (f32, f32, f32, f32);
pub const fn rgba(hex: u32, a: f32) -> Rgba { (((hex >> 16) & 255) as f32 / 255.0, ((hex >> 8) & 255) as f32 / 255.0, (hex & 255) as f32 / 255.0, a) }
fn col(c: Rgba) -> D2D1_COLOR_F { D2D1_COLOR_F { r: c.0, g: c.1, b: c.2, a: c.3 } }
fn rc(x: f32, y: f32, w: f32, h: f32) -> D2D_RECT_F { D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h } }
const ID: Matrix3x2 = Matrix3x2 { M11: 1.0, M12: 0.0, M21: 0.0, M22: 1.0, M31: 0.0, M32: 0.0 };

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fmt { Title, Body, BodyStrong, Small, Clock, ClockLive, Big, Head, Row, RowStrong, Tiny }

pub struct Gfx {
    dc: ID2D1DeviceContext, dc5: Option<ID2D1DeviceContext5>, swap: IDXGISwapChain1, pub wait: HANDLE,
    dcomp: IDCompositionDevice, _target: IDCompositionTarget, _visual: IDCompositionVisual,
    d2d: ID2D1Factory1, dw: IDWriteFactory, brush: ID2D1SolidColorBrush, fmts: HashMap<Fmt, IDWriteTextFormat>,
    svgs: HashMap<(&'static str, u32), ID2D1SvgDocument>, covers: HashMap<u64, ID2D1Bitmap1>, thumbs: HashMap<u64, ID2D1Bitmap1>,
    pub size: (u32, u32), pub scale: f32, pub font: &'static str,
}

macro_rules! ic { ($n:literal) => { include_str!(concat!("../../assets/icons/", $n, ".svg")) }; }
fn icon_src(name: &str) -> &'static str {
    match name {
        "play" => ic!("play"), "pause" => ic!("pause"), "skip-back" => ic!("skip-back"), "skip-forward" => ic!("skip-forward"),
        "music" => ic!("music"), "timer" => ic!("timer"), "inbox" => ic!("inbox"), "list-checks" => ic!("list-checks"), "settings" => ic!("settings"),
        "chevron-left" => ic!("chevron-left"), "chevron-right" => ic!("chevron-right"), "message-square-quote" => ic!("message-square-quote"),
        "volume-2" => ic!("volume-2"), "volume-1" => ic!("volume-1"), "volume-x" => ic!("volume-x"), "rotate-ccw" => ic!("rotate-ccw"),
        "coffee" => ic!("coffee"), "target" => ic!("target"), "x" => ic!("x"), "file" => ic!("file"), "folder" => ic!("folder"),
        "battery" => ic!("battery"), "battery-charging" => ic!("battery-charging"), "trash-2" => ic!("trash-2"), "eye-off" => ic!("eye-off"),
        "power" => ic!("power"), "log-out" => ic!("log-out"), "calendar" => ic!("calendar"), "circle-plus" => ic!("circle-plus"), "pencil" => ic!("pencil"),
        "app" => include_str!("../../assets/app/eavely-icon.svg"), "plus" => ic!("plus"), "circle-check" => ic!("circle-check"), "check" => ic!("check"), "sun" => ic!("sun"), "battery-low" => ic!("battery-low"), "wifi" => ic!("wifi"), "wifi-off" => ic!("wifi-off"), "headphones" => ic!("headphones"), "arrow-big-up" => ic!("arrow-big-up"),
        "eye" => ic!("eye"), "bell" => ic!("bell"), "chevron-down" => ic!("chevron-down"), "info" => ic!("info"), "keyboard" => ic!("keyboard"), "minus" => ic!("minus"), "zap" => ic!("zap"),
        "wifi-high" => ic!("wifi-high"), "wifi-low" => ic!("wifi-low"), "wifi-zero" => ic!("wifi-zero"), "ethernet-port" => ic!("ethernet-port"),
        _ => ic!("circle"),
    }
}

impl Drop for Gfx { fn drop(&mut self) { unsafe { if !self.wait.is_invalid() { let _ = windows::Win32::Foundation::CloseHandle(self.wait); } } } } // HANDLE do swap chain (1 por ciclo da ilha)

impl Gfx {
    pub fn new(hwnd: HWND, w: u32, h: u32, scale: f32) -> Result<Gfx> { unsafe {
        let mut dev = None;
        let hw = D3D11CreateDevice(None::<&IDXGIAdapter>, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut dev), None, None);
        if hw.is_err() || dev.is_none() { // VM / RDP sem GPU / driver quebrado: renderizador por software (WARP)
            crate::log("gfx: GPU indisponivel, usando WARP"); dev = None;
            D3D11CreateDevice(None::<&IDXGIAdapter>, D3D_DRIVER_TYPE_WARP, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut dev), None, None)?;
        }
        let dxgi: IDXGIDevice = dev.ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?.cast()?;
        let factory: IDXGIFactory2 = dxgi.GetAdapter()?.GetParent()?;
        let desc = DXGI_SWAP_CHAIN_DESC1 { Width: w, Height: h, Format: DXGI_FORMAT_B8G8R8A8_UNORM, Stereo: false.into(), SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT, BufferCount: 2, Scaling: DXGI_SCALING_STRETCH, SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED, Flags: DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as u32 };
        let swap = factory.CreateSwapChainForComposition(&dxgi, &desc, None)?;
        let s2: IDXGISwapChain2 = swap.cast()?;
        s2.SetMaximumFrameLatency(1)?;
        let wait = s2.GetFrameLatencyWaitableObject();
        let d2d: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let dc = d2d.CreateDevice(&dxgi)?.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
        dc.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
        let target = dcomp.CreateTargetForHwnd(hwnd, true)?;
        let visual = dcomp.CreateVisual()?;
        visual.SetContent(&swap)?; target.SetRoot(&visual)?; dcomp.Commit()?;
        let brush = dc.CreateSolidColorBrush(&col((1.0, 1.0, 1.0, 1.0)), None)?;
        let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let font = pick_font(&dw);
        let mk = |size: f32, weight: DWRITE_FONT_WEIGHT| -> Result<IDWriteTextFormat> {
            let f = dw.CreateTextFormat(&windows::core::HSTRING::from(font), None, weight, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, size, w!("pt-BR"))?;
            f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            let sign = dw.CreateEllipsisTrimmingSign(&f)?;
            f.SetTrimming(&DWRITE_TRIMMING { granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER, delimiter: 0, delimiterCount: 0 }, &sign)?;
            Ok(f)
        };
        let mut fmts = HashMap::new();
        for (k, sz, wt) in [(Fmt::Title, 14.0, DWRITE_FONT_WEIGHT_SEMI_BOLD), (Fmt::Body, 12.0, DWRITE_FONT_WEIGHT_NORMAL), (Fmt::BodyStrong, 12.0, DWRITE_FONT_WEIGHT_SEMI_BOLD),
            (Fmt::Small, 11.0, DWRITE_FONT_WEIGHT_NORMAL), (Fmt::Clock, 16.0, DWRITE_FONT_WEIGHT_SEMI_BOLD), (Fmt::ClockLive, 14.0, DWRITE_FONT_WEIGHT_SEMI_BOLD),
            (Fmt::Big, 30.0, DWRITE_FONT_WEIGHT_BOLD), (Fmt::Head, 18.0, DWRITE_FONT_WEIGHT_SEMI_BOLD), (Fmt::Row, 13.0, DWRITE_FONT_WEIGHT_NORMAL), (Fmt::RowStrong, 13.0, DWRITE_FONT_WEIGHT_MEDIUM), (Fmt::Tiny, 11.5, DWRITE_FONT_WEIGHT_MEDIUM)] { fmts.insert(k, mk(sz, wt)?); }
        let dc5 = dc.cast::<ID2D1DeviceContext5>().ok();
        crate::log(&format!("gfx: fonte={font} svg={}", dc5.is_some()));
        Ok(Gfx { dc, dc5, swap, wait, dcomp, _target: target, _visual: visual, d2d, dw: dw.clone(), brush, fmts, svgs: HashMap::new(), covers: HashMap::new(), thumbs: HashMap::new(), size: (w, h), scale, font })
    } }

    pub fn resize(&mut self, w: u32, h: u32, scale: f32) { unsafe {
        self.scale = scale;
        if (w, h) == self.size { return; }
        self.dc.SetTarget(None::<&ID2D1Image>);
        if self.swap.ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT).is_ok() { self.size = (w, h); }
        let _ = self.dcomp.Commit();
    } }

    pub fn begin(&self, clear: (f32, f32, f32, f32)) -> Result<()> { unsafe {
        let surf: IDXGISurface = self.swap.GetBuffer(0)?;
        let props = D2D1_BITMAP_PROPERTIES1 { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
            dpiX: 96.0, dpiY: 96.0, bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW, colorContext: ManuallyDrop::new(None) };
        let bmp = self.dc.CreateBitmapFromDxgiSurface(&surf, Some(&props))?;
        self.dc.SetTarget(&bmp);
        self.dc.BeginDraw();
        self.set_t(0.0, 0.0, 1.0);
        self.dc.PushAxisAlignedClip(&rc(clear.0, clear.1, clear.2, clear.3), D2D1_ANTIALIAS_MODE_ALIASED);
        self.dc.Clear(Some(&col((0.0, 0.0, 0.0, 0.0))));
        self.dc.PopAxisAlignedClip();
        Ok(())
    } }
    pub fn end(&self) -> Result<()> { unsafe {
        self.dc.EndDraw(None, None)?;
        self.swap.Present(0, DXGI_PRESENT(0)).ok() // o ritmo vem do DwmFlush no laco (um quadro por vblank)
    } }

    /// Transformacao: translate (px logicos) + escala extra, sobre a escala do monitor.
    pub fn set_t(&self, tx: f32, ty: f32, sc: f32) { unsafe {
        let s = self.scale * sc;
        self.dc.SetTransform(&Matrix3x2 { M11: s, M12: 0.0, M21: 0.0, M22: s, M31: tx * self.scale, M32: ty * self.scale });
    } }
    /// rotulo de botao: o GRUPO icone + espaco de 6 px + texto centrado na pilula (horizontal e vertical); `col` aplica aos dois
    pub fn icon_label(&mut self, ic: &'static str, label: &str, f: Fmt, x: f32, y: f32, w: f32, h: f32, col: Rgba, ic_px: f32) {
        let tw = self.measure(label, f); let group = ic_px + 6.0 + tw; let x0 = x + (w - group) / 2.0;
        self.icon(ic, x0, y + (h - ic_px) / 2.0, ic_px, col, false);
        self.text(label, f, x0 + ic_px + 6.0, y + (h - 16.0) / 2.0 - 1.5, tw + 6.0, 16.0, col, false); // -1,5: centro das MAIUSCULAS (QA5 C5), nao da caixa de linha
    }
    /// escala em torno de (cx,cy) (coords locais) sobre a transformacao atual; devolve a anterior para `set_matrix`
    pub fn scale_about(&self, cx: f32, cy: f32, sc: f32) -> Matrix3x2 { unsafe {
        let mut t = Matrix3x2::default(); self.dc.GetTransform(&mut t); let old = t;
        let (sx, sy) = (cx * (1.0 - sc), cy * (1.0 - sc));
        self.dc.SetTransform(&Matrix3x2 { M11: sc * t.M11, M12: sc * t.M12, M21: sc * t.M21, M22: sc * t.M22, M31: sx * t.M11 + sy * t.M21 + t.M31, M32: sx * t.M12 + sy * t.M22 + t.M32 });
        old
    } }
    /// escala NAO uniforme em torno de (cx,cy) (giro da capa: so a largura encolhe); devolve a anterior
    pub fn scale_xy(&self, cx: f32, cy: f32, sx: f32, sy: f32) -> Matrix3x2 { unsafe {
        let mut t = Matrix3x2::default(); self.dc.GetTransform(&mut t); let old = t;
        let (ox, oy) = (cx * (1.0 - sx), cy * (1.0 - sy));
        self.dc.SetTransform(&Matrix3x2 { M11: sx * t.M11, M12: sx * t.M12, M21: sy * t.M21, M22: sy * t.M22, M31: ox * t.M11 + oy * t.M21 + t.M31, M32: ox * t.M12 + oy * t.M22 + t.M32 });
        old
    } }
    pub fn set_matrix(&self, m: Matrix3x2) { unsafe { self.dc.SetTransform(&m); } }
    fn br(&self, c: Rgba) -> &ID2D1SolidColorBrush { unsafe { self.brush.SetColor(&col(c)); &self.brush } }

    pub fn clip(&self, x: f32, y: f32, w: f32, h: f32) { unsafe { self.dc.PushAxisAlignedClip(&rc(x, y, w, h), D2D1_ANTIALIAS_MODE_ALIASED); } }
    pub fn unclip(&self) { unsafe { self.dc.PopAxisAlignedClip(); } }
    /// Grupo com opacidade (fade de conteudo).
    pub fn layer(&self, a: f32) { unsafe {
        let p = D2D1_LAYER_PARAMETERS1 { contentBounds: rc(-10000.0, -10000.0, 20000.0, 20000.0), geometricMask: ManuallyDrop::new(None), maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            maskTransform: ID, opacity: a, opacityBrush: ManuallyDrop::new(None), layerOptions: D2D1_LAYER_OPTIONS1_NONE };
        self.dc.PushLayer(&p, None::<&ID2D1Layer>);
    } }
    pub fn unlayer(&self) { unsafe { self.dc.PopLayer(); } }

    pub fn rrect(&self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Rgba) { unsafe {
        self.dc.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rc(x, y, w, h), radiusX: r, radiusY: r }, self.br(c));
    } }
    pub fn rrect_stroke(&self, x: f32, y: f32, w: f32, h: f32, r: f32, sw: f32, c: Rgba) { unsafe {
        self.dc.DrawRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rc(x, y, w, h), radiusX: r, radiusY: r }, self.br(c), sw, None);
    } }
    pub fn ellipse(&self, cx: f32, cy: f32, rx: f32, ry: f32, c: Rgba) { unsafe { self.dc.FillEllipse(&D2D1_ELLIPSE { point: Vector2 { X: cx, Y: cy }, radiusX: rx, radiusY: ry }, self.br(c)); } }
    pub fn circle(&self, cx: f32, cy: f32, r: f32, c: Rgba) { unsafe {
        self.dc.FillEllipse(&D2D1_ELLIPSE { point: Vector2 { X: cx, Y: cy }, radiusX: r, radiusY: r }, self.br(c));
    } }
    pub fn ring(&self, cx: f32, cy: f32, r: f32, sw: f32, c: Rgba) { unsafe {
        self.dc.DrawEllipse(&D2D1_ELLIPSE { point: Vector2 { X: cx, Y: cy }, radiusX: r, radiusY: r }, self.br(c), sw, None);
    } }
    /// Arco horario a partir do topo, fracao 0..1.
    pub fn arc(&self, cx: f32, cy: f32, r: f32, sw: f32, frac: f32, c: Rgba) { unsafe {
        let f = frac.clamp(0.0, 1.0);
        if f >= 0.999 { return self.ring(cx, cy, r, sw, c); }
        if f <= 0.0 { return; }
        let a = f * std::f32::consts::TAU;
        let (ex, ey) = (cx + r * a.sin(), cy - r * a.cos());
        let Ok(g) = self.d2d.CreatePathGeometry() else { return };
        let Ok(sink) = g.Open() else { return };
        sink.BeginFigure(Vector2 { X: cx, Y: cy - r }, D2D1_FIGURE_BEGIN_HOLLOW);
        sink.AddArc(&D2D1_ARC_SEGMENT { point: Vector2 { X: ex, Y: ey }, size: D2D_SIZE_F { width: r, height: r }, rotationAngle: 0.0,
            sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE, arcSize: if f > 0.5 { D2D1_ARC_SIZE_LARGE } else { D2D1_ARC_SIZE_SMALL } });
        sink.EndFigure(D2D1_FIGURE_END_OPEN);
        let _ = sink.Close();
        let ss = D2D1_STROKE_STYLE_PROPERTIES1 { startCap: D2D1_CAP_STYLE_ROUND, endCap: D2D1_CAP_STYLE_ROUND, ..Default::default() };
        let st = self.d2d.CreateStrokeStyle(&ss, None).ok().and_then(|s| s.cast::<ID2D1StrokeStyle>().ok());
        self.dc.DrawGeometry(&g, self.br(c), sw, st.as_ref());
    } }
    /// Gradiente vertical (transparente em `start` -> `c` no fundo) sobre o retangulo (cantos de baixo seguem o clip externo).
    pub fn vgrad(&self, x: f32, y: f32, w: f32, h: f32, c: Rgba, start: f32, alpha: f32) { unsafe {
        let stops = [D2D1_GRADIENT_STOP { position: 0.0, color: col((c.0, c.1, c.2, 0.0)) }, D2D1_GRADIENT_STOP { position: start, color: col((c.0, c.1, c.2, 0.0)) },
                     D2D1_GRADIENT_STOP { position: 1.0, color: col((c.0, c.1, c.2, c.3 * alpha)) }];
        let Ok(coll) = self.dc.CreateGradientStopCollection(&stops, D2D1_COLOR_SPACE_SRGB, D2D1_COLOR_SPACE_SRGB, D2D1_BUFFER_PRECISION_8BPC_UNORM, D2D1_EXTEND_MODE_CLAMP, D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED) else { return };
        let props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: Vector2 { X: 0.0, Y: y }, endPoint: Vector2 { X: 0.0, Y: y + h } };
        let Ok(b) = self.dc.CreateLinearGradientBrush(&props, None, &coll) else { return };
        self.dc.FillRectangle(&rc(x, y, w, h), &b);
    } }
    /// gradiente linear com varias paradas (posicao 0..1, cor); `alpha` multiplica o alfa de todas. horiz = esquerda->direita, senao cima->baixo.
    pub fn grad(&self, x: f32, y: f32, w: f32, h: f32, horiz: bool, st: &[(f32, Rgba)], alpha: f32) { unsafe {
        let stops: Vec<D2D1_GRADIENT_STOP> = st.iter().map(|(p, c)| D2D1_GRADIENT_STOP { position: *p, color: col((c.0, c.1, c.2, c.3 * alpha)) }).collect();
        let Ok(coll) = self.dc.CreateGradientStopCollection(&stops, D2D1_COLOR_SPACE_SRGB, D2D1_COLOR_SPACE_SRGB, D2D1_BUFFER_PRECISION_8BPC_UNORM, D2D1_EXTEND_MODE_CLAMP, D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED) else { return };
        let props = if horiz { D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: Vector2 { X: x, Y: 0.0 }, endPoint: Vector2 { X: x + w, Y: 0.0 } } } else { D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: Vector2 { X: 0.0, Y: y }, endPoint: Vector2 { X: 0.0, Y: y + h } } };
        let Ok(b) = self.dc.CreateLinearGradientBrush(&props, None, &coll) else { return };
        self.dc.FillRectangle(&rc(x, y, w, h), &b);
    } }
    /// borda tracejada (dash/gap em unidades da espessura)
    pub fn rrect_dashed(&self, x: f32, y: f32, w: f32, h: f32, r: f32, sw: f32, c: Rgba, dash: f32, gap: f32) { unsafe {
        let props = D2D1_STROKE_STYLE_PROPERTIES1 { startCap: D2D1_CAP_STYLE_FLAT, endCap: D2D1_CAP_STYLE_FLAT, dashCap: D2D1_CAP_STYLE_FLAT, lineJoin: D2D1_LINE_JOIN_ROUND, miterLimit: 1.0, dashStyle: D2D1_DASH_STYLE_CUSTOM, dashOffset: 0.0, transformType: D2D1_STROKE_TRANSFORM_TYPE_NORMAL };
        let style: Option<ID2D1StrokeStyle> = self.d2d.CreateStrokeStyle(&props, Some(&[dash, gap])).ok().and_then(|s| windows::core::Interface::cast(&s).ok());
        self.dc.DrawRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rc(x, y, w, h), radiusX: r, radiusY: r }, self.br(c), sw, style.as_ref());
    } }
    /// miniatura BGRA premultiplicada (iw x ih), recortada para preencher o retangulo arredondado; `key` identifica o conteudo (cache)
    pub fn thumb(&mut self, key: u64, px: &[u8], iw: u32, ih: u32, x: f32, y: f32, w: f32, h: f32, r: f32, alpha: f32) { unsafe {
        if !self.thumbs.contains_key(&key) {
            if self.thumbs.len() > 40 { self.thumbs.clear(); }
            let props = D2D1_BITMAP_PROPERTIES1 { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED }, dpiX: 96.0, dpiY: 96.0, bitmapOptions: D2D1_BITMAP_OPTIONS_NONE, colorContext: ManuallyDrop::new(None) };
            match self.dc.CreateBitmap(D2D_SIZE_U { width: iw, height: ih }, Some(px.as_ptr() as *const _), iw * 4, &props) { Ok(b) => { self.thumbs.insert(key, b); } Err(_) => return }
        }
        let Some(b) = self.thumbs.get(&key) else { return };
        let Ok(br) = self.dc.CreateBitmapBrush(b, None, None) else { return };
        br.SetInterpolationMode1(D2D1_INTERPOLATION_MODE_LINEAR); br.SetOpacity(alpha);
        let k = (w / iw as f32).max(h / ih as f32); let (ox, oy) = (x + (w - iw as f32 * k) / 2.0, y + (h - ih as f32 * k) / 2.0);
        br.SetTransform(&Matrix3x2 { M11: k, M12: 0.0, M21: 0.0, M22: k, M31: ox, M32: oy });
        self.dc.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rc(x, y, w, h), radiusX: r, radiusY: r }, &br);
    } }
    /// Largura de um texto (px logicos) para alinhar grupos a direita.
    pub fn measure(&self, s: &str, f: Fmt) -> f32 { unsafe {
        let v: Vec<u16> = s.encode_utf16().collect();
        match self.dw.CreateTextLayout(&v, &self.fmts[&f], 2000.0, 100.0) {
            Ok(l) => { let mut m = DWRITE_TEXT_METRICS::default(); let _ = l.GetMetrics(&mut m); m.width }
            Err(_) => s.chars().count() as f32 * 6.5,
        }
    } }
    /// Recorte arredondado (a forma da ilha): tudo desenhado depois fica dentro dela (gradiente/vinhetas nao vazam nos cantos).
    pub fn rclip(&self, w: f32, h: f32, r: f32) { unsafe {
        let Ok(g) = self.d2d.CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT { rect: rc(0.0, -r, w, h + r), radiusX: r, radiusY: r }) else { return };
        let p = D2D1_LAYER_PARAMETERS1 { contentBounds: rc(-10000.0, -10000.0, 20000.0, 20000.0), geometricMask: ManuallyDrop::new(g.cast::<ID2D1Geometry>().ok()), maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            maskTransform: ID, opacity: 1.0, opacityBrush: ManuallyDrop::new(None), layerOptions: D2D1_LAYER_OPTIONS1_NONE };
        self.dc.PushLayer(&p, None::<&ID2D1Layer>);
        drop(ManuallyDrop::into_inner(p.geometricMask)); // o D2D ja guardou a sua referencia
    } }
    pub fn unrclip(&self) { unsafe { self.dc.PopLayer(); } }
    /// Vinheta radial (cor -> transparente) centrada em (cx, cy).
    pub fn vignette(&self, cx: f32, cy: f32, r: f32, c: Rgba) { unsafe {
        let stops = [D2D1_GRADIENT_STOP { position: 0.0, color: col(c) }, D2D1_GRADIENT_STOP { position: 1.0, color: col((c.0, c.1, c.2, 0.0)) }];
        let Ok(coll) = self.dc.CreateGradientStopCollection(&stops, D2D1_COLOR_SPACE_SRGB, D2D1_COLOR_SPACE_SRGB, D2D1_BUFFER_PRECISION_8BPC_UNORM, D2D1_EXTEND_MODE_CLAMP, D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED) else { return };
        let props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES { center: Vector2 { X: cx, Y: cy }, gradientOriginOffset: Vector2 { X: 0.0, Y: 0.0 }, radiusX: r, radiusY: r };
        let Ok(b) = self.dc.CreateRadialGradientBrush(&props, None, &coll) else { return };
        self.dc.FillRectangle(&rc(cx - r, cy - r, r * 2.0, r * 2.0), &b);
    } }
    /// "Flares": cantos concavos de 6 px que fundem o topo da ilha a borda da tela (DESIGN-NATIVO §2). Largura w da ilha, centrada em x=0..w.
    pub fn flares(&self, w: f32, r: f32, c: Rgba) { unsafe {
        for left in [true, false] {
            let Ok(g) = self.d2d.CreatePathGeometry() else { return };
            let Ok(sink) = g.Open() else { return };
            let (x0, x1, cx) = if left { (-r, 0.0, -r) } else { (w, w + r, w + r) };
            if left {
                sink.BeginFigure(Vector2 { X: x0, Y: 0.0 }, D2D1_FIGURE_BEGIN_FILLED);
                sink.AddLine(Vector2 { X: x1, Y: 0.0 }); sink.AddLine(Vector2 { X: x1, Y: r });
                sink.AddArc(&D2D1_ARC_SEGMENT { point: Vector2 { X: x0, Y: 0.0 }, size: D2D_SIZE_F { width: r, height: r }, rotationAngle: 0.0, sweepDirection: D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, arcSize: D2D1_ARC_SIZE_SMALL });
            } else {
                sink.BeginFigure(Vector2 { X: x0, Y: 0.0 }, D2D1_FIGURE_BEGIN_FILLED);
                sink.AddLine(Vector2 { X: x1, Y: 0.0 });
                sink.AddArc(&D2D1_ARC_SEGMENT { point: Vector2 { X: x0, Y: r }, size: D2D_SIZE_F { width: r, height: r }, rotationAngle: 0.0, sweepDirection: D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, arcSize: D2D1_ARC_SIZE_SMALL });
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED); let _ = sink.Close(); let _ = cx;
            self.dc.FillGeometry(&g, self.br(c), None::<&ID2D1Brush>);
        }
    } }
    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32, sw: f32, c: Rgba) { unsafe {
        self.dc.DrawLine(Vector2 { X: x1, Y: y1 }, Vector2 { X: x2, Y: y2 }, self.br(c), sw, None);
    } }

    pub fn text(&self, s: &str, f: Fmt, x: f32, y: f32, w: f32, h: f32, c: Rgba, right: bool) { unsafe {
        if s.is_empty() { return; }
        let fm = &self.fmts[&f];
        let _ = fm.SetTextAlignment(if right { DWRITE_TEXT_ALIGNMENT_TRAILING } else { DWRITE_TEXT_ALIGNMENT_LEADING });
        let v: Vec<u16> = s.encode_utf16().collect();
        self.dc.DrawText(&v, fm, &rc(x, y, w, h + 4.0), self.br(c), D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL);
    } }

    /// Icone SVG (Lucide) 24x24 desenhado em `size` px logicos. `solid` = tambem preenche (play/pause/skip).
    /// Usa a transformacao atual (translate + escala) como base.
    pub fn icon(&mut self, name: &'static str, x: f32, y: f32, size: f32, c: Rgba, solid: bool) { unsafe {
        let key = (name, ((c.0 * 255.0) as u32) << 16 | ((c.1 * 255.0) as u32) << 8 | (c.2 * 255.0) as u32 | ((solid as u32) << 24));
        if !self.svgs.contains_key(&key) {
            let Some(dc5) = &self.dc5 else { return };
            let base = if name == "app" { 256.0 } else { 24.0 };
            let hex = format!("#{:02x}{:02x}{:02x}", (c.0 * 255.0) as u8, (c.1 * 255.0) as u8, (c.2 * 255.0) as u8);
            let mut src = icon_src(name).replace("currentColor", &hex);
            if solid { src = src.replace("fill=\"none\"", &format!("fill=\"{hex}\"")); }
            let Some(stream) = SHCreateMemStream(Some(src.as_bytes())) else { return };
            match dc5.CreateSvgDocument(&stream, D2D_SIZE_F { width: base, height: base }) { Ok(d) => { self.svgs.insert(key, d); } Err(e) => { crate::log(&format!("svg {name}: {e}")); return; } }
        }
        let (Some(dc5), Some(doc)) = (&self.dc5, self.svgs.get(&key)) else { return };
        let mut old = ID; self.dc.GetTransform(&mut old);
        let k = size / if name == "app" { 256.0 } else { 24.0 };
        self.dc.SetTransform(&Matrix3x2 { M11: old.M11 * k, M12: 0.0, M21: 0.0, M22: old.M22 * k, M31: old.M31 + x * old.M11, M32: old.M32 + y * old.M22 });
        dc5.DrawSvgDocument(doc);
        self.dc.SetTransform(&old);
    } }

    /// Capa BGRA (side x side) com cantos arredondados. `gen` identifica o conteudo (cache).
    pub fn cover(&mut self, gen: u64, px: &[u8], side: u32, x: f32, y: f32, size: f32, r: f32, alpha: f32) { unsafe {
        if !self.covers.contains_key(&gen) {
            self.covers.clear();
            let props = D2D1_BITMAP_PROPERTIES1 { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: 96.0, dpiY: 96.0, bitmapOptions: D2D1_BITMAP_OPTIONS_NONE, colorContext: ManuallyDrop::new(None) };
            match self.dc.CreateBitmap(D2D_SIZE_U { width: side, height: side }, Some(px.as_ptr() as *const _), side * 4, &props) { Ok(b) => { self.covers.insert(gen, b); } Err(_) => return }
        }
        let Some(b) = self.covers.get(&gen) else { return };
        let Ok(br) = self.dc.CreateBitmapBrush(b, None, None) else { return };
        br.SetInterpolationMode1(D2D1_INTERPOLATION_MODE_LINEAR);
        br.SetOpacity(alpha);
        let k = size / side as f32;
        br.SetTransform(&Matrix3x2 { M11: k, M12: 0.0, M21: 0.0, M22: k, M31: x, M32: y });
        self.dc.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rc(x, y, size, size), radiusX: r, radiusY: r }, &br);
    } }
}

fn pick_font(dw: &IDWriteFactory) -> &'static str { unsafe {
    let mut coll = None;
    if dw.GetSystemFontCollection(&mut coll, false).is_ok() { if let Some(c) = coll {
        let (mut i, mut ex) = (0u32, false.into());
        if c.FindFamilyName(w!("Segoe UI Variable Text"), &mut i, &mut ex).is_ok() && ex.as_bool() { return "Segoe UI Variable Text"; }
    } }
    "Segoe UI"
} }
