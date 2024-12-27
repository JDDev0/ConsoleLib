use std::thread::sleep;
use std::time::Duration;
use console_lib::{Color, Console, Key};

fn main() {
    //Init console
    let console = Console::new().unwrap();

    let colors = [
        Color::Blue,
        Color::Green,
        Color::Cyan,
        Color::Red,
        Color::Pink,
        Color::Yellow,
        Color::White,
    ];
    let mut color_index = 0;
    let mut ticks = 0_usize;

    let (width, height) = console.get_console_size();

    loop {
        // Exit if the enter key was pressed
        if console.has_input() {
            if let Some(input) = console.get_key() {
                if input == Key::ENTER {
                    return;
                }
            }
        }

        // Mouse must be read even if it is not used.
        // On unix terminals the mouse pos prevents Console::get_key() from returning new key inputs.
        let _ = console.get_mouse_pos_clicked();

        if ticks % 10 == 0 {
            //Repaint
            console.repaint();

            //Set foreground color
            console.set_color(colors[color_index], Color::Default);

            if color_index < colors.len() - 1 {
                color_index += 1;
            }else {
                color_index = 0;
            }

            let text = "An example text";

            let x = (width.saturating_sub(text.len())) >> 1;
            let y = height >> 1;

            //Set cursor pos
            console.set_cursor_pos(x, y);

            //Draw text at cursor pos with color
            console.draw_text(text);
        }

        sleep(Duration::from_millis(50));
        ticks += 1;
    }
}
