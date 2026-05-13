//! Labelled input/textarea wrapper.

use dioxus::prelude::*;

#[derive(Props, PartialEq, Clone)]
pub struct FormFieldProps {
    pub label: String,
    pub value: Signal<String>,
    #[props(default)]
    pub hint: Option<String>,
    #[props(default)]
    pub placeholder: Option<String>,
    /// "text" | "password" | "email"
    #[props(default)]
    pub kind: Option<String>,
    #[props(default)]
    pub autocomplete: Option<String>,
    #[props(default)]
    pub required: bool,
    #[props(default)]
    pub rows: Option<u32>,
}

#[component]
pub fn FormField(props: FormFieldProps) -> Element {
    let id = props
        .label
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>();
    let id = format!("f-{id}");
    let input_cls =
        "w-full bg-bg-deep border border-border-strong rounded-sm px-3 py-2 text-[13px] text-text focus:outline-none focus:border-accent placeholder:text-text-faint";

    let placeholder = props.placeholder.clone().unwrap_or_default();
    let mut value = props.value;
    let value_str = value.read().clone();

    rsx! {
        label { r#for: "{id}", class: "block mb-3.5",
            div { class: "flex justify-between items-baseline mb-1.5",
                span { class: "text-[11px] text-text-muted", "{props.label}" }
                if let Some(h) = props.hint.as_ref() { span { class: "text-[10px] text-text-faint", "{h}" } }
            }
            if let Some(rows) = props.rows {
                textarea {
                    id: "{id}",
                    rows: "{rows}",
                    value: "{value_str}",
                    placeholder: "{placeholder}",
                    class: "{input_cls} font-mono resize-none",
                    oninput: move |e| value.set(e.value()),
                }
            } else {
                input {
                    id: "{id}",
                    r#type: props.kind.as_deref().unwrap_or("text"),
                    value: "{value_str}",
                    placeholder: "{placeholder}",
                    autocomplete: props.autocomplete.as_deref(),
                    required: props.required,
                    class: "{input_cls}",
                    oninput: move |e| value.set(e.value()),
                }
            }
        }
    }
}
