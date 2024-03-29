//
// Kornilios Kourtis
// <kkourt@kkourt.io>

extern crate termion;
extern crate dirs;

use std::io::{BufRead, Write};

use termion::raw::IntoRawMode;
use termion::input::TermRead;

#[derive(Debug)]
enum Mode {
    Command,
    ShowNote,
}

#[derive(Clone,Debug)]
struct ViewNotes {
    first: usize,
    selected: usize,
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
    view_notes: Option<ViewNotes>,
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

    if note.hdr.len() > 0 {
        ret.push(note)
    }

    Ok(ret)
}

impl State {

    pub fn new<P: AsRef<std::path::Path>>(fname: P) -> std::io::Result<State> {
        let notes = parse_notes(fname)?;
        let notes_nr = notes.len();
        let view_notes = if notes_nr > 0 { Some(ViewNotes { first: 0, selected: 0 } ) } else { None };
        Ok(State {
            cmd: String::from(""),
            msg: String::from("Type keywords, and use arrows to select. <Esc> and q quit."),
            notes: notes,
            matched_notes: (0..notes_nr).collect(),
            view_notes: view_notes,
            mode: Mode::Command,
        })
    }

    fn set_mode(&mut self, m: Mode) {
        self.mode = m;
        match self.mode {
            Mode::Command => {
                self.msg = String::from("Type keywords, and use arrows to select. <Esc> quits.")
            },
            Mode::ShowNote => {
                self.msg = String::from("Arrows scroll. <Esc> returns to the list.")
            }
        }
    }

    fn reset_view_notes(&mut self) {
        match self.mode {
            Mode::Command => if self.matched_notes.len() > 0 {
                self.view_notes = Some(ViewNotes { first: 0, selected: 0 } );
            },
            Mode::ShowNote => (),
        }

    }

    fn filter_notes(&mut self) {

        let kws: Vec<String> = self.cmd.split_whitespace().map(|s| s.to_lowercase()).collect();
            self.matched_notes = self.notes.iter().enumerate().filter_map(
            |(note_id, note)| {
                let hdr_lc = note.hdr.to_lowercase();
                let txt_lc = note.txt.to_lowercase();
                // match for _all_ keywords (AND)
                for kw in kws.iter() {
                    if hdr_lc.find(kw).is_none() && txt_lc.find(kw).is_none() {
                        return None;
                    }
                }
                // matched all keywords: return note id
                return Some(note_id);
            }
        ).collect();
    }

    fn prev_result(&mut self) {
        if let Some(vn) = &mut self.view_notes {
            assert!(vn.selected >= vn.first);
            if vn.selected > vn.first {
                vn.selected -= 1;
                return;
            } else if vn.first > 1 {
                vn.first -= 1;
                vn.selected -= 1;
                return;
            }
        }
    }

    fn next_result(&mut self) {
        // FIXME: this breaks the logic/presentation separation
        if let Some(vn) = &mut self.view_notes {
            let (_twidth, theight) = termion::terminal_size().unwrap();
            assert!(theight > 2); // FIXME
            let reslines_nr = (theight - 2) as usize;

            // we are already on the last note
            if vn.selected + 1 == self.matched_notes.len() {
                return
            }

            vn.selected += 1;
            if vn.selected - vn.first == reslines_nr {
                vn.first += 1;
            }

        }

    }
}

