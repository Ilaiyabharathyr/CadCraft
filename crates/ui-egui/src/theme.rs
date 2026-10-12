//! Design tokens. Colours were measured from the reference look (dark slate chrome, near-black
//! navy model space) and are CADCraft's own values; every widget reads them from here.
//!
//! Two palettes: [`Tokens::DARK`] (the original look) and [`Tokens::LIGHT`]. The interface theme
//! changes the chrome (title and tool bar, menus, palettes, tabs, command line, status bar,
//! dialogs); the drawing area keeps its model-space colours in both, as drawing colours are not
//! part of the interface theme. [`Tokens::get`] returns the palette of the theme in use, which
//! [`CadApp::logic`](crate::CadApp::logic) resolves once per frame from the user's [`ThemePref`].

use std::cell::Cell;
use std::sync::Arc;

use egui::{Color32, FontFamily, FontId, Theme, Visuals};
use serde::{Deserialize, Serialize};

/// The interface theme the user chose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    /// Follow the operating system's light/dark appearance, live.
    System,
    Light,
    /// The default: CADCraft's original look.
    #[default]
    Dark,
}

/// The theme `System` uses when the OS (or browser) reports no appearance.
pub const SYSTEM_FALLBACK: Theme = Theme::Dark;

impl ThemePref {
    pub const ALL: [ThemePref; 3] = [ThemePref::System, ThemePref::Light, ThemePref::Dark];

    pub fn as_str(self) -> &'static str {
        match self {
            ThemePref::System => "system",
            ThemePref::Light => "light",
            ThemePref::Dark => "dark",
        }
    }

    pub fn parse(s: &str) -> Option<ThemePref> {
        ThemePref::ALL.into_iter().find(|t| t.as_str().eq_ignore_ascii_case(s.trim()))
    }

    /// The theme to show, given what the OS reports (`ctx.system_theme()`).
    pub fn resolve(self, system: Option<Theme>) -> Theme {
        match self {
            ThemePref::System => system.unwrap_or(SYSTEM_FALLBACK),
            ThemePref::Light => Theme::Light,
            ThemePref::Dark => Theme::Dark,
        }
    }
}

thread_local! {
    /// The theme in use on the UI thread (set by [`set_active`] each frame).
    static ACTIVE: Cell<Theme> = const { Cell::new(Theme::Dark) };
}

/// The theme [`Tokens::get`] returns colours for.
pub fn active() -> Theme {
    ACTIVE.with(Cell::get)
}

pub fn set_active(theme: Theme) {
    ACTIVE.with(|a| a.set(theme));
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub chrome: Color32,
    pub chrome_dark: Color32,
    pub panel: Color32,
    pub tab_active: Color32,
    pub control: Color32,
    pub control_hover: Color32,
    pub border: Color32,
    pub canvas: Color32,
    pub grid_minor: Color32,
    pub grid_major: Color32,
    pub axis_x: Color32,
    pub axis_y: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_faint: Color32,
    pub icon: Color32,
    pub icon_accent: Color32,
    pub icon_point: Color32,
    pub accent: Color32,
    pub selection: Color32,
    pub hover: Color32,
    pub grip: Color32,
    pub grip_hot: Color32,
    pub snap: Color32,
    /// COMPARE: objects only in this drawing, only in the other one, and the areas of change.
    pub compare_this: Color32,
    pub compare_other: Color32,
    pub compare_area: Color32,
    pub window_fill: Color32,
    pub crossing_fill: Color32,
    pub window_stroke: Color32,
    pub crossing_stroke: Color32,
    pub cmd_bg: Color32,
    pub toggle_on: Color32,
    /// Text fields and other inset (extreme) backgrounds.
    pub field: Color32,
    /// Selected text and selected list rows.
    pub select_bg: Color32,
    /// Lines and labels drawn straight onto the drawing area (UCS icon, viewport label, rubber
    /// bands): they follow the model-space background, not the interface theme.
    pub canvas_ink: Color32,
    /// The command-line history lines behind the input bar.
    pub cmd_history: Color32,
    pub cmd_border: Color32,
    /// Command-line keywords (`[Undo/Close]`) and their hover background.
    pub cmd_keyword: Color32,
    pub cmd_keyword_hover: Color32,
    /// The command-line AutoComplete list: background and the highlighted first row.
    pub list_bg: Color32,
    pub list_row: Color32,
    /// Transient warnings (status bar messages).
    pub warn: Color32,
    /// The drawing-area crosshair cursor and its pick box.
    pub crosshair: Color32,
    /// The sheet of paper on a layout tab.
    pub paper: Color32,
}

