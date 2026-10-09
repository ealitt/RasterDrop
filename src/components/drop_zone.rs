use dioxus::html::HasFileData;
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn DropZone(
    active: bool,
    on_drag_state: EventHandler<bool>,
    on_files: EventHandler<Vec<PathBuf>>,
    on_pick: EventHandler<()>,
) -> Element {
    let class = if active {
        "drop-zone drag-active"
    } else {
        "drop-zone"
    };
    rsx! {
        section {
            class,
            role: "button",
            tabindex: "0",
            aria_label: "Drop images here or choose images",
            ondragenter: move |event| {
                event.prevent_default();
                on_drag_state.call(true);
            },
            ondragover: move |event| {
                event.prevent_default();
                on_drag_state.call(true);
            },
            ondragleave: move |event| {
                event.prevent_default();
                on_drag_state.call(false);
            },
            ondrop: move |event| {
                event.prevent_default();
                on_drag_state.call(false);
                let paths = event.files().into_iter().map(|file| file.path()).collect();
                on_files.call(paths);
            },
            div { class: "drop-icon", aria_hidden: "true", "⇩" }
            h2 { if active { "Release to convert" } else { "Drop images here" } }
            p { "or choose images from your computer" }
            button {
                class: "primary",
                r#type: "button",
                onclick: move |_| on_pick.call(()),
                "Add Images"
            }
        }
    }
}
