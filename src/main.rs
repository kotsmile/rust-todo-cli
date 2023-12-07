use std::cmp::Ordering;
use std::env;
use std::fs;
use std::io::{self, Stdout, Write};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize, Debug)]
struct Todo {
    text: String,
    complete: bool,
}

enum Mode {
    View,
    New,
    Edit(usize),
}

fn display(
    stdout: &mut RawTerminal<Stdout>,
    todos: &Vec<Todo>,
    new_text: &String,
) -> Result<(), String> {
    move_cursor(stdout, 1, 1)?;

    write!(stdout, "{}", termion::clear::All)
        .map_err(|err| format!("Cant write to stdout: {err}"))?;

    for (i, todo) in todos.iter().enumerate() {
        write!(stdout, "{}", termion::cursor::Goto(1, (i + 1) as u16))
            .map_err(|err| format!("Cant write to stdout: {err}"))?;
        if todo.complete {
            write!(stdout, "-✅").map_err(|err| format!("Cant write to stdout: {err}"))?;
        } else {
            write!(stdout, "-❌").map_err(|err| format!("Cant write to stdout: {err}"))?;
        }
        write!(stdout, " {}", todo.text).map_err(|err| format!("Cant write to stdout: {err}"))?;
    }

    if new_text.len() != 0 {
        move_cursor(stdout, 1, todos.len() + 1)?;
        write!(stdout, "{new_text}").map_err(|err| format!("Cant write to stdout: {err}"))?;
    }
    stdout.flush().unwrap();

    Ok(())
}

fn move_cursor(stdout: &mut RawTerminal<Stdout>, x: usize, y: usize) -> Result<(), String> {
    write!(stdout, "{}", termion::cursor::Goto(x as u16, y as u16))
        .map_err(|err| format!("Cant write to stdout: {err}"))?;
    stdout.flush().unwrap();
    Ok(())
}

fn parse_file(filepath: &str) -> Vec<Todo> {
    let mut todos = Vec::<Todo>::new();

    match fs::read_to_string(filepath) {
        Ok(content) => {
            for line in content.split('\n') {
                if line.starts_with("- [") {
                    let complete = line.chars().nth(3).unwrap_or(' ') == 'x';
                    let text = line[6..].to_string();
                    todos.push(Todo { text, complete })
                } else {
                    continue;
                }
            }
        }
        Err(err) => eprintln!("{err}"),
    };

    todos
}

