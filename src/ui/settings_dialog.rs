use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, Box as GtkBox, Button, CheckButton, Entry, Label,
    Notebook, Orientation, ScrolledWindow, SpinButton, Adjustment, StringList,
};
use gtk4::glib;
use std::rc::Rc;
use std::cell::RefCell;

use crate::config::{ColorConfig, Config, OllamaConfig, Profile, StreamingConfig, TtsInputConfig, UiConfig};
use crate::state::SharedState;

/// Build the settings dialog as a modal window transient for `parent`.
/// Returns the window so callers can connect to lifecycle signals (e.g. `connect_destroy`).
/// The caller is responsible for calling `.present()`.
pub fn create_settings_dialog(
    parent: &ApplicationWindow,
    shared_state: SharedState,
    css_provider: gtk4::CssProvider,
) -> gtk4::Window {
    let dialog = gtk4::Window::builder()
        .title("Settings")
        .transient_for(parent)
        .modal(true)
        .default_width(640)
        .default_height(580)
        .resizable(false)
        .build();

    let root = GtkBox::new(Orientation::Vertical, 0);

    let notebook = Notebook::new();
    notebook.set_vexpand(true);
    notebook.set_margin_top(8);
    notebook.set_margin_start(8);
    notebook.set_margin_end(8);

    let (general_tab, general_widgets) = build_general_tab(&shared_state.borrow().config);
    notebook.append_page(&general_tab, Some(&Label::new(Some("General"))));

    let (profiles_tab_widget, profiles_save_data) = build_profiles_tab(&shared_state);
    notebook.append_page(&profiles_tab_widget, Some(&Label::new(Some("Profiles"))));

    // Button bar
    let button_bar = GtkBox::new(Orientation::Horizontal, 8);
    button_bar.set_margin_top(12);
    button_bar.set_margin_bottom(12);
    button_bar.set_margin_start(12);
    button_bar.set_margin_end(12);
    button_bar.set_halign(gtk4::Align::End);

    let cancel_btn = Button::with_label("Cancel");
    let save_btn = Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    button_bar.append(&cancel_btn);
    button_bar.append(&save_btn);

    root.append(&notebook);
    root.append(&button_bar);
    dialog.set_child(Some(&root));

    cancel_btn.connect_clicked(glib::clone!(
        #[weak] dialog,
        move |_| dialog.close()
    ));

    save_btn.connect_clicked(glib::clone!(
        #[weak] dialog,
        move |_| {
            // ── Save general settings ─────────────────────────────────────
            let new_config = read_general_tab(&general_widgets);
            if let Err(e) = new_config.save() {
                eprintln!("Failed to save config: {}", e);
            }
            {
                let mut state = shared_state.borrow_mut();
                state.ollama_url = new_config.ollama.url.clone();
                if state.active_profile.is_none() {
                    state.system_prompt = if new_config.ollama.system_prompt.is_empty() {
                        None
                    } else {
                        Some(new_config.ollama.system_prompt.clone())
                    };
                }
                state.config = new_config.clone();
            }
            crate::app::apply_css(&css_provider, &new_config);

            // ── Save profiles ─────────────────────────────────────────────
            // Flush current form into the profiles vec
            if let Some(idx) = *profiles_save_data.current_idx.borrow() {
                let updated = read_profile_form(&profiles_save_data.form);
                if let Some(p) = profiles_save_data.profiles.borrow_mut().get_mut(idx) {
                    *p = updated;
                }
            }

            let state = shared_state.borrow();
            if let Some(ref db) = state.db {
                let profiles = profiles_save_data.profiles.borrow();

                // Delete profiles that were removed
                let current_ids: std::collections::HashSet<i64> = profiles
                    .iter()
                    .filter_map(|p| p.id)
                    .collect();
                for &id in &profiles_save_data.original_ids {
                    if !current_ids.contains(&id) {
                        if let Err(e) = db.delete_profile(id) {
                            eprintln!("Failed to delete profile {}: {}", id, e);
                        }
                    }
                }

                // Upsert remaining profiles
                drop(profiles); // release immutable borrow
                let mut profiles = profiles_save_data.profiles.borrow_mut();
                for profile in profiles.iter_mut() {
                    match db.save_profile(profile) {
                        Ok(id) => profile.id = Some(id),
                        Err(e) => eprintln!("Failed to save profile: {}", e),
                    }
                }
            }

            dialog.close();
        }
    ));

    dialog
}

