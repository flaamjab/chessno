use std::{collections::HashSet, hash::Hash};

pub use winit::event::{MouseButton, VirtualKeyCode};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Key {
    MouseButton(MouseButton),
    KeyboardKey(VirtualKeyCode),
}

pub struct InputState {
    pressed_keys: HashSet<Key>,
    held_keys: HashSet<Key>,
    released_keys: HashSet<Key>,
    mouse_offset: MouseOffset,
    mouse_position: MousePosition,
}

pub type MouseOffset = (f64, f64);
pub type MousePosition = (f64, f64);

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            held_keys: HashSet::new(),
            released_keys: HashSet::new(),
            mouse_offset: (0.0, 0.0),
            mouse_position: (0.0, 0.0),
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

    pub fn mouse_offset(&self) -> MouseOffset {
        self.mouse_offset
    }

    pub fn mouse_position(&self) -> MousePosition {
        self.mouse_position
    }

    pub fn start_update(&mut self) -> InputStateUpdate {
        InputStateUpdate::new(self)
    }
}

pub struct InputStateUpdate<'a> {
    input_state: &'a mut InputState,
    pressed_keys: HashSet<Key>,
    released_keys: HashSet<Key>,
    mouse_offset: MouseOffset,
    mouse_position: MousePosition,
}

impl<'a> Drop for InputStateUpdate<'a> {
    fn drop(&mut self) {
        self.finish_fn()
    }
}

impl<'a> InputStateUpdate<'a> {
    fn new(input_state: &'a mut InputState) -> Self {
        Self {
            input_state,
            pressed_keys: HashSet::new(),
            released_keys: HashSet::new(),
            mouse_offset: (0.0, 0.0),
            mouse_position: (0.0, 0.0),
        }
    }

    pub fn set_pressed(&mut self, key: Key) {
        self.pressed_keys.insert(key);
    }

    pub fn set_released(&mut self, key: Key) {
        self.released_keys.insert(key);
    }

    pub fn set_mouse_offset(&mut self, mouse_offset: (f64, f64)) {
        self.mouse_offset = mouse_offset;
    }

    pub fn set_mouse_position(&mut self, mouse_position: (f64, f64)) {
        self.mouse_position = mouse_position;
    }

    pub fn finish(self) { /* Indirectly calls drop */
    }

    // The values not set are reset to their defaults
    fn finish_fn(&mut self) {
        let actual_state = &mut self.input_state;

        for k in &self.released_keys {
            actual_state.held_keys.remove(&k);
        }

        // New held keys are the keys that were pressed during previous update
        // but not released in current update.
        actual_state
            .held_keys
            .extend(actual_state.pressed_keys.difference(&self.released_keys));

        actual_state.pressed_keys = std::mem::replace(&mut self.pressed_keys, HashSet::new());
        actual_state.released_keys = std::mem::replace(&mut self.released_keys, HashSet::new());

        actual_state.mouse_offset = self.mouse_offset;
        actual_state.mouse_position = self.mouse_position;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_update_updates() {
        let mut input_state = InputState::new();
        let mut input_state_update = input_state.start_update();

        let a = Key::KeyboardKey(VirtualKeyCode::A);
        let middle_mb = Key::MouseButton(MouseButton::Middle);
        let mouse_offset = (1.0, 1.0);

        input_state_update.set_mouse_offset((1.0, 1.0));
        input_state_update.set_pressed(a);
        input_state_update.set_released(middle_mb);
        input_state_update.finish();

        assert!(input_state.is_pressed(a));
        assert!(input_state.is_released(middle_mb));
        assert!(input_state.held_keys.is_empty());
        assert_eq!(input_state.mouse_offset(), mouse_offset);
    }

    #[test]
    pub fn test_held_keys_updated_correctly() {
        let mut input_state = InputState::new();
        let mut input_state_update = input_state.start_update();

        let a = Key::KeyboardKey(VirtualKeyCode::A);
        let b = Key::KeyboardKey(VirtualKeyCode::B);

        input_state_update.set_pressed(a);
        input_state_update.set_pressed(b);
        input_state_update.finish();

        let mut input_state_update = input_state.start_update();
        input_state_update.set_released(b);
        input_state_update.finish();

        assert!(input_state.is_held(a));
        assert!(!input_state.is_held(b));
        assert!(input_state.is_released(b));
    }
}
