use crate::FRAMEBUFFER;
use crate::gpu::framebuffer::*;
use crate::gpu::pixel::*;

pub fn gpu_putc(byte: u8) {

}

pub fn draw_screen(color: &str) {
    let color = match color.to_lowercase().as_str() {
        "white" => WHITE,
        "black" => BLACK,
        "red" => RED,
        "green" => GREEN,
        "blue" => BLUE,
        _ => WHITE
    };
    for x in 0.. WIDTH {
        for y in 0..HEIGHT {
            FRAMEBUFFER.write_pixel(x, y, color);
        }
    }
}

pub fn draw_line(x1: u32, y1: u32, x2: u32, y2: u32) {

}
