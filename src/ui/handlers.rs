use gtk4::prelude::*;
use gtk4::glib::{spawn_future_local, clone};
use gtk4::ApplicationWindow;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

use crate::state::{SharedState, AppResult, AppError, ButtonState};
use crate::ui::{chat::ChatView, input::InputArea, controls::ControlsArea};
use crate::api;

pub fn setup_handlers(
    shared_state: SharedState,
    chat_view: ChatView,
    input_area: InputArea,
    controls_area: ControlsArea,
    window: ApplicationWindow,
    css_provider: gtk4::CssProvider,
) {
    // Load models on startup
    load_models(shared_state.clone(), &controls_area);

    // Load profiles on startup and populate the profile dropdown
    load_profiles(shared_state.clone(), &controls_area);

    // Setup action button handler
    setup_action_button_handler(shared_state.clone(), &chat_view, &input_area, &controls_area);

    // Setup keyboard shortcut
    setup_keyboard_shortcut(&input_area, shared_state.clone());

    // Setup profile dropdown handler
    setup_profile_dropdown_handler(shared_state.clone(), &controls_area, window.clone());

    // Setup settings gear button
    setup_settings_button_handler(shared_state, &controls_area, window, css_provider);
}

fn setup_action_button_handler(
    shared_state: SharedState,
    chat_view: &ChatView,
    input_area: &InputArea,
    controls_area: &ControlsArea,
) {
    let action_button = &input_area.action_button;
    let text_buffer = input_area.text_buffer.clone();
    let chat_view_clone = chat_view.clone();
    let controls_clone = controls_area.clone();
    let button_clone = action_button.clone();
    
    action_button.connect_clicked(clone!(
        #[strong] shared_state,
        #[strong] text_buffer,
        #[strong] chat_view_clone,
        #[strong] controls_clone,
        #[strong] button_clone,
        move |_| {
            if let Err(e) = handle_action_button_click(
                &shared_state,
                &text_buffer,
                &chat_view_clone,
                &controls_clone,
                &button_clone
            ) {
                controls_clone.set_status(&format!("Error: {}", e));
                update_button_state(&shared_state, &button_clone);
            }
        }
    ));
    
    // Initialize button appearance
    update_button_state(&shared_state, action_button);
}

fn handle_action_button_click(
    shared_state: &SharedState,
    text_buffer: &gtk4::TextBuffer,
    chat_view: &ChatView,
    controls: &ControlsArea,
    button: &gtk4::Button,
) -> AppResult<()> {
    let current_state = shared_state.borrow().button_state;
    
    match current_state {
        ButtonState::Send => handle_send_click(shared_state, text_buffer, chat_view, controls, button),
        ButtonState::Stop => handle_stop_click(shared_state, controls, button),
    }
}

fn handle_send_click(
    shared_state: &SharedState,
    text_buffer: &gtk4::TextBuffer,
    chat_view: &ChatView,
    controls: &ControlsArea,
    button: &gtk4::Button,
) -> AppResult<()> {
    // Validate input
    let text = get_input_text(text_buffer)?;
    let model = get_selected_model(controls)?;
    
    // Clear input and start generation
    clear_input(text_buffer);
    set_generating_state(shared_state, controls, button, true);
    
    // Add user message to conversation and chat
    shared_state.borrow_mut().add_user_message(text.clone());
    let config = shared_state.borrow().config.clone();
    chat_view.append_message("You", &text, &config);
    
    // Start streaming
    start_streaming_task(shared_state, chat_view, controls, button, model);
    
    Ok(())
}

fn handle_stop_click(
    shared_state: &SharedState,
    controls: &ControlsArea,
    button: &gtk4::Button,
) -> AppResult<()> {
    shared_state.borrow_mut().abort_current_task();
    update_button_state(shared_state, button);
    controls.set_status("Generation stopped");
    Ok(())
}

fn set_generating_state(
    shared_state: &SharedState,
    controls: &ControlsArea,
    button: &gtk4::Button,
    generating: bool,
) {
    let status = if generating { "Assistant is typing..." } else { "Ready" };
    shared_state.borrow_mut().set_generating(generating);
    update_button_state(shared_state, button);
    controls.set_status(status);
}

fn update_button_state(shared_state: &SharedState, button: &gtk4::Button) {
    let is_generating = shared_state.borrow().is_generating;
    
    if is_generating {
        button.set_label("Stop");
        button.remove_css_class("send-button");
        button.add_css_class("stop-button");
    } else {
        button.set_label("Send");
        button.remove_css_class("stop-button");
        button.add_css_class("send-button");
    }
}

