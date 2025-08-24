#![allow(unused_imports)]
#![allow(dead_code)]

mod framebuffer;
mod maze;
mod player;
mod caster;
mod textures;
mod enemy;

use raylib::prelude::*;
use std::ffi::CString;
use raylib::ffi;
use std::thread;
use std::time::Duration;
use std::collections::VecDeque;
use framebuffer::Framebuffer;
use maze::{Maze,load_maze};
use player::Player;
use std::f32::consts::PI;
use textures::TextureManager;
use enemy::Enemy;

use crate::{caster::cast_ray, player::process_events};


fn maze_filename_for_level(level: i32) -> &'static str {
    // Alterna entre maze_odd y maze_even excepto el nivel final 7
    if level == 7 {
        "maze_final.txt"
    } else if level % 2 == 0 {
        "maze_even.txt"
    } else {
        "maze_odd.txt"
    }
}

fn draw_cell(
    framebuffer: &mut Framebuffer,
    xo: usize,
    yo: usize,
    block_size: usize,
    cell: char,
) {
    if cell == ' ' {
        return;
    }

    framebuffer.set_current_color(Color::RED);

    for x in xo..xo + block_size {
        for y in yo..yo + block_size {
            framebuffer.set_pixel(x as i32, y as i32);
        }
    }
}

pub fn render_maze(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    block_size: usize,
    player: &Player
) {
    // Padding en píxeles alrededor del mapa 2D
    let pad = 16.0_f32;

    let maze_h = maze.len();
    let maze_w = if maze_h > 0 { maze[0].len() } else { 0 };
    if maze_w == 0 || maze_h == 0 {
        return;
    }

    // Tamaño del mundo en px (coordenadas del juego)
    let world_w = (maze_w * block_size) as f32;
    let world_h = (maze_h * block_size) as f32;

    // Escala para que todo quepa en la ventana (con padding)
    let avail_w = (framebuffer.width as f32) - pad * 2.0;
    let avail_h = (framebuffer.height as f32) - pad * 2.0;
    let scale_x = avail_w / world_w;
    let scale_y = avail_h / world_h;
    // usar la menor para que quepa entero; si es >1, permitimos escalar hacia arriba
    let scale = scale_x.min(scale_y);

    // Origen (arriba-izquierda) para centrar el laberinto
    let offset_x = ((framebuffer.width as f32) - world_w * scale) / 2.0;
    let offset_y = ((framebuffer.height as f32) - world_h * scale) / 2.0;

    // Dibujar celdas (paredes)
    for (row_index, row) in maze.iter().enumerate() {
        for (col_index, &cell) in row.iter().enumerate() {
            if cell == ' ' || cell == 'g' || cell == 's' { continue; }
            let cell_x = offset_x + (col_index as f32) * (block_size as f32) * scale;
            let cell_y = offset_y + (row_index as f32) * (block_size as f32) * scale;
            let cell_w = (block_size as f32) * scale;
            let cell_h = (block_size as f32) * scale;

            // color de pared (ajusta si quieres)
            framebuffer.set_current_color(Color::new(200, 40, 40, 255));
            let ix0 = cell_x.floor() as i32;
            let iy0 = cell_y.floor() as i32;
            let ix1 = (cell_x + cell_w).ceil() as i32;
            let iy1 = (cell_y + cell_h).ceil() as i32;
            for px in ix0..ix1 {
                for py in iy0..iy1 {
                    framebuffer.set_pixel(px, py);
                }
            }
        }
    }

    // Dibujar jugador (posición escalada)
    let player_sx = offset_x + player.pos.x * scale;
    let player_sy = offset_y + player.pos.y * scale;
    framebuffer.set_current_color(Color::WHITE);
    let pxi = player_sx as i32;
    let pyi = player_sy as i32;
    for dx in -2..=2 {
        for dy in -2..=2 {
            framebuffer.set_pixel(pxi + dx, pyi + dy);
        }
    }

    // Dibujar rayos (debug) escalados — usamos cast_ray con draw_line=false y calculamos punta
    let num_rays = 32usize.min(16); // ajustable
    for i in 0..num_rays {
        let current_ray = i as f32 / num_rays as f32;
        let a = player.a - (player.fov / 2.0) + (player.fov * current_ray);
        // pedimos pero sin dibujar
        let intersect = cast_ray(framebuffer, maze, player, a, block_size, false);

        // Punto de impacto en coordenadas del mundo
        let d = intersect.distance;
        let ix_world = player.pos.x + d * a.cos();
        let iy_world = player.pos.y + d * a.sin();

        // convertir a coordenadas de pantalla escaladas
        let sx = offset_x + ix_world * scale;
        let sy = offset_y + iy_world * scale;

        // dibujar línea desde jugador hasta impacto (muestreo simple)
        let steps = ((d * scale).max(4.0)) as usize;
        framebuffer.set_current_color(Color::new(255, 255, 255, 255));
        for s in 0..=steps {
            let t = s as f32 / (steps as f32).max(1.0);
            let lx = player_sx + (sx - player_sx) * t;
            let ly = player_sy + (sy - player_sy) * t;
            framebuffer.set_pixel(lx as i32, ly as i32);
        }
    }
}


