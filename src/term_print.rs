extern crate term;

#[allow(dead_code)]
pub fn term_rprint(color: self::term::color::Color, status_text: &str, text: &str) {
    let mut t = self::term::stdout().unwrap();

    t.carriage_return().unwrap();
    t.delete_line().unwrap();
    t.attr(self::term::Attr::Bold).unwrap();
    t.fg(color).unwrap();
    write!(t, "{} ", status_text).unwrap();
    let _ = t.reset();
    write!(t, "{}", text).unwrap();
    t.flush().unwrap();
}

#[allow(dead_code)]
pub fn term_rprint_finish() {
    let mut t = self::term::stdout().unwrap();

    write!(t, "\n").unwrap();
    t.flush().unwrap();
}

#[allow(dead_code)]
pub fn term_print(color: self::term::color::Color,
                  status_text: &str,
                  text: &str) {
    term_print_(color, status_text, text, false);
}

pub fn term_println(color: self::term::color::Color,
                    status_text: &str,
                    text: &str) {
    term_print_(color, status_text, text, true);
}

fn term_print_(color: self::term::color::Color,
               status_text: &str,
               text: &str,
               newline: bool) {
    let mut t = self::term::stdout().unwrap();

    t.attr(self::term::Attr::Bold).unwrap();
    t.fg(color).unwrap();
    write!(t, "{} ", status_text).unwrap();
    let _ = t.reset();

    if newline {
        write!(t, "{}\n", text).unwrap();
    } else {
        write!(t, "{}", text).unwrap();
        t.flush().unwrap();
    }
}

#[cfg(not(debug_assertions))]
pub fn term_panic(text: &str) {
    term_println(self::term::color::BRIGHT_RED, "Failure:", text);
}

