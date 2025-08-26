use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow, Button, ComboBoxText, Label, ScrolledWindow, TextView, TextBuffer, TextTag, TextTagTable, Orientation, PolicyType, WrapMode, Align};
use gtk4::Box as GtkBox;
use glib::spawn_future_local;

use crate::api;
use crate::state::AppState;
use crate::types::ChatMessage;

pub fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Ollama Chat")
        .default_width(900)
        .default_height(700)
        .build();

    // Apply minimal CSS for larger fonts and spacing
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data(
        r#"
        window {
            font-size: 16px;
        }
        
        .chat-text {
            font-size: 16px;
            padding: 24px;
        }
        
        .input-text {
            font-size: 16px;
            padding: 16px;
        }
        
        button {
            font-size: 16px;
            padding: 16px 24px;
        }
        
        combobox {
            font-size: 16px;
        }
        "#
    );
    
    gtk4::style_context_add_provider_for_display(
        &gtk4::prelude::WidgetExt::display(&window),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Main container with padding
    let main_container = GtkBox::new(Orientation::Vertical, 24);
    main_container.set_margin_top(24);
    main_container.set_margin_bottom(24);
    main_container.set_margin_start(24);
    main_container.set_margin_end(24);

    // Chat display area
    let chat_scroll = ScrolledWindow::new();
    chat_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    chat_scroll.set_vexpand(true);
    
    let (chat_view, chat_buffer) = create_chat_view();
    chat_scroll.set_child(Some(&chat_view));

    // Input area
    let input_container = GtkBox::new(Orientation::Vertical, 16);
    
    let input_area_container = GtkBox::new(Orientation::Horizontal, 16);
    
    let input_scroll = ScrolledWindow::new();
    input_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    input_scroll.set_max_content_height(150);
    input_scroll.set_propagate_natural_height(true);
    input_scroll.set_hexpand(true);
    
    let input_view = TextView::new();
    input_view.add_css_class("input-text");
    input_view.set_wrap_mode(WrapMode::WordChar);
    input_view.set_accepts_tab(false);
    let input_buffer = input_view.buffer();
    input_scroll.set_child(Some(&input_view));
    
    let send_button = Button::with_label("Send");
    send_button.set_valign(Align::End);
    
    input_area_container.append(&input_scroll);
    input_area_container.append(&send_button);

    // Bottom controls
    let controls_container = GtkBox::new(Orientation::Horizontal, 16);
    
    let model_label = Label::new(Some("Model:"));
    let model_combo = ComboBoxText::new();
    let status_label = Label::new(Some("Ready"));
    status_label.set_hexpand(true);
    status_label.set_halign(Align::End);

    controls_container.append(&model_label);
    controls_container.append(&model_combo);
    controls_container.append(&status_label);

    input_container.append(&input_area_container);
    input_container.append(&controls_container);

    // Assemble main UI
    main_container.append(&chat_scroll);
    main_container.append(&input_container);
    window.set_child(Some(&main_container));

    // Initialize app state
    let app_state = AppState::default();

    // Load available models
    load_models(model_combo.clone(), status_label.clone(), app_state.clone());

    // Set up event handlers
    setup_send_handler(
        send_button.clone(),
        input_buffer,
        chat_buffer,
        model_combo,
        status_label,
        app_state,
    );
    
    setup_keyboard_shortcut(input_view, send_button);

    window.present();
}

fn create_chat_view() -> (TextView, TextBuffer) {
    let chat_view = TextView::new();
    chat_view.set_editable(false);
    chat_view.set_cursor_visible(false);
    chat_view.set_wrap_mode(WrapMode::WordChar);
    chat_view.add_css_class("chat-text");
    
    let chat_buffer = TextBuffer::new(None);
    chat_view.set_buffer(Some(&chat_buffer));
    
    (chat_view, chat_buffer)
}

