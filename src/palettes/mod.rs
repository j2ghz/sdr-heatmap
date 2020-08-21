use log::*;
mod default;
mod extended;

#[derive(Copy, Clone)]
pub enum Palette {
    Default,
    Extended,
}

impl Palette {
    fn instance(&self) -> Box<dyn PaletteColorize> {
        match self {
            Palette::Default => Box::from(default::DefaultPalette {}),
            Palette::Extended => Box::from(extended::ExtendedPalette {}),
        }
    }
}

trait PaletteColorize {
    fn get_color(&self, value: f32) -> [u8; 3];
    fn get_color_under_range(&self) -> [u8; 3];
    fn get_color_over_range(&self) -> [u8; 3];
}

/// Scale a value from between min and max to between 0 and 1
fn rescale_value_from(value: f32, min: f32, max: f32) -> f32 {
    let old_range = max - min;
    (value - min) / old_range
}

/// Scale a value from between 0 and 1 to between min and max
fn rescale_value_to(value: f32, min: f32, max: f32) -> f32 {
    value.mul_add(max, min)
}

/// Places value on a scale from min to max, and transforms it to an integer scale from 0 to 255. Returns a color using the specified palette.
pub fn scale_tocolor(palette: Palette, value: f32, min: f32, max: f32) -> [u8; 3] {
    let scaled = rescale_value_from(value, min, max);
    let palette = palette.instance();
    if scaled < 0.0 {
        warn!("Computed invalid color! Value range: {} to {}, Value: {}, Color range: 0-255, Color: {}", min,max,value,scaled);
        palette.get_color_under_range()
    } else if scaled > 1.0 {
        warn!("Computed invalid color! Value range: {} to {}, Value: {}, Color range: 0-255, Color: {}", min,max,value,scaled);
        palette.get_color_over_range()
    } else {
        palette.get_color(scaled)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use proptest::prelude::*;
    #[test]
    fn normalize_goes_up() {
        assert_eq!(
            (0..255)
                .map(|v| v as f32)
                .map(|v| scale_tocolor(Palette::Default, v, 0.0, 255.0)
                    .first()
                    .cloned()
                    .unwrap())
                .collect::<Vec<_>>(),
            (0..255).map(|v| v as u8).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scale_default() {
        assert_eq!(
            scale_tocolor(Palette::Default, 23.02, -29.4, 23.02),
            [255, 255, 50]
        );
    }
    #[test]
    fn scale_over_under() {
        assert_eq!(
            scale_tocolor(Palette::Default, f32::INFINITY, 0.0, 1.0),
            [255, 255, 255]
        );
        assert_eq!(
            scale_tocolor(Palette::Default, f32::NEG_INFINITY, 0.0, 1.0),
            [0, 0, 0]
        );
        assert_eq!(
            scale_tocolor(Palette::Extended, f32::INFINITY, 0.0, 1.0),
            [255, 255, 255]
        );
        assert_eq!(
            scale_tocolor(Palette::Extended, f32::NEG_INFINITY, 0.0, 1.0),
            [0, 0, 0]
        );
    }
    proptest! {
        #[test]
        fn scale_tocolor_within_bounds(
            a in proptest::num::f32::ANY,
            b in proptest::num::f32::ANY,
            c in proptest::num::f32::ANY)
          {
            let min = a.min(b).min(c);
            let mid = a.min(b).max( a.max(b).min(c));
            let max = a.max(b).max(c);
            scale_tocolor(Palette::Default,mid,min,max);
            scale_tocolor(Palette::Extended,mid,min,max);
        }
    }
}
