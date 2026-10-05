use aula_f75::types::{Key, KeyLayer, KeyType, MacroData, MacroType};
use aula_f75::utils::parse_hex;
use aula_f75::{DEFAULT_CONFIG, parse_config, parse_profile, serialize_config, validate_keys};

fn key() -> Key {
    Key::new_basic("Test".into(), "0x00000004".into(), 0, KeyLayer::Normal)
}

#[test]
fn global_effect_colours_round_trip_and_require_a_colour_effect() {
    use aula_f75::serialize_profile_with_color;
    use aula_f75::types::EffectColor;
    let color = EffectColor {
        color: aula_f75::types::Color::create(10, 20, 30),
        rainbow: false,
    };
    let keys = vec![key()];
    let text = serialize_profile_with_color(&keys, Some(12), Some(color.clone())).unwrap();
    let profile = parse_profile(&text).unwrap();
    assert_eq!(profile.effect_color, Some(color.clone()));
    assert_eq!(profile.keys, keys);
    for effect in [None, Some(0), Some(3), Some(21), Some(65535)] {
        let text = serialize_profile_with_color(&keys, effect, Some(color.clone())).unwrap();
        assert!(parse_profile(&text).is_err());
    }
    assert!(
        parse_profile(&serialize_config(&keys).unwrap())
            .unwrap()
            .effect_color
            .is_none()
    );
}

#[test]
fn hexadecimal_values_are_strict_and_zero_is_valid() {
    for value in [
        "",
        "0x",
        "0x0x04",
        "garbage",
        "0xGG",
        "100000000",
        "-1",
        "+4",
        " 04",
        "04 ",
    ] {
        assert!(parse_hex(value).is_err(), "accepted {value:?}");
    }
    for (value, expected) in [
        ("0x0", 0),
        ("04", 4),
        ("0XABCDEF01", 0xabcdef01),
        ("ffffffff", u32::MAX),
    ] {
        assert_eq!(parse_hex(value).unwrap(), expected);
    }
}

#[test]
fn invalid_positions_and_macros_are_rejected() {
    let mut cases = Vec::new();
    for pos in [1, 3, 501, 504, 508, usize::MAX] {
        cases.push(Key { pos, ..key() });
    }
    for light_pos in [126, 255, usize::MAX] {
        cases.push(Key { light_pos, ..key() });
    }
    cases.push(Key {
        key_type: KeyType::Macro,
        ..key()
    });
    cases.push(Key {
        macro_data: Some(MacroData {
            macro_type: MacroType::Button,
            count: 1,
            data: None,
        }),
        ..key()
    });
    cases.push(Key {
        value: "not hex".into(),
        ..key()
    });
    for k in cases {
        assert!(
            validate_keys(std::slice::from_ref(&k)).is_err(),
            "accepted {k:?}"
        );
        // Loading a CLI config / GUI profile has the same guard as direct driver calls.
        if let Ok(text) = serialize_config(&[k]) {
            assert!(parse_config(&text).is_err());
        }
    }
    assert!(
        validate_keys(&[Key {
            pos: 500,
            light_pos: 125,
            ..key()
        }])
        .is_ok()
    );
}

#[test]
fn duplicate_slots_are_scoped_to_each_layer() {
    let first = key();
    assert!(
        validate_keys(&[
            first.clone(),
            Key {
                light_pos: 1,
                ..key()
            }
        ])
        .is_err()
    );
    assert!(validate_keys(&[first.clone(), Key { pos: 4, ..key() }]).is_err());
    assert!(
        validate_keys(&[
            first,
            Key {
                layer: KeyLayer::Fn,
                ..key()
            }
        ])
        .is_ok()
    );
}

#[test]
fn bundled_layouts_are_valid_and_unsupported_types_report_errors() {
    assert!(!parse_config(DEFAULT_CONFIG).unwrap().is_empty());
    assert!(
        !parse_config(include_str!("../examples/finnish-ansi.toml"))
            .unwrap()
            .is_empty()
    );
    let text = serialize_config(&[key()]).unwrap();
    let unsupported = text.replace("key_type = \"basic\"", "key_type = \"unsupported\"");
    assert!(parse_config(&unsupported).is_err());
    assert!(parse_profile(&format!("effect = 65535\n{text}")).is_err());
    let error = parse_config(&text.replace("0x00000004", "oops")).unwrap_err();
    let message = format!("{error:#}");
    assert!(
        message.contains("Test") && message.contains("hexadecimal"),
        "{message}"
    );
}

#[test]
fn saved_configs_round_trip_numeric_layers() {
    let keys: Vec<Key> = [KeyLayer::Normal, KeyLayer::Fn, KeyLayer::Fn1]
        .into_iter()
        .map(|layer| Key { layer, ..key() })
        .collect();
    assert_eq!(
        parse_config(&serialize_config(&keys).unwrap()).unwrap(),
        keys
    );
}

#[test]
fn rgb_reads_use_planar_led_indices() {
    use aula_f75::utils::extract_color_from_lights;
    let mut lights = [0; 384];
    lights[0] = 255;
    lights[2] = 10;
    lights[128] = 20;
    lights[254] = 30;
    assert_eq!(
        extract_color_from_lights(&lights, 0),
        aula_f75::types::Color::create(255, 0, 0)
    );
    assert_eq!(
        extract_color_from_lights(&lights, 2),
        aula_f75::types::Color::create(10, 20, 30)
    );
    assert_eq!(
        extract_color_from_lights(&lights, usize::MAX),
        aula_f75::types::Color::default()
    );
    let keys = serialize_config(&[key()]).unwrap();
    assert_eq!(
        parse_profile(&format!("effect = 21\n{keys}"))
            .unwrap()
            .effect,
        Some(21)
    );
}
