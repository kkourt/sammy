//
// Kornilios Kourtis
// <kkourt@kkourt.io>

extern crate termion;

use std::io::{BufRead, Write};

use termion::raw::IntoRawMode;
use termion::input::TermRead;


// state
struct State {
    usr: String,       // user (visible) string
    top: Vec<String>,
    bot: Vec<String>
}

fn render_state<W: std::io::Write>(out: &mut W, st: &State) -> std::io::Result<()> {
    // let (twidth, theight) = termion::terminal_size()?;
    write!(out, "{}", termion::clear::All)?;
    write!(out, "{}", termion::cursor::Goto(1,1))?;
    write!(out, "> {}", st.usr)?;
    write!(out, "{}", termion::cursor::Show)?;
    out.flush()?;
    Ok(())
}


fn load_words() -> std::io::Result<Vec<String>> {
    let mut f = std::fs::File::open("/usr/share/dict/words")?;
    let mut bf = std::io::BufReader::new(f);

    let words = bf.lines().map(|l| l.unwrap() ).collect();
    Ok(words)
}

fn main() {

    let mut st = State {
        usr: String::from(""),
        top: vec![],
        bot: vec![],
    };

    let mut out = std::io::stdout().into_raw_mode().unwrap();
    // let mut inp = std::io::stdin().keys();
    let mut inp = termion::async_stdin().keys();
    loop {

        if let Result::Err(err) = render_state(&mut out, &st) {
            break;
        }

        use termion::event::Key;
        if let Some(uk) = inp.next() {
            match uk {
                Ok(Key::Char('\n')) => (),
                Ok(Key::Char(c)) => st.usr.push(c),
                Ok(Key::Esc) =>  break,
                Ok(Key::Backspace) => { st.usr.pop(); () },
                Ok(Key::Ctrl('w')) => st.usr.truncate(0),
                Ok(_) => break,
                Err(err) => {
                    write!(out, "{}", termion::clear::All).unwrap_or(());
                    write!(out, "{}", termion::cursor::Goto(1,1)).unwrap_or(());
                    write!(out, "Error: {:?}", err).unwrap_or(());
                    break
                },
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(100))
        }
    }
}
