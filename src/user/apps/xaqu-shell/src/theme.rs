//! The theme engine, as the design's `applyTheme` has it (Xaqu's
//! `docs/DESKTOP.md` §6.1): a theme is a triad of hues -- one of the
//! design's five, or the wallpaper's own -- drawn dark or light at a
//! vibrance, and every token of Nocturne follows from it: the 100-900 ramps
//! of the neutrals and the three accents, the ground, the surface, the
//! text.
//!
//! The person's choice is `~/.config/xaqu-shell/theme` (`mode`,
//! `appearance`, `vibrance` lines). Every shell program reads it and the
//! wallpaper in force, and draws in the tokens; the dock also hands the
//! compositor the window frames' colours when either changes
//! (`frame_settings`), kept by `hyprctl setting` as Settings keeps its own.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tole::{Color, Ui};

use crate::quick::oklch;

/// The design's named themes and their triads; `Wallpaper` takes the
/// wallpaper's hues.
pub const TRIADS: [(&str, [f32; 3]); 5] = [
    ("Nocturne", [289.0, 200.0, 340.0]),
    ("Sakura", [352.0, 300.0, 70.0]),
    ("Mint", [165.0, 230.0, 95.0]),
    ("Sky", [236.0, 190.0, 310.0]),
    ("Sunset", [48.0, 10.0, 300.0]),
];

/// Where the theme's hues come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// The wallpaper's.
    #[default]
    Wallpaper,
    /// The design's triad `n` (`TRIADS`).
    Named(usize),
}

impl Mode {
    /// Its name, in the file and as shown.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Mode::Wallpaper => "Wallpaper",
            Mode::Named(n) => TRIADS.get(n).map_or("Nocturne", |t| t.0),
        }
    }

    /// The mode named `name`.
    #[must_use]
    pub fn of(name: &str) -> Option<Self> {
        if name == "Wallpaper" {
            return Some(Mode::Wallpaper);
        }
        TRIADS.iter().position(|t| t.0 == name).map(Mode::Named)
    }

    /// Every mode, as the quick settings list them.
    #[must_use]
    pub fn all() -> Vec<Mode> {
        std::iter::once(Mode::Wallpaper)
            .chain((0..TRIADS.len()).map(Mode::Named))
            .collect()
    }
}

/// Dark, light, or as the wallpaper's luminance says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Appearance {
    /// Light over a wallpaper brighter than 0.56, dark under.
    #[default]
    Auto,
    /// Dark.
    Dark,
    /// Light.
    Light,
}

impl Appearance {
    /// Every one, in the quick settings' order.
    pub const ALL: [Appearance; 3] = [Appearance::Auto, Appearance::Dark, Appearance::Light];

    /// Its name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Appearance::Auto => "Auto",
            Appearance::Dark => "Dark",
            Appearance::Light => "Light",
        }
    }
}

/// The person's choice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Where the hues come from.
    pub mode: Mode,
    /// Dark or light.
    pub appearance: Appearance,
    /// How colourful, 0.4 to 1.8.
    pub vibrance: f32,
}

/// Until a person chooses, Xaqu's light glass (`Tokens::GLASS`, the
/// markup's own values; spec §4).
impl Default for Theme {
    fn default() -> Self {
        Theme {
            mode: Mode::Named(0),
            appearance: Appearance::Light,
            vibrance: 1.3,
        }
    }
}

impl Theme {
    /// The theme a file says; what it leaves out is the default.
    #[must_use]
    pub fn read(text: &str) -> Self {
        let mut theme = Theme::default();
        for line in text.lines() {
            let Some((key, value)) = line.trim().split_once(' ') else {
                continue;
            };
            let value = value.trim();
            match key {
                "mode" => {
                    if let Some(mode) = Mode::of(value) {
                        theme.mode = mode;
                    }
                }
                "appearance" => {
                    if let Some(a) = Appearance::ALL.into_iter().find(|a| a.name() == value) {
                        theme.appearance = a;
                    }
                }
                "vibrance" => {
                    if let Ok(v) = value.parse::<f32>()
                        && v.is_finite()
                    {
                        theme.vibrance = v.clamp(0.4, 1.8);
                    }
                }
                _ => {}
            }
        }
        theme
    }