pub fn render_world(
    framebuffer: &mut Framebuffer,
    player: &Player,
    maze: &Maze,
    texture_cache: &TextureManager,
    depth_buffer: &mut [f32],
    current_level: usize,
) {
    let block_size = 100;
    let num_rays = framebuffer.width as usize;

    let hh = framebuffer.height as f32 / 2.0;

    //techo
    let ceiling_tex_key = 'c'; 
    let tex_width = 578.0;
    let tex_height = 347.0;
    for y in 0..hh as usize {
        for x in 0..framebuffer.width as usize {
            let tx = (x as f32 / framebuffer.width as f32) * tex_width;
            let ty = (y as f32 / hh) * tex_height;

            let color = texture_cache.get_pixel_color(ceiling_tex_key, tx as u32, ty as u32);
            framebuffer.set_current_color(color);
            framebuffer.set_pixel(x as i32, y as i32);
        }
    }

    let mut min_stake_top = framebuffer.height as usize; 
    for i in 0..num_rays {
        let current_ray = i as f32 / num_rays as f32;
        let a = player.a - (player.fov / 2.0) + (player.fov * current_ray);
        let intersect = cast_ray(framebuffer, maze, player, a, block_size, false);
        depth_buffer[i] = intersect.distance;

        let angle_diff = a - player.a;
        let mut distance_to_wall = intersect.distance * angle_diff.cos();
        if distance_to_wall < 0.1 {
            distance_to_wall = 0.2;
        }

        // Altura de la pared
        let stake_height = (hh / distance_to_wall) * 70.0;
        let stake_top = (hh - (stake_height / 2.0)).max(0.0) as usize;
        let stake_bottom = (hh + (stake_height / 2.0)).min(framebuffer.height as f32) as usize;
        if stake_top < min_stake_top {min_stake_top = stake_top;}

        // --- Pared ---
        for y in stake_top..stake_bottom {
            let tx = intersect.tx;
            let ty = (y as f32 - stake_top as f32) / (stake_bottom as f32 - stake_top as f32) * 128.0;
            let impact = intersect.impact;
            let tex_key = if impact == 'L' {
                std::char::from_digit(current_level as u32, 10).unwrap_or('L')
            } else {
                impact
            };
            let color = texture_cache.get_pixel_color(tex_key, tx as u32, ty as u32);
            framebuffer.set_current_color(color);
            framebuffer.set_pixel(i as i32, y as i32);
        }

        // --- Suelo (optimizado) ---
        let floor_tex_key = 'f';
        let cos_a = a.cos();
        let sin_a = a.sin();
        let cos_angle_diff = angle_diff.cos();

        // Saltamos filas para reducir carga: cada 5 píxeles (si no no llega a 60fps xd)
        let step = 5;

        for y in (stake_bottom..framebuffer.height as usize).step_by(step) {
            let perspective = hh / (y as f32 - hh);
            let dist = perspective / cos_angle_diff;

            let floor_x = player.pos.x + dist * cos_a;
            let floor_y = player.pos.y + dist * sin_a;

            let tx = ((floor_x as usize % block_size) as f32 / block_size as f32) * 200.0;
            let ty = ((floor_y as usize % block_size) as f32 / block_size as f32) * 150.0;

            let color = texture_cache.get_pixel_color(floor_tex_key, tx as u32, ty as u32);
            framebuffer.set_current_color(color);

            // Rellenar los píxeles faltantes entre pasos
            for dy in 0..step {
                if y + dy < framebuffer.height as usize {
                    framebuffer.set_pixel(i as i32, (y + dy) as i32);
                }
            }
        }
    }
}


