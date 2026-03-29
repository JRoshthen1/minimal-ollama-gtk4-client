use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use gtk4::Orientation;
use gtk4::Box as GtkBox;
use std::rc::Rc;
use std::cell::RefCell;

use crate::state::{AppState, SharedState};
use crate::ui::{chat, input, controls, handlers};
use crate::config::Config;

pub fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Ollama Chat")
        .default_width(900)
        .default_height(700)
        .build();

    // Initialize shared state (this loads config)
    let shared_state: SharedState = Rc::new(RefCell::new(AppState::default()));

    // Create a single CSS provider that persists for the app lifetime.
    // Settings dialog re-uses this provider to hot-reload CSS on save.
    let css_provider = gtk4::CssProvider::new();
    apply_css(&css_provider, &shared_state.borrow().config);
    gtk4::style_context_add_provider_for_display(
        &gtk4::prelude::WidgetExt::display(&window),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Root: vertical stack — chat, input, toolbar
    let main_container = GtkBox::new(Orientation::Vertical, 0);

    // Create UI components
    let chat_view = chat::create_chat_view();
    let input_area = input::create_input_area();
    let controls_area = controls::create_controls();

    // Content area: chat + input, with consistent margins on all sides
    let content_container = GtkBox::new(Orientation::Vertical, 12);
    content_container.set_vexpand(true);
    content_container.set_hexpand(true);
    content_container.set_margin_top(16);
    content_container.set_margin_bottom(4);
    content_container.set_margin_start(16);
    content_container.set_margin_end(16);

    chat_view.widget().set_vexpand(true);
    chat_view.widget().set_hexpand(true);
    input_area.container.set_vexpand(false);
    input_area.container.set_hexpand(true);

    content_container.append(chat_view.widget());
    content_container.append(&input_area.container);

    // Assemble — content on top, toolbar at bottom spanning full width
    main_container.append(&content_container);
    main_container.append(&controls_area.container);

    window.set_child(Some(&main_container));

    // Setup event handlers
    handlers::setup_handlers(
        shared_state,
        chat_view,
        input_area,
        controls_area,
        window.clone(),
        css_provider,
    );

    window.present();
}

/// Update an existing CSS provider from config. Call this at startup and after settings save.
pub fn apply_css(provider: &gtk4::CssProvider, config: &Config) {
    provider.load_from_string(&generate_css_from_config(config));
}

pub fn generate_css_from_config(config: &Config) -> String {
    format!(
        r#"
        window {{
            font-size: {}px;
        }}

        .input-container, .input-text, .input-text > * {{
            background-color: {};
            border-radius: 12px;
        }}

        .input-text {{
            font-size: {}px;
            margin-left: 12px;
            padding: 12px;
            min-height: 60px;
            color: {};
        }}

        .chat-container, .chat-text, .chat-text > * {{
            background-color: {};
            border-radius: 12px;
        }}

        .chat-text {{
            font-size: {}px;
            padding: 24px;
            color: {};
        }}

        .input-text:focus {{
            border-color: {};
            outline: none;
        }}

        .stop-button {{
            background-color: {};
            color: white;
        }}

        .send-button {{
            background-color: {};
            color: white;
        }}

        .status-label {{
            font-size: 14px;
            color: #4caf50;
        }}

        .status-error {{
            color: #f44336;
        }}

        .status-busy {{
            color: #ff9800;
        }}

        .toolbar {{
            border-top: 1px solid alpha(currentColor, 0.12);
            padding: 2px 0;
        }}

        .toolbar-button,
        .toolbar-button > * {{
            margin: 0 1px;
            padding: 4px 10px;
            min-height: 32px;
            border-radius: 6px;
        }}

        .toolbar-button.active {{
            background-color: alpha({}, 0.15);
            color: {};
        }}

        .selector-list row {{
            border-radius: 6px;
        }}

        .selector-list row:hover {{
            background-color: alpha(currentColor, 0.07);
        }}

        .settings-text-container,
        .settings-text-container > * {{
            background-color: {};
            border-radius: 6px;
        }}

        .settings-text-view {{
            font-size: {}px;
            padding: 6px;
            color: {};
        }}

        .md-table-frame {{
            margin: 4px 0;
        }}

        .md-table-header {{
            border-bottom: 1px solid alpha(currentColor, 0.35);
        }}

        .md-table-cell {{
            border-bottom: 1px solid alpha(currentColor, 0.1);
        }}
        "#,
        config.ui.window_font_size,                    // window font-size
        config.colors.chat_background,                 // input area background
        config.ui.chat_font_size,                      // input font-size
        config.colors.primary_text,                    // input color
        config.colors.chat_background,                 // chat area background
        config.ui.input_font_size,                     // chat font-size
        config.colors.primary_text,                    // chat color
        config.colors.link_text,                       // input focus border
        config.colors.stop_button,                     // stop button background
        config.colors.send_button,                     // send button background
        config.colors.link_text,                       // thinking button active background
        config.colors.link_text,                       // thinking button active icon color
        config.colors.chat_background,                 // settings-text-container background
        config.ui.input_font_size,                     // settings-text-view font-size
        config.colors.primary_text,                    // settings-text-view color
    )
}