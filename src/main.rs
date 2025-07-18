#![allow(unused_imports)]
#![allow(dead_code)]

mod framebuffer;
mod maze;
mod player;
mod caster;

use raylib::prelude::*;
use std::thread;
use std::time::Duration;
use framebuffer::Framebuffer;
use maze::{Maze,load_maze};
use player::Player;
use std::f32::consts::PI;

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
    cast_ray(framebuffer, maze, player, block_size);
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

    let mut framebuffer = Framebuffer::new(window_width as i32, window_height as i32,Color::BLACK);

    framebuffer.set_background_color(Color::new(50, 50, 100, 255));

    // Load the maze once before the loop
    let maze = load_maze("./maze.txt");
    let mut player = Player{pos:(Vector2::new(180.0,180.0)), a: PI/3.0 };

    while !window.window_should_close() {

        process_events(&window, &mut player);
        // 1. clear framebuffer
        framebuffer.clear();

        // 2. draw the maze, passing the maze and block size
        render_maze(&mut framebuffer, &maze, block_size,&player);

        // 3. swap buffers
        framebuffer.swap_buffers(&mut window, &raylib_thread);

        thread::sleep(Duration::from_millis(16));
    }
}