    /// The file's text.
    #[must_use]
    pub fn text(&self) -> String {
        format!(
            "mode {}\nappearance {}\nvibrance {}\n",
            self.mode.name(),
            self.appearance.name(),
            self.vibrance
        )
    }
}

/// What the wallpaper gives a theme: its hues and its luminance. A video
/// or a wallpaper that cannot be read is Nocturne's night.
#[derive(Clone, Debug, PartialEq)]
pub struct Wallpaper {
    /// Its main hue, if it has one.
    pub hue: Option<f32>,
    /// Up to three hues (`walls::hues`).
    pub hues: Vec<f32>,
    /// Its mean luminance.
    pub lum: f32,
}

impl Default for Wallpaper {
    fn default() -> Self {
        Wallpaper {
            hue: Some(289.0),
            hues: Vec::new(),
            lum: 0.12,
        }
    }
}

impl Wallpaper {
    /// The picture at `path`, read.
    #[must_use]
    pub fn of(path: &Path) -> Self {
        let video = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("ivf"));
        if video {
            return Wallpaper::default();
        }
        crate::walls::sample_of(path).map_or_else(Wallpaper::default, |sample| {
            let look = crate::walls::look(&sample);
            Wallpaper {
                hue: look.hue,
                hues: crate::walls::hues(&sample),
                lum: look.lum,
            }
        })
    }
}

/// The triad `theme` draws in over `wall`.
#[must_use]
pub fn hues(theme: &Theme, wall: &Wallpaper) -> [f32; 3] {
    match theme.mode {
        Mode::Named(n) => TRIADS.get(n).map_or(TRIADS[0].1, |t| t.1),
        Mode::Wallpaper => {
            let first = wall.hues.first().copied().or(wall.hue).unwrap_or(289.0);
            let at = |i: usize, turn: f32| {
                wall.hues
                    .get(i)
                    .copied()
                    .unwrap_or((first + turn).rem_euclid(360.0))
            };
            [first, at(1, 150.0), at(2, 300.0)].map(f32::round)
        }
    }
}

/// Whether `theme` is drawn light over `wall`.
#[must_use]
pub fn light(theme: &Theme, wall: &Wallpaper) -> bool {
    match theme.appearance {
        Appearance::Light => true,
        Appearance::Dark => false,
        Appearance::Auto => wall.lum > 0.56,
    }
}

/// An sRGB colour.
pub type Rgb = [u8; 3];

/// Every token of Nocturne.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tokens {
    /// Light rather than dark.
    pub light: bool,
    /// The ground.
    pub bg: Rgb,
    /// A raised surface.
    pub surface: Rgb,
    /// The ink.
    pub text: Rgb,
    /// The accent.
    pub accent: Rgb,
    /// The neutrals, 100 to 900.
    pub n: [Rgb; 9],
    /// The accent's ramp.
    pub a: [Rgb; 9],
    /// The second accent's.
    pub b: [Rgb; 9],
    /// The third accent's.
    pub c: [Rgb; 9],
}

/// The ramps' lightness and chroma factors, dark and light (the design's
/// `L0`, `F0` and theirs for light).
const DARK_L: [f32; 9] = [0.97, 0.92, 0.85, 0.77, 0.69, 0.59, 0.48, 0.37, 0.27];
const DARK_F: [f32; 9] = [0.22, 0.42, 0.68, 0.9, 1.0, 0.95, 0.8, 0.62, 0.42];
const LIGHT_L: [f32; 9] = [0.24, 0.32, 0.42, 0.52, 0.6, 0.7, 0.8, 0.89, 0.95];
const LIGHT_F: [f32; 9] = [0.55, 0.75, 0.92, 1.0, 1.0, 0.85, 0.6, 0.38, 0.22];

