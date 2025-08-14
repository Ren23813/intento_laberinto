use raylib::prelude::*;
use crate::player::Player;
use crate::maze::Maze;

pub struct Enemy {
    pub pos: Vector2,
    pub frames: Vec<char>,     
    pub current_frame: usize,
    pub step_counter: usize,
    pub steps_per_frame: usize,
}

impl Enemy {
    /// frames: lista de claves de textura; steps_per_frame: cuantos pasos (movimientos) para cambiar frame
    pub fn new(x: f32, y: f32, frames: Vec<char>, steps_per_frame: usize) -> Self {
        Enemy {
            pos: Vector2::new(x, y),
            frames,
            current_frame: 0,
            step_counter: 0,
            steps_per_frame: steps_per_frame.max(1),
        }
    }

    /// Devuelve la clave de textura actual
    pub fn current_key(&self) -> char {
        if self.frames.is_empty() { 'e' } else { self.frames[self.current_frame] }
    }

    /// Retorna true si se movió (para contar pasos).
    pub fn update(&mut self, player: &Player, maze: &Maze, block_size: usize, speed: f32) -> bool {
        // vector hacia el jugador
        let mut dx = player.pos.x - self.pos.x;
        let mut dy = player.pos.y - self.pos.y;
        let dist = (dx*dx + dy*dy).sqrt();

        // si está muy cerca no moverse ni animar
        if dist < 1.0 { return false; }

        // normalizar y escalar por speed
        dx = dx / dist * speed;
        dy = dy / dist * speed;

        let mut moved = false;

        // Intentar mover en X (permite sliding)
        let new_x = self.pos.x + dx;
        let new_y_for_x = self.pos.y;
        if Self::is_free(new_x, new_y_for_x, maze, block_size) {
            self.pos.x = new_x;
            moved = true;
        }

        // Intentar mover en Y
        let new_y = self.pos.y + dy;
        let new_x_for_y = self.pos.x;
        if Self::is_free(new_x_for_y, new_y, maze, block_size) {
            self.pos.y = new_y;
            moved = true;
        }

        // Si se movió, actualizar animación por pasos
        if moved {
            self.step_counter += 1;
            if self.step_counter >= self.steps_per_frame {
                self.step_counter = 0;
                // advance frame (wrap)
                if !self.frames.is_empty() {
                    self.current_frame = (self.current_frame + 1) % self.frames.len();
                }
            }
        }

        moved
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