// ── General Settings tab ─────────────────────────────────────────────────────

struct GeneralWidgets {
    // Ollama
    ollama_url: Entry,
    ollama_timeout: SpinButton,
    ollama_max_context: SpinButton,
    ollama_system_prompt: gtk4::TextView,
    // UI
    ui_window_font: SpinButton,
    ui_chat_font: SpinButton,
    ui_input_font: SpinButton,
    ui_code_family: Entry,
    // Colors
    color_chat_bg: Entry,
    color_code_bg: Entry,
    color_window_bg: Entry,
    color_primary_text: Entry,
    color_code_text: Entry,
    color_link_text: Entry,
    color_think_text: Entry,
    color_send_btn: Entry,
    color_stop_btn: Entry,
    // Carry-through for fields not shown in the form
    base_streaming: StreamingConfig,
    base_tts_input: TtsInputConfig,
}

fn build_general_tab(config: &Config) -> (ScrolledWindow, GeneralWidgets) {
    let scrolled = ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let content = GtkBox::new(Orientation::Vertical, 0);
    content.set_margin_top(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_bottom(16);

    // Ollama
    content.append(&section_label("Ollama"));
    let ollama_url = labeled_entry("Server URL", &config.ollama.url);
    let ollama_timeout = labeled_spin("Timeout (seconds)", config.ollama.timeout_seconds as f64, 1.0, 600.0);
    let ollama_max_context = labeled_spin("Max context messages", config.ollama.max_context_messages as f64, 1.0, 200.0);
    let (sp_row, ollama_system_prompt) = labeled_text_view("Default system prompt", &config.ollama.system_prompt);
    content.append(&ollama_url.0);
    content.append(&ollama_timeout.0);
    content.append(&ollama_max_context.0);
    content.append(&sp_row);

    // UI Fonts
    content.append(&section_label("UI Fonts"));
    let ui_window_font = labeled_spin("Window font size", config.ui.window_font_size as f64, 8.0, 48.0);
    let ui_chat_font = labeled_spin("Chat font size", config.ui.chat_font_size as f64, 8.0, 48.0);
    let ui_input_font = labeled_spin("Input font size", config.ui.input_font_size as f64, 8.0, 48.0);
    let ui_code_family = labeled_entry("Code font family", &config.ui.code_font_family);
    content.append(&ui_window_font.0);
    content.append(&ui_chat_font.0);
    content.append(&ui_input_font.0);
    content.append(&ui_code_family.0);

    // Colors
    content.append(&section_label("Colors (hex, e.g. #ffffff)"));
    let color_chat_bg = labeled_entry("Chat background", &config.colors.chat_background);
    let color_code_bg = labeled_entry("Code background", &config.colors.code_background);
    let color_window_bg = labeled_entry("Window background", &config.colors.window_background);
    let color_primary_text = labeled_entry("Primary text", &config.colors.primary_text);
    let color_code_text = labeled_entry("Code text", &config.colors.code_text);
    let color_link_text = labeled_entry("Link text", &config.colors.link_text);
    let color_think_text = labeled_entry("Think text", &config.colors.think_text);
    let color_send_btn = labeled_entry("Send button", &config.colors.send_button);
    let color_stop_btn = labeled_entry("Stop button", &config.colors.stop_button);
    content.append(&color_chat_bg.0);
    content.append(&color_code_bg.0);
    content.append(&color_window_bg.0);
    content.append(&color_primary_text.0);
    content.append(&color_code_text.0);
    content.append(&color_link_text.0);
    content.append(&color_think_text.0);
    content.append(&color_send_btn.0);
    content.append(&color_stop_btn.0);

    // TTS Input stub
    content.append(&section_label("Speech Input (coming soon)"));
    let tts_note = Label::new(Some("Speech-to-text input will be configurable here in a future release."));
    tts_note.set_halign(gtk4::Align::Start);
    tts_note.set_sensitive(false);
    tts_note.set_margin_bottom(8);
    content.append(&tts_note);

    scrolled.set_child(Some(&content));

    let widgets = GeneralWidgets {
        ollama_url: ollama_url.1,
        ollama_timeout: ollama_timeout.1,
        ollama_max_context: ollama_max_context.1,
        ollama_system_prompt,
        ui_window_font: ui_window_font.1,
        ui_chat_font: ui_chat_font.1,
        ui_input_font: ui_input_font.1,
        ui_code_family: ui_code_family.1,
        color_chat_bg: color_chat_bg.1,
        color_code_bg: color_code_bg.1,
        color_window_bg: color_window_bg.1,
        color_primary_text: color_primary_text.1,
        color_code_text: color_code_text.1,
        color_link_text: color_link_text.1,
        color_think_text: color_think_text.1,
        color_send_btn: color_send_btn.1,
        color_stop_btn: color_stop_btn.1,
        base_streaming: config.streaming.clone(),
        base_tts_input: config.tts_input.clone(),
    };

    (scrolled, widgets)
}

fn read_general_tab(w: &GeneralWidgets) -> Config {
    let buf = w.ollama_system_prompt.buffer();
    let system_prompt = buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string();

    Config {
        ollama: OllamaConfig {
            url: w.ollama_url.text().trim().to_string(),
            timeout_seconds: w.ollama_timeout.value() as u64,
            max_context_messages: w.ollama_max_context.value() as usize,
            system_prompt,
        },
        ui: UiConfig {
            window_font_size: w.ui_window_font.value() as u32,
            chat_font_size: w.ui_chat_font.value() as u32,
            input_font_size: w.ui_input_font.value() as u32,
            code_font_family: w.ui_code_family.text().trim().to_string(),
        },
        colors: ColorConfig {
            chat_background: w.color_chat_bg.text().trim().to_string(),
            code_background: w.color_code_bg.text().trim().to_string(),
            window_background: w.color_window_bg.text().trim().to_string(),
            primary_text: w.color_primary_text.text().trim().to_string(),
            code_text: w.color_code_text.text().trim().to_string(),
            link_text: w.color_link_text.text().trim().to_string(),
            think_text: w.color_think_text.text().trim().to_string(),
            send_button: w.color_send_btn.text().trim().to_string(),
            stop_button: w.color_stop_btn.text().trim().to_string(),
        },
        streaming: w.base_streaming.clone(),
        tts_input: w.base_tts_input.clone(),
    }
}

// ── Profiles tab ─────────────────────────────────────────────────────────────

/// Holds the form widgets so the save handler can flush the current profile.
#[derive(Clone)]
struct ProfileFormWidgets {
    name_entry: Entry,
    model_override_entry: Entry,
    temp_spin: SpinButton,
    temp_use_default: CheckButton,
    batch_size_spin: SpinButton,
    batch_timeout_spin: SpinButton,
    max_context_spin: SpinButton,
    system_prompt_view: gtk4::TextView,
    // TTS output stubs
    tts_enabled: CheckButton,
    tts_voice_entry: Entry,
    tts_speed_spin: SpinButton,
    // RAG stubs
    rag_enabled: CheckButton,
    rag_collection_entry: Entry,
}

/// Data the dialog-level Save handler needs to persist profiles.
struct ProfilesSaveData {
    profiles: Rc<RefCell<Vec<Profile>>>,
    original_ids: Vec<i64>,
    current_idx: Rc<RefCell<Option<usize>>>,
    form: ProfileFormWidgets,
}

fn build_profiles_tab(shared_state: &SharedState) -> (GtkBox, ProfilesSaveData) {
    // Load existing profiles from DB
    let loaded: Vec<Profile> = shared_state
        .borrow()
        .db
        .as_ref()
        .and_then(|db| db.get_profiles().ok())
        .unwrap_or_default();

    let original_ids: Vec<i64> = loaded.iter().filter_map(|p| p.id).collect();
    let profiles = Rc::new(RefCell::new(loaded));
    let current_idx: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

    // ── Top row: dropdown + New + Delete ─────────────────────────────────────
    let tab = GtkBox::new(Orientation::Vertical, 8);
    tab.set_margin_top(12);
    tab.set_margin_start(16);
    tab.set_margin_end(16);
    tab.set_margin_bottom(8);

    let top_row = GtkBox::new(Orientation::Horizontal, 8);

    let dropdown_model = StringList::new(&[]);
    let profile_dropdown = gtk4::DropDown::new(
        Some(dropdown_model.clone()),
        None::<gtk4::Expression>,
    );
    profile_dropdown.set_hexpand(true);

    let new_btn = Button::with_label("+");
    new_btn.set_tooltip_text(Some("New profile"));
    let delete_btn = Button::with_label("–");
    delete_btn.set_tooltip_text(Some("Delete selected profile"));

    top_row.append(&profile_dropdown);
    top_row.append(&new_btn);
    top_row.append(&delete_btn);
    tab.append(&top_row);

    // ── Form (shown when a profile is selected) ───────────────────────────────
    let form_box = GtkBox::new(Orientation::Vertical, 0);
    let scroll = ScrolledWindow::new();
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scroll.set_vexpand(true);

    let form_content = GtkBox::new(Orientation::Vertical, 0);
    form_content.set_margin_top(8);
    form_content.set_margin_bottom(8);

    // Basic
    form_content.append(&section_label("Profile"));
    let name_entry_row = labeled_entry("Name", "");
    let model_override_row = labeled_entry("Model override (blank = use dropdown)", "");
    form_content.append(&name_entry_row.0);
    form_content.append(&model_override_row.0);

    // Temperature
    form_content.append(&section_label("Generation"));
    let temp_row = GtkBox::new(Orientation::Horizontal, 8);
    temp_row.set_margin_bottom(4);
    let temp_lbl = Label::new(Some("Temperature"));
    temp_lbl.set_width_chars(24);
    temp_lbl.set_halign(gtk4::Align::Start);
    temp_lbl.set_xalign(0.0);
    let temp_adj = Adjustment::new(0.8, 0.0, 2.0, 0.05, 0.1, 0.0);
    let temp_spin = SpinButton::new(Some(&temp_adj), 0.05, 2);
    temp_spin.set_width_chars(8);
    let temp_use_default = CheckButton::with_label("Use server default");
    temp_row.append(&temp_lbl);
    temp_row.append(&temp_spin);
    temp_row.append(&temp_use_default);
    form_content.append(&temp_row);

    // When "use default" is checked, disable the spin
    temp_use_default.connect_toggled(glib::clone!(
        #[weak] temp_spin,
        move |cb| temp_spin.set_sensitive(!cb.is_active())
    ));

    let batch_size_row = labeled_spin("Batch size (tokens)", 20.0, 1.0, 500.0);
    let batch_timeout_row = labeled_spin("Batch timeout (ms)", 100.0, 10.0, 5000.0);
    let max_context_row = labeled_spin("Max context messages", 20.0, 1.0, 200.0);
    form_content.append(&batch_size_row.0);
    form_content.append(&batch_timeout_row.0);
    form_content.append(&max_context_row.0);

    // System prompt
    let (sp_row, system_prompt_view) = labeled_text_view("System prompt", "");
    form_content.append(&sp_row);

    // TTS output stubs
    form_content.append(&section_label("TTS Output (coming soon)"));
    let tts_row = GtkBox::new(Orientation::Horizontal, 8);
    tts_row.set_margin_bottom(4);
    let tts_enabled = CheckButton::with_label("Enable TTS output");
    tts_enabled.set_sensitive(false);
    let tts_voice_row = labeled_entry("Voice", "");
    tts_voice_row.1.set_sensitive(false);
    let tts_speed_row = labeled_spin("Speed", 1.0, 0.5, 2.0);
    tts_speed_row.1.set_sensitive(false);
    tts_row.append(&tts_enabled);
    form_content.append(&tts_row);
    form_content.append(&tts_voice_row.0);
    form_content.append(&tts_speed_row.0);

    // RAG stubs
    form_content.append(&section_label("RAG (coming soon)"));
    let rag_row = GtkBox::new(Orientation::Horizontal, 8);
    rag_row.set_margin_bottom(4);
    let rag_enabled = CheckButton::with_label("Enable RAG");
    rag_enabled.set_sensitive(false);
    let rag_collection_row = labeled_entry("Collection name", "");
    rag_collection_row.1.set_sensitive(false);
    rag_row.append(&rag_enabled);
    form_content.append(&rag_row);
    form_content.append(&rag_collection_row.0);

    scroll.set_child(Some(&form_content));
    form_box.append(&scroll);
    tab.append(&form_box);

    // Placeholder shown when list is empty
    let empty_label = Label::new(Some("No profiles yet — click + to create one."));
    empty_label.set_halign(gtk4::Align::Center);
    empty_label.set_valign(gtk4::Align::Center);
    empty_label.set_vexpand(true);
    empty_label.set_sensitive(false);
    tab.append(&empty_label);

    // Collect form widgets into a shareable struct
    let form = ProfileFormWidgets {
        name_entry: name_entry_row.1.clone(),
        model_override_entry: model_override_row.1.clone(),
        temp_spin: temp_spin.clone(),
        temp_use_default: temp_use_default.clone(),
        batch_size_spin: batch_size_row.1.clone(),
        batch_timeout_spin: batch_timeout_row.1.clone(),
        max_context_spin: max_context_row.1.clone(),
        system_prompt_view: system_prompt_view.clone(),
        tts_enabled: tts_enabled.clone(),
        tts_voice_entry: tts_voice_row.1.clone(),
        tts_speed_spin: tts_speed_row.1.clone(),
        rag_enabled: rag_enabled.clone(),
        rag_collection_entry: rag_collection_row.1.clone(),
    };

    // ── Populate dropdown from profiles ───────────────────────────────────────
    {
        let profs = profiles.borrow();
        let names: Vec<&str> = profs.iter().map(|p| p.name.as_str()).collect();
        dropdown_model.splice(0, dropdown_model.n_items(), &names);
    }

    let has_profiles = !profiles.borrow().is_empty();
    form_box.set_visible(has_profiles);
    empty_label.set_visible(!has_profiles);

    // Select first profile if available
    if has_profiles {
        profile_dropdown.set_selected(0);
        *current_idx.borrow_mut() = Some(0);
        populate_profile_form(&profiles.borrow()[0], &form);
    }

    // ── Dropdown selection change: save current, load new ─────────────────────
    let profiles_dc = profiles.clone();
    let current_idx_dc = current_idx.clone();
    let form_dc = form.clone();
    let form_box_dc = form_box.clone();
    let empty_label_dc = empty_label.clone();
    profile_dropdown.connect_selected_notify(move |dd| {
        let selected = dd.selected();
        if selected == gtk4::INVALID_LIST_POSITION {
            return;
        }
        let new_idx = selected as usize;
        let profs_len = profiles_dc.borrow().len();
        if new_idx >= profs_len {
            return;
        }

        // Flush current form to current profile before switching
        if let Some(old_idx) = *current_idx_dc.borrow() {
            if old_idx < profs_len {
                let updated = read_profile_form(&form_dc);
                profiles_dc.borrow_mut()[old_idx] = updated;
            }
        }

        *current_idx_dc.borrow_mut() = Some(new_idx);
        populate_profile_form(&profiles_dc.borrow()[new_idx], &form_dc);
        form_box_dc.set_visible(true);
        empty_label_dc.set_visible(false);
    });

    // ── New button ────────────────────────────────────────────────────────────
    let profiles_nb = profiles.clone();
    let current_idx_nb = current_idx.clone();
    let form_nb = form.clone();
    let dropdown_model_nb = dropdown_model.clone();
    let profile_dropdown_nb = profile_dropdown.clone();
    let form_box_nb = form_box.clone();
    let empty_label_nb = empty_label.clone();
    new_btn.connect_clicked(move |_| {
        // Auto-save current form first
        if let Some(idx) = *current_idx_nb.borrow() {
            if idx < profiles_nb.borrow().len() {
                let updated = read_profile_form(&form_nb);
                profiles_nb.borrow_mut()[idx] = updated;
            }
        }

        let n = profiles_nb.borrow().len() + 1;
        let new_profile = Profile {
            name: format!("Profile {}", n),
            ..Profile::default()
        };
        let new_idx = profiles_nb.borrow().len();
        profiles_nb.borrow_mut().push(new_profile.clone());

        // Update dropdown
        dropdown_model_nb.splice(
            dropdown_model_nb.n_items(),
            0,
            &[new_profile.name.as_str()],
        );

        // Select and populate
        *current_idx_nb.borrow_mut() = Some(new_idx);
        profile_dropdown_nb.set_selected(new_idx as u32);
        populate_profile_form(&profiles_nb.borrow()[new_idx], &form_nb);
        form_box_nb.set_visible(true);
        empty_label_nb.set_visible(false);
    });

    // ── Delete button ─────────────────────────────────────────────────────────
    let profiles_db = profiles.clone();
    let current_idx_db = current_idx.clone();
    let form_db = form.clone();
    let dropdown_model_db = dropdown_model.clone();
    let profile_dropdown_db = profile_dropdown.clone();
    let form_box_db = form_box.clone();
    let empty_label_db = empty_label.clone();
    delete_btn.connect_clicked(move |_| {
        let idx = match *current_idx_db.borrow() {
            Some(i) => i,
            None => return,
        };

        profiles_db.borrow_mut().remove(idx);

        // Rebuild dropdown model
        let names: Vec<String> = profiles_db.borrow().iter().map(|p| p.name.clone()).collect();
        let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        dropdown_model_db.splice(0, dropdown_model_db.n_items(), &name_refs);

        if profiles_db.borrow().is_empty() {
            *current_idx_db.borrow_mut() = None;
            form_box_db.set_visible(false);
            empty_label_db.set_visible(true);
        } else {
            let new_sel = idx.saturating_sub(1);
            *current_idx_db.borrow_mut() = Some(new_sel);
            profile_dropdown_db.set_selected(new_sel as u32);
            populate_profile_form(&profiles_db.borrow()[new_sel], &form_db);
        }
    });

    let save_data = ProfilesSaveData {
        profiles,
        original_ids,
        current_idx,
        form,
    };

    (tab, save_data)
}

/// Load a `Profile` into the form widgets.
fn populate_profile_form(p: &Profile, w: &ProfileFormWidgets) {
    w.name_entry.set_text(&p.name);
    w.model_override_entry
        .set_text(p.model_override.as_deref().unwrap_or(""));
    match p.temperature {
        Some(t) => {
            w.temp_use_default.set_active(false);
            w.temp_spin.set_sensitive(true);
            w.temp_spin.set_value(t as f64);
        }
        None => {
            w.temp_use_default.set_active(true);
            w.temp_spin.set_sensitive(false);
        }
    }
    w.batch_size_spin.set_value(p.batch_size as f64);
    w.batch_timeout_spin.set_value(p.batch_timeout_ms as f64);
    w.max_context_spin.set_value(p.max_context_messages as f64);
    w.system_prompt_view.buffer().set_text(&p.system_prompt);
}

/// Read form widgets into a `Profile`. Preserves `id` from the form's current profile
/// by returning a Profile with `id: None` — the caller must restore it from the vec.
fn read_profile_form(w: &ProfileFormWidgets) -> Profile {
    let buf = w.system_prompt_view.buffer();
    let system_prompt = buf
        .text(&buf.start_iter(), &buf.end_iter(), false)
        .to_string();
    let model_override = {
        let s = w.model_override_entry.text().trim().to_string();
        if s.is_empty() { None } else { Some(s) }
    };
    let temperature = if w.temp_use_default.is_active() {
        None
    } else {
        Some(w.temp_spin.value() as f32)
    };

    Profile {
        id: None, // caller restores from existing vec entry
        name: w.name_entry.text().trim().to_string(),
        system_prompt,
        batch_size: w.batch_size_spin.value() as usize,
        batch_timeout_ms: w.batch_timeout_spin.value() as u64,
        max_context_messages: w.max_context_spin.value() as usize,
        model_override,
        temperature,
        rag_enabled: false,
        rag_collection: None,
        tts_enabled: false,
        tts_voice: None,
        tts_speed: None,
    }
}

// ── Widget helpers ────────────────────────────────────────────────────────────

fn section_label(text: &str) -> Label {
    let label = Label::new(None);
    label.set_halign(gtk4::Align::Start);
    label.set_margin_top(12);
    label.set_margin_bottom(4);
    label.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(text)));
    label
}

