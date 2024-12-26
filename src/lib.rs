use std::error::Error;
use std::ffi::c_int;
use std::fmt::{Display, Formatter};
use std::sync::{Mutex, MutexGuard};

#[cfg(feature = "custom_panic_hook")]
use std::sync::Once;

mod bindings {
    use std::ffi::{c_char, c_int};

    extern "C" {
        pub fn clrscr();

        pub fn initConsole();
        pub fn reset();

        pub fn getConsoleSize(columns_ret: *mut c_int, rows_ret: *mut c_int);

        pub fn hasInput() -> c_int;
        pub fn getKey() -> c_int;

        pub fn getMousePosClicked(column: *mut c_int, row: *mut c_int);

        pub fn drawText(text: *const c_char);

        pub fn setColor(fg: c_int, bg: c_int);
        pub fn resetColor();

        pub fn setUnderline(underline: c_int);

        pub fn setCursorPos(x: c_int, y: c_int);
    }
}

pub struct Console<'a> {
    _lock: MutexGuard<'a, ()>
}

static CONSOLE_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(feature = "custom_panic_hook")]
static CONSOLE_PANIC_HOOK: Once = Once::new();

impl Console<'_> {
    pub fn new() -> Result<Self, Box<ConsoleError>> {
        let lock = match CONSOLE_MUTEX.try_lock() {
            Ok(lock) => lock,
            Err(_) => {
                return Err(Box::new(ConsoleError::new("Only one instance of Console can exist at once!")));
            },
        };

        unsafe { bindings::initConsole() };

        #[cfg(feature = "custom_panic_hook")]
        {
            CONSOLE_PANIC_HOOK.call_once(|| {
                let default_panic_hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |panic_info| {
                    //Reset Console before printing panic message if console was initialized (= CONSOLE_MUTEX is locked)
                    if CONSOLE_MUTEX.try_lock().is_err() {
                        unsafe { bindings::reset() };
                    }

                    default_panic_hook(panic_info);
                }));
            });
        }

        Ok(Self { _lock: lock })
    }

    pub fn repaint(&self) {
        unsafe { bindings::clrscr() }
    }

    /// Returns (x, y)
    pub fn get_console_size(&self) -> (usize, usize) {
        let mut columns_int: c_int = -1;
        let mut rows_int: c_int = -1;

        unsafe { bindings::getConsoleSize(&mut columns_int, &mut rows_int) }

        (columns_int as usize, rows_int as usize)
    }

    pub fn has_input(&self) -> bool {
        unsafe { bindings::hasInput() != 0 }
    }

    pub fn get_key(&self) -> Key {
        let key = unsafe { bindings::getKey() as i32 };

        Key(key)
    }

    /// Returns (x, y)
    pub fn get_mouse_pos_clicked(&self) -> (isize, isize) {
        let mut column_int: c_int = -1;
        let mut row_int: c_int = -1;

        unsafe { bindings::getMousePosClicked(&mut column_int, &mut row_int) }

        (column_int as isize, row_int as isize)
    }

    pub fn draw_text(&self, text: impl Into<String>) {
        let text = std::ffi::CString::new(text.into()).unwrap();

        unsafe { bindings::drawText(text.as_ptr()) }
    }

    pub fn set_color(&self, fg: Color, bg: Color) {
        unsafe { bindings::setColor(fg as c_int, bg as c_int) }
    }

    pub fn set_color_invertible(&self, fg: Color, bg: Color, inverted: bool) {
        if inverted {
            self.set_color(bg, fg);
        }else {
            self.set_color(fg, bg);
        }
    }

    pub fn reset_color(&self) {
        unsafe { bindings::resetColor() }
    }

    pub fn set_underline(&self, underline: bool) {
        unsafe { bindings::setUnderline(underline as c_int) }
    }

    pub fn set_cursor_pos(&self, x: usize, y: usize) {
        unsafe { bindings::setCursorPos(x as c_int, y as c_int) }
    }
}

