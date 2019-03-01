#![allow(unused_variables)]
// TODO: be sure to remove the #! when building release version. We don't want no forgotten pesky unused variables living freely in our code
extern crate libc;
extern crate termios;

use std::collections::
use std::os::unix::io::RawFd;
use termios::Termios;
use termios::tcsetattr;

pub fn setup_rawmode(fd: RawFd) -> std::io::Result<Termios> {
    let mut term = Termios::from_fd(fd)?;
    let original_setting = Termios::from_fd(0).unwrap();

    term.c_iflag &= !(libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON);
    term.c_oflag &= !(libc::OPOST);
    term.c_cflag |= libc::CS8 | libc::CREAD | libc::CLOCAL;
    term.c_lflag &= !(libc::ECHO | libc::ICANON | libc::IEXTEN | libc::ISIG);

    term.c_cc[libc::VMIN] = 0;
    term.c_cc[libc::VTIME] = 1;
    term.c_lflag = 0;


    termios::cfsetspeed(&mut term, libc::B9600)?;
    termios::tcsetattr(fd, libc::TCSANOW, &term)?;
    termios::tcflush(fd, libc::TCIOFLUSH)?;
    Ok(original_setting)
}
pub fn restore_mode(t: &Termios) -> std::io::Result<bool> {
    match tcsetattr(0, libc::TCSANOW, &t) {
        Ok(_) => {
            println!("Restored terminal settings");
            Ok(true)
        },
        Err(e) => {
            println!("Some error occurred! {}", e);
            Ok(false)
        }
    }
}


fn main() {
    let settings = match setup_rawmode(0) {
        Ok(original) => {
            println!("Successfully setup raw mode\r");
            Some(original)
        },
        Err(e) => {
            println!("Some error occured while setting up terminal raw mode, {}", e);
            None
        }
    };

}
