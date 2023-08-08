mod config;
use config::*;

use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io,
    path::PathBuf,
    time::{Duration, Instant},
};
use tui::{prelude::*, widgets::*};

#[derive(Debug)]
struct App {
    timer: Option<livesplit::Timer>,
    table_state: TableState,
    config: Config,
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            timer: Default::default(),
            table_state: Default::default(),
            config: Config::load().unwrap_or_default(),
        };
        if let Some(split_file) = app.config.split_file.clone() {
            app.load_run(split_file).ok(); // dont care if it fails lol
        }
        app
    }
}

impl App {
    fn load_run<A: Into<PathBuf>>(&mut self, path: A) -> Result<(), Box<dyn Error>> {
        let path: PathBuf = path.into();
        if path.try_exists()? {
            self.config.split_file = Some(path.clone().to_str().unwrap().to_string());
            let bytes = std::fs::read(&path)?;
            let run = livesplit::run::parser::parse_and_fix(&bytes, Some(&path))?.run;
            self.timer = Some(livesplit::Timer::new(run)?);
            Ok(())
        } else {
            Err(String::from("file not found").into())
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 30 frames per second. todo: make it configurable
    let tick_rate = Duration::from_secs_f32(1.0 / 30.0);
    let app = App::default();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                return Ok(())
                            }
                            KeyCode::Char(' ') => {
                                if let Some(ref mut timer) = app.timer {
                                    timer.split_or_start()
                                }
                            }
                            KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                use nfde::*;
                                let Ok(file_dialog) = Nfd::new() else { continue };
                                let res = file_dialog
                                    .open_file()
                                    .add_filter("LiveSplit file", "lss")
                                    .unwrap()
                                    .show();

                                match res {
                                    DialogResult::Ok(path) => {
                                        if path.try_exists().ok() == Some(true) {
                                            app.load_run(path.as_path()).ok();
                                            app.config.save().unwrap();
                                        }
                                    }
                                    _ => continue,
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Event::Paste(data) => {
                    let path = PathBuf::from(data);
                    if path.try_exists().ok() == Some(true) {
                        app.load_run(path).unwrap();
                        app.config.save().ok();
                    }
                }
                _ => (),
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    if app.timer.is_none() {
        let block = Block::default()
            .title("shplit")
            .borders(Borders::ALL)
            .title_alignment(Alignment::Center);
        f.render_widget(
            Paragraph::new("Drag and drop a splits file onto the window, or press CTRL + O.")
                .block(block)
                .alignment(Alignment::Center),
            f.size(),
        );
        return;
    }

    let timer = app.timer.as_mut().unwrap();
    let rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100), Constraint::Min(1)].as_ref())
        .split(f.size());
    app.table_state.select(timer.current_split_index());

    let header = Row::new(["Segment", "Time"]).height(1).bottom_margin(1);

    let rows: Vec<Row> = timer
        .run()
        .segments()
        .iter()
        .map(|segment| {
            let time = segment
                .split_time()
                .game_time
                .map_or(String::from("0:00"), |time| time.to_duration().to_string());
            Row::new([segment.name().to_string(), time])
        })
        .collect();

    let table = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .widths(&[Constraint::Percentage(70), Constraint::Min(5)]);

    f.render_stateful_widget(table, rects[0], &mut app.table_state);
    // hhmmssxxx asf
    let duration = timer.current_attempt_duration().to_duration();
    let timer_text = format!(
        "{:02}:{:02}:{:02}.{:03}",
        duration.whole_hours(),
        duration.whole_minutes() % 60,
        duration.whole_seconds() % 60,
        duration.subsec_milliseconds()
    );
    let paragraph = match timer.current_split().is_some() {
        true => Paragraph::new(timer_text).bold(),
        false => Paragraph::new(timer_text).slow_blink(),
    };
    f.render_widget(paragraph, rects[1]);
}
