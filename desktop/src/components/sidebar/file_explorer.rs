use dioxus::document;
use dioxus::prelude::*;
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;

use crate::components::icon::{Icon, IconName};
use crate::state::AppState;
use crate::utils::file::is_markdown_file;
use crate::watcher::FILE_WATCHER;

// Sort entries: directories first, then files, both alphabetically
fn sort_entries(items: &mut [PathBuf]) {
    items.sort_by(|a, b| {
        let a_is_dir = a.is_dir();
        let b_is_dir = b.is_dir();

        match (a_is_dir, b_is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });
}

// Read and sort directory entries
fn read_sorted_entries(path: &PathBuf) -> Vec<PathBuf> {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut items: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
            sort_entries(&mut items);
            items
        }
        Err(err) => {
            tracing::error!("Failed to read directory {:?}: {}", path, err);
            vec![]
        }
    }
}

#[component]
pub fn FileExplorer() -> Element {
    let state = use_context::<AppState>();
    let root_directory = state.directory.read().clone();

    // Refresh counter to force DirectoryTree re-render
    let refresh_counter = use_signal(|| 0u32);

    // Watch directory for file system changes
    use_directory_watcher(root_directory.clone(), refresh_counter);

    rsx! {
        div {
            class: "file-explorer",
            key: "{refresh_counter}",

            if let Some(root) = root_directory {
                ParentNavigation { current_dir: root.clone(), refresh_counter }
                DirectoryTree { path: root, refresh_counter }
            } else {
                div {
                    class: "file-explorer-empty",
                    "No directory open"
                }
            }
        }
    }
}

