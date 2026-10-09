use dioxus::prelude::*;

#[component]
pub fn ProgressPanel(
    processed: usize,
    total: usize,
    completed: usize,
    skipped: usize,
    failed: usize,
    cancelled: usize,
    percentage: u8,
    on_cancel: EventHandler<()>,
) -> Element {
    rsx! {
        section { class: "progress-panel", aria_live: "polite",
            div { class: "progress-heading",
                strong { "Converting images" }
                span { "{percentage}%" }
            }
            progress { max: "100", value: "{percentage}", aria_label: "Overall conversion progress" }
            div { class: "progress-meta",
                span { "{processed} of {total} processed" }
                span { "✓ {completed}  ↷ {skipped}  ! {failed}  × {cancelled}" }
            }
            button { class: "danger subtle", onclick: move |_| on_cancel.call(()), "Cancel" }
        }
    }
}
