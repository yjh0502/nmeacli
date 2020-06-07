#[allow(dead_code)]
mod util;

use std::{collections::VecDeque, io, net::TcpStream};

use anyhow::Error;
use io::BufRead;
use nmea::Nmea;
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Text},
    Terminal,
};
use util::{Event, Events};

fn datetime_str(nmea: &Nmea) -> Option<String> {
    let date = nmea.fix_date?;
    let time = nmea.fix_time?;

    Some(format!("{} {}", date, time))
}

fn latlonalt_str(nmea: &Nmea) -> Option<String> {
    Some(format!(
        "{:.6} / {:.6} / {:.6}",
        nmea.latitude?, nmea.longitude?, nmea.altitude?
    ))
}

fn dop_str(nmea: &Nmea) -> Option<String> {
    Some(format!(
        "{:.2} / {:.2} / {:.2}",
        nmea.hdop?, nmea.vdop?, nmea.pdop?
    ))
}

fn option_str(s: Option<String>) -> String {
    match s {
        Some(s) => s,
        None => "<not available>".to_owned(),
    }
}

pub fn tcpaddr() -> String {
    std::env::var("NMEACLI_ADDR").unwrap()
}

fn main() -> Result<(), Error> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    let events = Events::new();

    let stream = TcpStream::connect(tcpaddr())?;
    let bufread = std::io::BufReader::new(stream);

    let mut lines = bufread.lines();
    lines.next();

    let mut nmea = Nmea::new();
    let mut messages = VecDeque::new();

    for line in lines {
        let line = line?;
        {
            nmea.parse(&line).ok();
        }

        {
            messages.push_front(Text::raw("\n".to_owned()));
            messages.push_front(Text::raw(line.clone()));

            while messages.len() > 100 {
                messages.pop_back();
            }
        }

        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(5),
                        Constraint::Min(15),
                        Constraint::Min(10),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            {
                let chunk = chunks[0];
                let block = Block::default().title("Status").borders(Borders::ALL);

                let mut msgs = Vec::new();

                msgs.push(Text::raw(format!(
                    "datetime   : {}\n",
                    option_str(datetime_str(&nmea)),
                )));
                msgs.push(Text::raw(format!(
                    "latlonalt  : {}\n",
                    option_str(latlonalt_str(&nmea)),
                )));
                msgs.push(Text::raw(format!(
                    "dop (h/v/p): {}\n",
                    option_str(dop_str(&nmea)),
                )));

                let body_rect = block.inner(chunk);
                let paragraph = Paragraph::new(msgs.iter()).wrap(true);

                f.render_widget(block, chunk);
                f.render_widget(paragraph, body_rect);
            }

            {
                let chunk = chunks[1];
                let title = format!(
                    "Satlites (fixed={})",
                    option_str(nmea.num_of_fix_satellites.map(|v| v.to_string()))
                );
                let block = Block::default().title(&title).borders(Borders::ALL);

                let mut msgs = Vec::new();

                for sat in &nmea.satellites {
                    msgs.push(Text::raw(format!("{}\n", sat)));
                }

                let body_rect = block.inner(chunk);
                let paragraph = Paragraph::new(msgs.iter()).wrap(true);

                f.render_widget(block, chunk);
                f.render_widget(paragraph, body_rect);
            }

            {
                let chunk = chunks[2];

                let block = Block::default().title("Messages").borders(Borders::ALL);

                let body_rect = block.inner(chunk);
                let paragraph = Paragraph::new(messages.iter()).wrap(true);

                f.render_widget(block, chunk);
                f.render_widget(paragraph, body_rect);
            }
        })?;

        /*
        if let Event::Input(input) = events.next()? {
            if let Key::Char('q') = input {
                break;
            }
        }
        */
    }

    Ok(())
}