const TRANSPARENT_COLOR: Color = Color::new(152, 0, 136, 255);
fn draw_sprite(
    framebuffer: &mut Framebuffer,
    player: &Player,
    enemy: &Enemy,
    texture_manager: &TextureManager,
    depth_buffer: &[f32]
) {
    let sprite_a = (enemy.pos.y - player.pos.y).atan2(enemy.pos.x - player.pos.x);
    let mut angle_diff = sprite_a - player.a;
    while angle_diff > PI {
        angle_diff -= 2.0 * PI;
    }
    while angle_diff < -PI {
        angle_diff += 2.0 * PI;
    }

    // fuera del FOV
    if angle_diff.abs() > player.fov / 2.5 {
        return;
    }

    let sprite_d: f32 = ((player.pos.x - enemy.pos.x).powi(2) + (player.pos.y - enemy.pos.y).powi(2)).sqrt();

    // near plane / far plane
    if sprite_d < 50.0 || sprite_d > 1000.0 {
        return;
    }

    let screen_height = framebuffer.height as f32;
    let screen_width = framebuffer.width as f32;

    let sprite_size = (screen_height / sprite_d) * 70.0;
    if sprite_size < 1.0 {
        return;
    }

    let screen_x = ((angle_diff / player.fov) + 0.5) * screen_width;

    let start_x = (screen_x - sprite_size / 2.0).max(0.0) as usize;
    let start_y = (screen_height / 2.0 - sprite_size / 2.0).max(0.0) as usize;
    let sprite_size_usize: usize = sprite_size.max(1.0) as usize;
    let end_x = (start_x + sprite_size_usize).min(framebuffer.width as usize);
    let end_y = (start_y + sprite_size_usize).min(framebuffer.height as usize);

    // obtener tamaño real de la textura desde Texture2D si existe
    let (tex_w, tex_h) = if let Some(tex) = texture_manager.get_texture(enemy.current_key()) {
        (tex.width() as f32, tex.height() as f32)
    } else {
        // fallback si no existe la textura
        (128.0, 128.0)
    };

     for x in start_x..end_x {
        // Ocultación por columna: si sprite está detrás de la pared en esta columna, saltamos toda la columna
        // depth_buffer usa distancias crudas (igual que sprite_d)
        if sprite_d >= depth_buffer[x] {
            continue;
        }

        for y in start_y..end_y {
            let tx_f = ((x as f32 - start_x as f32) / sprite_size) * tex_w;
            let ty_f = ((y as f32 - start_y as f32) / sprite_size) * tex_h;

            let tx_u32 = tx_f.max(0.0).min(tex_w - 1.0) as u32;
            let ty_u32 = ty_f.max(0.0).min(tex_h - 1.0) as u32;

            let key = enemy.current_key();
            let color = texture_manager.get_pixel_color(key, tx_u32, ty_u32);

            if color.a == 0 {
                continue;
            }

            framebuffer.set_current_color(color);
            framebuffer.set_pixel(x as i32, y as i32);
        }
    }
}



fn render_enemies(
    framebuffer: &mut Framebuffer,
    player: &Player,
    texture_cache: &TextureManager,
    depth_buffer: &[f32],
    enemies: &[Enemy],
) {
    for enemy in enemies {
        draw_sprite(framebuffer, player, enemy, texture_cache, depth_buffer);
    }
}

