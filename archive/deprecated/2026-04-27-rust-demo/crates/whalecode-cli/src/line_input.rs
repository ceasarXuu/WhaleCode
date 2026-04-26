use std::io::{self, IsTerminal, Write};

use crossterm::{
    cursor::MoveToColumn,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};

pub(crate) enum LineInput {
    Submit(String),
    Exit,
}

pub(crate) struct LineReader {
    prompt: String,
    interactive: bool,
}

impl LineReader {
    pub(crate) fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            interactive: io::stdin().is_terminal() && io::stdout().is_terminal(),
        }
    }

    pub(crate) fn read_line(&mut self) -> io::Result<LineInput> {
        if !self.interactive {
            return self.read_line_fallback();
        }

        let _raw = RawModeGuard::enable()?;
        let mut stdout = io::stdout();
        write!(stdout, "{}", self.prompt)?;
        stdout.flush()?;

        let mut line = String::new();
        loop {
            let event = event::read().map_err(io::Error::other)?;
            match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match handle_key_event(key, &mut line) {
                        KeyAction::Submit => {
                            write!(stdout, "\r\n")?;
                            stdout.flush()?;
                            return Ok(LineInput::Submit(line));
                        }
                        KeyAction::Exit => {
                            write!(stdout, "\r\n")?;
                            stdout.flush()?;
                            return Ok(LineInput::Exit);
                        }
                        KeyAction::Redraw => redraw_line(&mut stdout, &self.prompt, &line)?,
                        KeyAction::Ignore => {}
                    }
                }
                Event::Paste(text) => {
                    line.push_str(&text);
                    redraw_line(&mut stdout, &self.prompt, &line)?;
                }
                _ => {}
            }
        }
    }

    fn read_line_fallback(&self) -> io::Result<LineInput> {
        let mut stdout = io::stdout();
        write!(stdout, "{}", self.prompt)?;
        stdout.flush()?;

        let mut line = String::new();
        let bytes = io::stdin().read_line(&mut line)?;
        if bytes == 0 {
            return Ok(LineInput::Exit);
        }
        Ok(LineInput::Submit(line))
    }
}

enum KeyAction {
    Submit,
    Exit,
    Redraw,
    Ignore,
}

fn handle_key_event(key: KeyEvent, line: &mut String) -> KeyAction {
    match key.code {
        KeyCode::Enter | KeyCode::Char('\n' | '\r') => KeyAction::Submit,
        KeyCode::Char('j' | 'm') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            KeyAction::Submit
        }
        KeyCode::Backspace => {
            delete_last_char(line);
            KeyAction::Redraw
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Exit,
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) && line.is_empty() => {
            KeyAction::Exit
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            line.push(ch);
            KeyAction::Redraw
        }
        _ => KeyAction::Ignore,
    }
}

fn delete_last_char(line: &mut String) -> bool {
    line.pop().is_some()
}

fn redraw_line(stdout: &mut io::Stdout, prompt: &str, line: &str) -> io::Result<()> {
    execute!(
        stdout,
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        Print(prompt),
        Print(line)
    )
    .map_err(io::Error::other)?;
    stdout.flush()
}

struct RawModeGuard;

impl RawModeGuard {
    fn enable() -> io::Result<Self> {
        enable_raw_mode().map_err(io::Error::other)?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{delete_last_char, handle_key_event, KeyAction};

    #[test]
    fn deletes_cjk_input_by_unicode_scalar_value() {
        let mut line = "中文测试五".to_owned();

        for expected in ["中文测试", "中文测", "中文", "中", ""] {
            assert!(delete_last_char(&mut line));
            assert_eq!(line, expected);
        }
        assert!(!delete_last_char(&mut line));
    }

    #[test]
    fn ctrl_c_exits_the_interactive_prompt() {
        let mut line = String::new();

        let action = handle_key_event(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            &mut line,
        );

        assert!(matches!(action, KeyAction::Exit));
    }
}