fn load_models(combo: ComboBoxText, status_label: Label, app_state: AppState) {
    status_label.set_text("Loading models...");
    
    let combo_weak = combo.downgrade();
    let status_weak = status_label.downgrade();
    
    spawn_future_local(async move {
        match api::fetch_models(&app_state.ollama_url).await {
            Ok(models) => {
                if let (Some(combo), Some(status_label)) = (combo_weak.upgrade(), status_weak.upgrade()) {
                    combo.remove_all();
                    for model in models {
                        combo.append_text(&model.name);
                    }
                    if combo.active().is_none() && combo.model().unwrap().iter_n_children(None) > 0 {
                        combo.set_active(Some(0));
                    }
                    status_label.set_text("Ready");
                }
            }
            Err(e) => {
                if let Some(status_label) = status_weak.upgrade() {
                    status_label.set_text(&format!("Error loading models: {}", e));
                }
            }
        }
    });
}

fn setup_send_handler(
    send_button: Button,
    input_buffer: TextBuffer,
    chat_buffer: TextBuffer,
    model_combo: ComboBoxText,
    status_label: Label,
    app_state: AppState,
) {
    send_button.connect_clicked(move |_| {
        let start_iter = input_buffer.start_iter();
        let end_iter = input_buffer.end_iter();
        let text = input_buffer.text(&start_iter, &end_iter, false);
        
        if text.trim().is_empty() {
            return;
        }
        
        let selected_model = model_combo.active_text();
        if selected_model.is_none() {
            status_label.set_text("Please select a model first");
            return;
        }
        
        let model = selected_model.unwrap().to_string();
        input_buffer.delete(&mut input_buffer.start_iter(), &mut input_buffer.end_iter());
        
        send_message(
            text.to_string(),
            model,
            chat_buffer.clone(),
            status_label.clone(),
            app_state.clone(),
        );
    });
}

fn setup_keyboard_shortcut(input_view: TextView, send_button: Button) {
    let input_controller = gtk4::EventControllerKey::new();
    input_controller.connect_key_pressed(move |_, key, _, modifier| {
        if key == gtk4::gdk::Key::Return && modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
            send_button.emit_clicked();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    input_view.add_controller(input_controller);
}

fn send_message(
    message: String,
    model: String,
    chat_buffer: TextBuffer,
    status_label: Label,
    app_state: AppState,
) {
    // Add user message to conversation
    {
        let mut conversation = app_state.conversation.lock().unwrap();
        conversation.push(ChatMessage {
            role: "user".to_string(),
            content: message.clone(),
        });
    }

    append_to_chat(&chat_buffer, "You", &message);
    status_label.set_text("Sending message...");

    let buffer_weak = chat_buffer.downgrade();
    let status_weak = status_label.downgrade();
    
    spawn_future_local(async move {
        match api::send_chat_request(&app_state.ollama_url, &model, &app_state.conversation).await {
            Ok(response_text) => {
                // Add assistant response to conversation
                {
                    let mut conversation = app_state.conversation.lock().unwrap();
                    conversation.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: response_text.clone(),
                    });
                }
                
                if let (Some(chat_buffer), Some(status_label)) = (buffer_weak.upgrade(), status_weak.upgrade()) {
                    append_to_chat(&chat_buffer, "Assistant", &response_text);
                    status_label.set_text("Ready");
                }
            }
            Err(e) => {
                if let Some(status_label) = status_weak.upgrade() {
                    status_label.set_text(&format!("Error: {}", e));
                }
            }
        }
    });
}

fn append_to_chat(buffer: &TextBuffer, sender: &str, message: &str) {
    let mut end_iter = buffer.end_iter();
    
    // Add spacing if buffer is not empty
    if buffer.char_count() > 0 {
        buffer.insert(&mut end_iter, "\n\n");
        end_iter = buffer.end_iter();
    }
    
    // Add sender label and message
    buffer.insert(&mut end_iter, &format!("{}:\n{}", sender, message));
}