use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, TextView, TextBuffer, Button, ScrolledWindow, PolicyType, WrapMode};

#[derive(Clone)]
pub struct InputArea {
    pub container: GtkBox,
    pub text_view: TextView,
    pub text_buffer: TextBuffer,
    pub action_button: Button,
}

impl InputArea {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 0);
        
        // Text input area with proper sizing
        let scrolled_window = ScrolledWindow::new();
        scrolled_window.add_css_class("input-container");
        scrolled_window.set_policy(PolicyType::Never, PolicyType::Automatic);
        scrolled_window.set_min_content_height(80);  // Minimum height
        scrolled_window.set_max_content_height(200); // Maximum height
        scrolled_window.set_propagate_natural_height(true);
        scrolled_window.set_hexpand(true);
        scrolled_window.set_vexpand(false);
        
        let text_view = TextView::new();
        text_view.set_wrap_mode(WrapMode::WordChar);
        text_view.set_editable(true);  // Explicitly set editable
        text_view.set_cursor_visible(true);  // Make cursor visible
        text_view.set_accepts_tab(false);  // Don't consume tab events
        text_view.add_css_class("input-text");
        text_view.set_hexpand(true);
        text_view.set_vexpand(true);
        
        // Set some placeholder-like behavior
        let text_buffer = text_view.buffer();
        
        scrolled_window.set_child(Some(&text_view));
        
        // Action button container
        let button_container = GtkBox::new(Orientation::Horizontal, 8);
        button_container.set_halign(gtk4::Align::End);
        
        let action_button = Button::with_label("Send");
        action_button.add_css_class("send-button");
        
        button_container.append(&action_button);
        
        // Assemble container
        container.append(&scrolled_window);
        container.append(&button_container);
        
        Self {
            container,
            text_view,
            text_buffer,
            action_button,
        }
    }
}

pub fn create_input_area() -> InputArea {
    InputArea::new()
}