// #![allow(unused_variables)]
// TODO: be sure to remove the #! when building release version. We don't want no forgotten pesky unused variables living freely in our code
// TODO: clean up a BUNCH of code in editor.rs and view.rs (especially). Sane line & view management has been implemented
//      but in that process a bunch of TRASH code was produced and basically trialed and error'd to success. A true nightmare.
extern crate libc;
extern crate termios;
use std::env::args;
use std::path::Path;

use cxedit::editor::editor::Editor;



fn main() {

    let default_config = Path::new("config.rc");

    if default_config.exists() {

    } else {

    }

    let args = args().collect::<Vec<String>>();
    let f = if args.len() == 2 {
        let p = Path::new(&args[1]);
        if p.exists() {
            Some(p)
        } else {
            None
        }
    } else if args.len() > 2 {
        panic!("You can only provide one file to open at this time...");
        None
    } else {
        None
    };
    let mut editor = Editor::new();
    if let Ok(original) = editor.setup_rawmode(0) {
        editor.init(Some(original));
        f.and_then(|file_path| {
            editor.open(file_path);
            Some(file_path)
        });
        editor.run();
    }
}
