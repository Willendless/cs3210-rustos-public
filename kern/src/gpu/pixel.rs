#[derive(Copy, Clone)]
pub struct Pixel {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Pixel {
    fn new(red: u8, green: u8, blue: u8) -> Pixel {
        Pixel {
            red,
            green,
            blue,
        }
    }
}

#[allow(dead_code)]
pub const WHITE: Pixel = Pixel {
    red: 0xff, 
    green: 0xff,
    blue: 0xff
};
pub const BLACK: Pixel = Pixel {
    red: 0x0,
    green: 0x0,
    blue: 0x0,
};
pub const RED: Pixel = Pixel {
    red: 0xff,
    green: 0,
    blue: 0
};
pub const GREEN: Pixel = Pixel {
    red: 0,
    green: 0xff,
    blue: 0
};
pub const BLUE: Pixel = Pixel {
    red: 0,
    green: 0xff,
    blue: 0xff
};