/// Dibuja un minimapa en la esquina inferior derecha del framebuffer.
/// - `block_size` es el tamaño (en px) de una celda del mundo (tú usas 100).
pub fn draw_minimap(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    enemies: &[Enemy],
    block_size: usize,
) {
    // Tamaño del minimapa en píxeles (ajusta si quieres más grande/pequeño)
    const MAP_W: usize = 260;
    const MAP_H: usize = 160;
    const PAD: usize = 10;

    let fb_w = framebuffer.width as usize;
    let fb_h = framebuffer.height as usize;

    // Origen (esquina inferior derecha)
    let origin_x = fb_w.saturating_sub(PAD + MAP_W);
    let origin_y = fb_h.saturating_sub(PAD + MAP_H);

    // Dimensiones del mundo en píxeles (según laberinto)
    let maze_h = maze.len();
    let maze_w = if maze_h > 0 { maze[0].len() } else { 0 };
    if maze_w == 0 || maze_h == 0 { return; }

    let world_w = (maze_w * block_size) as f32;
    let world_h = (maze_h * block_size) as f32;

    // escala del mundo -> minimapa (mantener proporción)
    let scale_x = MAP_W as f32 / world_w;
    let scale_y = MAP_H as f32 / world_h;
    let scale = scale_x.min(scale_y);

    // centrar el mapa dentro del recuadro minimapa
    let used_w = world_w * scale;
    let used_h = world_h * scale;
    let inner_offset_x = origin_x as f32 + ((MAP_W as f32 - used_w) / 2.0);
    let inner_offset_y = origin_y as f32 + ((MAP_H as f32 - used_h) / 2.0);

    // fondo minimapa
    let bg = Color::new(10, 10, 10, 220);
    framebuffer.set_current_color(bg);
    for mx in 0..MAP_W {
        for my in 0..MAP_H {
            framebuffer.set_pixel((origin_x + mx) as i32, (origin_y + my) as i32);
        }
    }

    // Dibujar paredes (cada celda del maze que no sea ' ' ni 'g' será pared)
    let wall_color = Color::new(160, 160, 160, 255);
    for j in 0..maze_h {
        for i in 0..maze_w {
            let ch = maze[j][i];
            if ch == ' ' || ch == 'g' || ch == 's' { continue; }
            let cell_x = inner_offset_x + (i as f32) * (block_size as f32) * scale;
            let cell_y = inner_offset_y + (j as f32) * (block_size as f32) * scale;
            let sw = (block_size as f32) * scale;
            let sh = (block_size as f32) * scale;
            let ix0 = cell_x.floor() as i32;
            let iy0 = cell_y.floor() as i32;
            let ix1 = (cell_x + sw).ceil() as i32;
            let iy1 = (cell_y + sh).ceil() as i32;
            framebuffer.set_current_color(wall_color);
            for px in ix0..ix1 {
                for py in iy0..iy1 {
                    framebuffer.set_pixel(px, py);
                }
            }
        }
    }

    // Dibujar player como punto con flecha de dirección (escala apropiada)
    let px = inner_offset_x + player.pos.x * scale;
    let py = inner_offset_y + player.pos.y * scale;
    let pxi = px as i32;
    let pyi = py as i32;
    framebuffer.set_current_color(Color::WHITE);
    for dx in -2..=2 {
        for dy in -2..=2 {
            framebuffer.set_pixel(pxi + dx, pyi + dy);
        }
    }
    // flecha/dirección proporcional al tamaño del minimapa
    let dir_len_world = (block_size as f32).max(24.0); // longitud en coordenadas del mundo
    let dir_len = dir_len_world * scale;
    let dir_x = px + player.a.cos() * dir_len;
    let dir_y = py + player.a.sin() * dir_len;
    let steps = 6usize;
    framebuffer.set_current_color(Color::WHITE);
    for s in 1..=steps {
        let t = s as f32 / steps as f32;
        let lx = px + (dir_x - px) * t;
        let ly = py + (dir_y - py) * t;
        framebuffer.set_pixel(lx as i32, ly as i32);
    }

    // Dibujar enemigos
    let enemy_color = Color::new(220, 40, 40, 255);
    for e in enemies {
        let ex = inner_offset_x + e.pos.x * scale;
        let ey = inner_offset_y + e.pos.y * scale;
        let exi = ex as i32;
        let eyi = ey as i32;
        framebuffer.set_current_color(enemy_color);
        // punto 3x3
        for dx in -1..=1 {
            for dy in -1..=1 {
                framebuffer.set_pixel(exi + dx, eyi + dy);
            }
        }
    }

    // Borde del minimapa
    let border_color = Color::new(220, 220, 220, 180);
    framebuffer.set_current_color(border_color);
    for x in 0..MAP_W {
        framebuffer.set_pixel((origin_x + x) as i32, origin_y as i32);
        framebuffer.set_pixel((origin_x + x) as i32, (origin_y + MAP_H - 1) as i32);
    }
    for y in 0..MAP_H {
        framebuffer.set_pixel(origin_x as i32, (origin_y + y) as i32);
        framebuffer.set_pixel((origin_x + MAP_W - 1) as i32, (origin_y + y) as i32);
    }
}


fn tile_center_pos(i: usize, j: usize, block_size: usize) -> Vector2 {
    Vector2::new(
        i as f32 * block_size as f32 + (block_size as f32) * 0.5,
        j as f32 * block_size as f32 + (block_size as f32) * 0.5,
    )
}

/// Busca la primera celda con carácter `ch` en el laberinto y devuelve (i,j).
fn find_tile(maze: &Maze, ch: char) -> Option<(usize, usize)> {
    for (j, row) in maze.iter().enumerate() {
        for (i, &c) in row.iter().enumerate() {
            if c == ch {
                return Some((i, j));
            }
        }
    }
    None
}

