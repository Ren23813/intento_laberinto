use raylib::prelude::*;
use std::f32::consts::PI;
use crate::player;
use crate::maze::{Maze};

pub struct Player{
    pub pos:Vector2,
    pub a: f32,
    pub fov: f32,
}

pub fn process_events(window: &RaylibHandle, player: &mut Player, maze: &Maze) {
    const MOVE_SPEED: f32 = 7.0;
    const ROTATION_SPEED: f32 = PI / 20.0;

    let mut new_x = player.pos.x;
    let mut new_y = player.pos.y;

    if window.is_key_down(KeyboardKey::KEY_LEFT) {
        player.a -= ROTATION_SPEED;
    }

    if window.is_key_down(KeyboardKey::KEY_RIGHT) {
        player.a += ROTATION_SPEED;
    }

    if window.is_key_down(KeyboardKey::KEY_UP) {
        new_x = player.pos.x + MOVE_SPEED * player.a.cos();
        new_y = player.pos.y + MOVE_SPEED * player.a.sin();
    }

    if window.is_key_down(KeyboardKey::KEY_DOWN) {
        new_x = player.pos.x - MOVE_SPEED * player.a.cos();
        new_y = player.pos.y - MOVE_SPEED * player.a.sin();
    }

    let i = (new_x / 100.0).floor() as usize; 
    let j = (new_y / 100.0).floor() as usize; 

    if maze.get(j).and_then(|row| row.get(i)) != Some(&' ' ) && maze.get(j).and_then(|row| row.get(i)) != Some(&'g') {
        return; 
    }

    player.pos.x = new_x;
    player.pos.y = new_y;
}