impl Tokens {
    /// The tokens of the triad `[h, h2, h3]` at vibrance `v`, light or
    /// dark.
    #[must_use]
    pub fn new([h, h2, h3]: [f32; 3], v: f32, light: bool) -> Self {
        let c = (0.14 * v).min(0.26);
        let (ls, fs) = if light {
            (LIGHT_L, LIGHT_F)
        } else {
            (DARK_L, DARK_F)
        };
        let ramp = |hue: f32| {
            let mut out = [[0; 3]; 9];
            for (slot, (l, f)) in out.iter_mut().zip(ls.iter().zip(fs.iter())) {
                *slot = oklch(*l, c * f, hue);
            }
            out
        };
        let mut n = [[0; 3]; 9];
        for (slot, (l, f)) in n.iter_mut().zip(ls.iter().zip(fs.iter())) {
            *slot = oklch(*l, 0.01 + 0.014 * v * f, h);
        }
        let (accent, text, bg, surface) = if light {
            (
                oklch(0.58, c, h),
                oklch(0.2, 0.02 * v, h),
                oklch(0.955, 0.014 * v, h),
                oklch(0.99, 0.006 * v, h),
            )
        } else {
            (
                oklch(0.71, c, h),
                oklch(0.94, 0.008 * v, h),
                oklch(0.19, 0.022 * v, h),
                oklch(0.245, 0.028 * v, h),
            )
        };
        Tokens {
            light,
            bg,
            surface,
            text,
            accent,
            n,
            a: ramp(h),
            b: ramp(h2),
            c: ramp(h3),
        }
    }

