use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{error::Error, io};
use unicode_width::UnicodeWidthStr;
use dungeon_raid_core::game::{Game, DEFAULT_BOARD_WIDTH, DEFAULT_BOARD_HEIGHT, tile::TilePosition};

enum InputMode {
    Normal,
    Editing,
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

#[derive(Copy, Clone)]
enum CursorMove {
    Up,
    Right,
    Down,
    Left,
}

const CURSOR_MOVE: u16 = 2;
const CURSOR_MAX_UP: u16 = 1;
const CURSOR_MAX_RIGHT: u16 = DEFAULT_BOARD_WIDTH as u16 * 2 - 1;
const CURSOR_MAX_DOWN: u16 = DEFAULT_BOARD_HEIGHT as u16 * 2 - 1;
const CURSOR_MAX_LEFT: u16 = 1;

fn move_cursor<B: Backend>(terminal: &mut Terminal<B>, m: CursorMove) -> io::Result<()> {
    let mut cursor_pos = terminal.get_cursor()?;
    match m {
        CursorMove::Up => {if cursor_pos.1 >= CURSOR_MAX_UP + CURSOR_MOVE {cursor_pos.1 -= CURSOR_MOVE;}},
        CursorMove::Right => {if cursor_pos.0 <= CURSOR_MAX_RIGHT + CURSOR_MOVE {cursor_pos.0 += CURSOR_MOVE;}},
        CursorMove::Down => {if cursor_pos.1 <= CURSOR_MAX_DOWN + CURSOR_MOVE {cursor_pos.1 += CURSOR_MOVE;}},
        CursorMove::Left => {if cursor_pos.0 >= CURSOR_MAX_LEFT + CURSOR_MOVE {cursor_pos.0 -= CURSOR_MOVE;}},
    };
    Ok(())
}

fn tile_position_from_cursor_position(cursor_position: (u16, u16)) -> TilePosition {
    let (x, y) = cursor_position;
    TilePosition::new((2 * y - CURSOR_MAX_UP) as isize, (2 * x - CURSOR_MAX_LEFT) as isize)
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut game = Game::default();
    loop {
        terminal.draw(|f| ui(f, &game))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char(' ') => {game.drop_selection(); game.apply_gravity_and_randomize_new_tiles();},
                KeyCode::Char('x') => {game.select_tile(tile_position_from_cursor_position(terminal.get_cursor()?));},
                KeyCode::Char('h') | KeyCode::Left => move_cursor(terminal, CursorMove::Left)?,
                KeyCode::Char('j') | KeyCode::Down => move_cursor(terminal, CursorMove::Down)?,
                KeyCode::Char('k') | KeyCode::Up => move_cursor(terminal, CursorMove::Up)?,
                KeyCode::Char('l') | KeyCode::Right => move_cursor(terminal, CursorMove::Right)?,
                _ => {},
            };
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, game: &Game) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Min(DEFAULT_BOARD_HEIGHT as u16 * 2 - 1),
            ]
            .as_ref(),
        )
        .split(f.size());

    //let (msg, style) = match app.input_mode {
    //    InputMode::Normal => (
    //        vec![
    //            Span::raw("Press "),
    //            Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
    //            Span::raw(" to exit, "),
    //            Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
    //            Span::raw(" to start editing."),
    //        ],
    //        Style::default().add_modifier(Modifier::RAPID_BLINK),
    //    ),
    //    InputMode::Editing => (
    //        vec![
    //            Span::raw("Press "),
    //            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
    //            Span::raw(" to stop editing, "),
    //            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
    //            Span::raw(" to record the message"),
    //        ],
    //        Style::default(),
    //    ),
    //};
    //let mut text = Text::from("foo");
    //text.patch_style(style);
    //let help_message = Paragraph::new(text);
    //f.render_widget(help_message, chunks[0]);

    //let input = Paragraph::new(app.input.as_str())
    //    .style(match app.input_mode {
    //        InputMode::Normal => Style::default(),
    //        InputMode::Editing => Style::default().fg(Color::Yellow),
    //    })
    //    .block(Block::default().borders(Borders::ALL).title("Input"));
    //f.render_widget(input, chunks[1]);
    //match app.input_mode {
    //    InputMode::Normal =>
    //        // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
    //        {}
//
//        InputMode::Editing => {
//            // Make the cursor visible and ask ratatui to put it at the specified coordinates after rendering
//            f.set_cursor(
//                // Put cursor past the end of the input text
//                chunks[1].x + app.input.width() as u16 + 1,
//                // Move one line down, from the border to the input line
//                chunks[1].y + 1,
//            )
//        }
//    }
}