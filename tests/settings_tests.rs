use rasterdrop::models::{CollisionPolicy, OutputFormat, OutputLocation};
use rasterdrop::settings::{AppSettings, SettingsStore};
use tempfile::tempdir;

#[test]
fn settings_round_trip_is_versioned() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let settings = AppSettings {
        schema_version: 1,
        output_format: OutputFormat::WebP,
        output_location: OutputLocation::FixedDirectory(dir.path().into()),
        collision_policy: CollisionPolicy::Overwrite,
        jpeg_quality: 87,
    };
    SettingsStore::save_to(&path, &settings).unwrap();
    assert_eq!(SettingsStore::load_from(&path), settings);
}

#[test]
fn missing_or_corrupted_settings_use_safe_defaults() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("missing.json");
    assert_eq!(SettingsStore::load_from(&missing), AppSettings::default());

    let corrupted = dir.path().join("bad.json");
    std::fs::write(&corrupted, b"{ definitely not JSON").unwrap();
    assert_eq!(SettingsStore::load_from(&corrupted), AppSettings::default());
}

#[test]
fn jpeg_quality_is_sanitized() {
    let mut settings = AppSettings {
        jpeg_quality: 255,
        ..AppSettings::default()
    };
    settings.sanitize();
    assert_eq!(settings.jpeg_quality, 100);
}

#[test]
fn default_and_last_selected_formats_have_stable_dropdown_values() {
    assert_eq!(AppSettings::default().output_format, OutputFormat::Jpeg);
    assert_eq!(OutputFormat::Jpeg.selector_value(), "jpeg");

    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let selected = AppSettings {
        output_format: OutputFormat::Avif,
        ..AppSettings::default()
    };
    SettingsStore::save_to(&path, &selected).unwrap();
    let restored = SettingsStore::load_from(&path);
    assert_eq!(restored.output_format, OutputFormat::Avif);
    assert_eq!(restored.output_format.selector_value(), "avif");
}
