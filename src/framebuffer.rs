use raylib::prelude::*;

pub struct Framebuffer {
    pub width: i32,
    pub height: i32,
    pub color_buffer: Image,
    background_color: Color,
    current_color: Color,
    pixel_data: Vec<Color>,
    overlays: Vec<(String, i32, i32, i32, Color)>,
    reuse_texture: Option<Texture2D>,

    circle_overlays: Vec<(i32, i32, f32, f32)>,
    mask_texture: Option<Texture2D>,
    health_to_draw: Option<(i32, i32)>, // (current_lives, max_lives)

}

impl Framebuffer {
    pub fn new(width: i32, height: i32, background_color: Color) -> Self {
        let size = (width * height) as usize;
        let pixel_data = vec![background_color; size];
        let color_buffer = Image::gen_image_color(width, height, background_color);
        Framebuffer {
            width,
            height,
            color_buffer,
            background_color,
            current_color: Color::WHITE,
            pixel_data,
            overlays: Vec::new(),
            reuse_texture: None,
            circle_overlays: Vec::new(),
            mask_texture: None,
            health_to_draw:None
        }
    }

    pub fn clear(&mut self) {
        self.pixel_data.fill(self.background_color);
        self.color_buffer =
            Image::gen_image_color(self.width, self.height, self.background_color)
    }

    pub fn set_pixel(&mut self, x: i32, y: i32) {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            let index = (y * self.width + x) as usize;
            self.pixel_data[index] = self.current_color;
            Image::draw_pixel(&mut self.color_buffer, x as i32, y as i32, self.current_color);
        }
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
        self.clear();
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    pub fn render_to_file(&self, file_path: &str) {
        Image::export_image(&self.color_buffer, file_path);
    }

    /// Añade un overlay de texto que se dibujará en el próximo `swap_buffers`.
    /// Parámetros: texto, x, y, font_size, color
    pub fn draw_text(&mut self, text: &str, x: i32, y: i32, font_size: i32, color: Color) {
        self.overlays.push((text.to_string(), x, y, font_size, color));
    }

    /// Añade un overlay de "vignette" circular: fuera del radio estará oscuro.
    /// center in screen coords (px), max_radius en px, darkness en [0.0..1.0].
    /// Llamar antes de swap_buffers cada frame (se limpia después de dibujar).
    pub fn draw_vignette(&mut self, center_x: i32, center_y: i32, max_radius: f32, darkness: f32) {
        self.circle_overlays.push((center_x, center_y, max_radius, darkness.max(0.0).min(1.0)));
    }

pub fn swap_buffers(&mut self, window: &mut RaylibHandle, raylib_thread: &RaylibThread) {
    // 1) actualizar o crear la textura que contiene el framebuffer
    if let Some(texture) = &mut self.reuse_texture {
        let byte_len = (self.width * self.height * 4) as usize;
        let pixels: &[u8] = unsafe {
            std::slice::from_raw_parts(self.color_buffer.data as *const u8, byte_len)
        };
        texture.update_texture(pixels).unwrap();
    } else {
        let texture = window
            .load_texture_from_image(raylib_thread, &self.color_buffer)
            .expect("Failed to create reuse_texture");
        self.reuse_texture = Some(texture);
    }

    // 2) si hay overlays de máscara, aseguramos la mask_texture ANTES de entrar a begin_drawing
    if !self.circle_overlays.is_empty() {
        self.ensure_mask_texture(window, raylib_thread);
    }

    // 3) comenzar a dibujar
    if let Some(texture) = &self.reuse_texture {
        let mut d = window.begin_drawing(raylib_thread);

        // dibujar framebuffer (pantalla principal)
        d.draw_texture(texture, 0, 0, Color::WHITE);

        // dibujar overlays de texto
        for (text, x, y, font_size, color) in &self.overlays {
            d.draw_text(text, *x, *y, *font_size, *color);
        }

        // 4) dibujar máscaras (linterna / oscuridad) usando la mask_texture escalada y recortada
        if let Some(mask_tex) = &self.mask_texture {
            // tamaño de pantalla (dest full-screen)
            let dest_w = self.width as f32;
            let dest_h = self.height as f32;

            // Tamaño original de la textura de máscara (asumimos cuadrada)
            let mask_w = mask_tex.width() as f32;
            let mask_h = mask_tex.height() as f32;

            // FRACTION que define el "radio claro de referencia" dentro de la máscara.
            // Si quieres que el hueco pueda llegar a ser muy pequeño en pantalla,
            // reduce CLEAR_FRAC (ej. 0.05). Si quieres hueco más grande, aumenta.
            const CLEAR_FRAC: f32 = 0.08; // 0.05..0.15 es rango razonable

            for (cx, cy, max_r, darkness) in &self.circle_overlays {
                // si darkness == 0 no aplicamos
                if *darkness <= 0.0 {
                    continue;
                }

                // desired_radius = radio en píxeles que queremos que tenga el hueco claro en pantalla
                let desired_radius = (*max_r).max(1.0);

                // radio de referencia en la máscara (en píxeles): 
                // toma la mitad de la menor dimensión y multiplícalo por CLEAR_FRAC
                let mask_half = (mask_w.min(mask_h)) / 2.0;
                let ref_radius_in_mask = mask_half * CLEAR_FRAC;

                // fórmula: desired_radius = ref_radius_in_mask * (dest_w / src_w)
                // => src_w = ref_radius_in_mask * dest_w / desired_radius
                //
                // NOTA: con CLEAR_FRAC pequeño, src_w será razonable y no siempre se clampa.
                let mut src_w = (ref_radius_in_mask * dest_w) / desired_radius;

                // clamp src_w a rango util
                if src_w < 1.0 { src_w = 1.0; }
                if src_w > mask_w { src_w = mask_w; }

                // Calculamos rect fuente centrado en la textura de máscara
                let src_cx = mask_w / 2.0;
                let src_cy = mask_h / 2.0;
                let src_x = (src_cx - src_w / 2.0).max(0.0);
                let src_y = (src_cy - src_w / 2.0).max(0.0);

                let src = Rectangle::new(src_x, src_y, src_w, src_w);

                // Dibujamos la máscara ocupando toda la pantalla, pero desplazada para que
                // el punto (cx,cy) del hueco quede en la posición correcta relativa
                // dentro del dest (esto permite centrar la linterna en pantalla o sobre el enemigo).
                let dest_x = (*cx as f32) - (dest_w / 2.0);
                let dest_y = (*cy as f32) - (dest_h / 2.0);
                let dest = Rectangle::new(dest_x, dest_y, dest_w, dest_h);

                let origin = Vector2::new(0.0, 0.0);

                // tint: modulamos alpha por darkness (0..1)
                let tint_alpha = (darkness.clamp(0.0, 1.0) * 255.0).round() as u8;
                let tint = Color::new(255, 255, 255, tint_alpha);

                // draw call rápido (GPU)
                d.draw_texture_pro(mask_tex, src, dest, origin, 0.0, tint);
            }
        }

        // --- dibujar barras de vida ---
        if let Some((current_lives, max_lives)) = self.health_to_draw {
            // parámetros visuales
            let pad_left = 10_i32;
            let pad_bottom = 10_i32;
            let bar_w = 140_i32;
            let bar_h = 18_i32;
            let spacing = 8_i32;

            // dibujamos barras apiladas verticalmente en la esquina inferior izquierda
            for i in 0..max_lives {
                // i=0 -> barra inferior, i=1 -> barra encima, etc.
                let idx = i as i32;
                let x = pad_left;
                let y = (self.height - pad_bottom) - ((idx + 1) * (bar_h + spacing));

                // fondo de la barra (gris)
                d.draw_rectangle(x, y, bar_w, bar_h, Color::new(60, 60, 60, 200));

                // si la barra está "viva", dibujar en verde (o parcialmente rellenada)
                if i < current_lives {
                    d.draw_rectangle(x + 3, y + 3, bar_w - 6, bar_h - 6, Color::new(40, 200, 40, 255));
                } else {
                    // barra vacía: un tono más oscuro/rojo tenue
                    d.draw_rectangle(x + 3, y + 3, bar_w - 6, bar_h - 6, Color::new(120, 0, 0, 200));
                }
            }
        }
    }

    // 5) limpiar overlays ya dibujados
    self.overlays.clear();
    self.circle_overlays.clear();
    self.health_to_draw = None;
}



