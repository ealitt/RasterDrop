use crate::models::{CollisionPolicy, OutputLocation};
use crate::settings::AppSettings;
use dioxus::prelude::*;

#[component]
pub fn SettingsView(
    settings: AppSettings,
    validation_error: Option<String>,
    on_update: EventHandler<AppSettings>,
    on_choose_folder: EventHandler<()>,
    on_use_downloads: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    let fixed = matches!(settings.output_location, OutputLocation::FixedDirectory(_));
    let selected_folder = match &settings.output_location {
        OutputLocation::FixedDirectory(path) => path.display().to_string(),
        OutputLocation::BesideSource => "No folder selected".into(),
    };
    rsx! {
        section { class: "settings-view", aria_label: "Settings",
            div { class: "settings-header",
                div { h2 { "Settings" } p { "Changes are saved automatically." } }
                button { class: "icon-button", aria_label: "Close settings", onclick: move |_| on_close.call(()), "×" }
            }
            fieldset {
                legend { "Output location" }
                label { class: "choice",
                    input {
                        r#type: "radio",
                        name: "output-location",
                        checked: !fixed,
                        onchange: {
                            let settings = settings.clone();
                            move |_| { let mut next = settings.clone(); next.output_location = OutputLocation::BesideSource; on_update.call(next); }
                        }
                    }
                    span { strong { "Same folder as each original" } small { "Recommended for quick conversions" } }
                }
                label { class: "choice",
                    input {
                        r#type: "radio",
                        name: "output-location",
                        checked: fixed,
                        onchange: {
                            let settings = settings.clone();
                            move |_| { if !matches!(settings.output_location, OutputLocation::FixedDirectory(_)) { on_choose_folder.call(()); } }
                        }
                    }
                    span { strong { "Always save to this folder" } small { "{selected_folder}" } }
                }
                div { class: "folder-actions",
                    button { disabled: !fixed, onclick: move |_| on_choose_folder.call(()), "Choose Folder…" }
                    button { onclick: move |_| on_use_downloads.call(()), "Use Downloads" }
                }
                if let Some(error) = validation_error { p { class: "validation-error", role: "alert", "{error}" } }
            }
            fieldset {
                legend { "Existing files" }
                label { class: "choice",
                    input {
                        r#type: "radio",
                        name: "collision",
                        checked: settings.collision_policy == CollisionPolicy::CreateNumberedFile,
                        onchange: {
                            let settings = settings.clone();
                            move |_| { let mut next = settings.clone(); next.collision_policy = CollisionPolicy::CreateNumberedFile; on_update.call(next); }
                        }
                    }
                    span { strong { "Create a numbered filename" } small { "photo (1).jpg" } }
                }
                label { class: "choice",
                    input {
                        r#type: "radio",
                        name: "collision",
                        checked: settings.collision_policy == CollisionPolicy::Overwrite,
                        onchange: {
                            let settings = settings.clone();
                            move |_| { let mut next = settings.clone(); next.collision_policy = CollisionPolicy::Overwrite; on_update.call(next); }
                        }
                    }
                    span { strong { "Overwrite existing converted file" } small { "The original source is always protected" } }
                }
            }
            fieldset {
                legend { "JPEG quality" }
                label { class: "quality-row",
                    input {
                        r#type: "range",
                        min: "50",
                        max: "100",
                        value: "{settings.jpeg_quality}",
                        oninput: {
                            let settings = settings.clone();
                            move |event| if let Ok(value) = event.value().parse::<u8>() { let mut next = settings.clone(); next.jpeg_quality = value; on_update.call(next); }
                        }
                    }
                    output { "{settings.jpeg_quality}%" }
                }
            }
        }
    }
}
