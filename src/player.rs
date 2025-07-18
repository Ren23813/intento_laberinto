use raylib::prelude::*;
use std::f32::consts::PI;
use crate::player;

pub struct Player{
    pub pos:Vector2,
    pub a: f32,
}

pub fn process_events(window: &RaylibHandle, player: &mut Player) {
    const MOVE_SPEED: f32 = 10.0;
    const ROTATION_SPEED: f32 = PI / 10.0;

    if window.is_key_down(KeyboardKey::KEY_LEFT) {
        player.a -= ROTATION_SPEED; 
    } 
    if window.is_key_down(KeyboardKey::KEY_RIGHT) {
        player.a += ROTATION_SPEED; 
    } 
    if window.is_key_down(KeyboardKey::KEY_UP) {
        player.pos.x += MOVE_SPEED * player.a.cos(); 
        player.pos.y += MOVE_SPEED * player.a.sin();
    }
    if window.is_key_down(KeyboardKey::KEY_DOWN) {
        player.pos.x -= MOVE_SPEED * player.a.cos(); 
        player.pos.y -= MOVE_SPEED * player.a.sin();
    } 
}