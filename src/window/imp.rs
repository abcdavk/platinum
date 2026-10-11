use glib::subclass::InitializingObject;
use gtk::gio::prelude::FileExt;
use gtk::glib::types::StaticType;
use gtk::prelude::{GtkWindowExt, WidgetExt};
use gtk::subclass::prelude::*;
use gtk::{CompositeTemplate, glib};

use crate::appimage::handle_file;

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

        window.connect_is_active_notify(|window| {
            if !window.is_active() {
                window.close();
            }
        });

        let target = gtk::DropTarget::new(
            gtk::gdk::FileList::static_type(),
            gtk::gdk::DragAction::COPY,
        );
        let window_weak = window.downgrade();

        target.connect_drop(move |_, value, _, _| {
            let Ok(file_list) = value.get::<gtk::gdk::FileList>() else {
                return false;
            };

            let mut installed = false;
            for file in file_list.files() {
                if let Some(path) = file.path() {
                    match handle_file(&path) {
                        Ok(()) => installed = true,
                        Err(error) => eprintln!("{error}"),
                    }
                }
            }

            if installed && let Some(window) = window_weak.upgrade() {
                window.close();
            }

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
