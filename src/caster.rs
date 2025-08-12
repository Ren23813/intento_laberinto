use std::hint;

use raylib::color::Color;

use crate::framebuffer::Framebuffer;
use crate::player::Player;
use crate::maze::Maze;

pub struct Intersect {
    pub distance: f32,
    pub impact:char,
    pub tx: usize,
}

pub fn cast_ray(framebuffer: &mut Framebuffer, maze: &Maze, player: &Player, a: f32, block_size: usize, draw_line: bool) -> Intersect {
    let mut d = 0.0;
    let maze_height = maze.len();
    let maze_width = if maze_height > 0 { maze[0].len() } else { 0 };

    framebuffer.set_current_color(Color::WHITE);

    loop {
        let cos = d * a.cos();
        let sin = d * a.sin();
        let x = (player.pos.x + cos) as usize;
        let y = (player.pos.y + sin) as usize;

        let i = x / block_size;
        let j = y / block_size;
        

        // Verificar si el rayo se sale de los límites del laberinto
        if j >= maze_height || i >= maze_width {
        let hitx = x - i*block_size;
        let hity = y - j * block_size;
        let mut maxhit = hity; 

        if 1 <hitx && hitx < block_size -1 {
            maxhit = hitx;
        }
        let tx = (maxhit  * 128 )/block_size;

            return Intersect {
                distance: d,
                impact: ' ',  // Rayo sale del laberinto
                tx:tx
            };
        }

        if maze[j][i] != ' ' {
        let hitx = x - i*block_size;
        let hity = y - j * block_size;
        let mut maxhit = hity; 

        if 1 <hitx && hitx < block_size -1 {
            maxhit = hitx;
        }
        let tx = (maxhit  * 128) /block_size;

            return Intersect {
                distance: d,
                impact: maze[j][i],  // Pared encontrada
                tx:tx
            };
        }

        if draw_line {
            framebuffer.set_pixel(x as i32, y as i32);
        }

        d += 1.0;

        // Evitar que el rayo recorra una distancia demasiado larga
        if d > 2000.0 {
            break;
        }
    }

    // Si no se encuentra nada, retornar un impacto vacío
    Intersect {
        distance: d,
        impact: ' ', // ''
        tx: 0
    }
}
 