mod imp;

use glib::Object;
use gtk::prelude::IsA;
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{gio, glib};

glib::wrapper! {
    pub struct ManagerWindow(ObjectSubclass<imp::ManagerWindow>)
        @extends gtk::ApplicationWindow,
                 gtk::Window,
                 gtk::Widget,
        @implements gio::ActionGroup,
                    gio::ActionMap,
                    gtk::Accessible,
                    gtk::Buildable,
                    gtk::ConstraintTarget,
                    gtk::Native,
                    gtk::Root,
                    gtk::ShortcutManager;
}

impl ManagerWindow {
    pub fn new(app: &impl IsA<gtk::Application>) -> Self {
        Object::builder().property("application", app).build()
    }

    pub fn refresh(&self) {
        self.imp().refresh_list();
    }
}
