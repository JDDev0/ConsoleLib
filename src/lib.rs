use std::error::Error;
use std::ffi::c_int;
use std::fmt::{Display, Formatter};
use std::sync::{Mutex, MutexGuard};

#[cfg(feature = "custom_panic_hook")]
use std::sync::Once;

mod bindings {
    use std::ffi::{c_char, c_int};

    unsafe extern "C" {
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

/// An abstraction for the console lib which automatically changes the console / terminal mode
/// in [Console::new] and in the [Drop] implementation of [Console].
pub struct Console<'a> {
    _lock: MutexGuard<'a, ()>
}

static CONSOLE_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(feature = "custom_panic_hook")]
static CONSOLE_PANIC_HOOK: Once = Once::new();

impl Console<'_> {
    /// Creates a new console lib abstraction.
    ///
    /// The [Console::new] method changes the console / terminal mode (Like disabling text echo).
    ///
    /// The [Drop] implementation of [Console] will reset the console / terminal to the original
    /// state it was in prior to the creation of [Console].
    ///
    /// If a panic occurred if the console / terminal mode was already changed, the panic error message would be lost,
    /// because the [Drop] implementation of [Console] would be called after the panic message was printed.
    ///
    /// Because of the mode changes there can only be one instance of a Console struct at once,
    /// the [Err] variant is returned if there exists another instance of Console.
    ///
    /// # Custom Panic Hook
    ///
    /// With the `custom_panic_hook` feature the lost panic error message problem can be prevented.
    /// If the `custom_panic_hook` feature is enabled a panic hook will be created after the first
    /// time the console / terminal mode is changed inside the [Console::new] method -
    /// the newly set panic hook persists until the whole program is terminated.
    /// When a panic occurs, the panic hook checks if a Console instance is still present.
    /// If this is the case, the console / terminal mode will be reset to the original state.
    /// Afterward the standard panic hook will be run which will print the panic error message.
    ///
    /// If the `custom_panic_hook` feature is enabled and the program is panicking,
    /// the [Drop] implementation of [Console] would not reset the console / terminal mode again.
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

    /// Repaints the screen
    pub fn repaint(&self) {
        unsafe { bindings::clrscr() }
    }

    /// Returns the size of the console in characters as (width, rows).
    ///
    /// At the moment Console / Terminal resizing is currently not supported.
    /// The size is read once after the console is initialized and internal
    /// buffers are allocated for that size.
    pub fn get_console_size(&self) -> (usize, usize) {
        let mut columns_int: c_int = -1;
        let mut rows_int: c_int = -1;

        unsafe { bindings::getConsoleSize(&mut columns_int, &mut rows_int) }

        (columns_int as usize, rows_int as usize)
    }

    /// Checks if key input is available
    pub fn has_input(&self) -> bool {
        unsafe { bindings::hasInput() != 0 }
    }

    /// Returns the key which was pressed or None
    pub fn get_key(&self) -> Option<Key> {
        let key = unsafe { bindings::getKey() as i32 };

        if key < 0 {
            None
        }else {
            Some(Key(key as u16))
        }
    }

    /// Returns the coordinates of the pos where a left click occurred as (x, y).
    ///
    /// x and y represent character positions.
    ///
    /// If None, no left click occurred.
    pub fn get_mouse_pos_clicked(&self) -> Option<(usize, usize)> {
        let mut column_int: c_int = -1;
        let mut row_int: c_int = -1;

        unsafe { bindings::getMousePosClicked(&mut column_int, &mut row_int) }

        if column_int < 0 || row_int < 0 {
            None
        }else {
            Some((column_int as usize, row_int as usize))
        }
    }

    /// Draws text at the current cursor position.
    ///
    /// Behavior for Non-ASCII strings is terminal dependent.
    ///
    /// Characters which are out of bounds will be ignored and not drawn.
    pub fn draw_text(&self, text: impl Into<String>) {
        let text = std::ffi::CString::new(text.into()).unwrap();

        unsafe { bindings::drawText(text.as_ptr()) }
    }

    /// Sets the color for foreground and background
    pub fn set_color(&self, fg: Color, bg: Color) {
        unsafe { bindings::setColor(fg as c_int, bg as c_int) }
    }

    /// Sets the color for foreground and background
    ///
    /// Foreground and background colors are swapped if inverted is true
    pub fn set_color_invertible(&self, fg: Color, bg: Color, inverted: bool) {
        if inverted {
            self.set_color(bg, fg);
        }else {
            self.set_color(fg, bg);
        }
    }

    /// Resets the color for foreground and background to [Color::Default]
    pub fn reset_color(&self) {
        unsafe { bindings::resetColor() }
    }

    pub fn set_underline(&self, underline: bool) {
        unsafe { bindings::setUnderline(underline as c_int) }
    }

    /// Sets the cursor pos to x and y
    pub fn set_cursor_pos(&self, x: usize, y: usize) {
        let x = x as c_int;
        let y = y as c_int;

        if x < 0 || y < 0 {
            return;
        }

        unsafe { bindings::setCursorPos(x, y) }
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

/// A representation of a key code from the console lib binding.
///
/// The key should be checked with the constants provided in the [Key] implementation (Like [Key::SPACE]).
///
/// Unknown keys map to undefined values.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Key(u16);

impl Key {
    //Ascii
    pub const SPACE: Key = Key(b' ' as u16);
    pub const EXCLAMATION_MARK: Key = Key(b'!' as u16);
    pub const QUOTATION_MARK: Key = Key(b'"' as u16);
    pub const NUMBER_SIGN: Key = Key(b'#' as u16);
    pub const DOLLAR: Key = Key(b'$' as u16);
    pub const PERCENT_SIGN: Key = Key(b'%' as u16);
    pub const AMPERSAND: Key = Key(b'&' as u16);
    pub const APOSTROPHE: Key = Key(b'\'' as u16);
    pub const LEFT_PARENTHESIS: Key = Key(b'(' as u16);
    pub const RIGHT_PARENTHESIS: Key = Key(b')' as u16);
    pub const ASTERISK: Key = Key(b'*' as u16);
    pub const PLUS: Key = Key(b'+' as u16);
    pub const COMMA: Key = Key(b',' as u16);
    pub const MINUS: Key = Key(b'-' as u16);
    pub const DOT: Key = Key(b'.' as u16);
    pub const SLASH: Key = Key(b'/' as u16);

    pub const COLON: Key = Key(b':' as u16);
    pub const SEMICOLON: Key = Key(b';' as u16);
    pub const LESS_THAN_SIGN: Key = Key(b'<' as u16);
    pub const EQUALS_SIGN: Key = Key(b'=' as u16);
    pub const GREATER_THAN_SIGN: Key = Key(b'>' as u16);
    pub const QUESTION_MARK: Key = Key(b'?' as u16);
    pub const AT_SIGN: Key = Key(b'@' as u16);

    pub const LEFT_BRACKET: Key = Key(b'[' as u16);
    pub const BACKSLASH: Key = Key(b'\\' as u16);
    pub const RIGHT_BRACKET: Key = Key(b']' as u16);
    pub const CARET: Key = Key(b'^' as u16);
    pub const UNDERSCORE: Key = Key(b'_' as u16);
    pub const BACKTICK: Key = Key(b'`' as u16);

    pub const LEFT_CURLY_BRACKET: Key = Key(b'{' as u16);
    pub const VERTICAL_BAR: Key = Key(b'|' as u16);
    pub const RIGHT_CURLY_BRACKET: Key = Key(b'}' as u16);
    pub const TILDE: Key = Key(b'~' as u16);

    pub const DIGIT_0: Key = Key(b'0' as u16);
    pub const DIGIT_1: Key = Key(b'1' as u16);
    pub const DIGIT_2: Key = Key(b'2' as u16);
    pub const DIGIT_3: Key = Key(b'3' as u16);
    pub const DIGIT_4: Key = Key(b'4' as u16);
    pub const DIGIT_5: Key = Key(b'5' as u16);
    pub const DIGIT_6: Key = Key(b'6' as u16);
    pub const DIGIT_7: Key = Key(b'7' as u16);
    pub const DIGIT_8: Key = Key(b'8' as u16);
    pub const DIGIT_9: Key = Key(b'9' as u16);

    pub const A: Key = Key(b'a' as u16);
    pub const B: Key = Key(b'b' as u16);
    pub const C: Key = Key(b'c' as u16);
    pub const D: Key = Key(b'd' as u16);
    pub const E: Key = Key(b'e' as u16);
    pub const F: Key = Key(b'f' as u16);
    pub const G: Key = Key(b'g' as u16);
    pub const H: Key = Key(b'h' as u16);
    pub const I: Key = Key(b'i' as u16);
    pub const J: Key = Key(b'j' as u16);
    pub const K: Key = Key(b'k' as u16);
    pub const L: Key = Key(b'l' as u16);
    pub const M: Key = Key(b'm' as u16);
    pub const N: Key = Key(b'n' as u16);
    pub const O: Key = Key(b'o' as u16);
    pub const P: Key = Key(b'p' as u16);
    pub const Q: Key = Key(b'q' as u16);
    pub const R: Key = Key(b'r' as u16);
    pub const S: Key = Key(b's' as u16);
    pub const T: Key = Key(b't' as u16);
    pub const U: Key = Key(b'u' as u16);
    pub const V: Key = Key(b'v' as u16);
    pub const W: Key = Key(b'w' as u16);
    pub const X: Key = Key(b'x' as u16);
    pub const Y: Key = Key(b'y' as u16);
    pub const Z: Key = Key(b'z' as u16);

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

    /// Converts the keycode to an ASCII character if the key represents an ASCII character.
    pub fn to_ascii(&self) -> Option<u8> {
        self.is_ascii().then_some(self.0 as u8)
    }
    
    pub fn is_ascii(&self) -> bool {
        (0..=127).contains(&self.0)
    }

    /// Checks if a keycode is ASCII and numeric.
    pub fn is_numeric(&self) -> bool {
        self.is_ascii() && (self.0 as u8 as char).is_numeric()
    }

    /// Checks if a keycode is ASCII and alphanumeric.
    pub fn is_alphanumeric(&self) -> bool {
        self.is_ascii() && (self.0 as u8 as char).is_alphanumeric()
    }
}

/// 4-bit ANSI Color definitions for the native console lib bindings.
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

    /// Default color is [Color::Black] on unix and default color attributes on Windows.
    Default = -1
}

/// An error that occurred during creation of the [Console] struct.
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
