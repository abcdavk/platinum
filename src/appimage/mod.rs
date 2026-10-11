mod imp;

use std::path::Path;

pub fn handle_file(path: &Path) -> Result<(), String> {
    imp::handle_file(path)
}