impl Drop for Console<'_> {
    fn drop(&mut self) {
        #[cfg(feature = "custom_panic_hook")]
        if std::thread::panicking() {
            //Custom panic hook will call "reset()" instead of this Drop implementation
            return;
        }

        unsafe { bindings::reset() };
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Key(i32);

impl Key {
    //Ascii
    pub const SPACE: Key = Key(b' ' as i32);
    pub const EXCLAMATION_MARK: Key = Key(b'!' as i32);
    pub const QUOTATION_MARK: Key = Key(b'"' as i32);
    pub const NUMBER_SIGN: Key = Key(b'#' as i32);
    pub const DOLLAR: Key = Key(b'$' as i32);
    pub const PERCENT_SIGN: Key = Key(b'%' as i32);
    pub const AMPERSAND: Key = Key(b'&' as i32);
    pub const APOSTROPHE: Key = Key(b'\'' as i32);
    pub const LEFT_PARENTHESIS: Key = Key(b'(' as i32);
    pub const RIGHT_PARENTHESIS: Key = Key(b')' as i32);
    pub const ASTERISK: Key = Key(b'*' as i32);
    pub const PLUS: Key = Key(b'+' as i32);
    pub const COMMA: Key = Key(b',' as i32);
    pub const MINUS: Key = Key(b'-' as i32);
    pub const DOT: Key = Key(b'.' as i32);
    pub const SLASH: Key = Key(b'/' as i32);

    pub const COLON: Key = Key(b':' as i32);
    pub const SEMICOLON: Key = Key(b';' as i32);
    pub const LESS_THAN_SIGN: Key = Key(b'<' as i32);
    pub const EQUALS_SIGN: Key = Key(b'=' as i32);
    pub const GREATER_THAN_SIGN: Key = Key(b'>' as i32);
    pub const QUESTION_MARK: Key = Key(b'?' as i32);
    pub const AT_SIGN: Key = Key(b'@' as i32);

    pub const LEFT_BRACKET: Key = Key(b'[' as i32);
    pub const BACKSLASH: Key = Key(b'\\' as i32);
    pub const RIGHT_BRACKET: Key = Key(b']' as i32);
    pub const CARET: Key = Key(b'^' as i32);
    pub const UNDERSCORE: Key = Key(b'_' as i32);
    pub const BACKTICK: Key = Key(b'`' as i32);

    pub const LEFT_CURLY_BRACKET: Key = Key(b'{' as i32);
    pub const VERTICAL_BAR: Key = Key(b'|' as i32);
    pub const RIGHT_CURLY_BRACKET: Key = Key(b'}' as i32);
    pub const TILDE: Key = Key(b'~' as i32);

    pub const DIGIT_0: Key = Key(b'0' as i32);
    pub const DIGIT_1: Key = Key(b'1' as i32);
    pub const DIGIT_2: Key = Key(b'2' as i32);
    pub const DIGIT_3: Key = Key(b'3' as i32);
    pub const DIGIT_4: Key = Key(b'4' as i32);
    pub const DIGIT_5: Key = Key(b'5' as i32);
    pub const DIGIT_6: Key = Key(b'6' as i32);
    pub const DIGIT_7: Key = Key(b'7' as i32);
    pub const DIGIT_8: Key = Key(b'8' as i32);
    pub const DIGIT_9: Key = Key(b'9' as i32);

    pub const A: Key = Key(b'a' as i32);
    pub const B: Key = Key(b'b' as i32);
    pub const C: Key = Key(b'c' as i32);
    pub const D: Key = Key(b'd' as i32);
    pub const E: Key = Key(b'e' as i32);
    pub const F: Key = Key(b'f' as i32);
    pub const G: Key = Key(b'g' as i32);
    pub const H: Key = Key(b'h' as i32);
    pub const I: Key = Key(b'i' as i32);
    pub const J: Key = Key(b'j' as i32);
    pub const K: Key = Key(b'k' as i32);
    pub const L: Key = Key(b'l' as i32);
    pub const M: Key = Key(b'm' as i32);
    pub const N: Key = Key(b'n' as i32);
    pub const O: Key = Key(b'o' as i32);
    pub const P: Key = Key(b'p' as i32);
    pub const Q: Key = Key(b'q' as i32);
    pub const R: Key = Key(b'r' as i32);
    pub const S: Key = Key(b's' as i32);
    pub const T: Key = Key(b't' as i32);
    pub const U: Key = Key(b'u' as i32);
    pub const V: Key = Key(b'v' as i32);
    pub const W: Key = Key(b'w' as i32);
    pub const X: Key = Key(b'x' as i32);
    pub const Y: Key = Key(b'y' as i32);
    pub const Z: Key = Key(b'z' as i32);

    //Arrow keys
    pub const LEFT: Key = Key(5000);
    pub const UP: Key = Key(5001);
    pub const RIGHT: Key = Key(5002);
    pub const DOWN: Key = Key(5003);

    //F keys
    pub const F1: Key = Key(5004);
    pub const F2: Key = Key(5005);
    pub const F3: Key = Key(5006);
    pub const F4: Key = Key(5007);
    pub const F5: Key = Key(5008);
    pub const F6: Key = Key(5009);
    pub const F7: Key = Key(5010);
    pub const F8: Key = Key(5011);
    pub const F9: Key = Key(5012);
    pub const F10: Key = Key(5013);
    pub const F11: Key = Key(5014);
    pub const F12: Key = Key(5015);

    //Other keys
    pub const ESC: Key = Key(5016);
    pub const DELETE: Key = Key(5017);
    pub const ENTER: Key = Key(5018);
    pub const TAB: Key = Key(5019);
}

impl Key {
    pub fn is_arrow_key(&self) -> bool {
        (Key::LEFT..=Key::DOWN).contains(self)
    }
    
    pub fn to_ascii(&self) -> Option<u8> {
        self.is_ascii().then_some(self.0 as u8)
    }
    
    pub fn is_ascii(&self) -> bool {
        (0..=127).contains(&self.0)
    }
    
    pub fn is_numeric(&self) -> bool {
        self.is_ascii() && (self.0 as u8 as char).is_numeric()
    }
    
    pub fn is_alphanumeric(&self) -> bool {
        self.is_ascii() && (self.0 as u8 as char).is_alphanumeric()
    }
}

#[repr(i8)]
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Color {
    Black,
    Blue,
    Green,
    Cyan,
    Red,
    Pink,
    Yellow,
    White,
    LightBlack,
    LightBlue,
    LightGreen,
    LightCyan,
    LightRed,
    LightPink,
    LightYellow,
    LightWhite,
    Default = -1
}

#[derive(Debug)]
pub struct ConsoleError {
    message: String
}

impl ConsoleError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl Display for ConsoleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ConsoleError {}
