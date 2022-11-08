use std::{collections::HashSet, hash::Hash};

pub use winit::event::{MouseButton, VirtualKeyCode};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Key {
    MouseButton(MouseButton),
    KeyboardKey(VirtualKeyCode),
}

pub type Offset = (f32, f32);
pub type Position = (f64, f64);

pub struct InputState {
    pressed_keys: HashSet<Key>,
    held_keys: HashSet<Key>,
    released_keys: HashSet<Key>,
    mouse_offset: Offset,
    touch_start_position: Position,
    touch_move_position: Position,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            held_keys: HashSet::new(),
            released_keys: HashSet::new(),
            mouse_offset: Default::default(),
            touch_start_position: Default::default(),
            touch_move_position: Default::default(),
        }
    }

    pub fn is_pressed(&self, key: Key) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn is_held(&self, key: Key) -> bool {
        self.held_keys.contains(&key)
    }

    pub fn is_released(&self, key: Key) -> bool {
        self.released_keys.contains(&key)
    }

    pub fn mouse_offset(&self) -> Offset {
        self.mouse_offset
    }

    pub fn touch_start_position(&self) -> Position {
        self.touch_start_position
    }

    pub fn touch_position(&self) -> Position {
        self.touch_move_position
    }

    pub fn end_frame(&mut self) {
        self.released_keys.clear();
        self.pressed_keys.clear();
        self.mouse_offset = (0.0, 0.0);
    }

    pub fn set_mouse_offset(&mut self, offset: Offset) {
        self.mouse_offset = offset;
    }

    pub fn set_touch_start_position(&mut self, position: Position) {
        self.touch_start_position = position;
    }

    pub fn set_touch_move_position(&mut self, position: Position) {
        self.touch_move_position = position;
    }

    pub fn set_pressed(&mut self, key: Key) {
        self.pressed_keys.insert(key);
        self.held_keys.insert(key);
    }

    pub fn set_released(&mut self, key: Key) {
        self.released_keys.insert(key);
        self.held_keys.remove(&key);
    }
}
