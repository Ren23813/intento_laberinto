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
use framebuffer::Framebuffer;
use maze::{Maze,load_maze};
use player::Player;
use std::f32::consts::PI;
use textures::TextureManager;
use enemy::Enemy;

use crate::{caster::cast_ray, player::process_events};


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
    for (row_index, row) in maze.iter().enumerate() {
        for (col_index, &cell) in row.iter().enumerate() {
            let xo = col_index * block_size;
            let yo = row_index * block_size;
            
            draw_cell(framebuffer, xo, yo, block_size, cell);
        }
    }

    //draw player
    framebuffer.set_current_color(Color::WHITE);
    framebuffer.set_pixel(player.pos.x as i32, player.pos.y as i32);

    //dibujar n rayos
    let num_rays = 5; //framebuffer.width;
    for i in 0..num_rays {
        let current_ray = i as f32 / num_rays as f32;
        let a = player.a -(player.fov/2.0)+(player.fov*current_ray);
        cast_ray(framebuffer, maze, player, a,block_size,true);
    }
}

pub fn render_world(
    framebuffer: &mut Framebuffer,
    player: &Player,
    maze: &Maze,
    texture_cache: &TextureManager,
    depth_buffer: &mut [f32],
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

            let color = texture_cache.get_pixel_color(intersect.impact, tx as u32, ty as u32);
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
    const MAP_W: usize = 220;
    const MAP_H: usize = 150;
    const PAD: usize = 10;

    let fb_w = framebuffer.width as usize;
    let fb_h = framebuffer.height as usize;

    // Origen (esquina inferior derecha)
    let origin_x = fb_w.saturating_sub(PAD + MAP_W);
    let origin_y = fb_h.saturating_sub(PAD + MAP_H);

    // Dimensiones del mundo en píxeles (según laberinto)
    let maze_h = maze.len();
    let maze_w = if maze_h > 0 { maze[0].len() } else { 0 };
    let world_w = (maze_w * block_size) as f32;
    let world_h = (maze_h * block_size) as f32;

    // Si el laberinto está vacío, no dibujamos
    if maze_w == 0 || maze_h == 0 { return; }

    // escala del mundo -> minimapa (mantener proporción)
    let scale_x = MAP_W as f32 / world_w;
    let scale_y = MAP_H as f32 / world_h;
    // usamos la menor para que todo quepa
    let scale = scale_x.min(scale_y);

    // Color de fondo del minimapa (oscuro semitransparente)
    let bg = Color::new(10, 10, 10, 220);
    framebuffer.set_current_color(bg);
    // rellenar fondo (MAP_W x MAP_H)
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
            if ch == ' ' || ch == 'g' { continue; } // transitable
            // rect de la celda en coordenadas mundo (px)
            let cell_x = (i * block_size) as f32;
            let cell_y = (j * block_size) as f32;
            // mapear a minimapa
            let sx = origin_x as f32 + cell_x * scale;
            let sy = origin_y as f32 + cell_y * scale;
            let sw = (block_size as f32 * scale).max(1.0);
            let sh = (block_size as f32 * scale).max(1.0);
            let ix0 = sx.floor() as i32;
            let iy0 = sy.floor() as i32;
            let ix1 = (sx + sw).ceil() as i32;
            let iy1 = (sy + sh).ceil() as i32;

            framebuffer.set_current_color(wall_color);
            for px in ix0..ix1 {
                for py in iy0..iy1 {
                    framebuffer.set_pixel(px, py);
                }
            }
        }
    }

    // Dibujar jugador como punto blanco y una pequeña línea que indique la dirección
    let player_color = Color::WHITE;
    let px = origin_x as f32 + player.pos.x * scale;
    let py = origin_y as f32 + player.pos.y * scale;
    let pxi = px as i32;
    let pyi = py as i32;

    framebuffer.set_current_color(player_color);
    // punto central 3x3
    for dx in -2..=2 {
        for dy in -2..=2 {
            framebuffer.set_pixel(pxi + dx, pyi + dy);
        }
    }
    // dirección (un pequeño rayo)
    let dir_len_world = 40.0_f32; // longitud en coordenadas del mundo
    let dir_x = player.pos.x + player.a.cos() * dir_len_world;
    let dir_y = player.pos.y + player.a.sin() * dir_len_world;
    let dir_sx = origin_x as f32 + dir_x * scale;
    let dir_sy = origin_y as f32 + dir_y * scale;
    // dibuja una línea simple entre (px,py) y (dir_sx,dir_sy) con pasos
    let steps = 8;
    for s in 1..=steps {
        let t = s as f32 / steps as f32;
        let lx = px + (dir_sx - px) * t;
        let ly = py + (dir_sy - py) * t;
        framebuffer.set_pixel(lx as i32, ly as i32);
    }

    // Dibujar enemigos como puntos rojos
    let enemy_color = Color::new(220, 40, 40, 255);
    for e in enemies {
        let ex = origin_x as f32 + e.pos.x * scale;
        let ey = origin_y as f32 + e.pos.y * scale;
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

    // Borde del minimapa (opcional)
    let border_color = Color::new(220, 220, 220, 160);
    framebuffer.set_current_color(border_color);
    // superior/inferior
    for x in 0..MAP_W {
        framebuffer.set_pixel((origin_x + x) as i32, origin_y as i32);
        framebuffer.set_pixel((origin_x + x) as i32, (origin_y + MAP_H - 1) as i32);
    }
    // izquierda/derecha
    for y in 0..MAP_H {
        framebuffer.set_pixel(origin_x as i32, (origin_y + y) as i32);
        framebuffer.set_pixel((origin_x + MAP_W - 1) as i32, (origin_y + y) as i32);
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

    // Load the maze once before the loop
    let maze = load_maze("./maze.txt");
    let mut player = Player{pos:(Vector2::new(180.0,180.0)), a: PI/3.0, fov: PI/2.0 };
    let texture_cache = TextureManager::new(&mut window, &raylib_thread);
    let mut depth_buffer = vec![f32::INFINITY; window_width as usize];
    let mut enemies = vec![Enemy::new(250.0, 250.0, vec!['e', 'E'], 20)];
    let mut lives: i32 = 3;
    let max_lives: i32 = 3;
    let mut invuln_timer: f32 = 0.0;
    const INVULN_DURATION: f32 = 1.5_f32; 

    let hit_path = CString::new("assets/hit_sound.wav").expect("CString::new failed");
    let hit_sound = unsafe { ffi::LoadSound(hit_path.as_ptr()) };
    let music_path = CString::new("assets/music.ogg").expect("CString::new failed");
    let music = unsafe { ffi::LoadMusicStream(music_path.as_ptr()) };
    unsafe { ffi::PlayMusicStream(music); }

    while !window.window_should_close() {
        unsafe { ffi::UpdateMusicStream(music); }
        framebuffer.clear();
        process_events(&window, &mut player, &maze);
        // 1. clear framebuffer
        let mut mode = "3D";
        let enemy_speed = 2.7;
        for e in enemies.iter_mut() {
            e.update(&player, &maze, block_size, enemy_speed);
        }
        let dt = 1.0_f32 / 60.0_f32;
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

        if mode == "2D"{
            render_maze(&mut framebuffer, &maze, block_size,&player);
        }
        else {
            for d in depth_buffer.iter_mut() { *d = f32::INFINITY; }
            render_world(&mut framebuffer,&player,&maze,&texture_cache,&mut depth_buffer);
            render_enemies(&mut framebuffer, &player, &texture_cache, &depth_buffer, &enemies);
            draw_minimap(&mut framebuffer, &maze, &player, &enemies, block_size);
        }

        {
            let fps = window.get_fps();
            let text = format!("FPS: {}", fps);
            framebuffer.draw_text(&text, 10, 10, 20, Color::WHITE); // <- nuevo método

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
    }

    unsafe {
        ffi::UnloadSound(hit_sound);     // liberar memoria del sound
        ffi::StopMusicStream(music);
        ffi::UnloadMusicStream(music);
        ffi::CloseAudioDevice();         // cerrar dispositivo de audio
    }
}
