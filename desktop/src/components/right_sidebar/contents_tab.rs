use dioxus::document;
use dioxus::prelude::*;

use crate::markdown::HeadingInfo;

#[component]
pub fn ContentsTab(headings: Vec<HeadingInfo>) -> Element {
    rsx! {
        div {
            class: "contents-tab",

            if headings.is_empty() {
                div {
                    class: "contents-empty",
                    "No headings found"
                }
            } else {
                ul {
                    class: "contents-list",
                    for heading in headings.iter() {
                        HeadingItem { heading: heading.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn HeadingItem(heading: HeadingInfo) -> Element {
    let id = heading.id.clone();
    let level = heading.level;

    rsx! {
        li {
            class: "contents-item",
            "data-level": "{level}",

            button {
                class: "contents-item-button",
                onclick: move |_| {
                    let id = id.clone();
                    spawn(async move {
                        let js = format!(
                            r#"
                            (() => {{
                                const el = document.getElementById('{}');
                                if (el) {{
                                    el.scrollIntoView({{ behavior: 'smooth', block: 'start' }});
                                }}
                            }})();
                            "#,
                            id
                        );
                        let _ = document::eval(&js).await;
                    });
                },
                "{heading.text}"
            }
        }
    }
}
