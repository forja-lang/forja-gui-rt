// Forja GUI — Helper de sombra de elevación para Masonry BoxShadow
//
// Convierte la configuración de Shadow del ElevationSystem al tipo BoxShadow
// nativo de Masonry, que se puede usar con el método .box_shadow() en Xilem.

use crate::theme::elevation::Shadow;
use xilem::masonry::kurbo::Point;
use xilem::masonry::properties::BoxShadow;
use xilem::masonry::vello::peniko::color::AlphaColor;

/// Convierte un Shadow de ElevationSystem al BoxShadow nativo de Masonry.
///
/// # Ejemplo
/// ```ignore
/// let shadow = theme.elevation.shadow_for_level(2);
/// let box_shadow = shadow_to_box_shadow(&shadow);
/// view::sized_box(child).box_shadow(box_shadow)
/// ```
pub fn shadow_to_box_shadow(shadow: &Shadow) -> BoxShadow {
    let color = AlphaColor::from_rgba8(
        shadow.color.0,
        shadow.color.1,
        shadow.color.2,
        shadow.color.3,
    );
    BoxShadow::new(color, Point::new(shadow.offset_x, shadow.offset_y)).blur(shadow.blur_radius)
}

/// Versión simplificada que crea un BoxShadow solo con blur y color,
/// sin offset (adecuado para sombras interiores o glows).
pub fn glow_box_shadow(color: (u8, u8, u8, u8), blur_radius: f64) -> BoxShadow {
    let alpha_color = AlphaColor::from_rgba8(color.0, color.1, color.2, color.3);
    BoxShadow::new(alpha_color, Point::ZERO).blur(blur_radius)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::elevation::ElevationSystem;

    #[test]
    fn test_shadow_to_box_shadow() {
        let elevation = ElevationSystem::default();
        let shadow = elevation.shadow_for_level(2);
        let bs = shadow_to_box_shadow(&shadow);
        // Verificar que se crea correctamente
        assert!(
            bs.blur_radius > 0.0,
            "blur_radius should be > 0 for level 2"
        );
    }

    #[test]
    fn test_glow_box_shadow() {
        let glow = glow_box_shadow((0, 0, 0, 100), 5.0);
        assert!((glow.blur_radius - 5.0).abs() < 0.001);
    }
}