impl Tokens {
    pub const DARK: Tokens = Tokens {
        chrome: Color32::from_rgb(0x3b, 0x44, 0x53),
        chrome_dark: Color32::from_rgb(0x31, 0x38, 0x44),
        panel: Color32::from_rgb(0x3c, 0x44, 0x52),
        tab_active: Color32::from_rgb(0x4e, 0x5a, 0x6e),
        control: Color32::from_rgb(0x50, 0x5a, 0x6d),
        control_hover: Color32::from_rgb(0x5c, 0x67, 0x7c),
        border: Color32::from_rgb(0x23, 0x29, 0x30),
        canvas: Color32::from_rgb(0x22, 0x28, 0x2f),
        grid_minor: Color32::from_rgb(0x29, 0x30, 0x3a),
        grid_major: Color32::from_rgb(0x31, 0x39, 0x45),
        axis_x: Color32::from_rgb(0x56, 0x28, 0x29),
        axis_y: Color32::from_rgb(0x28, 0x4a, 0x2e),
        text: Color32::from_rgb(0xd7, 0xdb, 0xe0),
        text_dim: Color32::from_rgb(0xa3, 0xaa, 0xb4),
        text_faint: Color32::from_rgb(0x87, 0x8d, 0x96),
        icon: Color32::from_rgb(0xcf, 0xd4, 0xdb),
        icon_accent: Color32::from_rgb(0x4f, 0xa3, 0xf7),
        icon_point: Color32::from_rgb(0xf0, 0x7a, 0x3a),
        accent: Color32::from_rgb(0x3d, 0x8b, 0xfd),
        selection: Color32::from_rgb(0x4a, 0x8f, 0xff),
        hover: Color32::from_rgb(0x9c, 0xc6, 0xff),
        grip: Color32::from_rgb(0x2f, 0x6b, 0xff),
        grip_hot: Color32::from_rgb(0xff, 0x3b, 0x3b),
        snap: Color32::from_rgb(0x2f, 0xd0, 0x62),
        compare_this: Color32::from_rgb(0x4c, 0xd9, 0x7b),
        compare_other: Color32::from_rgb(0xff, 0x5c, 0x5c),
        compare_area: Color32::from_rgb(0xff, 0xb4, 0x3c),
        window_fill: Color32::from_rgba_premultiplied(0x18, 0x30, 0x60, 0x50),
        crossing_fill: Color32::from_rgba_premultiplied(0x18, 0x48, 0x18, 0x50),
        window_stroke: Color32::from_rgb(0x6f, 0xa8, 0xff),
        crossing_stroke: Color32::from_rgb(0x7f, 0xe0, 0x7f),
        cmd_bg: Color32::from_rgba_premultiplied(0x34, 0x3c, 0x4a, 0xf0),
        toggle_on: Color32::from_rgb(0x3d, 0x8b, 0xfd),
        field: Color32::from_rgb(0x31, 0x38, 0x44),
        select_bg: Color32::from_rgb(0x3d, 0x8b, 0xfd),
        canvas_ink: Color32::from_rgb(0xa3, 0xaa, 0xb4),
        cmd_history: Color32::from_rgba_unmultiplied_const(0x2a, 0x30, 0x3a, 170),
        cmd_border: Color32::from_rgb(0x55, 0x5f, 0x70),
        cmd_keyword: Color32::from_rgb(0x8f, 0xc1, 0xff),
        cmd_keyword_hover: Color32::from_rgb(0x2f, 0x5e, 0xa8),
        list_bg: Color32::from_rgb(0x2b, 0x31, 0x3b),
        list_row: Color32::from_rgb(0x3a, 0x42, 0x50),
        warn: Color32::from_rgb(0xff, 0xd0, 0x80),
        crosshair: Color32::from_rgb(0xe8, 0xe8, 0xe8),
        paper: Color32::WHITE,
    };