fn get_input_text(text_buffer: &gtk4::TextBuffer) -> AppResult<String> {
    let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
    let text = text.trim();
    
    if text.is_empty() {
        return Err(AppError::Validation("Message cannot be empty".to_string()));
    }
    
    Ok(text.to_string())
}

fn get_selected_model(controls: &ControlsArea) -> AppResult<String> {
    controls.get_selected_model()
        .ok_or_else(|| AppError::Validation("Please select a model first".to_string()))
}

fn clear_input(text_buffer: &gtk4::TextBuffer) {
    text_buffer.delete(&mut text_buffer.start_iter(), &mut text_buffer.end_iter());
}

fn start_streaming_task(
    shared_state: &SharedState,
    chat_view: &ChatView,
    controls: &ControlsArea,
    button: &gtk4::Button,
    model: String,
) {
    let (content_sender, content_receiver) = async_channel::bounded::<String>(100);
    let (result_sender, result_receiver) = async_channel::bounded::<Result<String, crate::api::ApiError>>(1);
    
    // Extract data from shared state for API call.
    // Active profile overrides streaming params and context window; falls back to global config.
    // Only send the most recent `max_context_messages` turns to stay within the model's
    // context window. Prepend the system prompt (if set) as the first message.
    let (messages, ollama_url, batch_size, batch_timeout_ms, temperature) = {
        let state = shared_state.borrow();
        let (max, batch_size, batch_timeout_ms, temperature) =
            if let Some(ref p) = state.active_profile {
                (p.max_context_messages, p.batch_size, p.batch_timeout_ms, p.temperature)
            } else {
                (
                    state.config.ollama.max_context_messages,
                    state.config.streaming.batch_size,
                    state.config.streaming.batch_timeout_ms,
                    None,
                )
            };
        let skip = state.conversation.len().saturating_sub(max);
        let mut msgs: Vec<_> = state.conversation[skip..].to_vec();
        if let Some(ref prompt) = state.system_prompt {
            msgs.insert(0, crate::types::ChatMessage {
                role: "system".to_string(),
                content: prompt.clone(),
            });
        }
        (msgs, state.ollama_url.clone(), batch_size, batch_timeout_ms, temperature)
    };

    // Spawn API task
    let task_handle = runtime().spawn(async move {
        let result = api::send_chat_request_streaming(
            &ollama_url,
            &model,
            messages,
            content_sender,
            batch_size,
            batch_timeout_ms,
            temperature,
        ).await;
        let _ = result_sender.send(result).await;
    });
    
    // Store task handle
    shared_state.borrow_mut().current_task = Some(task_handle);
    
    // Setup streaming handlers
    setup_streaming_handlers(
        shared_state,
        chat_view,
        controls,
        button,
        content_receiver,
        result_receiver
    );
}

fn setup_streaming_handlers(
    shared_state: &SharedState,
    chat_view: &ChatView,
    controls: &ControlsArea,
    button: &gtk4::Button,
    content_receiver: async_channel::Receiver<String>,
    result_receiver: async_channel::Receiver<Result<String, crate::api::ApiError>>,
) {
    // Insert styled "Assistant:" header and get a mark for streaming content
    let config = shared_state.borrow().config.clone();
    let response_mark = chat_view.begin_assistant_response(&config);
    
    // Handle response content updates with live markdown
    let response_mark_clone = response_mark.clone();
    let chat_view_content = chat_view.clone();
    
    let shared_state_streaming = shared_state.clone();
    
    spawn_future_local(async move {
        let mut accumulated_content = String::new();
        
        while let Ok(content_batch) = content_receiver.recv().await {
            accumulated_content.push_str(&content_batch);
            let config = shared_state_streaming.borrow().config.clone();
            chat_view_content.update_streaming_markdown(&response_mark_clone, &accumulated_content, &config);
        }
    });
    
    // Handle final result
    let shared_state_final = shared_state.clone();
    let controls_final = controls.clone();
    let button_final = button.clone();
    let chat_view_final = chat_view.clone();
    

    spawn_future_local(async move {
        if let Ok(result) = result_receiver.recv().await {
            match result {
                Ok(response_text) => {
                    // Apply final markdown formatting
                    let config = shared_state_final.borrow().config.clone();
                    chat_view_final.insert_formatted_at_mark(&response_mark, &response_text, &config);

                    // Update conversation state
                    shared_state_final.borrow_mut().add_assistant_message(response_text);
                    set_generating_state(&shared_state_final, &controls_final, &button_final, false);
                }
                Err(e) => {
                    // Display error in response section
                    let error_message = format!("**Error:** {}", e);
                    let config = shared_state_final.borrow().config.clone();
                    chat_view_final.insert_formatted_at_mark(&response_mark, &error_message, &config);
                    
                    // Update state
                    set_generating_state(&shared_state_final, &controls_final, &button_final, false);
                    controls_final.set_status(&format!("Error: {}", e));
                }
            }
            
            chat_view_final.scroll_to_bottom();
        }
    });
}

