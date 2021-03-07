use super::{rescale_value_to, PaletteColorize};

pub struct DefaultPalette {}
impl PaletteColorize for DefaultPalette {
    fn get_color(&self, value: f32) -> [u8; 3] {
        let value = rescale_value_to(value, 0.0, 255.0) as u8;
        [value, value, 50]
    }
    fn get_color_under_range(&self) -> [u8; 3] {
        [0, 0, 0]
    }
    fn get_color_over_range(&self) -> [u8; 3] {
        [255, 255, 255]
    }
}
#[cfg(test)]
mod tests {

    #[test]
    fn dummy() {
        assert_eq!(4, 2 + 2);
    }
}