    /// Light grey chrome with dark text. The drawing-area colours (canvas, grid, axes, selection,
    /// grips, snap markers, selection windows) are the same as [`Tokens::DARK`]'s.
    pub const LIGHT: Tokens = Tokens {
        chrome: Color32::from_rgb(0xe4, 0xe7, 0xeb),
        chrome_dark: Color32::from_rgb(0xd5, 0xd9, 0xdf),
        panel: Color32::from_rgb(0xee, 0xf0, 0xf3),
        tab_active: Color32::from_rgb(0xc8, 0xd3, 0xe2),
        control: Color32::from_rgb(0xdc, 0xe0, 0xe6),
        control_hover: Color32::from_rgb(0xcb, 0xd2, 0xdc),
        border: Color32::from_rgb(0xb3, 0xba, 0xc4),
        text: Color32::from_rgb(0x1d, 0x23, 0x2b),
        text_dim: Color32::from_rgb(0x47, 0x50, 0x5c),
        text_faint: Color32::from_rgb(0x62, 0x6a, 0x76),
        icon: Color32::from_rgb(0x34, 0x3c, 0x48),
        icon_accent: Color32::from_rgb(0x1b, 0x6f, 0xd6),
        icon_point: Color32::from_rgb(0xd3, 0x5a, 0x1c),
        accent: Color32::from_rgb(0x2f, 0x7c, 0xf6),
        cmd_bg: Color32::from_rgba_unmultiplied_const(0xf9, 0xfa, 0xfb, 0xf0),
        toggle_on: Color32::from_rgb(0x2f, 0x7c, 0xf6),
        field: Color32::from_rgb(0xff, 0xff, 0xff),
        select_bg: Color32::from_rgb(0xb6, 0xd3, 0xfb),
        cmd_history: Color32::from_rgba_unmultiplied_const(0xf0, 0xf2, 0xf5, 0xdc),
        cmd_border: Color32::from_rgb(0xa9, 0xb0, 0xba),
        cmd_keyword: Color32::from_rgb(0x15, 0x59, 0xc0),
        cmd_keyword_hover: Color32::from_rgb(0xcf, 0xe0, 0xfa),
        list_bg: Color32::from_rgb(0xfb, 0xfc, 0xfd),
        list_row: Color32::from_rgb(0xe3, 0xe8, 0xef),
        warn: Color32::from_rgb(0x8a, 0x53, 0x00),
        ..Tokens::DARK
    };

    /// The palette of `theme`.
    pub fn of(theme: Theme) -> Tokens {
        match theme {
            Theme::Dark => Tokens::DARK,
            Theme::Light => Tokens::LIGHT,
        }
    }

    /// The palette of the theme in use, with the user's drawing-area colours ([`set_canvas`]).
    pub fn get() -> Tokens {
        CANVAS.with(Cell::get).over(Tokens::of(active()))
    }

    /// These tokens with `bg` as the model-space background: the grid, axes, crosshair and the
    /// lines drawn on the drawing area are recomputed to stay visible on it.
    pub fn with_model_background(self, bg: Color32) -> Tokens {
        let light = luminance(bg) > 0.5;
        let ink = if light { Color32::BLACK } else { Color32::WHITE };
        Tokens {
            canvas: bg,
            grid_minor: mix(bg, ink, 0.05),
            grid_major: mix(bg, ink, 0.11),
            axis_x: mix(bg, Color32::from_rgb(0xd0, 0x30, 0x30), 0.3),
            axis_y: mix(bg, Color32::from_rgb(0x30, 0xb0, 0x40), 0.3),
            canvas_ink: if light { Color32::from_rgb(0x47, 0x50, 0x5c) } else { Tokens::DARK.canvas_ink },
            crosshair: if light { Color32::from_rgb(0x20, 0x20, 0x20) } else { Tokens::DARK.crosshair },
            ..self
        }
    }
}

