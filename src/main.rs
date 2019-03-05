// #![allow(unused_variables)]
// TODO: be sure to remove the #! when building release version. We don't want no forgotten pesky unused variables living freely in our code
extern crate libc;
extern crate termios;
use cxedit::editor::editor::Editor;

fn main() {

    let mut editor = Editor::new();
    if let Ok(original) = editor.setup_rawmode(0) {
        editor.init(Some(original));
        editor.run();
    }
}
