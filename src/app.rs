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
    
    // Setup CSS with config
    setup_css(&window, &shared_state.borrow().config);
    
    // Create main container with proper spacing
    let main_container = GtkBox::new(Orientation::Vertical, 12);
    main_container.set_margin_top(16);
    main_container.set_margin_bottom(16);
    main_container.set_margin_start(16);
    main_container.set_margin_end(16);

    // Create UI components
    let chat_view = chat::create_chat_view();
    let input_area = input::create_input_area();
    let controls_area = controls::create_controls();

    // Set proper expansion properties
    // Chat view should expand to fill available space
    chat_view.widget().set_vexpand(true);
    chat_view.widget().set_hexpand(true);
    
    // Input area should not expand vertically but should expand horizontally
    input_area.container.set_vexpand(false);
    input_area.container.set_hexpand(true);
    
    // Controls should not expand
    controls_area.container.set_vexpand(false);
    controls_area.container.set_hexpand(true);

    // Assemble main UI
    main_container.append(chat_view.widget());
    main_container.append(&input_area.container);
    main_container.append(&controls_area.container);
    
    window.set_child(Some(&main_container));

    // Setup event handlers
    handlers::setup_handlers(
        shared_state,
        chat_view,
        input_area,
        controls_area,
    );

    window.present();
}

fn setup_css(window: &ApplicationWindow, config: &Config) {
    let css_provider = gtk4::CssProvider::new();
    
    // Generate CSS from config
    let css_content = generate_css_from_config(config);
    css_provider.load_from_string(&css_content);
    
    gtk4::style_context_add_provider_for_display(
        &gtk4::prelude::WidgetExt::display(window),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn generate_css_from_config(config: &Config) -> String {
    format!(
        r#"
        window {{
            font-size: {}px;
            background-color: {};
        }}
        
        .input-container, .input-text, .input-text > *  {{
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
        
        button {{
            font-size: {}px;
            margin-left: 8px;
            margin-right: 12px;
            border-radius: 8px;
            height: 100%;
        }}
        
        .stop-button {{
            background-color: {};
            color: white;
        }}
        
        .send-button {{
            background-color: {};
            color: white;
        }}
        
        dropdown {{
            font-size: {}px;
            border-radius: 8px;
            min-height: 40px;
        }}
        
        checkbutton {{
            font-size: {}px;
        }}
        
        .status-label {{
            font-size: 14px;
            color: #555;
        }}
        "#,
        config.ui.window_font_size,                    // window font-size
        config.colors.window_background,               // window background
        config.colors.chat_background,                 // chat background
        config.ui.chat_font_size,                      // chat font-size
        config.colors.primary_text,                    // chat color
        config.colors.chat_background,                 // input background (reuse chat)
        config.ui.input_font_size,                     // input font-size
        config.colors.primary_text,                    // input color
        config.colors.link_text,                       // input focus border
        config.ui.window_font_size,                    // button font-size
        config.colors.stop_button,                     // stop button background
        config.colors.send_button,                     // send button background
        config.ui.window_font_size,                    // dropdown font-size
        config.ui.window_font_size,                    // checkbutton font-size
    )
}