    /// Nocturne dark as the design system draws it: the markup's values
    /// (`ui/nocturne.tole`), which the engine's triad comes near but not
    /// to; the second and third accents' ends, which the markup leaves out,
    /// as their nearest.
    pub const NOCTURNE: Tokens = {
        const fn c(hex: u32) -> Rgb {
            [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
        }
        Tokens {
            light: false,
            bg: c(0x16_1826),
            surface: c(0x23_2532),
            text: c(0xe9_e9ed),
            accent: c(0x91_84d9),
            n: [
                c(0xf3_f5fe),
                c(0xe4_e7f5),
                c(0xcf_d3e5),
                c(0xb2_b6ca),
                c(0x93_97ab),
                c(0x75_798c),
                c(0x59_5d6c),
                c(0x3f_424d),
                c(0x29_2b31),
            ],
            a: [
                c(0xf5_f4ff),
                c(0xe7_e5fe),
                c(0xd2_cefd),
                c(0xb5_abfc),
                c(0x96_8ae0),
                c(0x79_6cbf),
                c(0x5d_5294),
                c(0x42_3a6a),
                c(0x2b_2741),
            ],
            b: [
                c(0x87_dfe4),
                c(0x87_dfe4),
                c(0x87_dfe4),
                c(0x46_cad0),
                c(0x00_b2ba),
                c(0x00_9199),
                c(0x00_9199),
                c(0x00_9199),
                c(0x00_9199),
            ],
            c: [
                c(0xf4_b8df),
                c(0xf4_b8df),
                c(0xf4_b8df),
                c(0xe4_97cb),
                c(0xce_7bb4),
                c(0xab_5f93),
                c(0xab_5f93),
                c(0xab_5f93),
                c(0xab_5f93),
            ],
        }
    };

    /// Xaqu's light glass, the default (spec §4): a white surface, ink near
    /// black, a clear blue accent. The ramps run from 100 the darkest to
    /// 900 the lightest, as the markup uses them on a light ground (hover
    /// on `n900`, quiet ink on `n300`); accent-2's 600 is the stash badge's
    /// teal, as before.
    pub const GLASS: Tokens = {
        const fn c(hex: u32) -> Rgb {
            [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
        }
        Tokens {
            light: true,
            bg: c(0xf2_f2f5),
            surface: c(0xff_ffff),
            text: c(0x1d_1d1f),
            accent: c(0x0a_84ff),
            n: [
                c(0x1f_2024),
                c(0x33_353b),
                c(0x4a_4d55),
                c(0x64_6872),
                c(0x86_8a94),
                c(0xa7_abb4),
                c(0xc6_c9d0),
                c(0xdc_dee3),
                c(0xec_edf0),
            ],
            a: [
                c(0x06_2a57),
                c(0x0a_3d7d),
                c(0x0d_53a8),
                c(0x0a_6fdc),
                c(0x0a_84ff),
                c(0x4a_a3ff),
                c(0x8c_c4ff),
                c(0xc4_e0ff),
                c(0xe6_f1ff),
            ],
            b: [
                c(0x05_3a3d),
                c(0x08_494d),
                c(0x0b_5d62),
                c(0x00_7980),
                c(0x00_868d),
                c(0x00_9199),
                c(0x55_c3c8),
                c(0xa6_e2e5),
                c(0xdc_f4f5),
            ],
            c: [
                c(0x4f_1037),
                c(0x6c_1a4c),
                c(0x8a_2763),
                c(0xa8_387b),
                c(0xc2_5294),
                c(0xd4_7fb0),
                c(0xe3_a9cb),
                c(0xf0_d0e3),
                c(0xf9_ebf3),
            ],
        }
    };

    /// The tokens of `theme` over `wall`: the default's are `GLASS`.
    #[must_use]
    pub fn of(theme: &Theme, wall: &Wallpaper) -> Self {
        if *theme == Theme::default() {
            return Tokens::GLASS;
        }
        Tokens::new(hues(theme, wall), theme.vibrance, light(theme, wall))
    }

    /// Hand them to the markup's `Nocturne`.
    pub fn apply(&self, ui: &mut Ui) {
        let rgb = |[r, g, b]: Rgb| Color::rgba(r, g, b, 0xff);
        let t = crate::ui::Nocturne::instance(ui);
        for (prop, value) in [
            (t.bg, self.bg),
            (t.surface, self.surface),
            (t.text, self.text),
            (t.accent, self.accent),
        ] {
            prop.set(ui, rgb(value));
        }
        let [r, g, b] = self.text;
        t.divider.set(ui, Color::rgba(r, g, b, 0x29));
        let ns = [
            t.n100, t.n200, t.n300, t.n400, t.n500, t.n600, t.n700, t.n800, t.n900,
        ];
        let accents = [
            t.a100, t.a200, t.a300, t.a400, t.a500, t.a600, t.a700, t.a800, t.a900,
        ];
        for (props, values) in [(ns, self.n), (accents, self.a)] {
            for (prop, value) in props.into_iter().zip(values) {
                prop.set(ui, rgb(value));
            }
        }
        let [_, _, b300, b400, b500, b600, ..] = self.b;
        let [_, _, c300, c400, c500, c600, ..] = self.c;
        for (prop, value) in [
            (t.b300, b300),
            (t.b400, b400),
            (t.b500, b500),
            (t.b600, b600),
            (t.c300, c300),
            (t.c400, c400),
            (t.c500, c500),
            (t.c600, c600),
        ] {
            prop.set(ui, rgb(value));
        }
    }

    /// The window frames' colours in these tokens, as the compositor's
    /// options: the focused ring accent-600, the others' neutral-700, the
    /// title bars on the surface with the text and the accents' line.
    #[must_use]
    pub fn frame_settings(&self) -> Vec<(&'static str, String)> {
        let hex = |[r, g, b]: Rgb, alpha: u8| format!("rgba({r:02x}{g:02x}{b:02x}{alpha:02x})");
        vec![
            ("general:col.active_border", hex(self.a[5], 0xff)),
            ("general:col.inactive_border", hex(self.n[6], 0x99)),
            ("hyprix:titlebar:col.active", hex(self.surface, 0xff)),
            ("hyprix:titlebar:col.inactive", hex(self.surface, 0xff)),
            ("hyprix:titlebar:col.text", hex(self.text, 0xff)),
            ("hyprix:titlebar:col.text_inactive", hex(self.n[3], 0xff)),
            (
                "hyprix:titlebar:col.line",
                format!(
                    "{} {} {}",
                    hex(self.accent, 0xff),
                    hex(self.b[4], 0xff),
                    hex(self.c[4], 0xff)
                ),
            ),
            ("hyprix:titlebar:col.separator", hex(self.text, 0x0f)),
        ]
    }

    /// What hyprix is asked so the window frames wear these tokens: for the
    /// glass, the theme file's own frames back, every kept setting reset in
    /// one request so hyprix reads its configuration again once; for any
    /// other, each setting said.
    #[must_use]
    pub fn frame_requests(&self) -> Vec<String> {
        let settings = self.frame_settings();
        if *self == Tokens::GLASS {
            let keys: Vec<&str> = settings.iter().map(|(key, _)| *key).collect();
            return vec![format!("setting reset {}", keys.join(", "))];
        }
        settings
            .into_iter()
            .map(|(key, value)| format!("setting {key} {value}"))
            .collect()
    }
}

/// The theme's file.
#[must_use]
pub fn theme_file() -> Option<PathBuf> {
    crate::widgets::config_file("theme")
}

/// hyprix's settings file, whose `wallpaper` line is the one in force.
#[must_use]
pub fn hypr_settings_file() -> Option<PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|h| !h.is_empty())
                .map(|h| Path::new(&h).join(".config"))
        })
        .map(|d| d.join("hypr/settings.conf"))
}

