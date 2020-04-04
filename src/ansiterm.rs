use std::cell::RefCell;
use std::io::Write;

const ESC: &[u8] = b"\x1b";
const CSI: &[u8] = b"\x1b[";

struct LineDraw<'a> {
    horiz: &'a [u8],
    verti: &'a [u8],
    topleft: &'a [u8],
    topright: &'a [u8],
    bottomright: &'a [u8],
    bottomleft: &'a [u8],
}

const LINEDRAW_VT100: LineDraw<'static> = LineDraw {
    horiz: b"\x71",
    verti: b"\x78",
    topleft: b"\x6c",
    topright: b"\x6b",
    bottomright: b"\x6a",
    bottomleft: b"\x6d",
};

pub struct ATInner {
    out: std::io::Stdout,
    linedraw_count: u32,
    size: super::rawmode::WinSize,
}

pub struct ANSITerm<'a> {
    linedraw: &'a LineDraw<'a>,
    rawmode: super::rawmode::RawMode,
    inner: RefCell<ATInner>,
}

/*
 * XXX
 * Need:
 *  drawBox(x0, y0, x1, y1)
 */

impl ANSITerm<'_> {
    pub fn new() -> std::io::Result<ANSITerm<'static>> {
        let rawmode = super::rawmode::RawMode::enable()?;
        let inner = RefCell::new(ATInner {
            out: std::io::stdout(),
            linedraw_count: 0,
            size: rawmode.size()?,
        });

        Ok(ANSITerm {
            linedraw: &LINEDRAW_VT100,
            rawmode,
            inner,
        })
    }

    fn writeb(&self, b: &[u8]) {
        self.inner.borrow_mut().out.write(b);
    }

    pub fn flush(&self) {
        self.inner.borrow_mut().out.flush();
    }

    pub fn size(&self) -> super::rawmode::WinSize {
        self.inner.borrow().size.clone()
    }

    pub fn linedraw_enable(&self) {
        let en = {
            let mut inner = self.inner.borrow_mut();
            inner.linedraw_count += 1;
            inner.linedraw_count == 1
        };

        if en {
            self.writeb(ESC);
            self.writeb(b"(0");
            // self.flush();
        }
    }

    pub fn linedraw_disable(&self) {
        let di = {
            let mut inner = self.inner.borrow_mut();

            assert!(inner.linedraw_count > 0);
            inner.linedraw_count -= 1;
            inner.linedraw_count == 0
        };

        if di {
            self.writeb(ESC);
            self.writeb(b"(B");
            // self.flush();
        }
    }

    pub fn draw_horiz_line(&self, y: i32, xfrom: i32, xto: i32) {
        self.moveto(xfrom, y);
        self.linedraw_enable();
        for _ in 0..=(xto - xfrom) {
            self.writeb(self.linedraw.horiz);
        }
        self.linedraw_disable();
        // self.flush();
    }

    pub fn draw_verti_line(&self, x: i32, yfrom: i32, yto: i32) {
        self.moveto(x, yfrom);
        self.linedraw_enable();
        for _ in yfrom..=yto {
            self.writeb(self.linedraw.verti);
            self.writeb(CSI);
            self.writeb(b"B");
            self.writeb(CSI);
            self.writeb(format!("{}G", x).as_bytes());
        }
        self.linedraw_disable();
        // self.flush();
    }

    pub fn draw_box(&self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.linedraw_enable();
        self.moveto(x1, y1);
        self.writeb(self.linedraw.topleft);
        self.moveto(x2, y1);
        self.writeb(self.linedraw.topright);
        self.moveto(x1, y2);
        self.writeb(self.linedraw.bottomleft);
        self.moveto(x2, y2);
        self.writeb(self.linedraw.bottomright);
        self.draw_horiz_line(y1, x1 + 1, x2 - 1);
        self.draw_horiz_line(y2, x1 + 1, x2 - 1);
        self.draw_verti_line(x1, y1 + 1, y2 - 1);
        self.draw_verti_line(x2, y1 + 1, y2 - 1);
        self.linedraw_disable();
        // self.flush();
    }

    pub fn moveto(&self, mut x: i32, mut y: i32) {
        let s = self.size();
        if x < 0 {
            x = s.cols as i32 + x + 1;
        }
        if y < 0 {
            y = s.rows as i32 + y + 1;
        }
        let b = format!("{};{}f", y, x);
        self.writeb(CSI);
        self.writeb(b.as_bytes());
        // self.flush();
    }

    pub fn bold(&self) {
        self.writeb(CSI);
        self.writeb(b"1m");
        // self.flush();
    }

    pub fn fg8(&self, fg: u8) {
        self.writeb(CSI);
        self.writeb(format!("38;5;{}m", fg).as_bytes());
        // self.flush();
    }

    pub fn reset(&self) {
        self.writeb(CSI);
        self.writeb(b"m");
        // self.flush();
    }

    pub fn clear(&self) {
        self.writeb(CSI);
        self.writeb(b"2J");
        // self.flush();
    }

    pub fn cursor(&self, enable: bool) {
        self.writeb(CSI);
        self.writeb(b"?25");
        if enable {
            self.writeb(b"h");
        } else {
            self.writeb(b"l");
        }
        // self.flush();
    }

    pub fn replace_mode(&self) {
        self.writeb(CSI);
        self.writeb(b"4l");
        // self.flush();
    }

    pub fn insert_mode(&self) {
        self.writeb(CSI);
        self.writeb(b"4h");
        // self.flush();
    }

    pub fn alternate(&self) {
        self.writeb(CSI);
        self.writeb(b"?47h");
    }

    pub fn normal(&self) {
        self.writeb(CSI);
        self.writeb(b"?47l");
    }

    pub fn write(&self, s: &str) {
        self.writeb(s.as_bytes());
        // self.flush();
    }

    pub fn soft_reset(&self) {
        self.normal();
        self.cursor(true);
        self.replace_mode();
        self.reset();
    }
}

impl Drop for ANSITerm<'_> {
    fn drop(&mut self) {
        self.soft_reset();
        self.flush();
    }
}