fn render_state<W: std::io::Write>(out: &mut W, st: &State) -> std::io::Result<()> {
    let (_twidth, theight) = termion::terminal_size()?;

    if theight < 4 {
        let errmsg = format!("Terminal height too small: {}", theight);
        let err = std::io::Error::new(std::io::ErrorKind::Other, errmsg);
        return Err(err)
    }
    let reslines_nr = (theight - 2) as usize;

    write!(out, "{}", termion::clear::All)?;
    let gray = termion::color::AnsiValue::grayscale(12);
    let gray_fg = termion::color::Fg(gray);
    let color_reset = termion::color::Fg(termion::color::Reset);
    write!(out, "{}{}[{}]{}", termion::cursor::Goto(1,2), gray_fg, st.msg, color_reset)?;
    match st.mode {
        Mode::Command => {
            let first: usize = st.view_notes.as_ref().map_or(0, |x| x.first);
            let hl = st.view_notes.as_ref().map(|x| x.selected);
            let nlines = std::cmp::min(reslines_nr, st.matched_notes.len() - first);
            for (i,l) in (first..first+nlines).enumerate() {
                let note = &st.notes[st.matched_notes[l]];
                if Some(l) == hl {
                    write!(out, "{}-> {}", termion::cursor::Goto(1,3 + (i as u16)), note.hdr)?;
                } else {
                    write!(out, "{}   {}", termion::cursor::Goto(1,3 + (i as u16)), note.hdr)?;
                }
            }
        },
        Mode::ShowNote => {
            let sel = st.view_notes.as_ref().map(|x| x.selected).unwrap();
            let note = &st.notes[st.matched_notes[sel]];

            let mut i = 0;

            //let bold = termion::style::Bold;
            //let nobold = termion::style::NoBold;
            write!(out, "{}{}", termion::cursor::Goto(1,3 + (i as u16)), note.hdr)?;
            i += 2;

            let mut start = 0;
            for line in note.txt.lines() {
                if start > 0 { // skip lines until we reach the start
                    start -= 1;
                    continue;
                }
                write!(out, "{} {}", termion::cursor::Goto(1,3 + (i as u16)), line)?;
                i += 1;
                if i >= reslines_nr {
                    break;
                }
            }
        }
    }
    write!(out, "{}> {}", termion::cursor::Goto(1,1), st.cmd)?;
    write!(out, "{}", termion::cursor::Show)?;

    out.flush()?;

    Ok(())
}


fn main() {

    let notes_path = dirs::home_dir().expect("Cannot locate user's $HOME").join(".sammy-notes");

    let mut st : State = {
        match State::new(notes_path) {
            Err(err) => {
                eprintln!("Error initializing: {}", err);
                return
            },
            Ok(x) => x,
        }
    };

    // NB: played around with async, but not going to use it for now
    // let mut inp = termion::async_stdin().keys();
    // If the notes file become too big to do this synchrously, we can implement async.
    let mut inp = std::io::stdin().keys();
    let mut out = std::io::stdout().into_raw_mode().unwrap();

    loop {

        if let Result::Err(_err) = render_state(&mut out, &st) {
            break;
        }

        let mut cmd_changed = false;
        use termion::event::Key;
        if let Some(uk) = inp.next() {
            match uk {

                Ok(Key::Esc) => match st.mode {
                    Mode::Command => break,
                    Mode::ShowNote => st.set_mode(Mode::Command),
                },

                Ok(Key::Char('\n')) => match st.mode {
                    Mode::Command => {
                        st.set_mode(Mode::ShowNote);
                    },
                    Mode::ShowNote => (),
                },

                Ok(Key::Char(c)) => match st.mode {
                    Mode::Command => {
                        st.cmd.push(c);
                        cmd_changed = true
                    },
                    Mode::ShowNote => (),
                },

                Ok(Key::Up) => match st.mode {
                    Mode::Command => st.prev_result(),
                    Mode::ShowNote => (),
                },

                Ok(Key::Down) => match st.mode {
                    Mode::Command => st.next_result(),
                    Mode::ShowNote => (),
                },

                // TODO: support change cursor position
                // Ok(Key::Left) => match st.mode { }
                // Ok(Key::Right) => match st.mode { }

                Ok(Key::Backspace) => match st.mode {
                    Mode::Command =>  {
                        st.cmd.pop();
                        cmd_changed = true;
                    },
                    _ => ()
                },

                Ok(Key::Ctrl('w')) => match st.mode {
                    Mode::Command => {
                        st.cmd.truncate(0);
                        cmd_changed = true;
                    }
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

        if cmd_changed {
            st.filter_notes();
            st.reset_view_notes();
        }
    }
}
