use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dungeon_raid_core::game::{
    improvement_choices::ImprovementInfo,
    tile::{Tile, TileInfo, TilePosition, TileType, Wind8},
    Game, DEFAULT_BOARD_HEIGHT, DEFAULT_BOARD_WIDTH,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
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
    let mut file = std::fs::File::options()
        .append(true)
        .create(true)
        .open(LOG_FILE)
        .expect("failed to create file");
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
enum GameState {
    Playing,
    ChoosingImprovement(usize), //num_choices
}

#[derive(Copy, Clone)]
enum CursorMove {
    Up,
    Right,
    Down,
    Left,
}

const PLAYING_CURSOR_MOVE: u16 = 2;
const PLAYING_CURSOR_MAX_UP: u16 = 0;
const PLAYING_CURSOR_MAX_RIGHT: u16 = PLAYING_CURSOR_MAX_LEFT + DEFAULT_BOARD_WIDTH as u16 * 2 - 1;
const PLAYING_CURSOR_MAX_DOWN: u16 = PLAYING_CURSOR_MAX_UP + DEFAULT_BOARD_HEIGHT as u16 * 2 - 1;
const PLAYING_CURSOR_MAX_LEFT: u16 = 0;
const CHOOSING_IMPROVEMENT_CURSOR_MOVE: u16 = 1;
const CHOOSING_IMPROVEMENT_CURSOR_MAX_UP: u16 = 1;

fn move_cursor<B: Backend>(
    terminal: &mut Terminal<B>,
    m: CursorMove,
    gs: GameState,
) -> io::Result<(u16, u16)> {
    let mut cursor_pos = terminal.get_cursor()?;
    log_to_file(&format!(
        "move_cursor called with terminal.get_cursor() giving (x {}, y {})",
        cursor_pos.0, cursor_pos.1
    ));
    match gs {
        GameState::Playing => {
            match m {
                CursorMove::Up => {
                    if cursor_pos.1 >= PLAYING_CURSOR_MAX_UP + PLAYING_CURSOR_MOVE {
                        cursor_pos.1 -= PLAYING_CURSOR_MOVE;
                    }
                }
                CursorMove::Right => {
                    if cursor_pos.0 <= PLAYING_CURSOR_MAX_RIGHT - PLAYING_CURSOR_MOVE {
                        cursor_pos.0 += PLAYING_CURSOR_MOVE;
                    }
                }
                CursorMove::Down => {
                    if cursor_pos.1 <= PLAYING_CURSOR_MAX_DOWN - PLAYING_CURSOR_MOVE {
                        cursor_pos.1 += PLAYING_CURSOR_MOVE;
                    }
                }
                CursorMove::Left => {
                    if cursor_pos.0 >= PLAYING_CURSOR_MAX_LEFT + PLAYING_CURSOR_MOVE {
                        cursor_pos.0 -= PLAYING_CURSOR_MOVE;
                    }
                }
            };
        }
        GameState::ChoosingImprovement(num_choices) => match m {
            CursorMove::Up => {
                if cursor_pos.1
                    >= CHOOSING_IMPROVEMENT_CURSOR_MAX_UP + CHOOSING_IMPROVEMENT_CURSOR_MOVE
                {
                    cursor_pos.1 -= CHOOSING_IMPROVEMENT_CURSOR_MOVE;
                }
            }
            CursorMove::Down => {
                if cursor_pos.1
                    <= CHOOSING_IMPROVEMENT_CURSOR_MAX_UP + num_choices as u16
                        - 1
                        - CHOOSING_IMPROVEMENT_CURSOR_MOVE
                {
                    cursor_pos.1 += CHOOSING_IMPROVEMENT_CURSOR_MOVE;
                }
            }
            _ => unreachable!(""),
        },
    };
    Ok(cursor_pos)
}

fn tile_position_from_cursor_position(cursor_position: (u16, u16)) -> TilePosition {
    let (x, y) = cursor_position;
    TilePosition::new(
        ((y - PLAYING_CURSOR_MAX_UP) / 2) as isize,
        ((x - PLAYING_CURSOR_MAX_LEFT) / 2) as isize,
    )
}

fn improvement_choice_index_from_cursor_position(cursor_position: (u16, u16)) -> usize {
    let (_x, y) = cursor_position;
    (y - PLAYING_CURSOR_MAX_UP - 1) as usize
}

fn blot_char_from_tile_type(tile_type: TileType) -> char {
    match tile_type {
        TileType::Potion => 'p',
        TileType::Shield => 's',
        TileType::Coin => 'c',
        TileType::Sword => 'S',
        TileType::Enemy => 'E',
        TileType::Special => 'B',
        _ => '!',
    }
}

fn bg_fg_color_from_tile_type(tile_type: TileType) -> (Color, Color) {
    match tile_type {
        TileType::Potion => (Color::LightMagenta, Color::Black),
        TileType::Shield => (Color::Blue, Color::Black),
        TileType::Coin => (Color::Yellow, Color::Black),
        TileType::Sword => (Color::Green, Color::Black),
        TileType::Enemy => (Color::Red, Color::Black),
        TileType::Special => (Color::White, Color::Black),
        _ => (Color::Black, Color::White),
    }
}

struct GameWidget<'a> {
    pub game: &'a Game,
    pub cursor_pos: (u16, u16),
    pub improvement_choice_selection_positions: &'a Vec<(u16, u16)>,
}
impl<'a> Widget for GameWidget<'a> {
    fn render(self, _area: Rect, buf: &mut Buffer) {
        // selection positions
        for pos in self.improvement_choice_selection_positions.iter() {
            buf.get_mut(pos.0, pos.1).set_bg(Color::White);
        }

        // below text

        let mut text_y = PLAYING_CURSOR_MAX_DOWN + 1;

        // incoming damage
        let incoming_damage_display = format!("incoming damage: {}", self.game.incoming_damage());
        buf.set_string(0, text_y, incoming_damage_display, Style::default());
        text_y += 1;
        // player stats and whatnot
        let hit_points_display = format!(
            "hit points: {}/{}",
            self.game.player().being.hit_points,
            self.game.player().being.max_hit_points
        );
        buf.set_string(0, text_y, hit_points_display, Style::default());
        text_y += 1;
        let shields_display = format!(
            "shields: {}/{}",
            self.game.player().being.shields,
            self.game.player().being.max_shields
        );
        buf.set_string(0, text_y, shields_display, Style::default());
        text_y += 1;
        let coins_display = format!(
            "coins: {}/{}",
            self.game.player().coin_cents,
            self.game.player().coin_cents_per_purchase
        );
        buf.set_string(0, text_y, coins_display, Style::default());
        text_y += 1;
        let up_display = format!(
            "UP: {}/{}",
            self.game.player().excess_shield_cents,
            self.game.player().excess_shield_cents_per_upgrade
        );
        buf.set_string(0, text_y, up_display, Style::default());
        text_y += 1;
        let xp_display = format!(
            "XP: {}/{}",
            self.game.player().experience_point_cents,
            self.game.player().experience_point_cents_per_level_up
        );
        buf.set_string(0, text_y, xp_display, Style::default());
        text_y += 2;
        // player abilities
        for (idx, ability_opt) in self.game.player().abilities.iter().enumerate() {
            let mut ability_string = format!("{} - ", idx + 1);
            match ability_opt {
                Some(a) => {
                    let (name, _) = a.ability_type.name_description();
                    ability_string += name;
                    buf.set_string(0, text_y, ability_string, Style::default());
                    text_y += 1;
                    if a.running_cooldown > 0 {
                        buf.set_string(
                            4,
                            text_y,
                            format!("COOLDOWN: {}", a.running_cooldown),
                            Style::default(),
                        );
                        text_y += 1;
                    }
                }
                None => {
                    ability_string += "[empty]";
                    buf.set_string(0, text_y, ability_string, Style::default());
                    text_y += 1;
                }
            };
        }
        text_y += 1;
        // current special
        let specials_vec = self.game.specials();
        for (_tp, t, _sid) in specials_vec {
            if let TileInfo::Special(special) = t.tile_info {
                let (name, desc) = special.special_type.name_description();
                let special_display = format!("Special Monster: {} - {}", name, desc);
                buf.set_string(0, text_y, special_display, Style::default());
                text_y += 1;
            } else {
                unreachable!(
                    "Game::specials() gave a tile with tile.tile_info NOT TileInfo::Special(_)"
                );
            }
        }

        // improvement choice or board

        match self.game.improvement_choice_set() {
            Some(set) => {
                // improvement choice
                let mut choice_text_y = 0;
                buf.set_string(0, choice_text_y, String::from(set.header), Style::default());
                choice_text_y += 1;
                for display in set.displays.iter() {
                    buf.set_string(
                        1,
                        choice_text_y,
                        display.description.as_str(),
                        Style::default(),
                    );
                    choice_text_y += 1;
                }
            }
            None => {
                // board
                {
                    let hover_tile = self
                        .game
                        .get_tile(&tile_position_from_cursor_position(self.cursor_pos))
                        .expect("");
                    let mut hover_string = String::from("Hovered Tile: ");
                    hover_string += match hover_tile.tile_type {
                        TileType::Potion => "Potion",
                        TileType::Shield => "Shield",
                        TileType::Coin => "Coin",
                        TileType::Sword => "Sword",
                        TileType::Enemy => "Enemy",
                        TileType::Special => "Special",
                        _ => unreachable!(""),
                    };
                    let info_string;
                    match hover_tile.tile_info {
                        TileInfo::Enemy(b) => {
                            info_string = format!(
                                " {{ hp: {}, sh: {}, dmg: {} }}",
                                b.hit_points, b.shields, b.base_output_damage
                            )
                        }
                        TileInfo::Special(s) => {
                            info_string = format!(
                                " {{ type: {}, hp: {}, sh: {}, dmg: {} }}",
                                s.special_type.name_description().0,
                                s.being.hit_points,
                                s.being.shields,
                                s.being.base_output_damage
                            )
                        }
                        TileInfo::None => info_string = String::from(""),
                    };
                    hover_string += info_string.as_str();
                    buf.set_string(0, text_y, hover_string, Style::default());
                }
                for x in 0..(DEFAULT_BOARD_WIDTH as u16) {
                    let blot_x = x * 2;
                    for y in 0..(DEFAULT_BOARD_HEIGHT as u16) {
                        let blot_y = y * 2;
                        let t: Tile = self
                            .game
                            .get_tile(&TilePosition::new(y as isize, x as isize))
                            .expect("plz");
                        let blot = blot_char_from_tile_type(t.tile_type);
                        let (bg_color, fg_color) = bg_fg_color_from_tile_type(t.tile_type);
                        let mut style = Style::default().bg(bg_color).fg(fg_color);
                        match self.game.get_selection_start() {
                            Some(pos) => {
                                if pos == TilePosition::new(y as isize, x as isize) {
                                    style = style.add_modifier(Modifier::RAPID_BLINK);
                                }
                            }
                            None => {}
                        };
                        buf.get_mut(blot_x, blot_y).set_style(style).set_char(blot);
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
                            _ => {}
                        }
                        buf.get_mut(arrow_blot_x, arrow_blot_y).set_char(arrow_blot);
                    }
                }
            }
        }
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut game = Game::default();
    let mut playing_cursor_position: (u16, u16) = (0, 0);
    let mut choosing_improvement_cursor_position: (u16, u16) = (0, 1);
    let mut improvement_choice_indeces: Vec<usize> = vec![];
    let mut improvement_choice_selection_positions: Vec<(u16, u16)> = vec![];
    let mut game_state: GameState;
    terminal.show_cursor()?;
    loop {
        game_state = match game.improvement_choice_set() {
            Some(set) => {
                let num_choices = match set.info {
                    ImprovementInfo::ShieldUpgradeInfo(ref vec) => vec.len(),
                    ImprovementInfo::CoinPurchaseInfo(ref vec) => vec.len(),
                    ImprovementInfo::ExperiencePointLevelUpInfo(ref vec) => vec.len(),
                };
                GameState::ChoosingImprovement(num_choices)
            }
            None => GameState::Playing,
        };
        let cursor_position = match game_state {
            GameState::Playing => playing_cursor_position,
            GameState::ChoosingImprovement(_) => choosing_improvement_cursor_position,
        };

        terminal.draw(|f| {
            ui(
                f,
                &game,
                cursor_position,
                &improvement_choice_selection_positions,
            )
        })?;

        if let Event::Key(key) = event::read()? {
            match game.improvement_choice_set() {
                Some(set) => {
                    // choosing improvement
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char(' ') => {
                            let cursor_pos = terminal.get_cursor()?;
                            let index_pressed =
                                improvement_choice_index_from_cursor_position(cursor_pos);
                            let mut removed = false;
                            for (vec_idx, pressed_idx) in
                                improvement_choice_indeces.iter().enumerate()
                            {
                                if *pressed_idx == index_pressed {
                                    improvement_choice_indeces.remove(vec_idx);
                                    improvement_choice_selection_positions.remove(vec_idx);
                                    removed = true;
                                    break;
                                }
                            }
                            if !removed {
                                improvement_choice_indeces.push(index_pressed);
                                improvement_choice_selection_positions.push(cursor_pos);
                            }
                            if improvement_choice_indeces.len() == set.num_to_choose {
                                game.choose_improvements(&improvement_choice_indeces);
                                improvement_choice_indeces.clear();
                                improvement_choice_selection_positions.clear();
                                choosing_improvement_cursor_position = (0, 1);
                            }
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            choosing_improvement_cursor_position =
                                move_cursor(terminal, CursorMove::Down, game_state)?
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            choosing_improvement_cursor_position =
                                move_cursor(terminal, CursorMove::Up, game_state)?
                        }
                        _ => {}
                    }
                }
                None => {
                    // playing on board
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char(' ') => {
                            if game.drop_selection() {
                                // slashed tiles; have enemies attack and then pull down tiles,
                                // randomizing the new ones
                                game.apply_incoming_damage();
                                game.apply_gravity_and_randomize_new_tiles();
                                game.run_end_of_turn_on_specials();
                            }
                        }
                        KeyCode::Char('x') => {
                            game.select_tile(&tile_position_from_cursor_position(
                                terminal.get_cursor()?,
                            ));
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            playing_cursor_position =
                                move_cursor(terminal, CursorMove::Left, game_state)?
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            playing_cursor_position =
                                move_cursor(terminal, CursorMove::Down, game_state)?
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            playing_cursor_position =
                                move_cursor(terminal, CursorMove::Up, game_state)?
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            playing_cursor_position =
                                move_cursor(terminal, CursorMove::Right, game_state)?
                        }
                        KeyCode::Char('1') => {
                            game.cast_ability(0);
                        }
                        KeyCode::Char('2') => {
                            game.cast_ability(1);
                        }
                        KeyCode::Char('3') => {
                            game.cast_ability(2);
                        }
                        KeyCode::Char('4') => {
                            game.cast_ability(3);
                        }
                        _ => {}
                    };
                }
            }
        }
    }
}

fn ui<B: Backend>(
    f: &mut Frame<B>,
    game: &Game,
    cursor_pos: (u16, u16),
    improvement_choice_selection_positions: &Vec<(u16, u16)>,
) {
    let game_widget = GameWidget {
        game: game,
        cursor_pos,
        improvement_choice_selection_positions,
    };

    f.render_widget(
        game_widget,
        Rect::new(
            PLAYING_CURSOR_MAX_LEFT,
            PLAYING_CURSOR_MAX_UP,
            PLAYING_CURSOR_MAX_RIGHT - PLAYING_CURSOR_MAX_LEFT,
            PLAYING_CURSOR_MAX_DOWN - PLAYING_CURSOR_MAX_UP,
        ),
    );

    f.set_cursor(cursor_pos.0, cursor_pos.1);
}