fn setup_keyboard_shortcut(input_area: &InputArea, shared_state: SharedState) {
    let input_controller = gtk4::EventControllerKey::new();
    let action_button = input_area.action_button.clone();
    
    input_controller.connect_key_pressed(clone!(
        #[strong] shared_state,
        #[strong] action_button,
        move |_, key, _, modifier| {
            if key == gtk4::gdk::Key::Return && 
               modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                
                let is_ready = !shared_state.borrow().is_generating;
                if is_ready {
                    action_button.emit_clicked();
                }
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        }
    ));
    
    input_area.text_view.add_controller(input_controller);
}

fn load_profiles(shared_state: SharedState, controls: &ControlsArea) {
    let profile_names: Vec<String> = shared_state
        .borrow()
        .db
        .as_ref()
        .and_then(|db| db.get_profiles().ok())
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.name)
        .collect();
    controls.set_profiles(&profile_names);
}

fn setup_profile_dropdown_handler(shared_state: SharedState, controls: &ControlsArea, window: ApplicationWindow) {
    controls.profile_dropdown.connect_selected_notify(clone!(
        #[strong] shared_state,
        #[strong] controls,
        #[strong] window,
        move |_| {
            let selected_name = controls.get_selected_profile_name();

            // Reflect active profile in the window title
            match &selected_name {
                Some(name) => window.set_title(Some(&format!("Ollama Chat — {}", name))),
                None => window.set_title(Some("Ollama Chat")),
            }

            let profile = selected_name.as_ref().and_then(|name| {
                shared_state
                    .borrow()
                    .db
                    .as_ref()
                    .and_then(|db| db.get_profiles().ok())
                    .and_then(|profiles| profiles.into_iter().find(|p| &p.name == name))
            });

            // If the profile has a model_override, apply it to the model dropdown
            if let Some(ref p) = profile {
                if let Some(ref model) = p.model_override {
                    if let Some(pos) = find_model_position(&controls, model) {
                        controls.model_dropdown.set_selected(pos);
                    }
                }
            }

            shared_state.borrow_mut().apply_profile(profile);
        }
    ));
}

/// Find the position of a model name in the model dropdown, if present.
fn find_model_position(controls: &ControlsArea, model_name: &str) -> Option<u32> {
    let dropdown = &controls.model_dropdown;
    let model_obj = dropdown.model()?;
    let store = model_obj.downcast::<gtk4::StringList>().ok()?;
    for i in 0..store.n_items() {
        if store.string(i).as_deref() == Some(model_name) {
            return Some(i);
        }
    }
    None
}

fn setup_settings_button_handler(
    shared_state: SharedState,
    controls: &ControlsArea,
    window: ApplicationWindow,
    css_provider: gtk4::CssProvider,
) {
    controls.settings_button.connect_clicked(clone!(
        #[strong] shared_state,
        #[strong] controls,
        #[strong] window,
        #[strong] css_provider,
        move |_| {
            let dialog = crate::ui::settings_dialog::create_settings_dialog(
                &window,
                shared_state.clone(),
                css_provider.clone(),
            );
            // Reload profiles dropdown when the dialog is closed
            dialog.connect_destroy(clone!(
                #[strong] shared_state,
                #[strong] controls,
                move |_| load_profiles(shared_state.clone(), &controls)
            ));
            dialog.present();
        }
    ));
}

fn load_models(shared_state: SharedState, controls: &ControlsArea) {
    controls.set_status("Loading models...");
    
    let (sender, receiver) = async_channel::bounded(1);
    let controls_clone = controls.clone();
    
    // Extract URL from shared state for API call
    let ollama_url = shared_state.borrow().ollama_url.clone();
    
    // Spawn API task
    runtime().spawn(async move {
        let result = api::fetch_models(&ollama_url).await;
        let _ = sender.send(result).await;
    });
    
    // Handle response
    spawn_future_local(async move {
        if let Ok(result) = receiver.recv().await {
            match result {
                Ok(models) => {
                    let model_names: Vec<String> = models.into_iter().map(|m| m.name).collect();
                    controls_clone.set_models(&model_names);
                    
                    // Update shared state with first model
                    if !model_names.is_empty() {
                        shared_state.borrow_mut().selected_model = Some(model_names[0].clone());
                    }
                    
                    controls_clone.set_status("Ready");
                }
                Err(e) => {
                    controls_clone.set_status(&format!("Error loading models: {}", e));
                }
            }
        }
    });
}

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create tokio runtime")
    })
}