/// The user's drawing-area preferences (Options ▸ Display and Drafting); `None` keeps the theme's
/// colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canvas {
    pub model: Option<Color32>,
    pub paper: Option<Color32>,
    /// The side of an object snap marker, in points.
    pub snap_marker: f32,
}

/// The default object snap marker size.
pub const SNAP_MARKER: f32 = 11.0;

impl Default for Canvas {
    fn default() -> Self {
        Canvas { model: None, paper: None, snap_marker: SNAP_MARKER }
    }
}

impl Canvas {
    fn over(self, t: Tokens) -> Tokens {
        let t = match self.model {
            Some(bg) => t.with_model_background(bg),
            None => t,
        };
        Tokens { paper: self.paper.unwrap_or(t.paper), ..t }
    }
}

thread_local! {
    /// The drawing-area preferences in use on the UI thread (set by [`set_canvas`] each frame).
    static CANVAS: Cell<Canvas> = Cell::new(Canvas::default());
}

pub fn set_canvas(c: Canvas) {
    CANVAS.with(|v| v.set(c));
}

/// Half the side of an object snap marker, in points.
pub fn snap_marker_half() -> f32 {
    CANVAS.with(Cell::get).snap_marker / 2.0
}

/// Relative luminance of `c` (0 black … 1 white).
pub fn luminance(c: Color32) -> f32 {
    (0.299 * f32::from(c.r()) + 0.587 * f32::from(c.g()) + 0.114 * f32::from(c.b())) / 255.0
}

/// `a` moved the fraction `f` towards `b`.
fn mix(a: Color32, b: Color32, f: f32) -> Color32 {
    let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * f).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

pub fn small() -> FontId {
    FontId::new(11.5, FontFamily::Proportional)
}
pub fn body() -> FontId {
    FontId::new(12.5, FontFamily::Proportional)
}
pub fn mono() -> FontId {
    FontId::new(12.0, FontFamily::Monospace)
}

/// Scripts interface text may need a font for beyond the UI font (a set).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Scripts(u8);

impl Scripts {
    pub const NONE: Scripts = Scripts(0);
    /// Chinese characters, Japanese kana, bopomofo, CJK punctuation and full-width forms.
    pub const CJK: Scripts = Scripts(1);
    pub const HANGUL: Scripts = Scripts(1 << 1);
    pub const THAI: Scripts = Scripts(1 << 2);
    pub const DEVANAGARI: Scripts = Scripts(1 << 3);
    pub const ARABIC: Scripts = Scripts(1 << 4);
    pub const HEBREW: Scripts = Scripts(1 << 5);
    /// Each script with a character a font must have to show it.
    const SAMPLES: [(Scripts, char); 6] = [
        (Scripts::CJK, '中'),
        (Scripts::HANGUL, '한'),
        (Scripts::THAI, 'ก'),
        (Scripts::DEVANAGARI, 'क'),
        (Scripts::ARABIC, 'ع'),
        (Scripts::HEBREW, 'א'),
    ];

    /// The script of `c`, if it is one of these.
    pub fn of_char(c: char) -> Scripts {
        match u32::from(c) {
            0..=0x058F => Scripts::NONE,
            0x0590..=0x05FF | 0xFB1D..=0xFB4F => Scripts::HEBREW,
            0x0600..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => Scripts::ARABIC,
            0x0900..=0x097F | 0xA8E0..=0xA8FF => Scripts::DEVANAGARI,
            0x0E00..=0x0E7F => Scripts::THAI,
            0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7FF => Scripts::HANGUL,
            0x2E80..=0x2FDF
            | 0x3000..=0x312F
            | 0x3190..=0x31FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0xFF00..=0xFFEF
            | 0x20000..=0x3FFFF => Scripts::CJK,
            _ => Scripts::NONE,
        }
    }

    /// The scripts of `text`'s characters.
    pub fn of(text: &str) -> Scripts {
        text.chars().fold(Scripts::NONE, |s, c| s | Scripts::of_char(c))
    }

