#![allow(unused_imports)]
#![allow(dead_code)]

mod framebuffer;
mod maze;
mod player;
mod caster;
mod textures;
mod enemy;

use raylib::prelude::*;
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

    let hw = framebuffer.width as f32 / 2.0;
    let hh = framebuffer.height as f32 / 2.0;

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

        // --- Suelo ---
        let floor_tex_key = 'f'; // clave de textura para el suelo en TextureManager
        for y in stake_bottom..framebuffer.height as usize {
            // Distancia desde el jugador hasta este punto del suelo
            let perspective = hh / (y as f32 - hh);
            let dist = perspective / angle_diff.cos();

            let floor_x = player.pos.x + dist * a.cos();
            let floor_y = player.pos.y + dist * a.sin();

            // Convertir posición del mundo a coordenadas de textura (128x128)
            let tx = ((floor_x as usize % block_size) as f32 / block_size as f32) * 200.0;
            let ty = ((floor_y as usize % block_size) as f32 / block_size as f32) * 150.0;

            let color = texture_cache.get_pixel_color(floor_tex_key, tx as u32, ty as u32);
            framebuffer.set_current_color(color);
            framebuffer.set_pixel(i as i32, y as i32);
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
    let (tex_w, tex_h) = if let Some(tex) = texture_manager.get_texture(enemy.texture_key) {
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

            let color = texture_manager.get_pixel_color(enemy.texture_key, tx_u32, ty_u32);

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


fn main() {
    let window_width = 1300;
    let window_height = 900;
    let block_size = 100;

    let (mut window, raylib_thread) = raylib::init()
        .size(window_width, window_height)
        .title("Raycaster Example")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();
    window.set_target_fps(60); //Creo que hasta aquí llegó raylib (o mi compu), porque se queda colgado en 45fps en mi laptop con cargador...

    let mut framebuffer = Framebuffer::new(window_width as i32, window_height as i32,Color::BLACK);

    framebuffer.set_background_color(Color::new(50, 50, 100, 255));

    // Load the maze once before the loop
    let maze = load_maze("./maze.txt");
    let mut player = Player{pos:(Vector2::new(180.0,180.0)), a: PI/3.0, fov: PI/2.0 };
    let texture_cache = TextureManager::new(&mut window, &raylib_thread);
    let mut depth_buffer = vec![f32::INFINITY; window_width as usize];
    let mut enemies = vec![Enemy::new(250.0, 250.0, 'e')];


    while !window.window_should_close() {
        framebuffer.clear();
        process_events(&window, &mut player, &maze);
        // 1. clear framebuffer
        let mut mode = "3D";
        let enemy_speed = 2.7;
        for e in enemies.iter_mut() {
            e.update(&player, &maze, block_size, enemy_speed);
        }

        if window.is_key_down(KeyboardKey::KEY_M) {
            mode = if mode =="2D" {"3D"} else {"2D"};
        }
        // framebuffer.clear();

        if mode == "2D"{
            render_maze(&mut framebuffer, &maze, block_size,&player);
        }
        else {
            for d in depth_buffer.iter_mut() { *d = f32::INFINITY; }
            render_world(&mut framebuffer,&player,&maze,&texture_cache,&mut depth_buffer);
            render_enemies(&mut framebuffer, &player, &texture_cache, &depth_buffer, &enemies);
        }

        {
    let fps = window.get_fps();
    let text = format!("FPS: {}", fps);
    framebuffer.draw_text(&text, 10, 10, 20, Color::WHITE); // <- nuevo método

    // Mostrar en ventana
    framebuffer.swap_buffers(&mut window, &raylib_thread);

    }
}
}