fn save_file(todos: &Vec<Todo>, filepath: &str) {
    let mut file = fs::File::create(filepath).unwrap();

    write!(&mut file, "# TODO\n").unwrap();

    for todo in todos {
        let status = if todo.complete { 'x' } else { ' ' };
        write!(&mut file, "\n- [{status}] {text}", text = todo.text).unwrap();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let todo_path = args.get(1).unwrap();

    println!("Reading todos: {todo_path}");

    let mut todos = parse_file(todo_path);

    let stdin = io::stdin();
    let mut stdout = io::stdout()
        .into_raw_mode()
        .map_err(|err| eprintln!("Error: Cant raw mode: {err}"))
        .unwrap();

    let mut mode: Mode = Mode::View;

    let mut new_text: String = "".to_string();

    let mut cursor_y: usize = 1;
    let mut cursor_x: usize = 1;

    let text_start = 4;
    display(&mut stdout, &todos, &new_text)
        .map_err(|err| eprintln!("Error: cant display: {err}"))
        .unwrap();
    move_cursor(&mut stdout, cursor_x, cursor_y)
        .map_err(|err| eprintln!("Error: cant move the cursor: {err}"))
        .unwrap();

    for key in stdin.keys() {
        match mode {
            Mode::View => {
                cursor_x = 1;
                match key.unwrap() {
                    // functional
                    Key::Char('q') => {
                        write!(stdout, "{}", termion::clear::All)
                            .map_err(|err| format!("Cant write to stdout: {err}"))
                            .unwrap();
                        stdout.flush().unwrap();
                        move_cursor(&mut stdout, 1, 1)
                            .map_err(|err| eprintln!("Error: cant move the cursor: {err}"))
                            .unwrap();
                        break;
                    }
                    Key::Char('d') => {
                        if todos.len() > 0 {
                            if cursor_y <= todos.len() {
                                todos.remove(cursor_y - 1);
                            }
                            if cursor_y > todos.len() {
                                cursor_y = todos.len();
                            }
                        }
                    }
                    Key::Char(' ') => {
                        todos[cursor_y - 1].complete = !todos[cursor_y - 1].complete;
                    }
                    // modes
                    Key::Char('n') => {
                        mode = Mode::New;
                        cursor_x = 1;
                        cursor_y = todos.len() + 1;
                    }
                    Key::Char('\n') => {
                        mode = Mode::Edit(cursor_y);
                        cursor_x = text_start + 1 + todos[cursor_y - 1].text.len();
                        // cursor_x
                    }
                    // movement
                    Key::Char('j') => {
                        if cursor_y < todos.len() {
                            cursor_y += 1;
                        }
                    }
                    Key::Char('k') => {
                        if cursor_y > 0 {
                            cursor_y -= 1;
                        }
                    }
                    Key::Char('o') => {
                        cursor_x = 1;
                        cursor_y = 1;
                        todos.sort_by(|x, _| {
                            if x.complete {
                                Ordering::Less
                            } else {
                                Ordering::Greater
                            }
                        });
                    }
                    _ => {}
                }
            }
            Mode::New => match key.unwrap() {
                Key::Ctrl('w') => loop {
                    if new_text.len() == 0 {
                        break;
                    }

                    cursor_y = todos.len() + 1;
                    if new_text.chars().nth(new_text.len() - 1).unwrap() == ' ' {
                        if cursor_x > 1 {
                            cursor_x = cursor_x - 1;
                            new_text.pop();
                        } else {
                            break;
                        }
                        break;
                    } else {
                        if cursor_x > 1 {
                            cursor_x = cursor_x - 1;
                            new_text.pop();
                        } else {
                            break;
                        }
                    }
                },
                Key::Char('\n') => {
                    if new_text.len() != 0 {
                        todos.push(Todo {
                            text: new_text.clone(),
                            complete: false,
                        });
                    }

                    mode = Mode::View;
                    new_text = "".to_string();
                    cursor_x = 1;
                    cursor_y = todos.len();
                }
                Key::Backspace => {
                    if cursor_x > 1 {
                        cursor_x = cursor_x - 1;
                        new_text.pop();
                    }
                    cursor_y = todos.len() + 1;
                }
                Key::Char(ch) => {
                    new_text.push(ch);

                    cursor_x = cursor_x + 1;
                    cursor_y = todos.len() + 1;
                }
                _ => {}
            },
            Mode::Edit(n) => match key.unwrap() {
                Key::Ctrl('w') => loop {
                    if todos[n - 1].text.len() == 0 {
                        break;
                    }

                    cursor_y = n;
                    if todos[n - 1]
                        .text
                        .chars()
                        .nth(todos[n - 1].text.len() - 1)
                        .unwrap()
                        == ' '
                    {
                        if cursor_x > 1 {
                            cursor_x = cursor_x - 1;
                            todos[n - 1].text.pop();
                        } else {
                            break;
                        }
                        break;
                    } else {
                        if cursor_x > 1 {
                            cursor_x = cursor_x - 1;
                            todos[n - 1].text.pop();
                        } else {
                            break;
                        }
                    }
                },
                Key::Char('\n') => {
                    mode = Mode::View;
                    cursor_x = 1;
                    cursor_y = n;
                }
                Key::Backspace => {
                    if cursor_x > text_start + 1 {
                        cursor_x = cursor_x - 1;
                        todos[n - 1].text.pop();
                    }
                    cursor_y = n;
                }
                Key::Char(ch) => {
                    todos[n - 1].text.push(ch);

                    cursor_x = cursor_x + 1;
                    cursor_y = n;
                }
                _ => {}
            },
        }

        display(&mut stdout, &todos, &new_text)
            .map_err(|err| eprintln!("Error: cant display: {err}"))
            .unwrap();
        move_cursor(&mut stdout, cursor_x, cursor_y)
            .map_err(|err| eprintln!("Error: cant move the cursor: {err}"))
            .unwrap();

        save_file(&todos, todo_path);
    }
}