/// A file's time.
fn stamp(path: Option<&Path>) -> Option<SystemTime> {
    std::fs::metadata(path?).and_then(|m| m.modified()).ok()
}

/// The theme as a program follows it: read again when its file or the
/// wallpaper's changes.
#[derive(Debug)]
pub struct Live {
    files: (Option<PathBuf>, Option<PathBuf>),
    seen: (Option<SystemTime>, Option<SystemTime>),
    /// The choice.
    pub theme: Theme,
    /// The tokens in force.
    pub tokens: Tokens,
}

impl Live {
    /// The theme now.
    #[must_use]
    pub fn new() -> Self {
        let files = (theme_file(), hypr_settings_file());
        let seen = (stamp(files.0.as_deref()), stamp(files.1.as_deref()));
        let (theme, tokens) = Self::read(&files);
        Live {
            files,
            seen,
            theme,
            tokens,
        }
    }

    fn read(files: &(Option<PathBuf>, Option<PathBuf>)) -> (Theme, Tokens) {
        let text = |f: &Option<PathBuf>| {
            f.as_ref()
                .and_then(|f| std::fs::read_to_string(f).ok())
                .unwrap_or_default()
        };
        let theme = Theme::read(&text(&files.0));
        let needs_wall = theme.mode == Mode::Wallpaper || theme.appearance == Appearance::Auto;
        let wall = if needs_wall {
            crate::walls::in_force(&text(&files.1))
                .map_or_else(Wallpaper::default, |path| Wallpaper::of(Path::new(&path)))
        } else {
            Wallpaper::default()
        };
        (theme, Tokens::of(&theme, &wall))
    }

    /// The new tokens, if the theme or the wallpaper changed and so did
    /// they.
    pub fn poll(&mut self) -> Option<Tokens> {
        let seen = (
            stamp(self.files.0.as_deref()),
            stamp(self.files.1.as_deref()),
        );
        if seen == self.seen {
            return None;
        }
        self.seen = seen;
        let (theme, tokens) = Self::read(&self.files);
        self.theme = theme;
        (tokens != self.tokens).then(|| {
            self.tokens = tokens;
            tokens
        })
    }

    /// Keep `theme` as the person's choice.
    pub fn choose(&mut self, theme: &Theme) {
        if let Some(file) = &self.files.0 {
            crate::widgets::keep(file, &theme.text());
        }
    }
}

impl Default for Live {
    fn default() -> Self {
        Self::new()
    }
}

/// GTK's own settings for a look: Nocturne dark, or Adwaita light, Inter.
#[must_use]
pub fn gtk_settings(light: bool) -> String {
    format!(
        "[Settings]
gtk-theme-name = {}
gtk-application-prefer-dark-theme = {}
         gtk-font-name = Inter 11
gtk-cursor-theme-name = Adwaita
",
        if light { "Adwaita" } else { "Nocturne" },
        u8::from(!light)
    )
}

/// Say the look in force to the programs that are not the shell: `appearance`
/// in the shell's folder (portald follows it) and GTK's `settings.ini`, in
/// `home`, the configuration folder. True when something was written.
// Ponytail: GTK's file is written whole; a person's own keys in it are
// not kept. Upgrade path: change only the two lines, as Settings does.
pub fn publish_in(home: &Path, light: bool) -> bool {
    let mut wrote = false;
    let mut put = |path: PathBuf, text: String| {
        if std::fs::read_to_string(&path).ok().as_deref() != Some(&text) {
            crate::widgets::keep(&path, &text);
            wrote = true;
        }
    };
    put(
        home.join("xaqu-shell/appearance"),
        format!(
            "{}
",
            if light { "light" } else { "dark" }
        ),
    );
    for gtk in ["gtk-3.0", "gtk-4.0"] {
        put(home.join(gtk).join("settings.ini"), gtk_settings(light));
    }
    wrote
}

