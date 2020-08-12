use super::{rescale_value_to, PaletteColorize};

pub struct ExtendedPalette {}
impl PaletteColorize for ExtendedPalette {
    fn get_color(&self, value: f32) -> [u8; 3] {
        let value = rescale_value_to(value, 0.0, 255.0);
                let whole = value as u8;
                let fract = value.fract();
                if fract < 0.33 {
                    [whole, whole, 50]
                } else if fract < 0.66 {
                    [whole, whole + 1, 50]
                } else {
                    [whole + 1, whole + 1, 50]
                }
    }
    fn get_color_under_range(&self) -> [u8; 3] {
        [0, 0, 0]
    }
    fn get_color_over_range(&self) -> [u8; 3] {
        [255, 255, 255]
    }
}
