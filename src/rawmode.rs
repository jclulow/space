use libc::{ioctl, ___errno};
use libc::{tcgetattr, tcsetattr};
use libc::{winsize, termios};
use libc::TIOCGWINSZ;
use libc::{IMAXBEL, IGNBRK, BRKINT, PARMRK, ISTRIP, INLCR, IGNCR,
    ICRNL, IXON, OPOST, ECHO, ECHONL, ICANON, ISIG, IEXTEN, CSIZE,
    PARENB, CS8, VMIN, VTIME};
use libc::TCSADRAIN;


pub struct RawMode {
    orig: Box<termios>,
}

#[derive(Debug, Clone)]
pub struct WinSize {
    pub rows: u16,
    pub cols: u16,
}

impl RawMode {
    pub fn enable() -> std::io::Result<RawMode> {
        let orig = Box::into_raw(
            Box::new(unsafe { std::mem::zeroed::<termios>() }));

        let (r, e) = unsafe {
            let r = tcgetattr(1, orig);
            let e = *___errno();
            (r, e)
        };

        let orig = unsafe { Box::from_raw(orig) };

        if r != 0 {
            return Err(std::io::Error::from_raw_os_error(e));
        }

        let mut change = orig.clone();

        change.c_iflag &= !(IMAXBEL | IGNBRK | BRKINT | PARMRK | ISTRIP
            | INLCR | IGNCR | ICRNL | IXON);
        change.c_oflag &= !OPOST;
        change.c_lflag &= !(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
        change.c_cflag &= !(CSIZE | PARENB);
        change.c_cflag |= CS8;

        change.c_cc[VMIN] = 1;
        change.c_cc[VTIME] = 1;

        let change = Box::into_raw(change);

        let (r, e) = unsafe {
            let r = tcsetattr(1, TCSADRAIN, change);
            let e = *___errno();
            (r, e)
        };

        unsafe { Box::from_raw(change); };

        if r != 0 {
            return Err(std::io::Error::from_raw_os_error(e));
        }

        Ok(RawMode {
            orig,
        })
    }

    pub fn size(&self) -> std::io::Result<WinSize> {
        let ws = Box::into_raw(
            Box::new(unsafe { std::mem::zeroed::<winsize>() }));

        let (r, e) = unsafe {
            let r = ioctl(1, TIOCGWINSZ, ws);
            let e = *___errno();
            (r, e)
        };

        if r != 0 {
            return Err(std::io::Error::from_raw_os_error(e));
        }

        let ws = unsafe { Box::from_raw(ws) };

        Ok(WinSize {
            rows: ws.ws_row,
            cols: ws.ws_col,
        })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let orig = Box::into_raw(self.orig.to_owned());
        unsafe { tcsetattr(1, TCSADRAIN, orig); }
    }
}
