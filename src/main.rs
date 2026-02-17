use arboard::Clipboard;
use clap::Parser;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{self, Color, Stylize},
    terminal::{self, ClearType},
};
use std::fs;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(author, version, about = "A simple RSVP app in Rust")]
struct Cli {
    // file, if not provided, clipboard is used, if that doesn't work (god no), stdin
    file: Option<String>,

    // wpm
    #[arg(short, long, default_value_t = 250)]
    wpm: u32,

    // focus characters
    #[arg(short, long)]
    focus: bool,
}

fn get_orp_index(len: usize) -> usize {
    match len {
        0..=1 => 0,
        2..=5 => 1,
        6..=9 => 2,
        10..=13 => 3,
        _ => 4,
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let mut text = String::new();
    if let Some(ref file_path) = cli.file {
        text = fs::read_to_string(file_path)?;
    } else {
        // Try clipboard first
        if let Ok(mut clipboard) = Clipboard::new() {
            if let Ok(clipboard_text) = clipboard.get_text() {
                if !clipboard_text.trim().is_empty() {
                    text = clipboard_text;
                }
            }
        }

        // fallback to stdin if clipboard is empty and no file was provided
        if text.is_empty() {
            io::stdin().read_to_string(&mut text)?;
        }
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        eprintln!(
            "No text to display. Please provide a file, text in clipboard, or pipe text into the program."
        );
        return Ok(());
    }

    let mut current_idx = 0;
    let mut wpm = cli.wpm;
    let mut paused = true;
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide,)?;

    let mut last_update = Instant::now();

    loop {
        let delay = Duration::from_millis(60_000 / wpm as u64);

        execute!(stdout, terminal::Clear(ClearType::All))?;

        let (cols, rows) = terminal::size()?;
        let center_x = cols / 2;
        let center_y = rows / 2;

        if current_idx < words.len() {
            let word = words[current_idx];
            let orp_idx = if cli.focus {
                get_orp_index(word.len())
            } else {
                usize::MAX
            };

            // display at the center of the screen
            let x_start = if cli.focus && orp_idx < word.len() {
                center_x.saturating_sub(orp_idx as u16)
            } else {
                center_x.saturating_sub((word.len() / 2) as u16)
            };

            execute!(stdout, cursor::MoveTo(x_start, center_y))?;

            for (i, c) in word.chars().enumerate() {
                if cli.focus && i == orp_idx {
                    execute!(
                        stdout,
                        style::PrintStyledContent(c.to_string().with(Color::Red))
                    )?;
                } else {
                    execute!(stdout, style::Print(c))?;
                }
            }

            if cli.focus && orp_idx < word.len() {
                execute!(
                    stdout,
                    cursor::MoveTo(center_x, center_y + 1),
                    style::PrintStyledContent("^".with(Color::Red))
                )?;
            }

            // display WPM and status
            let status_line = format!(
                "WPM: {} | Word: {}/{} | Status: {} | [Space] Toggle [u/d] WPM [n/p] Prev/Next [q] Quit",
                wpm,
                current_idx + 1,
                words.len(),
                if paused { "Paused" } else { "Playing" }
            );
            execute!(
                stdout,
                cursor::MoveTo(0, rows - 1),
                style::Print(status_line)
            )?;
        } else {
            execute!(
                stdout,
                cursor::MoveTo(center_x.saturating_sub(4), center_y),
                style::Print("Finished!")
            )?;
        }

        stdout.flush()?;

        // wait for event or timeout
        let poll_duration = if paused {
            Duration::from_millis(100)
        } else {
            delay.saturating_sub(last_update.elapsed())
        };

        if event::poll(poll_duration)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char(' ') => {
                        paused = !paused;
                        last_update = Instant::now();
                    }
                    KeyCode::Char('n') => {
                        if current_idx + 1 < words.len() {
                            current_idx += 1;
                        }
                        last_update = Instant::now();
                    }
                    KeyCode::Char('p') => {
                        current_idx = current_idx.saturating_sub(1);
                        last_update = Instant::now();
                    }
                    KeyCode::Char('u') => {
                        wpm = wpm.saturating_add(25);
                    }
                    KeyCode::Char('d') => {
                        wpm = wpm.saturating_sub(25).max(25);
                    }
                    _ => {}
                }
            }
        }

        if !paused && last_update.elapsed() >= delay {
            if current_idx + 1 < words.len() {
                current_idx += 1;
                last_update = Instant::now();
            } else {
                paused = true;
                current_idx = words.len();
            }
        }
    }

    // restore terminal
    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
