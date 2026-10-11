mod appimage;
mod window;

use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use window::Window;

const APP_ID: &str = "org.abcdavk.platinum";

fn main() -> glib::ExitCode {
    // Register and include resources
    gio::resources_register_include!("resources.gresource").expect("Failed to register resources.");

    // Initialize libadwaita
    adw::init().expect("Failed to initialize libadwaita");

    // Get default display
    let display = gdk::Display::default().expect("Could not get default display");

    // Load icons
    register_icons(&display);

    // Load css
    load_css(&display);

    // Create a new application
    let app = adw::Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

fn build_ui(app: &adw::Application) {
    // Create new window and present it
    let window = Window::new(app);
    window.present();
}
fn load_css(display: &impl IsA<gdk::Display>) {
    let provider = gtk::CssProvider::new();

    provider.load_from_resource("/id/abcdavk/platinum/style.css");

    gtk::style_context_add_provider_for_display(
        display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
fn register_icons(display: &impl IsA<gdk::Display>) {
    let icon_theme = gtk::IconTheme::for_display(display);

    icon_theme.add_resource_path("/id/abcdavk/platinum/icons");
}
