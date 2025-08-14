use raylib::prelude::*;
use crate::textures::TextureManager;
use crate::player::Player;
use crate::maze::Maze;

pub struct Enemy {
    pub pos: Vector2,
    pub texture_key: char,
}

impl Enemy {
    pub fn new(x: f32, y: f32, texture_key: char) -> Self {
        Enemy { pos: Vector2::new(x, y), texture_key }
    }
    
    /// block_size: tamaño de celda en píxeles (ej. 100)
    pub fn update(&mut self, player: &Player, maze: &Maze, block_size: usize, speed: f32) {
        // vector hacia el jugador
        let mut dx = player.pos.x - self.pos.x;
        let mut dy = player.pos.y - self.pos.y;
        let dist = (dx*dx + dy*dy).sqrt();

        // si está muy cerca no moverse
        if dist < 1.0 { return; }

        // normalizar y escalar por speed
        dx = dx / dist * speed;
        dy = dy / dist * speed;

        // Intentar mover en X (permite sliding)
        let new_x = self.pos.x + dx;
        let new_y_for_x = self.pos.y;
        if Self::is_free(new_x, new_y_for_x, maze, block_size) {
            self.pos.x = new_x;
        }

        // Intentar mover en Y
        let new_y = self.pos.y + dy;
        let new_x_for_y = self.pos.x;
        if Self::is_free(new_x_for_y, new_y, maze, block_size) {
            self.pos.y = new_y;
        }
    }

    /// Comprueba si la posición (px,py) cae en una celda transitable (' ' o 'g').
    fn is_free(px: f32, py: f32, maze: &Maze, block_size: usize) -> bool {
        if px < 0.0 || py < 0.0 { return false; } // fuera -> tratar como pared
        let i = (px / block_size as f32).floor() as usize;
        let j = (py / block_size as f32).floor() as usize;

        if j >= maze.len() { return false; }
        if maze.get(j).map_or(true, |row| i >= row.len()) { return false; }

        match maze[j][i] {
            ' ' | 'g' => true,
            _ => false,
        }
    }
}
