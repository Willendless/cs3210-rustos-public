use crate::FRAMEBUFFER;
use crate::gpu::framebuffer::*;
use crate::gpu::pixel::*;
use crate::gpu::font::{Bitmap, CHAR_WIDTH, CHAR_HEIGHT, get_char_bitmap};
use crate::console::{kprintln, kprint};

const MASKS: [u8; 8] = [
    1 << 7,
    1 << 6,
    1 << 5,
    1 << 4,
    1 << 3,
    1 << 2,
    1 << 1,
    1 << 0,
];

pub fn gpu_put_buf(buf: &[u8], color: &str, back_color: &str) {
    for ch in buf {
        gpu_putc(*ch, color, back_color);
    }
}

pub fn gpu_put_str(s: &str, color: &str, back_color: &str) {
    for ch in s.bytes() {
        gpu_putc(ch, color, back_color);
    }
}

pub fn gpu_putc(byte: u8, color: &str, back_color: &str) {
    let color = get_color_pixel(color);
    let back_color = get_color_pixel(back_color);
    let bm = get_char_bitmap(byte);
    let mut x = FRAMEBUFFER.get_voffset_x();
    let mut y = FRAMEBUFFER.get_voffset_y();

    // move everything up one row
    if y >= HEIGHT {
        for row in CHAR_HEIGHT..HEIGHT {
            for col in 0..WIDTH {
                let p = FRAMEBUFFER.get_pixel(col, row); 
                FRAMEBUFFER.write_pixel(col, row - CHAR_HEIGHT, p);
            }
        }
        for row in HEIGHT - CHAR_HEIGHT..HEIGHT {
            for col in 0..WIDTH {
                FRAMEBUFFER.write_pixel(col, row, back_color);
            }
        }
        y = HEIGHT - CHAR_HEIGHT;
    }

    if byte == b'\n' {
        FRAMEBUFFER.set_voffset_x(0);
        FRAMEBUFFER.set_voffset_y(y + CHAR_HEIGHT);
        return;
    }

    for (i, row) in bm.iter().enumerate() {
        let row = row.reverse_bits();
        for (j, mask) in MASKS.iter().enumerate() {
            let offx = j as u32;
            let offy = i as u32;
            if row & (*mask) > 0 {
                FRAMEBUFFER.write_pixel(x + offx, y + offy, color)
            } else {
                FRAMEBUFFER.write_pixel(x + offx, y + offy, back_color);
            }
        }
    }

    x += CHAR_WIDTH;
    if x >= WIDTH {
        x = 0;
        y += CHAR_HEIGHT;
    }
    FRAMEBUFFER.set_voffset_x(x);
    FRAMEBUFFER.set_voffset_y(y);
}

/// Fill the whole screen with a color.
/// Mainly for test purpose.
pub fn draw_screen(color: &str) {
    let color = get_color_pixel(color);
    for x in 0.. WIDTH {
        for y in 0..HEIGHT {
            FRAMEBUFFER.write_pixel(x, y, color);
        }
    }
}

fn get_color_pixel(color: &str) -> Pixel {
    match color.to_lowercase().as_str() {
        "white" => WHITE,
        "black" => BLACK,
        "red" => RED,
        "green" => GREEN,
        "blue" => BLUE,
        _ => WHITE
    }
}

// Draw a line using DDA algorithm
// pub fn draw_line_dda(x1: u32, y1: u32, x2: u32, y2: u32, color: &str) {
//     let dx = x2 - x1;
//     let dy = y2 - y1;
//     let steps = abs(dx).max(abs(dy));
//     let x_step = (dx as f64) / steps;
//     let y_step = dy / steps;
//     let color = get_color_pixel(color);
//     for _ in 0..steps {
//         FRAMBUFFER.write_pixel(x1 + x_step, y1 + y_step, color);
//         x1 += x_step;
//         y1 += y_step;
//     }
// }
