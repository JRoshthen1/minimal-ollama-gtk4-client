use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, DropDown, Label, StringList};

#[derive(Clone)]
pub struct ControlsArea {
    pub container: GtkBox,
    pub model_dropdown: DropDown,
    pub status_label: Label,
    models: StringList,
}

impl ControlsArea {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 16);
        container.set_margin_top(16);
        
        // Model selection
        let models = StringList::new(&[]);
        let model_dropdown = DropDown::new(Some(models.clone()), None::<gtk4::Expression>);
        model_dropdown.set_hexpand(true);
        
        // Status label
        let status_label = Label::new(Some("Ready"));
        status_label.set_hexpand(true);
        status_label.set_halign(gtk4::Align::End);
        status_label.add_css_class("status-label");
        
        container.append(&model_dropdown);
        container.append(&status_label);
        
        Self {
            container,
            model_dropdown,
            status_label,
            models,
        }
    }
    
    pub fn set_models(&self, model_names: &[impl AsRef<str>]) {
        // Clear existing models
        let model_names_refs: Vec<&str> = model_names.iter().map(|s| s.as_ref()).collect();
        self.models.splice(0, self.models.n_items(), &model_names_refs);
        // Select first model if available
        if !model_names.is_empty() {
            self.model_dropdown.set_selected(0);
        }
    }
    
    pub fn get_selected_model(&self) -> Option<String> {
        let selected = self.model_dropdown.selected();
        if selected != gtk4::INVALID_LIST_POSITION {
            self.models.string(selected).map(|s| s.to_string())
        } else {
            None
        }
    }
    
    pub fn set_status(&self, status: &str) {
        self.status_label.set_text(status);
    }
}

pub fn create_controls() -> ControlsArea {
    ControlsArea::new()
}