/// [`publish_in`] for the person's own configuration folder.
pub fn publish(light: bool) -> bool {
    crate::widgets::config_file("appearance")
        .as_deref()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .is_some_and(|home| publish_in(home, light))
}

/// A line saying what the tokens are, for a boot to read.
#[must_use]
pub fn said(theme: &Theme, tokens: &Tokens) -> String {
    let [r, g, b] = tokens.accent;
    format!(
        "theme {} {}, accent {r:02x}{g:02x}{b:02x}",
        theme.mode.name(),
        if tokens.light { "light" } else { "dark" }
    )
}

#[cfg(test)]
mod tests {
    use super::{Appearance, Mode, Theme, Tokens, Wallpaper, hues, light};

    #[test]
    fn the_default_is_the_light_glass_and_the_markup_says_so() {
        assert_eq!(
            Tokens::of(&Theme::default(), &Wallpaper::default()),
            Tokens::GLASS
        );
        let g = Tokens::GLASS;
        assert!(g.light);
        let markup = include_str!("../ui/nocturne.tole");
        let said = |name: &str| -> Option<[u8; 3]> {
            let start = format!("property color {name}:");
            let line = markup
                .lines()
                .find(|line| line.trim_start().starts_with(&start))?;
            let hex = line.split('#').nth(1)?.get(..6)?;
            let [_, r, g, b] = u32::from_str_radix(hex, 16).ok()?.to_be_bytes();
            Some([r, g, b])
        };
        assert_eq!(said("bg"), Some(g.bg));
        assert_eq!(said("surface"), Some(g.surface));
        assert_eq!(said("text"), Some(g.text));
        assert_eq!(said("accent"), Some(g.accent));
        for i in 0..9 {
            assert_eq!(said(&format!("n{}00", i + 1)), Some(g.n[i]), "n{}00", i + 1);
            assert_eq!(said(&format!("a{}00", i + 1)), Some(g.a[i]), "a{}00", i + 1);
        }
        for i in 2..6 {
            assert_eq!(said(&format!("b{}00", i + 1)), Some(g.b[i]), "b{}00", i + 1);
            assert_eq!(said(&format!("c{}00", i + 1)), Some(g.c[i]), "c{}00", i + 1);
        }
        // The stash boot finds its badge by this colour.
        assert_eq!(g.b[5], [0x00, 0x91, 0x99]);
    }

    #[test]
    fn the_engine_stays_near_nocturne_for_its_hues() {
        let t = Tokens::new([289.0, 200.0, 340.0], 1.0, false);
        let near = |got: [u8; 3], want: [u8; 3]| {
            got.iter()
                .zip(want)
                .all(|(g, w)| (i16::from(*g) - i16::from(w)).abs() <= 16)
        };
        for (i, (got, want)) in t.a.iter().zip(Tokens::NOCTURNE.a).enumerate() {
            assert!(near(*got, want), "a{}00 {got:02x?} {want:02x?}", i + 1);
        }
        assert!(near(t.n[0], Tokens::NOCTURNE.n[0]));
        assert!(!t.light);
    }

    #[test]
    fn the_glass_hands_the_frames_back_to_the_theme_file() {
        // One request, so hyprix reads its configuration again once.
        let reset = Tokens::GLASS.frame_requests();
        assert_eq!(reset.len(), 1, "{reset:?}");
        let keys: Vec<&str> = reset[0]
            .strip_prefix("setting reset ")
            .expect("a reset")
            .split(", ")
            .collect();
        assert_eq!(keys.len(), Tokens::GLASS.frame_settings().len());
        assert!(keys.contains(&"general:col.active_border"), "{keys:?}");
        let own = Tokens::new([289.0, 200.0, 340.0], 1.3, false).frame_requests();
        assert!(
            own.iter()
                .all(|line| line.starts_with("setting ") && !line.starts_with("setting reset "))
        );
    }

