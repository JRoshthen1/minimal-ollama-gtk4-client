use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow, Button, DropDown, Label, ScrolledWindow, TextView, TextBuffer, StringList, Orientation, PolicyType, WrapMode, Align, CheckButton};
use gtk4::Box as GtkBox;
use glib::{spawn_future_local, clone};
use std::sync::{OnceLock, Arc, Mutex};
use tokio::runtime::Runtime;

use crate::api;
use crate::state::AppState;
use crate::types::ChatMessage;
use crate::markdown_processor::MarkdownProcessor;

// enum to track button state
#[derive(Clone, PartialEq)]
enum ButtonState {
    Send,
    Stop,
}

pub fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Ollama Chat")
        .default_width(900)
        .default_height(700)
        .build();

    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(
        r#"
        window {
            font-size: 16px;
        }
        
        .chat-text {
            font-size: 18px;
            padding: 24px;
            border-radius: 12px;
        }
        
        .input-text {
            font-size: 16px;
            padding: 16px;
            border-radius: 12px;
        }
        
        button {
            font-size: 16px;
            padding: 16px 24px;
            border-radius: 12px;
        }
        
        .stop-button {
            background-color: #dc3545;
            color: white;
        }
        
        .send-button {
            background-color: #007bff;
            color: white;
        }
        
        dropdown {
            font-size: 16px;
            border-radius: 12px;
        }
        
        checkbutton {
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
    
    let (chat_view, chat_buffer, markdown_processor) = create_chat_view();
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
    
    // Single button that changes between Send and Stop
    let action_button = Button::with_label("Send");
    action_button.add_css_class("send-button");
    action_button.set_valign(Align::End);
    
    // Track button state
    let button_state = Arc::new(Mutex::new(ButtonState::Send));
    
    input_area_container.append(&input_scroll);
    input_area_container.append(&action_button);

    // Bottom controls
    let controls_container = GtkBox::new(Orientation::Horizontal, 16);
    
    let model_label = Label::new(Some("Model:"));
    
    // Create StringList to hold model names
    let model_list = StringList::new(&[]);
    let model_dropdown = DropDown::new(Some(model_list.clone()), None::<gtk4::Expression>);
    
    // Add thinking checkbox
    let thinking_checkbox = CheckButton::with_label("Think");
    thinking_checkbox.set_tooltip_text(Some("Enable thinking mode for reasoning models (e.g., deepseek-r1)"));
    
    let status_label = Label::new(Some("Ready"));
    status_label.set_hexpand(true);
    status_label.set_halign(Align::End);

    controls_container.append(&model_label);
    controls_container.append(&model_dropdown);
    controls_container.append(&thinking_checkbox);
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
    load_models(model_list.clone(), model_dropdown.clone(), status_label.clone(), app_state.clone());

    // Set up event handlers
    setup_action_button_handler(
        action_button.clone(),
        button_state.clone(),
        input_buffer,
        chat_buffer.clone(),
        model_dropdown,
        model_list,
        thinking_checkbox.clone(),
        status_label.clone(),
        app_state.clone(),
        markdown_processor,
        chat_scroll.clone(),
    );
    
    // Connect thinking checkbox to app state
    setup_thinking_checkbox_handler(thinking_checkbox, app_state.clone());
    
    setup_keyboard_shortcut(input_view, action_button.clone(), button_state.clone());

    window.present();
}

// Helper function to update button appearance
fn update_button_state(button: &Button, state: ButtonState) {
    match state {
        ButtonState::Send => {
            button.set_label("Send");
            button.remove_css_class("stop-button");
            button.add_css_class("send-button");
        }
        ButtonState::Stop => {
            button.set_label("Stop");
            button.remove_css_class("send-button");
            button.add_css_class("stop-button");
        }
    }
}

fn setup_action_button_handler(
    action_button: Button,
    button_state: Arc<Mutex<ButtonState>>,
    input_buffer: TextBuffer,
    chat_buffer: TextBuffer,
    model_dropdown: DropDown,
    model_list: StringList,
    thinking_checkbox: CheckButton,
    status_label: Label,
    app_state: AppState,
    markdown_processor: Arc<MarkdownProcessor>,
    chat_scroll: ScrolledWindow,
) {
    action_button.connect_clicked(clone!(
        #[weak] input_buffer,
        #[weak] chat_buffer,
        #[weak] model_dropdown,
        #[weak] model_list,
        #[weak] thinking_checkbox,
        #[weak] status_label,
        #[weak] action_button,
        #[strong] button_state,
        #[strong] markdown_processor,
        move |_| {
            let current_state = {
                let state = button_state.lock().unwrap();
                state.clone()
            };
            
            match current_state {
                ButtonState::Send => {
                    // Handle send logic
                    let start_iter = input_buffer.start_iter();
                    let end_iter = input_buffer.end_iter();
                    let text = input_buffer.text(&start_iter, &end_iter, false);
                    
                    if text.trim().is_empty() {
                        return;
                    }
                    
                    // Get selected model
                    let selected_idx = model_dropdown.selected();
                    if selected_idx == gtk4::INVALID_LIST_POSITION {
                        status_label.set_text("Please select a model first");
                        return;
                    }
                    
                    let model = match model_list.string(selected_idx) {
                        Some(m) => m.to_string(),
                        None => {
                            status_label.set_text("Invalid model selection");
                            return;
                        }
                    };
                    
                    // Get thinking checkbox state
                    let thinking_enabled = thinking_checkbox.is_active();
                    
                    input_buffer.delete(&mut input_buffer.start_iter(), &mut input_buffer.end_iter());
                    
                    // Change button to Stop state
                    {
                        let mut state = button_state.lock().unwrap();
                        *state = ButtonState::Stop;
                    }
                    update_button_state(&action_button, ButtonState::Stop);
                    
                    send_message(
                        text.to_string(),
                        model,
                        thinking_enabled,
                        chat_buffer.clone(),
                        chat_scroll.clone(),
                        status_label.clone(),
                        action_button.clone(),
                        button_state.clone(),
                        app_state.clone(),
                        markdown_processor.clone(),
                    );
                }
                ButtonState::Stop => {
                    // Handle stop logic
                    let mut handle = app_state.current_request_handle.lock().unwrap();
                    if let Some(task) = handle.take() {
                        task.abort();
                        status_label.set_text("Generation stopped");
                        
                        // Change button back to Send state
                        {
                            let mut state = button_state.lock().unwrap();
                            *state = ButtonState::Send;
                        }
                        update_button_state(&action_button, ButtonState::Send);
                    }
                }
            }
        }
    ));
}

fn setup_keyboard_shortcut(input_view: TextView, action_button: Button, button_state: Arc<Mutex<ButtonState>>) {
    let input_controller = gtk4::EventControllerKey::new();
    input_controller.connect_key_pressed(clone!(
        #[weak] action_button,
        #[strong] button_state,
        #[upgrade_or] glib::Propagation::Proceed,
        move |_, key, _, modifier| {
            if key == gtk4::gdk::Key::Return && modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                // Only trigger if in Send state (don't allow Ctrl+Enter to stop)
                let current_state = {
                    let state = button_state.lock().unwrap();
                    state.clone()
                };
                
                if current_state == ButtonState::Send {
                    action_button.emit_clicked();
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    ));
    input_view.add_controller(input_controller);
}

fn send_message(
    message: String,
    model: String,
    thinking_enabled: bool,
    chat_buffer: TextBuffer,
    chat_scroll: ScrolledWindow,
    status_label: Label,
    action_button: Button,
    button_state: Arc<Mutex<ButtonState>>,
    app_state: AppState,
    markdown_processor: Arc<MarkdownProcessor>
) {
    // Add user message to conversation
    {
        let mut conversation = app_state.conversation.lock().unwrap();
        conversation.push(ChatMessage {
            role: "user".to_string(),
            content: message.clone(),
        });
    }

    append_to_chat(&chat_buffer, "You", &message, &markdown_processor);
    
    // Add placeholder for assistant message
    let mut end_iter = chat_buffer.end_iter();
    let assistant_prefix = if thinking_enabled {
        "\n\nAssistant (thinking enabled):\n"
    } else {
        "\n\nAssistant:\n"
    };
    chat_buffer.insert(&mut end_iter, assistant_prefix);
    
    // Create mark where assistant response will be inserted
    let assistant_start_mark = chat_buffer.create_mark(None, &chat_buffer.end_iter(), true);
    
    let status_text = if thinking_enabled {
        "Assistant is thinking..."
    } else {
        "Assistant is typing..."
    };
    status_label.set_text(status_text);

    // Create channels for streaming communication
    let (stream_sender, stream_receiver) = async_channel::bounded::<String>(50);
    let (result_sender, result_receiver) = async_channel::bounded::<Result<String, Box<dyn std::error::Error + Send + Sync>>>(1);
    
    // Spawn tokio task for API streaming
    let app_state_clone = app_state.clone();
    let model_clone = model.clone();
    let task_handle = runtime().spawn(async move {
        let result = api::send_chat_request_streaming(
            &app_state_clone.ollama_url,
            &model_clone,
            &app_state_clone.conversation,
            stream_sender,
            thinking_enabled,
        ).await;
        let _ = result_sender.send(result).await;
    });

    // Store the handle for potential cancellation
    {
        let mut handle = app_state.current_request_handle.lock().unwrap();
        *handle = Some(task_handle);
    }   
    
    // Handle streaming updates on the main loop
    spawn_future_local(clone!(
        #[weak] chat_buffer,
        #[weak] assistant_start_mark,
        #[weak] chat_scroll,
        async move {
            let mut accumulated_text = String::new();
            
            while let Ok(token_batch) = stream_receiver.recv().await {
                accumulated_text.push_str(&token_batch);
                
                // Update UI with accumulated text (plain text during streaming)
                let mut start_iter = chat_buffer.iter_at_mark(&assistant_start_mark);
                let mut end_iter = chat_buffer.end_iter();
                
                // Replace content from mark to end
                chat_buffer.delete(&mut start_iter, &mut end_iter);
                let mut insert_iter = chat_buffer.iter_at_mark(&assistant_start_mark);
                chat_buffer.insert(&mut insert_iter, &accumulated_text);
                
                // Auto-scroll to bottom after each update
                let adjustment = chat_scroll.vadjustment();
                adjustment.set_value(adjustment.upper() - adjustment.page_size());
            }
        }
    ));
    
    // Handle final result - apply markdown formatting when streaming completes
    let app_state_final = app_state.clone();
    spawn_future_local(clone!(
        #[weak] status_label,
        #[weak] chat_buffer,
        #[weak] assistant_start_mark,
        #[weak] action_button,
        #[strong] button_state,
        #[strong] markdown_processor,
        async move {
            if let Ok(result) = result_receiver.recv().await {
                match result {
                    Ok(response_text) => {
                        // Clear the plain streaming text
                        let mut start_iter = chat_buffer.iter_at_mark(&assistant_start_mark);
                        let mut end_iter = chat_buffer.end_iter();
                        chat_buffer.delete(&mut start_iter, &mut end_iter);
                        
                        // Insert formatted markdown text using the shared processor
                        let mut insert_iter = chat_buffer.iter_at_mark(&assistant_start_mark);
                        markdown_processor.insert_formatted_text(&chat_buffer, &response_text, &mut insert_iter);
                        
                        // Add complete response to conversation
                        {
                            let mut conversation = app_state_final.conversation.lock().unwrap();
                            conversation.push(ChatMessage {
                                role: "assistant".to_string(),
                                content: response_text,
                            });
                        }
                        
                        status_label.set_text("Ready");
                    }
                    Err(e) => {
                        status_label.set_text(&format!("Error: {}", e));
                        
                        // Show error in chat
                        let mut start_iter = chat_buffer.iter_at_mark(&assistant_start_mark);
                        let mut end_iter = chat_buffer.end_iter();
                        chat_buffer.delete(&mut start_iter, &mut end_iter);
                        let mut insert_iter = chat_buffer.iter_at_mark(&assistant_start_mark);
                        chat_buffer.insert(&mut insert_iter, &format!("[Error: {}]", e));
                    }
                }
                
                // Change button back to Send state when generation completes
                {
                    let mut state = button_state.lock().unwrap();
                    *state = ButtonState::Send;
                }
                update_button_state(&action_button, ButtonState::Send);
            }
        }
    ));
}

// Helper functions 
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Setting up tokio runtime needs to succeed.")
    })
}