/// Busca la celda transitable más cercana al centro del mapa (espiral simple).
/// Devuelve la posición central (en píxeles) de esa celda.
fn find_nearest_free_to_center(maze: &Maze, block_size: usize) -> Vector2 {
    let h = maze.len();
    if h == 0 {
        return Vector2::new(block_size as f32 * 0.5, block_size as f32 * 0.5);
    }
    let w = maze[0].len();

    let cx = (w / 2) as isize;
    let cy = (h / 2) as isize;

    let max_r = (w.max(h) / 2) as isize + 4;

    for r in 0..=max_r {
        // recorremos un "anillo" alrededor del centro
        for dx in -r..=r {
            for dy in -r..=r {
                // solo bordes del anillo
                if dx.abs() != r && dy.abs() != r { continue; }
                let ix = cx + dx;
                let iy = cy + dy;
                if ix < 0 || iy < 0 { continue; }
                let (ixu, iyu) = (ix as usize, iy as usize);
                if iyu >= h || ixu >= w { continue; }
                let c = maze[iyu][ixu];
                if c == ' ' || c == 'g' || c == 's' {
                    return tile_center_pos(ixu, iyu, block_size);
                }
            }
        }
    }

    // fallback: primera celda (0,0)
    tile_center_pos(0, 0, block_size)
}

/// Busca la celda transitable más cercana a (center_i, center_j) — espiral limitada por `max_r` (en celdas).
/// Devuelve la posición central (en píxeles) de la celda encontrada.
fn find_nearest_free_around(maze: &Maze, block_size: usize, center_i: usize, center_j: usize, max_r: usize) -> Vector2 {
    let h = maze.len();
    if h == 0 {
        return tile_center_pos(0, 0, block_size);
    }
    let w = maze[0].len();

    // convertimos a isize para poder movernos en anillos negativos
    let cx = center_i as isize;
    let cy = center_j as isize;

    let max_r_is = max_r as isize;
    for r in 0..=max_r_is {
        // recorrer el "borde" del anillo r alrededor de (cx,cy)
        for dx in -r..=r {
            for dy in -r..=r {
                if dx.abs() != r && dy.abs() != r { continue; } // solo borde
                let ix = cx + dx;
                let iy = cy + dy;
                if ix < 0 || iy < 0 { continue; }
                let (ixu, iyu) = (ix as usize, iy as usize);
                if iyu >= h || ixu >= w { continue; }
                let c = maze[iyu][ixu];
                if c == ' ' || c == 'g' || c == 's' {
                    return tile_center_pos(ixu, iyu, block_size);
                }
            }
        }
    }

    find_nearest_free_to_center(maze, block_size)
}

fn find_spawn_reachable(
    maze: &Maze,
    block_size: usize,
    player_pos: Vector2,
    min_dist_cells: usize, 
) -> Vector2 {
    let h = maze.len();
    if h == 0 {
        return tile_center_pos(0, 0, block_size);
    }
    let w = maze[0].len();

    // cell indices del jugador (clamped dentro del maze)
    let mut pi = (player_pos.x / block_size as f32).floor() as isize;
    let mut pj = (player_pos.y / block_size as f32).floor() as isize;
    if pi < 0 { pi = 0 }
    if pj < 0 { pj = 0 }
    if pj as usize >= h { pj = (h-1) as isize }
    if pi as usize >= w { pi = (w-1) as isize }

    let start_i = pi as usize;
    let start_j = pj as usize;

    let mut visited = vec![vec![false; w]; h];
    let mut q = VecDeque::new();
    q.push_back((start_i, start_j));
    visited[start_j][start_i] = true;

    let mut reachable = Vec::<(usize, usize)>::new();

    while let Some((i, j)) = q.pop_front() {
        // solo contar celdas que sean transitables
        let ch = maze[j][i];
        if ch == ' ' || ch == 'g' || ch == 's' {
            reachable.push((i, j));
            // expandir 4 vecinos
            let neighbors = [
                (i as isize + 1, j as isize),
                (i as isize - 1, j as isize),
                (i as isize, j as isize + 1),
                (i as isize, j as isize - 1),
            ];
            for (nx, ny) in neighbors.iter() {
                if *nx < 0 || *ny < 0 { continue; }
                let nxu = *nx as usize;
                let nyu = *ny as usize;
                if nyu >= h || nxu >= w { continue; }
                if visited[nyu][nxu] { continue; }
                let c = maze[nyu][nxu];
                // solo entrar en celdas que no sean paredes
                if c == ' ' || c == 'g' || c == 's' {
                    visited[nyu][nxu] = true;
                    q.push_back((nxu, nyu));
                }
            }
        }
    }

    // Si no hay celdas alcanzables, fallback al centro aproximado
    if reachable.is_empty() {
        return find_nearest_free_to_center(maze, block_size);
    }

    // Centro del mapa en coords de celda (floats)
    let center_fx = (w as f32 - 1.0) * 0.5;
    let center_fy = (h as f32 - 1.0) * 0.5;

    // posición celda jugador (para medir separación mínima)
    let player_ci = start_i as isize;
    let player_cj = start_j as isize;

    // elegir la celda alcanzable que esté más cerca del centro
    // pero que además cumpla la restricción de min_dist_cells (si es posible)
    let mut best: Option<(usize, usize, f32)> = None;

    for &(i, j) in &reachable {
        // distancia en celdas al jugador
        let dxp = (i as isize - player_ci).abs() as usize;
        let dyp = (j as isize - player_cj).abs() as usize;
        let manh_dist = dxp.max(dyp); // métrica simple

        if manh_dist < min_dist_cells {
            continue; // saltar, queremos separación
        }

        let dx = (i as f32) - center_fx;
        let dy = (j as f32) - center_fy;
        let dist_center = (dx*dx + dy*dy).sqrt();

        match best {
            None => best = Some((i, j, dist_center)),
            Some((_, _, best_d)) => {
                if dist_center < best_d {
                    best = Some((i, j, dist_center));
                }
            }
        }
    }
    // Si no encontramos con la restricción min_dist_cells, relajarla
    if best.is_none() && min_dist_cells > 0 {
        for &(i, j) in &reachable {
            let dx = (i as f32) - center_fx;
            let dy = (j as f32) - center_fy;
            let dist_center = (dx*dx + dy*dy).sqrt();
            match best {
                None => best = Some((i, j, dist_center)),
                Some((_, _, best_d)) => {
                    if dist_center < best_d {
                        best = Some((i, j, dist_center));
                    }
                }
            }
        }
    }

    if let Some((bi, bj, _)) = best {
        tile_center_pos(bi, bj, block_size)
    } else {
        // fallback
        find_nearest_free_to_center(maze, block_size)
    }
}


