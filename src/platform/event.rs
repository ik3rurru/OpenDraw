use super::input::PenSample;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    CloseRequested,
    Resized { width: u32, height: u32 },
    MouseMove { x: i32, y: i32 },
    MouseDown { button: MouseButton },
    MouseUp { button: MouseButton },
    MouseWheel { delta: f32 },
    KeyDown { key: Key },
    KeyUp { key: Key },
    TextInput { character: char },
    // Constructed by the platform tablet backends from PEN-002 onward.
    #[allow(dead_code)]
    PenProximityIn(PenSample),
    #[allow(dead_code)]
    PenDown(PenSample),
    #[allow(dead_code)]
    PenMove(PenSample),
    #[allow(dead_code)]
    PenUp(PenSample),
    #[allow(dead_code)]
    PenProximityOut { pointer_id: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Backspace,
    Tab,
    Enter,
    Shift,
    Control,
    Alt,
    Escape,
    Space,
    PageUp,
    PageDown,
    End,
    Home,
    Left,
    Up,
    Right,
    Down,
    Delete,
    Letter(char),
    Digit(u8),
    Function(u8),
    Unknown(u32),
}