fn create_chat_view() -> (TextView, TextBuffer, Arc<MarkdownProcessor>) {
    let chat_view = TextView::new();
    chat_view.set_editable(false);
    chat_view.set_cursor_visible(false);
    chat_view.set_wrap_mode(WrapMode::WordChar);
    chat_view.add_css_class("chat-text");
    
    let chat_buffer = TextBuffer::new(None);
    chat_view.set_buffer(Some(&chat_buffer));
    
    // Set up markdown formatting tags
    let markdown_processor = Arc::new(MarkdownProcessor::new());
    markdown_processor.setup_tags(&chat_buffer);
    
    (chat_view, chat_buffer, markdown_processor)
}

fn load_models(model_list: StringList, dropdown: DropDown, status_label: Label, app_state: AppState) {
    status_label.set_text("Loading models...");
    
    // Create communication channel
    let (sender, receiver) = async_channel::bounded(1);
    
    // Spawn tokio task for API call
    runtime().spawn(async move {
        let result = api::fetch_models(&app_state.ollama_url).await;
        let _ = sender.send(result).await;
    });
    
    // Handle response on main loop
    spawn_future_local(clone!(
        #[weak] model_list,
        #[weak] dropdown,
        #[weak] status_label,
        async move {
            if let Ok(result) = receiver.recv().await {
                match result {
                    Ok(models) => {
                        // Clear existing items and add new ones
                        model_list.splice(0, model_list.n_items(), 
                            &models.iter().map(|m| m.name.as_str()).collect::<Vec<_>>());
                        
                        // Select first model if available
                        if model_list.n_items() > 0 && dropdown.selected() == gtk4::INVALID_LIST_POSITION {
                            dropdown.set_selected(0);
                        }
                        
                        status_label.set_text("Ready");
                    }
                    Err(e) => {
                        status_label.set_text(&format!("Error loading models: {}", e));
                    }
                }
            }
        }
    ));
}

fn setup_thinking_checkbox_handler(thinking_checkbox: CheckButton, app_state: AppState) {
    thinking_checkbox.connect_toggled(clone!(
        #[strong] app_state,
        move |checkbox| {
            let is_active = checkbox.is_active();
            if let Ok(mut thinking_enabled) = app_state.thinking_enabled.lock() {
                *thinking_enabled = is_active;
            }
        }
    ));
}

fn append_to_chat(buffer: &TextBuffer, sender: &str, message: &str, markdown_processor: &MarkdownProcessor) {
    let mut end_iter = buffer.end_iter();
    
    // Add spacing if buffer is not empty
    if buffer.char_count() > 0 {
        buffer.insert(&mut end_iter, "\n\n");
        end_iter = buffer.end_iter();
    }
    
    // Add sender label
    buffer.insert(&mut end_iter, &format!("{}:\n", sender));
    end_iter = buffer.end_iter();
    
    // Add message - user messages are always plain text
    if sender == "You" {
        buffer.insert(&mut end_iter, message);
    } else {
        // For assistant messages, use markdown formatting
        markdown_processor.insert_formatted_text(buffer, message, &mut end_iter);
    }
}