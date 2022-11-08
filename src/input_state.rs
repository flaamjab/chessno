use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::Hash,
};

use smallvec::SmallVec;
pub use winit::event::{MouseButton, VirtualKeyCode};

use crate::{logging::warn, math::Point2D};

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Key {
    MouseButton(MouseButton),
    KeyboardKey(VirtualKeyCode),
}

pub type Offset = Point2D;
pub type Position = Point2D;

pub struct InputState {
    pressed_keys: HashSet<Key>,
    held_keys: HashSet<Key>,
    released_keys: HashSet<Key>,
    mouse_offset: Offset,
    touches: HashMap<u64, Touch>,
    ended_touch_ids: SmallVec<[u64; 8]>,
}

pub struct Touch {
    pub id: u64,
    pub start_position: Position,
    pub move_position: Position,
    pub end_position: Option<Position>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            held_keys: HashSet::new(),
            released_keys: HashSet::new(),
            mouse_offset: Default::default(),
            touches: HashMap::new(),
            ended_touch_ids: SmallVec::new(),
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

    pub fn touches(&self) -> impl Iterator<Item = &Touch> {
        self.touches.values()
    }

    pub fn end_frame(&mut self) {
        self.released_keys.clear();
        self.pressed_keys.clear();
        self.mouse_offset = Point2D::new(0.0, 0.0);
        for id in self.ended_touch_ids.drain(..) {
            self.touches.remove(&id);
        }
    }

    pub fn set_mouse_offset(&mut self, offset: Offset) {
        self.mouse_offset = offset;
    }

    pub fn set_touch_start_position(&mut self, id: u64, position: Position) {
        self.touches.insert(
            id,
            Touch {
                id,
                start_position: position,
                move_position: position,
                end_position: None,
            },
        );
    }

    pub fn set_touch_move_position(&mut self, id: u64, position: Position) {
        match self.touches.entry(id) {
            Entry::Occupied(mut e) => {
                let t = e.get_mut();
                t.move_position = position;
            }
            _ => {
                warn!("Ignoring touch move for a missing touch")
            }
        }
    }

    pub fn set_touch_end_position(&mut self, id: u64, position: Position) {
        match self.touches.entry(id) {
            Entry::Occupied(mut e) => {
                let t = e.get_mut();
                t.end_position = Some(position);
                self.ended_touch_ids.push(t.id);
            }
            _ => {
                warn!("Ignoring touch end for a missing touch")
            }
        }
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
