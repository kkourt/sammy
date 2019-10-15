//
// Kornilios Kourtis
// <kkourt@kkourt.io>

extern crate termion;

use std::io::{BufRead, Write};

use termion::raw::IntoRawMode;
use termion::input::TermRead;

// Updating results as user is typing.
// TODO:
//  - first version, where we update results synchronously for every key entered
//  - <up> and <down> selects across results
//  - <Enter> shows selected result


#[derive(Debug)]
enum Mode {
    Command,
    ShowResult,
}


#[derive(Clone,Debug)]
struct Note {
    hdr: String,
    txt: String,
}


#[derive(Debug)]
struct State {
    cmd: String, // user command (as they type it)
    msg: String, // message
    notes: Vec<Note>, // all notes
    matched_notes: Vec<usize>, // indexes of matched notes in the notes vector
    mode: Mode,
}

fn parse_notes<P: AsRef<std::path::Path>>(fname: P) -> std::io::Result<Vec<Note>> {
    let f = std::io::BufReader::new(std::fs::File::open(fname)?);

    let mut note = Note {
        hdr: String::from(""),
        txt: String::from(""),
    };

    let mut ret : Vec<Note> = vec![];
    for line_res in f.lines() {
        let line = line_res?;

        // empty line or comment, ignore
        if line.len() == 0 || line.starts_with("#") {
            continue
        }

        // note separator
        if line == "%" {
            if note.hdr.len() > 0 {
                ret.push(note);
                note = Note {
                    hdr: String::from(""),
                    txt: String::from(""),
                };
            }
            continue
        }

        if note.hdr.len() == 0 {
            note.hdr.push_str(&line);
        } else {
            if note.txt.len() > 0 {
                note.txt.push('\n');
            }
            note.txt.push_str(&line);
        }
    }

    Ok(ret)
}

impl State {

    pub fn new<P: AsRef<std::path::Path>>(fname: P) -> std::io::Result<State> {
        let notes = parse_notes(fname)?;
        Ok(State {
            cmd: String::from(""),
            msg: String::from("Type keywords and hit return to see results. <Esc> quits."),
            notes: notes,
            matched_notes: vec![],
            mode: Mode::Command,
        })
    }

    fn filter_notes(&mut self) {
        let kws: Vec<String> = self.cmd.split_whitespace().map(|s| String::from(s)).collect();
            self.matched_notes = self.notes.iter().enumerate().filter_map(
            |(note_id, note)| {
                for kw in kws.iter() {
                    if note.hdr.find(kw).is_some() || note.txt.find(kw).is_some() {
                        return Some(note_id);
                    }
                }
                return None;
            }
        ).collect();
    }

    fn list_results(&mut self) {
        self.filter_notes();
    }

    fn list_prev_result(&mut self) {
    }

    fn list_next_result(&mut self) {
    }
}

fn render_state<W: std::io::Write>(out: &mut W, st: &State) -> std::io::Result<()> {
    let (_twidth, theight) = termion::terminal_size()?;

    if theight < 3 {
        let errmsg = format!("Terminal height too small: {}", theight);
        let err = std::io::Error::new(std::io::ErrorKind::Other, errmsg);
        return Err(err)
    }
    let reslines_nr = theight - 2;

    write!(out, "{}", termion::clear::All)?;
    let gray = termion::color::AnsiValue::grayscale(12);
    let gray_fg = termion::color::Fg(gray);
    let color_reset = termion::color::Fg(termion::color::Reset);
    write!(out, "{}{}[{}]{}", termion::cursor::Goto(1,2), gray_fg, st.msg, color_reset)?;
    match st.mode {
        Mode::Command => {
            let nlines = std::cmp::min(reslines_nr, st.matched_notes.len() as u16);
            for i in 0..nlines {
                let note = &st.notes[st.matched_notes[i as usize]];
                write!(out, "{}{}", termion::cursor::Goto(1,3 + i), note.hdr)?;
            }
        },
        Mode::ShowResult => {
            unimplemented!()
        }
    }
    write!(out, "{}> {}", termion::cursor::Goto(1,1), st.cmd)?;
    write!(out, "{}", termion::cursor::Show)?;

    out.flush()?;

    Ok(())
}


fn load_words() -> std::io::Result<Vec<String>> {
    let mut f = std::fs::File::open("/usr/share/dict/words")?;
    let mut bf = std::io::BufReader::new(f);

    let words: Vec<String> = bf.lines().map(|l| l.unwrap() ).collect();
    Ok(words)
}

fn main() {

    let mut st : State = {
        match State::new("notes.txt") {
            Err(err) => {
                eprintln!("Error initializing: {}", err);
                return
            },
            Ok(x) => x,
        }
    };

    // NB: played around with async, but not going to use it for now
    // let mut inp = termion::async_stdin().keys();
    let mut inp = std::io::stdin().keys();
    let mut out = std::io::stdout().into_raw_mode().unwrap();

    loop {

        if let Result::Err(_err) = render_state(&mut out, &st) {
            break;
        }

        use termion::event::Key;
        if let Some(uk) = inp.next() {
            match uk {

                Ok(Key::Esc) => match st.mode {
                    Mode::Command => break,
                    Mode::ShowResult => unimplemented!(),
                },

                Ok(Key::Char('\n')) => match st.mode {
                    Mode::Command => (),
                    Mode::ShowResult => unimplemented!(),
                },

                Ok(Key::Char(c)) => match st.mode {
                    Mode::Command => st.cmd.push(c),
                    Mode::ShowResult => unimplemented!(),
                },

                Ok(Key::Up) => match st.mode {
                    _ => (),
                },

                Ok(Key::Down) => match st.mode {
                    _ => (),
                },

                // TODO: arbitrary cursor position
                // Ok(Key::Left) => match st.mode { }
                // Ok(Key::Right) => match st.mode { }

                Ok(Key::Backspace) => match st.mode {
                    Mode::Command =>  { st.cmd.pop(); },
                    _ => ()
                },

                Ok(Key::Ctrl('w')) => match st.mode {
                    Mode::Command => st.cmd.truncate(0),
                    _ => (),
                }

                Ok(_) => (),

                Err(err) => {
                    write!(out, "{}", termion::clear::All).unwrap_or(());
                    write!(out, "{}", termion::cursor::Goto(1,1)).unwrap_or(());
                    write!(out, "Error: {:?}", err).unwrap_or(());
                    break
                },
            }
        } else {
            // NB: This is going to be executed if we use async, and there is no input.
            std::thread::sleep(std::time::Duration::from_millis(100))
        }

        st.list_results();
    }
}