    pub fn contains(self, other: Scripts) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn intersects(self, other: Scripts) -> bool {
        self.0 & other.0 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn without(self, other: Scripts) -> Scripts {
        Scripts(self.0 & !other.0)
    }
}

impl std::ops::BitOr for Scripts {
    type Output = Scripts;
    fn bitor(self, rhs: Scripts) -> Scripts {
        Scripts(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Scripts {
    fn bitor_assign(&mut self, rhs: Scripts) {
        self.0 |= rhs.0;
    }
}

/// Interface fonts read so far. They stay for the process, so showing a script again never
/// re-reads a file, and egui shares these `Arc`s on every install instead of copying the bytes.
#[derive(Default)]
struct FontCache {
    /// The UI font (`None` until first read; `Some(None)`: none installed).
    primary: Option<Option<Arc<egui::FontData>>>,
    /// Loaded fallback faces, each once.
    faces: Vec<Arc<egui::FontData>>,
    /// Scripts looked for, with the face that shows them (`None`: the UI font has them, or no
    /// installed font does).
    resolved: Vec<(Scripts, Option<usize>)>,
    /// Scripts being looked for off the UI thread.
    loading: Scripts,
}

fn font_cache() -> std::sync::MutexGuard<'static, FontCache> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<FontCache>> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default).lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Bumped each time fonts finish loading off the UI thread, so the caller installs them.
static FONTS_LOADED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Changes when fallback fonts finished loading since the last [`install_fonts`].
pub fn fonts_generation() -> u64 {
    FONTS_LOADED.load(std::sync::atomic::Ordering::Acquire)
}

/// Install the interface fonts: a system UI font when one is installed (not bundled), else egui's,
/// then fallback faces for the `scripts` interface text needs (issue #658: a Chinese file name on
/// a tab). Chinese takes [`cjk_font`] first; other scripts, and Chinese without it, take an
/// installed font found the way drawing text finds one ([`cadcraft_fonts::ttf::fallback_for`]).
/// Faces not read yet are looked for off the UI thread; [`fonts_generation`] changes when they are
/// ready, and installing again shows them.
pub fn install_fonts(ctx: &egui::Context, scripts: Scripts) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (fonts, missing, primary) = {
            let mut c = font_cache();
            let primary = c.primary.get_or_insert_with(read_ui_font).clone();
            let mut fonts = egui::FontDefinitions::default();
            if let Some(data) = &primary {
                fonts.font_data.insert("system-ui".into(), data.clone());
                if let Some(f) = fonts.families.get_mut(&FontFamily::Proportional) {
                    f.insert(0, "system-ui".into());
                }
            }
            let mut missing = Scripts::NONE;
            let mut shown: Vec<usize> = Vec::new();
            for (s, _) in Scripts::SAMPLES {
                if !scripts.contains(s) {
                    continue;
                }
                match c.resolved.iter().find(|(k, _)| *k == s) {
                    Some((_, Some(i))) if !shown.contains(i) => shown.push(*i),
                    Some(_) => {}
                    None => missing |= s,
                }
            }
            // After the Latin fonts, so Latin text keeps its look.
            for i in shown {
                if let Some(data) = c.faces.get(i) {
                    let name = format!("system-fallback-{i}");
                    fonts.font_data.insert(name.clone(), data.clone());
                    for family in [FontFamily::Proportional, FontFamily::Monospace] {
                        if let Some(f) = fonts.families.get_mut(&family) {
                            f.push(name.clone());
                        }
                    }
                }
            }
            let missing = missing.without(c.loading);
            c.loading |= missing;
            (fonts, missing, primary)
        };
        ctx.set_fonts(fonts);
        if !missing.is_empty() {
            let ctx = ctx.clone();
            let spawned = std::thread::Builder::new().name("ui-fonts".into()).spawn(move || {
                load_fallback_faces(missing, primary.as_deref());
                ctx.request_repaint();
            });
            if let Err(e) = spawned {
                log::warn!("interface fonts for other scripts not loaded: {e}");
                font_cache().loading = Scripts::NONE;
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (ctx, scripts);
}

/// The system UI font, if one is installed.
#[cfg(not(target_arch = "wasm32"))]
fn read_ui_font() -> Option<Arc<egui::FontData>> {
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &["/System/Library/Fonts/SFNS.ttf", "/System/Library/Fonts/Helvetica.ttc", "/Library/Fonts/Arial.ttf"]
    } else if cfg!(windows) {
        &["C:\\Windows\\Fonts\\segoeui.ttf", "C:\\Windows\\Fonts\\arial.ttf"]
    } else {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        ]
    };
    candidates.iter().find_map(|p| read_capped(std::path::Path::new(p), MAX_UI_FONT_BYTES)).map(|b| Arc::new(egui::FontData::from_owned(b)))
}

/// Look for faces for `missing` (off the UI thread) and record them in the font cache.
#[cfg(not(target_arch = "wasm32"))]
fn load_fallback_faces(missing: Scripts, primary: Option<&egui::FontData>) {
    use cadcraft_fonts::ttf::{Face, fallback_for};
    let primary = primary.and_then(|d| Face::new(Arc::new(d.font.to_vec()), d.index));
    let mut found: Vec<(Scripts, Option<Face>)> = Vec::new();
    for (s, sample) in Scripts::SAMPLES {
        if !missing.contains(s) {
            continue;
        }
        let face = if primary.as_ref().is_some_and(|p| p.has_glyph(sample)) {
            None
        } else if let Some(f) = found.iter().filter_map(|(_, f)| f.as_ref()).find(|f| f.has_glyph(sample)) {
            Some(f.clone())
        } else {
            let preferred = (s == Scripts::CJK)
                .then(cjk_font)
                .flatten()
                .and_then(|p| read_capped(p, MAX_UI_FONT_BYTES))
                .and_then(|b| Face::new(Arc::new(b), 0))
                .filter(|f| f.has_glyph(sample));
            preferred.or_else(|| fallback_for(sample))
        };
        found.push((s, face));
    }
    let mut c = font_cache();
    let mut ids: Vec<(Face, usize)> = Vec::new();
    for (s, face) in found {
        let i = face.map(|f| match ids.iter().find(|(g, _)| *g == f) {
            Some((_, i)) => *i,
            None => {
                let (bytes, index) = f.data();
                // A file read here has no other owner, so its bytes move to egui; one from the font
                // table (shared with drawing text) is copied.
                let font = Arc::try_unwrap(bytes.clone()).unwrap_or_else(|shared| shared.to_vec());
                let mut data = egui::FontData::from_owned(font);
                data.index = index;
                c.faces.push(Arc::new(data));
                let i = c.faces.len() - 1;
                ids.push((f, i));
                i
            }
        });
        c.resolved.push((s, i));
    }
    c.loading = c.loading.without(missing);
    drop(c);
    FONTS_LOADED.fetch_add(1, std::sync::atomic::Ordering::Release);
}

/// Largest interface font file read (CJK collections are 15–60 MB).
#[cfg(not(target_arch = "wasm32"))]
const MAX_UI_FONT_BYTES: u64 = 96 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
fn read_capped(path: &std::path::Path, max: u64) -> Option<Vec<u8>> {
    let len = std::fs::metadata(path).ok()?.len();
    if len == 0 || len > max {
        return None;
    }
    std::fs::read(path).ok()
}

/// An installed font with Chinese characters, for the Chinese interface; fonts are never bundled
/// (only looked for where the OS or common packages put them). Checked once.
pub fn cjk_font() -> Option<&'static std::path::Path> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static FOUND: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
        FOUND
            .get_or_init(|| {
                let candidates: &[&str] = if cfg!(target_os = "macos") {
                    &["/System/Library/Fonts/PingFang.ttc", "/System/Library/Fonts/STHeiti Light.ttc", "/System/Library/Fonts/Hiragino Sans GB.ttc"]
                } else if cfg!(windows) {
                    &["C:\\Windows\\Fonts\\msyh.ttc", "C:\\Windows\\Fonts\\msyh.ttf", "C:\\Windows\\Fonts\\simsun.ttc"]
                } else {
                    &[
                        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
                        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
                        "/usr/local/share/fonts/noto/NotoSansCJK-Regular.ttc",
                        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
                        "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
                        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
                        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
                    ]
                };
                candidates.iter().map(std::path::PathBuf::from).find(|p| p.is_file())
            })
            .as_deref()
    }
    #[cfg(target_arch = "wasm32")]
    None
}

/// Whether `text` has Chinese, Japanese or Korean characters that only a CJK font can show.
pub fn needs_cjk(text: &str) -> bool {
    Scripts::of(text).intersects(Scripts::CJK | Scripts::HANGUL)
}

/// Whether the interface can show `language`: Chinese needs an installed CJK font.
pub fn can_show(language: &str) -> bool {
    !language.starts_with("zh") || cjk_font().is_some()
}

/// egui's visuals for `theme`, from its tokens.
fn visuals(theme: Theme) -> Visuals {
    let t = Tokens::of(theme);
    let mut v = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };
    v.panel_fill = t.panel;
    v.window_fill = t.chrome;
    v.extreme_bg_color = t.field;
    v.faint_bg_color = t.chrome_dark;
    v.override_text_color = Some(t.text);
    v.widgets.noninteractive.bg_fill = t.panel;
    v.widgets.noninteractive.fg_stroke.color = t.text;
    v.widgets.noninteractive.bg_stroke.color = t.border;
    v.widgets.inactive.bg_fill = t.control;
    v.widgets.inactive.weak_bg_fill = t.control;
    v.widgets.inactive.fg_stroke.color = t.text;
    v.widgets.hovered.bg_fill = t.control_hover;
    v.widgets.hovered.weak_bg_fill = t.control_hover;
    v.widgets.active.bg_fill = t.accent;
    v.widgets.active.weak_bg_fill = t.accent;
    v.selection.bg_fill = t.select_bg;
    v.window_stroke = egui::Stroke::new(1.0, t.border);
    let shadow = Color32::from_black_alpha(if theme == Theme::Dark { 90 } else { 40 });
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 12, spread: 0, color: shadow };
    v.window_corner_radius = egui::CornerRadius::same(6);
    v.menu_corner_radius = egui::CornerRadius::same(5);
    v
}

