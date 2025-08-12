use raylib::prelude::*;
use std::collections::HashMap;
use std::slice;

pub struct TextureManager {
    // Guardamos los colores ya decodificados por Raylib, más width/height
    images: HashMap<char, (Vec<Color>, i32, i32)>,
    textures: HashMap<char, Texture2D>, // GPU textures para dibujar
}

impl TextureManager {
    pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let mut images = HashMap::new();
        let mut textures = HashMap::new();


        let texture_files = vec![
            ('+', "assets/pared.png"),
            ('|', "assets/pared.png"),
            ('g', "assets/wallPaint.png"),
            ('f', "assets/alfombraCIT.png"),
            ('e', "assets/jack1.png")
        ];

        for (ch, path) in texture_files {
            match Image::load_image(path) {
                Ok(mut image) => {
                    println!("Cargada imagen para '{}': {} ({}x{})", ch, path, image.width, image.height);

                    // Load texture GPU (lo tuyo)
                    let texture = rl.load_texture(thread, path).expect(&format!("Failed to load texture {}", path));

                    textures.insert(ch, texture);

          
                    let w = image.width;
                    let h = image.height;
                    unsafe {
                    let colors_ptr: *mut raylib::ffi::Color =
                        raylib::ffi::LoadImageColors(*image.as_ref());

                    let len = (w as usize) * (h as usize);
                    let slice = std::slice::from_raw_parts(colors_ptr, len);
                    let colors_vec: Vec<Color> = slice
                        .iter()
                        .map(|c| Color { r: c.r, g: c.g, b: c.b, a: c.a })
                        .collect();
                    raylib::ffi::UnloadImageColors(colors_ptr);

                    images.insert(ch, (colors_vec, w, h));
                }


                }
                Err(e) => {
                    eprintln!("Error al cargar imagen para '{}': {}. Error: {}", ch, path, e);
                }
            }
        }

        TextureManager { images, textures }
    }

    pub fn get_pixel_color(&self, ch: char, tx: u32, ty: u32) -> Color {
        if let Some((colors, w, h)) = self.images.get(&ch) {
            let max_x = ( (*w as u32).saturating_sub(1) ) as usize;
            let max_y = ( (*h as u32).saturating_sub(1) ) as usize;
            let x = (tx as usize).min(max_x);
            let y = (ty as usize).min(max_y);
            let idx = y * (*w as usize) + x;
            // colors[idx] es Color (raylib::prelude::Color) y es Copy
            colors[idx]
        } else {
            Color::WHITE
        }
    }

    pub fn get_texture(&self, ch: char) -> Option<&Texture2D> {
        self.textures.get(&ch)
    }
}

