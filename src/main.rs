mod ansiterm;
mod rawmode;

use std::f64;
use std::io::{Read, BufReader};

use ansiterm::ANSITerm;

use serde::Deserialize;
use serde_json::Value;

use std::time::SystemTime;
use std::cell::RefCell;


const GRAV: f64 = 6.67e-11f64;

#[derive(Debug, Deserialize)]
struct Object {
    name: String,
    colour: u8,
    mass: f64,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    #[serde(default)]
    fixed: bool,
}

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Trace {
    p: Point,
    when: SystemTime,
}

#[derive(Debug, Deserialize)]
struct Settings {
    settings: bool,
    tracelife: f64,
}

struct Space<'a> {
    settings: Settings,
    objects: Vec<Object>,
    term: ANSITerm<'a>,
    scalemax: f64,
    traces: RefCell<Vec<Trace>>,

    xmax: i32,
    xmin: i32,
    ymax: i32,
    ymin: i32,
}

impl Space<'_> {
    fn xw(&self) -> f64 {
        (self.xmax - self.xmin) as f64
    }

    fn yw(&self) -> f64 {
        (self.ymax - self.ymin) as f64
    }

    fn xc(&self) -> f64 {
        self.xw() / 2.0 + self.xmin as f64
    }

    fn yc(&self) -> f64 {
        self.yw() / 2.0 + self.ymin as f64
    }

    fn ysp(&self) -> f64 {
        self.scalemax * 1.3_f64
    }

    fn xsp(&self) -> f64 {
        self.ysp() / (self.yw() / self.xw()) * 0.444_f64
    }

    fn add_trace(&self, now: &SystemTime, x: i32, y: i32) {
        let mut traces = self.traces.borrow_mut();

        for t in traces.iter_mut() {
            if t.p.x == x && t.p.y == y {
                t.when = *now;
                return;
            }
        }

        traces.push(Trace {
            p: Point { x, y, },
            when: *now,
        });
    }
}

fn find_scalemax(objects: &[Object]) -> f64 {
    let mut xmax = 0_f64;
    let mut ymax = 0_f64;

    for o in objects {
        let x = o.x;
        let y = o.y;

        xmax = xmax.max(x.abs() / 2.0).round();
        ymax = ymax.max(y.abs() / 2.0).round();
    }

    ymax.max(xmax / 0.6).round()
}

fn space_to_screen(s: &Space, x: f64, y: f64) -> Point {
    let x = s.xc() + (s.xw() / 2.0) * (x / s.xsp());
    let y = s.yc() + (s.yw() / 2.0) * (y / s.ysp());

    let x = x.round() as i32;
    let y = y.round() as i32;

    Point { x, y, }
}

fn draw_space(s: &mut Space, now: &SystemTime, clear: bool) {
    if !clear {
        s.term.reset();
        s.term.draw_box(s.xmin - 1, s.ymin - 1, s.xmax + 1, s.ymax + 1);
        s.term.moveto(s.xmin - 1 + (s.xmax - s.xmin) / 2 - 5, s.ymin - 1);
        s.term.write("  S P A C E  ");
    }

    let traces = s.traces.borrow();

    for t in traces.iter() {
        if let Ok(dur) = now.duration_since(t.when) {
            if dur.as_millis() > (s.settings.tracelife * 1000.0) as u128 {
                if clear {
                    s.term.moveto(t.p.x, t.p.y);
                    s.term.write(" ");
                }
                continue;
            }

            if clear {
                continue;
            }

            let intens = (255.0 -
                dur.as_millis() as f64 / (s.settings.tracelife * 1000.0) *
                (255.0 - 232.0)).floor() as u8;

            s.term.moveto(t.p.x, t.p.y);
            s.term.fg8(intens);
            s.term.write("·");
        }
    }
    s.term.reset();
    drop(traces);

    for o in &s.objects {
        let p = space_to_screen(s, o.x, o.y);

        if p.x < s.xmin || p.y < s.ymin || p.x > s.xmax || p.y > s.ymax {
            continue;
        }

        s.term.moveto(p.x, p.y);
        if clear {
            for _ in 0..o.name.len() {
                s.term.write(" ");
            }
        } else {
            s.add_trace(now, p.x, p.y);
            s.term.fg8(o.colour);
            s.term.write(&o.name);
        }
    }
    s.term.reset();
}

fn move_things(s: &mut Space) {
    for i in 0..s.objects.len() {
        let o = &s.objects[i];

        if o.fixed {
            continue;
        }

        let mut Fx = 0_f64;
        let mut Fy = 0_f64;

        for j in 0..s.objects.len() {
            if i == j {
                continue;
            }

            let oo = &s.objects[j];

            let dx = oo.x - o.x;
            let dy = oo.y - o.y;

            let theta = dy.atan2(dx);
            let rsq = dy.powi(2) + dx.powi(2);

            let Fg = GRAV * o.mass * oo.mass / rsq;

            Fx += Fg * theta.cos();
            Fy += Fg * theta.sin();
        }

        let o = &mut s.objects[i];

        o.vx += Fx / o.mass;
        o.vy += Fy / o.mass;

        o.x += o.vx;
        o.y += o.vy;
    }
}

fn load(n: &str)
    -> Result<(Vec<Object>, Settings), Box<dyn std::error::Error>>
{
    let p = jmclib::dirs::rootpath(&format!("defs/{}.json", n))?;

    let f = std::fs::File::open(p)?;
    let br = BufReader::new(f);

    let mut settings: Option<Settings> = None;
    let mut objects: Vec<Object> = Vec::new();

    let values: Vec<Value> = serde_json::from_reader(br)?;
    for v in values {
        if let Some(o) = v.as_object() {
            if o.contains_key("settings") {
                settings = serde_json::from_value(v)?;
                continue;
            }
        }

        objects.push(serde_json::from_value(v)?);
    }

    let settings = settings.unwrap_or_else(|| Settings {
        settings: true,
        tracelife: 6.0,
    });

    Ok((objects, settings))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (objects, settings) = if let Some(n) = std::env::args().nth(1) {
        load(&n)?
    } else {
        eprintln!("Usage: space <universe_name>");
        std::process::exit(1);
    };

    let term = ANSITerm::new()?;
    term.alternate();
    term.clear();
    term.cursor(false);

    let sz = term.size();
    let scalemax = find_scalemax(&objects);
    let xmin = 2_i32;
    let xmax = sz.cols as i32 - 1;
    let ymin = 2_i32;
    let ymax = sz.rows as i32 - 1;

    let mut space = Space {
        objects,
        settings,
        term,
        scalemax,
        xmin,
        xmax,
        ymin,
        ymax,
        traces: RefCell::new(Vec::new()),
    };

    let mut then = SystemTime::now();
    for _ in 0..1000 {
        for _ in 0..20_000 {
            move_things(&mut space);
        }

        let now = SystemTime::now();
        draw_space(&mut space, &then, true);
        draw_space(&mut space, &now, false);
        space.term.flush();
        then = now;

        std::thread::sleep(std::time::Duration::from_millis(40));
    }

    space.term.soft_reset();

    Ok(())
}
