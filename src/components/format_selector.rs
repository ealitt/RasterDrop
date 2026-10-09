use crate::models::OutputFormat;
use dioxus::prelude::*;
use std::str::FromStr;

#[component]
pub fn FormatSelector(
    selected: OutputFormat,
    disabled: bool,
    on_change: EventHandler<OutputFormat>,
) -> Element {
    rsx! {
        label { class: "format-selector",
            span { "Convert To" }
            select {
                disabled,
                value: selected.selector_value(),
                aria_label: "Output image format",
                onchange: move |event| {
                    if let Ok(format) = OutputFormat::from_str(&event.value()) {
                        on_change.call(format);
                    }
                },
                for format in OutputFormat::ALL {
                    option { value: format.selector_value(), "{format}" }
                }
            }
        }
    }
}
