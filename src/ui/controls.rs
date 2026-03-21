use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Orientation, DropDown, Label, StringList};

#[derive(Clone)]
pub struct ControlsArea {
    pub container: GtkBox,
    pub model_dropdown: DropDown,
    pub profile_dropdown: DropDown,
    pub status_label: Label,
    pub settings_button: Button,
    models: StringList,
    profiles: StringList,
}

impl ControlsArea {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 16);
        container.set_margin_bottom(4);

        // Model selection
        let models = StringList::new(&[]);
        let model_dropdown = DropDown::new(Some(models.clone()), None::<gtk4::Expression>);
        model_dropdown.set_hexpand(true);

        // Profile selection — "None" is always the first entry (index 0 = no active profile)
        let profiles = StringList::new(&["None"]);
        let profile_dropdown = DropDown::new(Some(profiles.clone()), None::<gtk4::Expression>);
        profile_dropdown.set_tooltip_text(Some("Active profile"));

        // Status label
        let status_label = Label::new(Some("Ready"));
        status_label.set_hexpand(true);
        status_label.set_halign(gtk4::Align::End);
        status_label.add_css_class("status-label");

        // Settings gear button
        let settings_button = Button::with_label("⚙");
        settings_button.set_tooltip_text(Some("Settings"));

        container.append(&model_dropdown);
        container.append(&profile_dropdown);
        container.append(&status_label);
        container.append(&settings_button);

        Self {
            container,
            model_dropdown,
            profile_dropdown,
            status_label,
            settings_button,
            models,
            profiles,
        }
    }

    pub fn set_models(&self, model_names: &[impl AsRef<str>]) {
        let model_names_refs: Vec<&str> = model_names.iter().map(|s| s.as_ref()).collect();
        self.models.splice(0, self.models.n_items(), &model_names_refs);
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

    /// Reload the profile dropdown from a fresh list of profile names.
    /// "None" (no profile) is always prepended as the first entry.
    pub fn set_profiles(&self, profile_names: &[String]) {
        // "None" + all profile names
        let mut entries: Vec<&str> = vec!["None"];
        let name_refs: Vec<&str> = profile_names.iter().map(|s| s.as_str()).collect();
        entries.extend_from_slice(&name_refs);
        self.profiles.splice(0, self.profiles.n_items(), &entries);
        self.profile_dropdown.set_selected(0);
    }

    /// Returns the selected profile name, or `None` if "None" (index 0) is selected.
    pub fn get_selected_profile_name(&self) -> Option<String> {
        let selected = self.profile_dropdown.selected();
        if selected == gtk4::INVALID_LIST_POSITION || selected == 0 {
            None
        } else {
            self.profiles.string(selected).map(|s| s.to_string())
        }
    }

    pub fn set_status(&self, status: &str) {
        self.status_label.set_text(status);
    }
}

pub fn create_controls() -> ControlsArea {
    ControlsArea::new()
}