/// Install the styles of both themes and show `theme` (egui's own widgets, popups and menus
/// follow it; our own painting follows [`Tokens::get`]). `follow_system` is true for the
/// [`ThemePref::System`] choice, whose `theme` was resolved from the OS report.
pub fn apply(ctx: &egui::Context, theme: Theme, follow_system: bool) {
    set_active(theme);
    ctx.set_visuals_of(Theme::Dark, visuals(Theme::Dark));
    ctx.set_visuals_of(Theme::Light, visuals(Theme::Light));
    // A System choice keeps egui's own System preference: a concrete Light/Dark would pin the native
    // window appearance (winit then stops reporting OS changes) and the app would never follow again.
    ctx.set_theme(if follow_system { egui::ThemePreference::System } else { egui::ThemePreference::from(theme) });
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(6.0, 3.0);
        s.spacing.interact_size.y = 20.0;
        s.text_styles.insert(egui::TextStyle::Body, body());
        s.text_styles.insert(egui::TextStyle::Button, body());
        s.text_styles.insert(egui::TextStyle::Small, small());
        s.text_styles.insert(egui::TextStyle::Monospace, mono());
    });
}

#[cfg(test)]
mod script_tests {
    use super::Scripts;

    #[test]
    fn scripts_of_interface_text() {
        assert_eq!(Scripts::of("Drawing1 Ünterlage_é.dwg"), Scripts::NONE);
        assert_eq!(Scripts::of("图纸A_t3.dwg"), Scripts::CJK);
        assert_eq!(Scripts::of("図面カタカナ"), Scripts::CJK);
        assert_eq!(Scripts::of("도면"), Scripts::HANGUL);
        assert_eq!(Scripts::of("แบบ"), Scripts::THAI);
        assert_eq!(Scripts::of("नक्शा"), Scripts::DEVANAGARI);
        assert_eq!(Scripts::of("خريطة"), Scripts::ARABIC);
        assert_eq!(Scripts::of("שרטוט"), Scripts::HEBREW);
        let mixed = Scripts::of("图纸 도면");
        assert!(mixed.contains(Scripts::CJK | Scripts::HANGUL) && !mixed.intersects(Scripts::THAI));
    }
}
