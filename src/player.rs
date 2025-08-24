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
    const ROTATION_SPEED: f32 = PI / 25.0;
    const TILE_SIZE: f32 = 100.0; // mantén en sincronía con tu main

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

    // calcular índices de celda de forma segura (isize)
    let i_isize = (new_x / TILE_SIZE).floor() as isize;
    let j_isize = (new_y / TILE_SIZE).floor() as isize;

    if i_isize < 0 || j_isize < 0 {
        // fuera de mapa: bloquear movimiento
        return;
    }
    let i = i_isize as usize;
    let j = j_isize as usize;

    // comprobar límites del maze
    if j >= maze.len() { return; }
    if maze.get(j).map_or(true, |row| i >= row.len()) { return; }

    // aceptar ' ', 'g' o 's' como transitables
    let ch = maze[j][i];
    if ch == ' ' || ch == 'g' || ch == 's' {
        player.pos.x = new_x;
        player.pos.y = new_y;
    }
}
