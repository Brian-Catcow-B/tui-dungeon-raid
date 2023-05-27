use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dungeon_raid_core::game::{
    tile::{Tile, TilePosition, TileType, Wind8},
    Game, DEFAULT_BOARD_HEIGHT, DEFAULT_BOARD_WIDTH,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::Rect,
    widgets::Widget,
    Frame, Terminal,
};
use std::{error::Error, io, io::prelude::*};

const LOG_FILE: &'static str = "log.txt";
fn clear_log_file() {
    let mut file = std::fs::File::create(LOG_FILE).expect("failed to create file");
    write!(&mut file, "").expect("failed to write file");
}
fn log_to_file(msg: &String) {
    let mut file = std::fs::File::options().append(true).create(true).open(LOG_FILE).expect("failed to create file");
    writeln!(&mut file, "{}", msg).expect("failed to write file");
}

fn main() -> Result<(), Box<dyn Error>> {
    clear_log_file();
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal);

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
const CURSOR_MAX_UP: u16 = 0;
const CURSOR_MAX_RIGHT: u16 = CURSOR_MAX_LEFT + DEFAULT_BOARD_WIDTH as u16 * 2 - 1;
const CURSOR_MAX_DOWN: u16 = CURSOR_MAX_UP + DEFAULT_BOARD_HEIGHT as u16 * 2 - 1;
const CURSOR_MAX_LEFT: u16 = 0;

fn move_cursor<B: Backend>(terminal: &mut Terminal<B>, m: CursorMove) -> io::Result<(u16, u16)> {
    let mut cursor_pos = terminal.get_cursor()?;
    log_to_file(&format!("move_cursor called with terminal.get_cursor() giving (x {}, y {})", cursor_pos.0, cursor_pos.1));
    match m {
        CursorMove::Up => {
            if cursor_pos.1 >= CURSOR_MAX_UP + CURSOR_MOVE {
                cursor_pos.1 -= CURSOR_MOVE;
            }
        }
        CursorMove::Right => {
            if cursor_pos.0 <= CURSOR_MAX_RIGHT - CURSOR_MOVE {
                cursor_pos.0 += CURSOR_MOVE;
            }
        }
        CursorMove::Down => {
            if cursor_pos.1 <= CURSOR_MAX_DOWN - CURSOR_MOVE {
                cursor_pos.1 += CURSOR_MOVE;
            }
        }
        CursorMove::Left => {
            if cursor_pos.0 >= CURSOR_MAX_LEFT + CURSOR_MOVE {
                cursor_pos.0 -= CURSOR_MOVE;
            }
        }
    };
    Ok(cursor_pos)
}

fn tile_position_from_cursor_position(cursor_position: (u16, u16)) -> TilePosition {
    log_to_file(&format!("tile_position_from_cursor_position: (cursor.x {}, cursor.y {})", cursor_position.0, cursor_position.1));
    let (x, y) = cursor_position;
    TilePosition::new(
        ((y - CURSOR_MAX_UP) / 2) as isize,
        ((x - CURSOR_MAX_LEFT) / 2) as isize,
    )
}

fn blot_char_from_tile_type(tile_type: TileType) -> char {
    match tile_type {
        TileType::Heart => 'h',
        TileType::Shield => 's',
        TileType::Coin => 'c',
        TileType::Sword => 'S',
        TileType::Enemy => 'E',
        TileType::Boss => 'B',
        _ => '!',
    }
}

struct GameWidget<'a> {
    pub game: &'a Game,
}
impl<'a> Widget for GameWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for x in 0..(DEFAULT_BOARD_WIDTH as u16) {
            let blot_x = x * 2;
            for y in 0..(DEFAULT_BOARD_HEIGHT as u16) {
                let blot_y = y * 2;
                let t: Tile = self
                    .game
                    .get_tile(TilePosition::new(y as isize, x as isize))
                    .expect("plz");
                let blot = blot_char_from_tile_type(t.tile_type);
                buf.get_mut(blot_x, blot_y).set_char(blot);
                let mut arrow_blot_x = blot_x;
                let mut arrow_blot_y = blot_y;
                let mut arrow_blot: char;
                let relative_next = t.next_selection;
                match relative_next {
                    Wind8::None => continue,
                    _ => {
                        let tp = TilePosition::try_from(relative_next).expect("TilePosition::TryFrom<Wind8> should always succeed when not Wind8::None");
                        arrow_blot = match tp.y {
                            -1 => {
                                arrow_blot_y -= 1;
                                match tp.x {
                                -1 => '\\',
                                0 => '|',
                                1 => '/',
                                _ => unreachable!("unattainable TilePosition resulting from TilePosition::TryFrom<Wind8>") ,
                            }},
                            0 => match tp.x {
                                -1 | 1 => '-',
                                _ => unreachable!("unattainable TilePosition resulting from TilePosition::TryFrom<Wind8>") ,
                            },
                            1 => {
                                arrow_blot_y += 1;
                                match tp.x {
                                -1 => '/',
                                0 => '|',
                                1 => '\\',
                                _ => unreachable!("unattainable TilePosition resulting from TilePosition::TryFrom<Wind8>") ,
                            }},
                            _ => unreachable!("unattainable TilePosition resulting from TilePosition::TryFrom<Wind8>") ,
                        };
                        match tp.x {
                            -1 => arrow_blot_x -= 1,
                            1 => arrow_blot_x += 1,
                            _ => {}
                        };
                    }
                };
                match buf.get(arrow_blot_x, arrow_blot_y).symbol.chars().next() {
                    Some('/') | Some('\\') => arrow_blot = 'X',
                    _ => {},
                }
                buf.get_mut(arrow_blot_x, arrow_blot_y).set_char(arrow_blot);
            }
        }
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut game = Game::default();
    let mut cursor_position: (u16, u16) = (0, 0);
    terminal.show_cursor()?;
    loop {
        terminal.draw(|f| ui(f, &game, cursor_position))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char(' ') => {
                    game.drop_selection();
                    log_to_file(&String::from("calling apply_gravity_and_randomize_new_tiles"));
                    game.apply_gravity_and_randomize_new_tiles();
                    log_to_file(&String::from("DONE: calling apply_gravity_and_randomize_new_tiles"));
                }
                KeyCode::Char('x') => {
                    game.select_tile(tile_position_from_cursor_position(terminal.get_cursor()?));
                }
                KeyCode::Char('h') | KeyCode::Left => cursor_position = move_cursor(terminal, CursorMove::Left)?,
                KeyCode::Char('j') | KeyCode::Down => cursor_position = move_cursor(terminal, CursorMove::Down)?,
                KeyCode::Char('k') | KeyCode::Up => cursor_position = move_cursor(terminal, CursorMove::Up)?,
                KeyCode::Char('l') | KeyCode::Right => cursor_position = move_cursor(terminal, CursorMove::Right)?,
                _ => {}
            };
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, game: &Game, cursor_pos: (u16, u16)) {
    let game_widget = GameWidget { game: game };

    f.render_widget(
        game_widget,
        Rect::new(
            CURSOR_MAX_LEFT,
            CURSOR_MAX_UP,
            CURSOR_MAX_RIGHT - CURSOR_MAX_LEFT,
            CURSOR_MAX_DOWN - CURSOR_MAX_UP,
        ),
    );

    f.set_cursor(cursor_pos.0, cursor_pos.1);
}
