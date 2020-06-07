use std::{collections::VecDeque, io, net::TcpStream, sync::mpsc, thread};

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

#[allow(dead_code)]
mod util {
    use std::io;
    use std::sync::mpsc;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::thread;
    use std::time::Duration;

    use termion::event::Key;
    use termion::input::TermRead;

    pub enum Event<I> {
        Input(I),
        Tick,
    }

    /// A small event handler that wrap termion input and tick events. Each event
    /// type is handled in its own thread and returned to a common `Receiver`
    pub struct Events {
        rx: mpsc::Receiver<Event<Key>>,
        input_handle: thread::JoinHandle<()>,
        ignore_exit_key: Arc<AtomicBool>,
        tick_handle: thread::JoinHandle<()>,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Config {
        pub exit_key: Key,
        pub tick_rate: Duration,
    }

    impl Default for Config {
        fn default() -> Config {
            Config {
                exit_key: Key::Char('q'),
                tick_rate: Duration::from_millis(250),
            }
        }
    }

    impl Events {
        pub fn new() -> Events {
            Events::with_config(Config::default())
        }

        pub fn with_config(config: Config) -> Events {
            let (tx, rx) = mpsc::channel();
            let ignore_exit_key = Arc::new(AtomicBool::new(false));
            let input_handle = {
                let tx = tx.clone();
                let ignore_exit_key = ignore_exit_key.clone();
                thread::spawn(move || {
                    let stdin = io::stdin();
                    for evt in stdin.keys() {
                        if let Ok(key) = evt {
                            if let Err(err) = tx.send(Event::Input(key)) {
                                eprintln!("{}", err);
                                return;
                            }
                            if !ignore_exit_key.load(Ordering::Relaxed) && key == config.exit_key {
                                return;
                            }
                        }
                    }
                })
            };

            let tick_handle = {
                thread::spawn(move || loop {
                    tx.send(Event::Tick).unwrap();
                    thread::sleep(config.tick_rate);
                })
            };
            Events {
                rx,
                ignore_exit_key,
                input_handle,
                tick_handle,
            }
        }

        pub fn next(&self) -> Result<Event<Key>, mpsc::TryRecvError> {
            self.rx.try_recv()
        }

        pub fn disable_exit_key(&mut self) {
            self.ignore_exit_key.store(true, Ordering::Relaxed);
        }

        pub fn enable_exit_key(&mut self) {
            self.ignore_exit_key.store(false, Ordering::Relaxed);
        }
    }
}

use util::*;

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

    let (tx, rx) = mpsc::channel();

    let bufread: io::BufReader<Box<dyn io::Read + Send>> =
        match (std::env::var("NMEACLI_ADDR"), std::env::var("NMEACLI_DEV")) {
            (Ok(addr), _) => {
                let stream = TcpStream::connect(addr)?;
                io::BufReader::new(Box::new(stream))
            }
            (_, Ok(dev)) => {
                let file = std::fs::File::open(dev)?;
                io::BufReader::new(Box::new(file))
            }
            _ => {
                panic!("NMEACLI_ADDR or NMEACLI_DEV should be specified");
            }
        };

    let _thread = thread::spawn(move || {
        let tx = tx.clone();

        let mut lines = bufread.lines();
        lines.next();

        for line in lines {
            let line = line.unwrap();
            tx.send(line).ok();
        }
    });

    let mut nmea = Nmea::new();
    let mut messages = VecDeque::new();

    loop {
        if let Ok(line) = rx.try_recv() {
            nmea.parse(&line).ok();
            {
                messages.push_front(Text::raw("\n".to_owned()));
                messages.push_front(Text::raw(line.clone()));

                while messages.len() > 100 {
                    messages.pop_back();
                }
            }
        };

        if let Ok(Event::Input(input)) = events.next() {
            if let Key::Char('q') = input {
                break;
            }
        };

        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(5),
                        Constraint::Min(15),
                        Constraint::Length(10),
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
    }

    terminal.clear()?;
    Ok(())
}