fn labeled_entry(label: &str, value: &str) -> (GtkBox, Entry) {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_margin_bottom(4);
    let lbl = Label::new(Some(label));
    lbl.set_width_chars(24);
    lbl.set_halign(gtk4::Align::Start);
    lbl.set_xalign(0.0);
    let entry = Entry::new();
    entry.set_text(value);
    entry.set_hexpand(true);
    row.append(&lbl);
    row.append(&entry);
    (row, entry)
}

fn labeled_spin(label: &str, value: f64, min: f64, max: f64) -> (GtkBox, SpinButton) {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_margin_bottom(4);
    let lbl = Label::new(Some(label));
    lbl.set_width_chars(24);
    lbl.set_halign(gtk4::Align::Start);
    lbl.set_xalign(0.0);
    let adj = Adjustment::new(value, min, max, 1.0, 10.0, 0.0);
    let spin = SpinButton::new(Some(&adj), 1.0, 0);
    spin.set_width_chars(8);
    row.append(&lbl);
    row.append(&spin);
    (row, spin)
}

fn labeled_text_view(label: &str, value: &str) -> (GtkBox, gtk4::TextView) {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_margin_bottom(4);
    let lbl = Label::new(Some(label));
    lbl.set_width_chars(24);
    lbl.set_halign(gtk4::Align::Start);
    lbl.set_yalign(0.0);
    let text_view = gtk4::TextView::new();
    text_view.set_wrap_mode(gtk4::WrapMode::Word);
    text_view.buffer().set_text(value);
    text_view.add_css_class("settings-text-view");
    let sw = ScrolledWindow::new();
    sw.add_css_class("settings-text-container");
    sw.set_child(Some(&text_view));
    sw.set_hexpand(true);
    sw.set_min_content_height(80);
    sw.set_max_content_height(120);
    sw.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    row.append(&lbl);
    row.append(&sw);
    (row, text_view)
}