#[component]
fn ParentNavigation(current_dir: PathBuf, mut refresh_counter: Signal<u32>) -> Element {
    let mut state = use_context::<AppState>();
    let show_all_files = state.sidebar.read().show_all_files;

    let has_parent = current_dir.parent().is_some();

    // Get current directory name
    let dir_name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("..")
        .to_string();

    // Copy feedback state
    let mut is_copied = use_signal(|| false);

    // Reload state for animation
    let is_reloading = use_signal(|| false);
    let mut is_reloading_write = is_reloading;

    let on_reload = {
        let current_dir = current_dir.clone();
        move |evt: Event<MouseData>| {
            evt.stop_propagation();

            // Set reloading state for animation
            is_reloading_write.set(true);

            // Increment counter to force DirectoryTree re-render
            refresh_counter.set(refresh_counter() + 1);

            // Reset reloading state after animation
            let current_dir = current_dir.clone();
            spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
                is_reloading_write.set(false);
                tracing::trace!(?current_dir, "Directory reloaded");
            });
        }
    };

    rsx! {
        div {
            class: "sidebar-header",

            // Parent directory navigation or root indicator
            if has_parent {
                div {
                    class: "sidebar-header-nav",
                    onclick: {
                        let current_dir = current_dir.clone();
                        move |_| {
                            if let Some(parent) = current_dir.parent() {
                                state.set_root_directory(parent.to_path_buf());
                            }
                        }
                    },

                    div {
                        class: "sidebar-header-content",
                        Icon {
                            name: IconName::ChevronLeft,
                            size: 16,
                            class: "sidebar-header-icon",
                        }
                        span {
                            class: "sidebar-header-label",
                            "{dir_name}"
                        }

                        // Action buttons (copy & reload) - shown on hover
                        div {
                            class: "sidebar-header-actions",

                            // Copy path button
                            button {
                                class: "sidebar-action-button copy-button",
                                class: if *is_copied.read() { "copied" },
                                title: "Copy directory path",
                                onclick: {
                                    let current_dir = current_dir.clone();
                                    move |evt: Event<MouseData>| {
                                        evt.stop_propagation();
                                        let path_str = current_dir.to_string_lossy().to_string();
                                        let escaped = path_str.replace('\\', "\\\\").replace('`', "\\`");
                                        spawn(async move {
                                            let js = format!("navigator.clipboard.writeText(`{}`)", escaped);
                                            let _ = document::eval(&js).await;
                                            is_copied.set(true);
                                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                            is_copied.set(false);
                                        });
                                    }
                                },
                                Icon {
                                    name: if *is_copied.read() { IconName::Check } else { IconName::Copy },
                                    size: 14,
                                }
                            }

                            // Reload button
                            button {
                                class: "sidebar-action-button reload-button",
                                class: if *is_reloading.read() { "reloading" },
                                title: "Reload file explorer",
                                onclick: on_reload,
                                Icon {
                                    name: IconName::Refresh,
                                    size: 14,
                                }
                            }
                        }
                    }
                }
            } else {
                // Show root indicator when at filesystem root
                div {
                    class: "sidebar-header-nav root-indicator",

                    div {
                        class: "sidebar-header-content",
                        Icon {
                            name: IconName::Server,
                            size: 16,
                            class: "sidebar-header-icon",
                        }
                        span {
                            class: "sidebar-header-label",
                            "/"
                        }

                        // Action buttons (copy & reload) - shown on hover
                        div {
                            class: "sidebar-header-actions",

                            // Copy path button
                            button {
                                class: "sidebar-action-button copy-button",
                                class: if *is_copied.read() { "copied" },
                                title: "Copy directory path",
                                onclick: {
                                    let current_dir = current_dir.clone();
                                    move |evt: Event<MouseData>| {
                                        evt.stop_propagation();
                                        let path_str = current_dir.to_string_lossy().to_string();
                                        let escaped = path_str.replace('\\', "\\\\").replace('`', "\\`");
                                        spawn(async move {
                                            let js = format!("navigator.clipboard.writeText(`{}`)", escaped);
                                            let _ = document::eval(&js).await;
                                            is_copied.set(true);
                                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                            is_copied.set(false);
                                        });
                                    }
                                },
                                Icon {
                                    name: if *is_copied.read() { IconName::Check } else { IconName::Copy },
                                    size: 14,
                                }
                            }

                            // Reload button
                            button {
                                class: "sidebar-action-button reload-button",
                                class: if *is_reloading.read() { "reloading" },
                                title: "Reload file explorer",
                                onclick: on_reload,
                                Icon {
                                    name: IconName::Refresh,
                                    size: 14,
                                }
                            }
                        }
                    }
                }
            }

            // Toolbar buttons container (visibility toggle only)
            div {
                class: "sidebar-header-toolbar",

                // File visibility toggle button
                button {
                    class: "sidebar-header-toolbar-button",
                    title: if show_all_files { "Hide non-markdown files" } else { "Show all files" },
                    onclick: move |_| {
                        state.sidebar.write().show_all_files = !show_all_files;
                    },
                    Icon {
                        name: if show_all_files { IconName::Eye } else { IconName::EyeOff },
                        size: 20,
                    }
                }
            }
        }
    }
}

#[component]
fn DirectoryTree(path: PathBuf, refresh_counter: Signal<u32>) -> Element {
    let entries = read_sorted_entries(&path);

    rsx! {
        div {
            class: "sidebar-tree",
            key: "{refresh_counter}",
            for entry in entries {
                FileTreeNode { path: entry, depth: 0, refresh_counter }
            }
        }
    }
}

