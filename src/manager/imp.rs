use adw::prelude::{AlertDialogExt, AlertDialogExtManual};
use glib::subclass::InitializingObject;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{CompositeTemplate, glib};

use crate::dotdesktop::{self, ManagedApp};

#[derive(CompositeTemplate, Default)]
#[template(resource = "/id/abcdavk/platinum/platinum-manager.ui")]
pub struct ManagerWindow {
    #[template_child]
    pub apps_list: TemplateChild<gtk::Box>,
    #[template_child]
    pub empty_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub refresh_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub close_button: TemplateChild<gtk::Button>,
}

#[glib::object_subclass]
impl ObjectSubclass for ManagerWindow {
    const NAME: &'static str = "PlatinumManagerWindow";
    type Type = super::ManagerWindow;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ManagerWindow {
    fn constructed(&self) {
        self.parent_constructed();
        let window = self.obj();
        let weak_window = window.downgrade();
        self.refresh_button.connect_clicked(move |_| {
            if let Some(window) = weak_window.upgrade() {
                window.refresh();
            }
        });
        let weak_window = window.downgrade();
        self.close_button.connect_clicked(move |_| {
            if let Some(window) = weak_window.upgrade() {
                window.close();
            }
        });
        self.refresh_list();
    }
}

impl WidgetImpl for ManagerWindow {}
impl WindowImpl for ManagerWindow {}
impl ApplicationWindowImpl for ManagerWindow {}

impl ManagerWindow {
    pub fn refresh_list(&self) {
        while let Some(child) = self.apps_list.first_child() {
            self.apps_list.remove(&child);
        }

        match dotdesktop::managed_apps() {
            Ok(apps) if apps.is_empty() => {
                self.apps_list.set_visible(false);
                self.empty_label.set_label("No managed AppImages found.");
                self.empty_label.set_visible(true);
            }
            Ok(apps) => {
                self.empty_label.set_visible(false);
                self.apps_list.set_visible(true);
                for app in apps {
                    self.apps_list.append(&self.app_row(app));
                }
            }
            Err(error) => {
                self.apps_list.set_visible(false);
                self.empty_label
                    .set_label(&format!("Unable to load installed apps:\n{error}"));
                self.empty_label.set_visible(true);
            }
        }
    }

    fn app_row(&self, app: ManagedApp) -> gtk::Box {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_hexpand(true);
        row.set_margin_top(8);
        row.set_margin_bottom(8);
        row.set_valign(gtk::Align::Center);

        let image = gtk::Image::new();
        image.set_pixel_size(48);
        image.set_valign(gtk::Align::Center);
        if let Some(icon_path) = app.icon_path.as_ref().filter(|path| path.is_file()) {
            image.set_from_file(Some(icon_path));
        } else {
            image.set_icon_name(Some("application-x-executable"));
        }
        row.append(&image);

        let details = gtk::Box::new(gtk::Orientation::Vertical, 3);
        details.set_hexpand(true);
        details.set_valign(gtk::Align::Center);
        let name = gtk::Label::new(Some(&app.name));
        name.set_xalign(0.0);
        name.set_valign(gtk::Align::Center);
        name.add_css_class("bold");
        let version = gtk::Label::new(Some(&format!("Version: {}", app.version)));
        version.set_xalign(0.0);
        version.set_valign(gtk::Align::Center);
        version.add_css_class("dim-label");
        details.append(&name);
        details.append(&version);
        row.append(&details);

        let manager = self.obj();

        let delete_button = gtk::Button::with_label("Remove");
        delete_button.add_css_class("destructive-action");
        delete_button.set_valign(gtk::Align::Center);
        let weak_window = manager.downgrade();
        let desktop_path = app.desktop_path;
        let appimage_name = app
            .appimage_path
            .as_deref()
            .and_then(std::path::Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| app.name.clone());
        delete_button.connect_clicked(move |_| {
            if let Some(window) = weak_window.upgrade() {
                confirm_remove_app(&window, desktop_path.clone(), &appimage_name);
            }
        });
        row.append(&delete_button);
        row
    }
}

fn confirm_remove_app(
    window: &super::ManagerWindow,
    desktop_path: std::path::PathBuf,
    appimage_name: &str,
) {
    let body = format!(
        "Are you sure you want to remove {appimage_name}?\n\n\
         This will delete the AppImage from ~/Applications and its launcher from \
         ~/.local/share/applications."
    );
    let dialog = adw::AlertDialog::new(Some("Remove AppImage?"), Some(&body));
    dialog.add_responses(&[("cancel", "_Cancel"), ("remove", "_Remove")]);
    dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    let weak_window = window.downgrade();
    dialog.choose(
        Some(window),
        None::<&gtk::gio::Cancellable>,
        move |response| {
            if response != "remove" {
                return;
            }
            match dotdesktop::remove_managed_app(&desktop_path) {
                Ok(()) => {
                    if let Some(window) = weak_window.upgrade() {
                        window.refresh();
                    }
                }
                Err(error) => {
                    if let Some(window) = weak_window.upgrade() {
                        show_error(&window, "Unable to remove AppImage", &error);
                    }
                }
            }
        },
    );
}

fn show_error(window: &super::ManagerWindow, heading: &str, message: &str) {
    let dialog = adw::AlertDialog::new(Some(heading), Some(message));
    dialog.add_response("close", "_Close");
    dialog.choose(Some(window), None::<&gtk::gio::Cancellable>, |_| {});
}