    #[test]
    fn light_turns_the_ground_and_ink_round() {
        let dark = Tokens::new([352.0, 300.0, 70.0], 1.3, false);
        let lit = Tokens::new([352.0, 300.0, 70.0], 1.3, true);
        let sum = |c: [u8; 3]| c.iter().map(|v| u32::from(*v)).sum::<u32>();
        assert!(sum(lit.bg) > 600 && sum(dark.bg) < 150);
        assert!(sum(lit.text) < 150 && sum(dark.text) > 600);
    }

    #[test]
    fn the_hues_are_the_triad_or_the_wallpapers() {
        let named = Theme {
            mode: Mode::of("Mint").expect("a theme"),
            ..Theme::default()
        };
        assert_eq!(hues(&named, &Wallpaper::default()), [165.0, 230.0, 95.0]);
        let wall = Wallpaper {
            hue: Some(30.0),
            hues: vec![40.0],
            lum: 0.7,
        };
        let own = Theme {
            mode: Mode::Wallpaper,
            appearance: Appearance::Auto,
            vibrance: 1.3,
        };
        assert_eq!(hues(&own, &wall), [40.0, 190.0, 340.0]);
        assert!(light(&own, &wall), "Auto over a bright wallpaper");
        assert!(!light(&own, &Wallpaper::default()));
        let dark = Theme {
            appearance: Appearance::Dark,
            ..own
        };
        assert!(!light(&dark, &wall));
    }

    #[test]
    fn the_file_round_trips_and_ignores_what_it_does_not_know() {
        let theme = Theme {
            mode: Mode::of("Sunset").expect("a theme"),
            appearance: Appearance::Light,
            vibrance: 0.9,
        };
        assert_eq!(Theme::read(&theme.text()), theme);
        let odd = Theme::read("mode Disco\nappearance Dim\nvibrance 9\n");
        assert_eq!(odd.mode, Mode::Named(0));
        assert_eq!(odd.appearance, Appearance::Light);
        assert!((odd.vibrance - 1.8).abs() < f32::EPSILON);
    }

    #[test]
    fn a_wallpapers_hues_come_strongest_first_and_apart() {
        // Half a strong red, a quarter blue, a quarter grey.
        let mut px = vec![[220, 30, 30, 255]; 40];
        px.extend(vec![[30, 60, 220, 255]; 20]);
        px.extend(vec![[128, 128, 128, 255]; 20]);
        let found = crate::walls::hues(&px);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0] < 45.0 || found[0] > 345.0, "red first: {found:?}");
        assert!(found[1] > 230.0 && found[1] < 290.0, "then blue: {found:?}");
    }

    #[test]
    fn the_look_is_published_once_for_portald_and_gtk() {
        let home = std::env::temp_dir().join(format!("xaqu-publish-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let read = |name: &str| std::fs::read_to_string(home.join(name)).expect(name);
        assert!(super::publish_in(&home, true));
        assert_eq!(
            read("xaqu-shell/appearance"),
            "light
"
        );
        for gtk in ["gtk-3.0", "gtk-4.0"] {
            let ini = read(&format!("{gtk}/settings.ini"));
            assert!(ini.contains("gtk-theme-name = Adwaita"), "{ini}");
            assert!(
                ini.contains("gtk-application-prefer-dark-theme = 0"),
                "{ini}"
            );
        }
        assert!(
            !super::publish_in(&home, true),
            "nothing changed, nothing written"
        );
        assert!(super::publish_in(&home, false));
        assert_eq!(
            read("xaqu-shell/appearance"),
            "dark
"
        );
        assert!(read("gtk-4.0/settings.ini").contains("gtk-theme-name = Nocturne"));
        assert!(super::gtk_settings(false).contains("prefer-dark-theme = 1"));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn frames_get_the_tokens_colours() {
        let t = Tokens::new([289.0, 200.0, 340.0], 1.3, false);
        let set = t.frame_settings();
        assert_eq!(set[0].0, "general:col.active_border");
        assert!(set[0].1.starts_with("rgba(") && set[0].1.ends_with("ff)"));
        assert_eq!(set[6].1.matches("rgba(").count(), 3);
    }
}