fn main() {
    let window_width = 1300;
    let window_height = 900;
    let block_size = 100;

    let (mut window, raylib_thread) = raylib::init()
        .size(window_width, window_height)
        .title("Raycaster Example")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();
    unsafe {
        ffi::InitAudioDevice();
    }
    window.set_target_fps(60); //fluctúa a un poco menos

    let mut framebuffer = Framebuffer::new(window_width as i32, window_height as i32,Color::BLACK);
    framebuffer.set_background_color(Color::new(50, 50, 100, 255));

    let levels = vec![3, 4, 5, 6, 7]; // pisos
    let mut level_index: usize = 0; // comienza en levels[0] = piso 3
    let mut current_level = levels[level_index];

    // Load the maze once before the loop
    let mut maze = load_maze(maze_filename_for_level(current_level));
    let player_pos = if let Some((si, sj)) = find_tile(&maze, 's') {
        tile_center_pos(si, sj, block_size)
    } else if let Some((gi, gj)) = find_tile(&maze, 'g') {
        tile_center_pos(gi, gj, block_size)
    } else {
        let temp_player = Vector2::new((block_size/2) as f32, (block_size/2) as f32);
        find_spawn_reachable(&maze, block_size, temp_player, 0)
    };
    let mut player = Player{pos:player_pos, a: PI/3.0, fov: PI/2.0 };
    let texture_cache = TextureManager::new(&mut window, &raylib_thread);
    let mut depth_buffer = vec![f32::INFINITY; window_width as usize];
    let mut enemies = vec![Enemy::new(250.0, 250.0, vec!['e', 'E'], 20)];
    let mut lives: i32 = 3;
    let max_lives: i32 = 3;
    let mut invuln_timer: f32 = 0.0;
    const INVULN_DURATION: f32 = 1.5_f32; 
    let base_enemy_speed = 2.7_f32;
    let mut levels_passed: usize = 0; 

    let hit_path = CString::new("assets/hit_sound.wav").expect("CString::new failed");
    let hit_sound = unsafe { ffi::LoadSound(hit_path.as_ptr()) };
    let music_path = CString::new("assets/music.ogg").expect("CString::new failed");
    let music = unsafe { ffi::LoadMusicStream(music_path.as_ptr()) };
    unsafe { ffi::PlayMusicStream(music); }

    let mut level_transition_cooldown: f32 = 0.0;
    const LEVEL_TRANSITION_COOLDOWN: f32 = 0.6_f32;

    while !window.window_should_close() {
        unsafe { ffi::UpdateMusicStream(music); }
        let dt = 1.0_f32 / 60.0_f32;
        if level_transition_cooldown > 0.0 {
            level_transition_cooldown -= dt;
            if level_transition_cooldown < 0.0 { level_transition_cooldown = 0.0; }
        }

        framebuffer.clear();
        process_events(&window, &mut player, &maze);
        // 1. clear framebuffer
        let mut mode = "3D";
        let enemy_speed = base_enemy_speed + 0.2_f32 * (levels_passed as f32);
        for e in enemies.iter_mut() {
            e.update(&player, &maze, block_size, enemy_speed);
        }
                if invuln_timer > 0.0 {
                    invuln_timer -= dt;
                    if invuln_timer < 0.0 { invuln_timer = 0.0; }
                }

        let collision_radius = 28.0_f32; // radio en pixeles para considerar "contacto"
        for enemy in enemies.iter_mut() {
            let dx = enemy.pos.x - player.pos.x;
            let dy = enemy.pos.y - player.pos.y;
            let dist = (dx*dx + dy*dy).sqrt();

            if dist <= collision_radius && invuln_timer <= 0.0 && lives > 0 {
                // perder 1 vida
                lives -= 1;
                invuln_timer = INVULN_DURATION;

                // reproducir sonido si existe
                unsafe {
                    ffi::PlaySound(hit_sound);
                }
                break;
            }
        }
        // Encolar estado de vidas para que el framebuffer lo dibuje en swap_buffers
        framebuffer.queue_health(lives, max_lives);

        if window.is_key_down(KeyboardKey::KEY_M) {
            mode = if mode =="2D" {"3D"} else {"2D"};
        }

        let cell_i = ((player.pos.x / block_size as f32).floor() as isize).max(0) as usize;
        let cell_j = ((player.pos.y / block_size as f32).floor() as isize).max(0) as usize;
        let at_exit = maze.get(cell_j).and_then(|row| row.get(cell_i)).map_or(false, |&c| c == 'g');

        if at_exit && window.is_key_pressed(KeyboardKey::KEY_E) {
            // guardamos coordenada de salida en el mapa antiguo (la 'g' donde el jugador estaba)
            let prev_exit = Some((cell_i, cell_j));

            // avanzar de nivel (si hay)
            if level_index + 1 < levels.len() {
                level_index += 1;
                current_level = levels[level_index];
                // re-cargar maze nuevo (destino)
                let filename = maze_filename_for_level(current_level);
                let new_maze = load_maze(filename);

                // --- COLOCAR JUGADOR en la misma CELDA (pi,pj) del mapa anterior ---
                // --- COLOCAR JUGADOR en la misma CELDA (pi,pj) del mapa anterior ---
                // Nueva lógica: si el nuevo mapa contiene 's' úsalo; si no, usar prev_exit como antes.
                if let Some((si, sj)) = find_tile(&new_maze, 's') {
                    // spawn explícito en la 's' del nuevo mapa (útil para mapa final 7)
                    player.pos = tile_center_pos(si, sj, block_size);
                } else if let Some((pi, pj)) = prev_exit {
                    let maze_h_new = new_maze.len();
                    let maze_w_new = if maze_h_new > 0 { new_maze[0].len() } else { 0 };

                    if maze_w_new == 0 || maze_h_new == 0 {
                        player.pos = find_nearest_free_to_center(&new_maze, block_size);
                    } else if pi < maze_w_new && pj < maze_h_new {
                        let ch = new_maze[pj][pi];
                        if ch == ' ' || ch == 'g' {
                            player.pos = tile_center_pos(pi, pj, block_size);
                        } else {
                            player.pos = find_nearest_free_around(&new_maze, block_size, pi, pj, 8);
                        }
                    } else {
                        let clamped_i = if maze_w_new == 0 { 0 } else { pi.min(maze_w_new.saturating_sub(1)) };
                        let clamped_j = if maze_h_new == 0 { 0 } else { pj.min(maze_h_new.saturating_sub(1)) };
                        player.pos = find_nearest_free_around(&new_maze, block_size, clamped_i, clamped_j, 8);
                    }
                } else {
                    // fallback si no hay prev_exit ni 's'
                    player.pos = find_nearest_free_to_center(&new_maze, block_size);
                }

                player.a = PI / 3.0; // reiniciar la rotación
                let enemy_spawn = find_spawn_reachable(&new_maze, block_size, player.pos, 3);
                enemies = vec![Enemy::new(enemy_spawn.x, enemy_spawn.y, vec!['e','E'], 20)];
                // refill vidas
                lives = max_lives;
                invuln_timer = 0.0;
                // aumentar contador de niveles completados
                levels_passed += 1;
                // re-asignar el maze cargado (nuevo)
                maze = new_maze;
                // cooldown para evitar triggers repetidos
                level_transition_cooldown = LEVEL_TRANSITION_COOLDOWN;
            } else {
                // último nivel: reiniciamos al primero (o podrías mostrar pantalla de victoria)
                println!("Has llegado al final (piso {}). Reiniciando.", current_level);
                level_index = 0;
                current_level = levels[level_index];
                maze = load_maze(maze_filename_for_level(current_level));
                // spawn player en la 'g' si existe, o centro libre
                if let Some((gi, gj)) = find_tile(&maze, 'g') {
                    player.pos = tile_center_pos(gi, gj, block_size);
                } else {
                    player.pos = find_nearest_free_to_center(&maze, block_size);
                }
                // reset enemigo al centro del nuevo mapa
                let ec = find_nearest_free_to_center(&maze, block_size);
                enemies = vec![Enemy::new(ec.x, ec.y, vec!['e', 'E'], 20)];
                lives = max_lives;
                levels_passed = 0;
                level_transition_cooldown = LEVEL_TRANSITION_COOLDOWN;
            }
        }


        if mode == "2D"{
            render_maze(&mut framebuffer, &maze, block_size,&player);
        }
        else {
            for d in depth_buffer.iter_mut() { *d = f32::INFINITY; }
            render_world(&mut framebuffer,&player,&maze,&texture_cache,&mut depth_buffer,current_level as usize);
            render_enemies(&mut framebuffer, &player, &texture_cache, &depth_buffer, &enemies);
            draw_minimap(&mut framebuffer, &maze, &player, &enemies, block_size);
        }

        {
            let fps = window.get_fps();
            let text = format!("FPS: {}", fps);
            framebuffer.draw_text(&text, 10, 10, 20, Color::WHITE); // <- nuevo método

            let piso_text = format!("piso {}", current_level);
            let font_size = 28;
            // ancho aproximado (estimación) para centrar: asumir 0.6 * font_size por carácter
            let approx_text_width = piso_text.len() as f32 * (font_size as f32) * 0.6;
            let x_center = (framebuffer.width as f32 / 2.0 - approx_text_width / 2.0) as i32;
            framebuffer.draw_text(&piso_text, x_center, 6, font_size, Color::YELLOW);

            // Calcula la distancia mínima del jugador a cualquier enemigo
            let mut min_enemy_dist = f32::INFINITY;
            for enemy in enemies.iter() {
                let dx = enemy.pos.x - player.pos.x;
                let dy = enemy.pos.y - player.pos.y;
                let d = (dx*dx + dy*dy).sqrt();
                if d < min_enemy_dist { min_enemy_dist = d; }
            }

            let max_effect_distance = 1500.0_f32; // comienza a afectar desde más lejos
            let min_effect_distance = 80.0_f32;  // muy cerca = círculo mínimo
            let max_radius = (framebuffer.width.min(framebuffer.height)) as f32 * 0.9; // radio máximo
            let min_radius = 40.0_f32; // radio mínimo visible al acercarse mucho

            // oscuridad base
            let min_darkness = 0.5_f32; // lejos = oscuridad leve
            let max_darkness = 1.0_f32; // cerca = oscuridad total

            // calcular factor de proximidad t en [0..1]
            let t = if min_enemy_dist >= max_effect_distance {
                0.0_f32
            } else if min_enemy_dist <= min_effect_distance {
                1.0_f32
            } else {
                (max_effect_distance - min_enemy_dist) / (max_effect_distance - min_effect_distance)
            };

            // radio interpolado
            let radius = max_radius * (1.0 - t) + min_radius * t;

            // oscuridad interpolada
            let darkness = min_darkness + (max_darkness - min_darkness) * t;

            // centro de pantalla (linterna centrada)
            let center_x = (framebuffer.width / 2) as i32;
            let center_y = (framebuffer.height / 2) as i32;

            // aplicar efecto
            framebuffer.draw_vignette(center_x, center_y, radius, darkness);
            framebuffer.swap_buffers(&mut window, &raylib_thread);
        }
                // thread::sleep(Duration::from_millis(1));

    }

    unsafe {
        ffi::UnloadSound(hit_sound);     // liberar memoria del sound
        ffi::StopMusicStream(music);
        ffi::UnloadMusicStream(music);
        ffi::CloseAudioDevice();         // cerrar dispositivo de audio
    }
}
