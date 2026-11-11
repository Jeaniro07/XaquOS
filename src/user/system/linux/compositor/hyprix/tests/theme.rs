//! The themes hyprix ships (`hyprix/themes/`, `docs/DESKTOP.md` §2) parse
//! with no diagnostic, and say what they are for.

use compositor_config::{NoSources, parse};

/// Ferrix light: every option and bind is one this compositor knows.
#[test]
fn ferrix_light_parses_and_floats() {
    let text = include_str!("../themes/ferrix-light.conf");
    let parsed = parse("ferrix-light.conf", text, &mut NoSources);
    assert_eq!(parsed.diagnostics, [], "the theme parses cleanly");
    let config = parsed.config;
    assert_eq!(config.str("general:layout"), Some("floating"));
    assert_eq!(config.int("hyprix:own_frames"), Some(1));
    let settings = compositor_layout::Settings::from_config(&config);
    assert_eq!(settings.layout, compositor_layout::Layout::Floating);
    for bind in &config.binds {
        // A key the keymap does not have is a bind the seat drops.
        if let compositor_config::Key::Sym(name) = &bind.key {
            assert!(
                compositor_xkb::code_of(name).is_some(),
                "{name} is a key of the default keymap"
            );
        }
        assert!(
            compositor_layout::Dispatcher::parse(&bind.dispatcher, &bind.arg).is_ok()
                || ["mouse", "overview", "switcher", "suspend", "resume"]
                    .contains(&bind.dispatcher.as_str()),
            "{bind:?} names a dispatcher hyprix runs"
        );
    }
}

/// Yoru (docs/DESKTOP.md §6): every option and bind is one this
/// compositor knows, its title bars and glow are set, and its wallpaper
/// line names the shipped picture.
#[test]
fn yoru_parses_floats_and_styles_its_frames() {
    let text = format!(
        "{}\n{}",
        include_str!("../themes/yoru.conf"),
        include_str!("../themes/yoru-keys.conf")
    );
    let parsed = parse("yoru.conf", &text, &mut NoSources);
    assert_eq!(parsed.diagnostics, [], "the theme parses cleanly");
    let config = parsed.config;
    assert_eq!(config.str("general:layout"), Some("floating"));
    assert_eq!(config.int("hyprix:titlebars"), Some(1));
    assert_eq!(config.int("hyprix:titlebar:height"), Some(30));
    assert_eq!(config.str("hyprix:titlebar:title_align"), Some("center"));
    assert_eq!(config.int("decoration:glow:enabled"), Some(0));
    let style = compositor_render::Style::from_config(&config);
    assert!(style.glow.is_none(), "light glass draws no glow");
    assert!(
        style.shadow_inactive.is_some(),
        "a lighter shadow elsewhere"
    );
    // Software screens draw no blur under Yoru; its shell's layers, drawn
    // for one, get the light ground behind them instead, under dark text.
    assert_eq!(config.int("hyprix:blur_in_software"), Some(0));
    let scrim = style.blur_scrim.expect("a scrim where there is no blur");
    assert!(scrim.alpha() >= 0xC0, "near-opaque: {scrim:?}");
    assert!(
        scrim.red() > 0xE0 && scrim.green() > 0xE0 && scrim.blue() > 0xE0,
        "light, under dark text: {scrim:?}"
    );
    let rules: Vec<_> = config
        .layer_rules
        .iter()
        .filter_map(|raw| compositor_config::LayerRule::parse(&raw.value).ok())
        .collect();
    for namespace in ["yoru-walls", "yoru-dock", "yoru-sidebar-left", "launcher"] {
        let layered = compositor_config::Layered::of(&rules, namespace);
        assert!(
            layered.blur,
            "{namespace} asks for the blur the scrim stands in for"
        );
    }
    let bars = hyprix::titlebar::Style::from_config(&config);
    assert_eq!(bars.height, 30);
    assert!(!bars.title_left);
    assert!(bars.buttons_left, "close, minimize, maximize at the left");
    for disc in [bars.close, bars.minimize, bars.maximize] {
        assert_eq!(disc.a, 0xff, "each button its own opaque disc");
    }
    assert_eq!(bars.disc.a, 0, "no disc behind the buttons");
    for bind in &config.binds {
        // SUPER is the Yoru key (the customer, 2026-10-24): no bind may take
        // a program's Alt or Ctrl keys.
        assert_eq!(
            bind.mods.0 & (compositor_config::Mods::ALT | compositor_config::Mods::CTRL),
            0,
            "{bind:?} takes a program's Alt or Ctrl key"
        );
        if let compositor_config::Key::Sym(name) = &bind.key {
            assert!(
                compositor_xkb::code_of(name).is_some(),
                "{name} is a key of the default keymap"
            );
        }
        assert!(
            compositor_layout::Dispatcher::parse(&bind.dispatcher, &bind.arg).is_ok()
                || ["exec", "mouse", "overview", "switcher", "suspend", "resume"]
                    .contains(&bind.dispatcher.as_str()),
            "{bind:?} names a dispatcher hyprix runs"
        );
    }
    assert!(
        config.wallpapers.iter().any(|line| line.source
            == compositor_config::WallpaperSource::Gradient {
                angle: 110,
                stops: vec![(0, 0xffa9_c6f5), (55, 0xffe8_d5f2), (100, 0xfff6_e3c9)],
            }),
        "{:?}",
        config.wallpapers
    );
}

/// Yoru's keys do what yoru-shell's menu bar says they do: Super Q quits
/// the focused program (`Quit`, killactive), Super W closes its window
/// (`Close`), Super M minimizes it.
#[test]
fn yoru_keys_are_the_menu_bars_shortcuts() {
    let parsed = parse(
        "yoru-keys.conf",
        include_str!("../themes/yoru-keys.conf"),
        &mut NoSources,
    );
    assert_eq!(parsed.diagnostics, [], "the keys parse cleanly");
    for (key, dispatcher) in [("Q", "killactive"), ("W", "killactive"), ("M", "minimize")] {
        let bound: Vec<&str> = parsed
            .config
            .binds
            .iter()
            .filter(|bind| {
                bind.mods == compositor_config::Mods::parse("SUPER")
                    && bind.key == compositor_config::Key::Sym(key.to_owned())
            })
            .map(|bind| bind.dispatcher.as_str())
            .collect();
        assert_eq!(bound, [dispatcher], "SUPER {key}");
    }
}