    pub fn get_pixel_color(&self, x: i32, y: i32) -> Option<Color> {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            let index = (y * self.width + x) as usize;
            Some(self.pixel_data[index])
        } else {
            None
        }
    }

        /// Asegura que exista mask_texture; si no, la crea a partir de una Image radial.
    /// mask_size: tamaño cuadrado de la máscara en px (p.ej. 512)
    fn ensure_mask_texture(&mut self, window: &mut RaylibHandle, raylib_thread: &RaylibThread) {
        if self.mask_texture.is_some() {
            return;
        }

        // Tamaño fijo de la máscara (cuadrada). 512 o 1024 según VRAM y suavidad.
        const MASK_SIZE: i32 = 512;
        let mut img = Image::gen_image_color(MASK_SIZE, MASK_SIZE, Color::new(0,0,0,0));

        // centro y radio máximo en la imagen de máscara
        let cx = (MASK_SIZE / 2) as f32;
        let cy = (MASK_SIZE / 2) as f32;
        let max_r = (MASK_SIZE as f32) / 2.0;

        // Factor gamma para el gradiente (ajusta suavidad)
        let gamma = 1.0_f32;

        for y in 0..MASK_SIZE {
            for x in 0..MASK_SIZE {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let dist = (dx*dx + dy*dy).sqrt();
                let t = (dist / max_r).min(1.0); // 0 centro, 1 borde
                // máscara alpha: 0 en centro, 1 en borde -> queremos alpha proporcional a t^gamma
                let mask_alpha_f = t.powf(gamma);
                let mask_alpha = (mask_alpha_f * 255.0).round().clamp(0.0, 255.0) as u8;

                // Color negro con alpha = mask_alpha
                let px_color = Color::new(0, 0, 0, mask_alpha);
                Image::draw_pixel(&mut img, x, y, px_color);
            }
        }

        // Crear textura GPU desde la image
        if let Ok(tex) = window.load_texture_from_image(raylib_thread, &img) {
            self.mask_texture = Some(tex);
            // liberamos la image (img sale de scope)
        } else {
            // Si falla, dejamos mask_texture = None y no usaremos la máscara
            eprintln!("Failed to create mask_texture from image");
        }
    }

     pub fn queue_health(&mut self, current: i32, max: i32) {
        self.health_to_draw = Some((current, max));
    }
    
}
