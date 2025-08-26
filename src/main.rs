use gtk4::prelude::*;
use gtk4::{glib, Application};

mod ui;
mod api;
mod types;
mod state;

use state::AppState;

const APP_ID: &str = "com.example.ollama-chat";

#[tokio::main]
async fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(ui::build_ui);
    app.run()
}
