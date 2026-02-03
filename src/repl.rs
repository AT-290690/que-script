#![allow(dead_code)]
#![allow(warnings)]
#[cfg(feature = "repl")]
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::cell::RefCell;
use std::io::{self, Write};
thread_local! {
    static STD: RefCell<crate::parser::Expression> = RefCell::new(crate::baked::load_ast());
}
fn run(program: String) -> Result<String, String> {
    STD.with(|std| {
        let std_ast = std.borrow();
        if let crate::parser::Expression::Apply(items) = &*std_ast {
            match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    match crate::infer::infer_with_builtins(
                        &wrapped_ast,
                        crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
                    ) {
                        Ok(typ) => match crate::vm::run(&wrapped_ast, crate::vm::VM::new()) {
                            Ok(res) => return Ok(format!(" {}: {:?}", typ, res)),
                            Err(err) => return Err(format!(" {}", err)),
                        },
                        Err(err) => return Err(format!(" {}", err)),
                    }
                }
                Err(err) => return Err(format!(" {}", err)),
            }
        }
        Err("No expressions...".to_string())
    })
}

#[cfg(feature = "repl")]
pub fn repl(initial: String) -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut buffer = initial;
    buffer.push('\n');
    let mut stdout = io::stdout();
    print!("\n");
    writeln!(
        stdout,
        "\rType anything. Press Enter for new line and Esc to exit.\n\r"
    )?;
    stdout.flush()?;

    loop {
        if let Event::Key(key_event) = event::read()? {
            match (key_event.code, key_event.modifiers) {
                (KeyCode::Enter, _) => match run(buffer.clone()) {
                    Ok(res) => {
                        println!("{}", res);
                        buffer.push('\n');
                        print!("\r");
                        stdout.flush()?;
                    }
                    Err(err) => {
                        println!("{}", err);
                        let mut temp: Vec<_> = buffer.lines().into_iter().collect();
                        temp.pop();
                        buffer = temp.join("\n");
                        print!("\r");
                        stdout.flush()?;
                    }
                },
                (KeyCode::Esc, _) => {
                    disable_raw_mode()?;
                    break;
                }
                (KeyCode::Backspace, _) => {
                    if let Some(last) = buffer.chars().last() {
                        if last != '\n' {
                            buffer.pop();
                            print!("\u{8} \u{8}");
                            stdout.flush()?;
                        }
                    }
                }
                (KeyCode::Char(c), _) => {
                    buffer.push(c);
                    print!("{}", c);
                    stdout.flush()?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
