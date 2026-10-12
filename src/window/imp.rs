use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::{AlertDialogExt, AlertDialogExtManual};
use glib::subclass::InitializingObject;
use gtk::gio::prelude::FileExt;
use gtk::glib::types::StaticType;
use gtk::prelude::{GtkWindowExt, ObjectExt, WidgetExt};
use gtk::subclass::prelude::*;
use gtk::{CompositeTemplate, glib};

use crate::appimage::{handle_file, is_appimage};

// Object holding the state
#[derive(CompositeTemplate, Default)]
#[template(resource = "/id/abcdavk/platinum/window.ui")]
pub struct Window {
    #[template_child]
    pub drop_area: TemplateChild<gtk::Box>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for Window {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "PlatinumMainWindow";
    type Type = super::Window;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for Window {
    fn constructed(&self) {
        self.parent_constructed();

        let window = self.obj();
        let confirmation_open = Rc::new(Cell::new(false));

        let confirmation_open_weak = Rc::clone(&confirmation_open);
        window.connect_is_active_notify(move |window| {
            if !window.is_active() && !confirmation_open_weak.get() {
                window.close();
            }
        });

        let target = gtk::DropTarget::new(
            gtk::gdk::FileList::static_type(),
            gtk::gdk::DragAction::COPY,
        );
        let window_weak = window.downgrade();
        let confirmation_open_for_drop = Rc::clone(&confirmation_open);

        target.connect_drop(move |_, value, _, _| {
            let Ok(file_list) = value.get::<gtk::gdk::FileList>() else {
                return false;
            };

            let mut appimages = Vec::new();
            for file in file_list.files() {
                if let Some(path) = file.path() {
                    if is_appimage(&path) {
                        appimages.push(path);
                    } else if let Err(error) = handle_file(&path) {
                        eprintln!("{error}");
                    }
                }
            }

            if appimages.is_empty() {
                return true;
            }

            let Some(window) = window_weak.upgrade() else {
                return true;
            };
            let names = appimages
                .iter()
                .filter_map(|path| path.file_name())
                .map(|name| name.to_string_lossy())
                .collect::<Vec<_>>()
                .join("\n");
            let body = if appimages.len() == 1 {
                format!("Are you sure you want to install {names}?")
            } else {
                format!("Are you sure you want to install these AppImages?\n\n{names}")
            };
            let dialog = adw::AlertDialog::new(Some("Install AppImage?"), Some(&body));
            dialog.add_responses(&[("cancel", "_Cancel"), ("install", "_Install")]);
            dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("install"));
            dialog.set_close_response("cancel");

            confirmation_open_for_drop.set(true);
            let window_weak = window.downgrade();
            let confirmation_open_for_response = Rc::clone(&confirmation_open_for_drop);
            dialog.choose(
                Some(&window),
                None::<&gtk::gio::Cancellable>,
                move |response| {
                    confirmation_open_for_response.set(false);

                    if response == "install" {
                        let mut installed = false;
                        for path in &appimages {
                            match handle_file(path) {
                                Ok(()) => installed = true,
                                Err(error) => eprintln!("{error}"),
                            }
                        }

                        if installed && let Some(window) = window_weak.upgrade() {
                            window.close();
                        }
                    } else {
                        if let Some(window) = window_weak.upgrade() {
                            window.close();
                        }
                    }
                },
            );

            true
        });

        self.drop_area.add_controller(target);
    }
}

// Trait shared by all widgets
impl WidgetImpl for Window {}

// Trait shared by all windows
impl WindowImpl for Window {}

// Trait shared by all application windows
impl ApplicationWindowImpl for Window {}