#[component]
fn FileTreeNode(path: PathBuf, depth: usize, refresh_counter: Signal<u32>) -> Element {
    let mut state = use_context::<AppState>();

    let is_dir = path.is_dir();
    let is_expanded = state.sidebar.read().expanded_dirs.contains(&path);
    let show_all_files = state.sidebar.read().show_all_files;

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let is_markdown = !is_dir && is_markdown_file(&path);

    // Hide non-markdown files if show_all_files is disabled
    if !show_all_files && !is_dir && !is_markdown {
        return rsx! {};
    }

    let current_tab = state.current_tab();
    let is_active = current_tab
        .and_then(|tab| tab.file().map(|f| f == path))
        .unwrap_or(false);

    let indent_style = format!("padding-left: {}px", depth * 20);

    // Copy feedback state
    let mut is_copied = use_signal(|| false);

    rsx! {
        div {
            class: "sidebar-tree-node",
            class: if is_active { "active" },

            div {
                class: "sidebar-tree-node-content",
                style: "{indent_style}",

                // Directory: chevron toggles expansion, folder+label changes root
                if is_dir {
                    // Chevron: click to expand/collapse
                    span {
                        class: "sidebar-tree-chevron-wrapper",
                        onclick: {
                            let path = path.clone();
                            move |evt| {
                                evt.stop_propagation();
                                state.toggle_directory_expansion(&path);
                            }
                        },
                        Icon {
                            name: if is_expanded { IconName::ChevronDown } else { IconName::ChevronRight },
                            size: 16,
                            class: "sidebar-tree-chevron",
                        }
                    }

                    // Folder icon + label: click to set as root directory
                    span {
                        class: "sidebar-tree-dir-link",
                        onclick: {
                            let path = path.clone();
                            move |evt| {
                                evt.stop_propagation();
                                state.set_root_directory(&path);
                            }
                        },
                        Icon {
                            name: if is_expanded { IconName::FolderOpen } else { IconName::Folder },
                            size: 16,
                            class: "sidebar-tree-icon",
                        }
                        span {
                            class: "sidebar-tree-label",
                            "{name}"
                        }
                    }
                } else {
                    // File: spacer + icon + label, click to open
                    span { class: "sidebar-tree-spacer" }
                    span {
                        class: "sidebar-tree-file-link",
                        onclick: {
                            let path = path.clone();
                            move |evt| {
                                evt.stop_propagation();
                                state.open_file(&path);
                            }
                        },
                        Icon {
                            name: IconName::File,
                            size: 16,
                            class: "sidebar-tree-icon",
                        }
                        span {
                            class: "sidebar-tree-label",
                            class: if !is_markdown { "disabled" },
                            "{name}"
                        }
                    }
                }

                // Copy path button
                button {
                    class: "sidebar-tree-copy-button",
                    class: if *is_copied.read() { "copied" },
                    title: "Copy full path",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        let path_str = path.to_string_lossy().to_string();
                        // Escape backticks and backslashes for JavaScript
                        let escaped = path_str.replace('\\', "\\\\").replace('`', "\\`");
                        spawn(async move {
                            let js = format!("navigator.clipboard.writeText(`{}`)", escaped);
                            let _ = document::eval(&js).await;
                            // Show success feedback
                            is_copied.set(true);
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            is_copied.set(false);
                        });
                    },
                    Icon {
                        name: if *is_copied.read() { IconName::Check } else { IconName::Copy },
                        size: 12,
                    }
                }
            }

            // Expanded directory children
            if is_dir && is_expanded {
                {
                    let children = read_sorted_entries(&path);
                    rsx! {
                        div {
                            key: "{refresh_counter}",
                            for child in children {
                                FileTreeNode { path: child, depth: depth + 1, refresh_counter }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Hook to watch a directory for file system changes and trigger refresh
fn use_directory_watcher(directory: Option<PathBuf>, mut refresh_counter: Signal<u32>) {
    use_effect(use_reactive!(|directory| {
        spawn(async move {
            let Some(dir) = directory else {
                return;
            };

            // Start watching the directory
            let Ok(mut watcher) = FILE_WATCHER.watch_directory(dir.clone()).await else {
                tracing::error!("Failed to start directory watcher for {:?}", dir);
                return;
            };

            tracing::debug!("Directory watcher started for {:?}", dir);

            // Listen for changes and trigger refresh
            while watcher.recv().await.is_some() {
                tracing::trace!(?dir, "Directory changed, triggering refresh");
                refresh_counter.set(refresh_counter() + 1);
            }

            // Cleanup when effect is re-run or component unmounts
            let _ = FILE_WATCHER.unwatch_directory(dir).await;
        });
    }));
}
