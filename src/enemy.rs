use raylib::prelude::*;
use crate::textures::TextureManager;

pub struct Enemy{
    pub pos:Vector2,
    pub texture_key: char
}

impl Enemy {
    pub fn new (x:f32, y:f32,texture_key:char) -> Self{
        Enemy { pos: (Vector2::new(x, y)), texture_key}
    }
}

