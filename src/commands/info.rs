use std::path::Path;

use crate::project::banner;

pub fn run(root: &Path) {
    super::ensure_dir(root);
    banner::print(root);
}
