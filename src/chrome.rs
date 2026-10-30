// Chrome nativo de la ventana: materiales y detalles del sistema operativo.
//
// En Windows 11 el fondo de la barra de título (y de la ventana, si el cliente es
// transparente) lo pinta DWM con el material Mica: el wallpaper del escritorio
// difuminado y teñido. Sin esto, "Fluent" era sólo una paleta sobre un marco gris
// del sistema.
//
// El material se pide al crear la ventana; en Windows 10 la llamada no existe y
// winit la ignora, así que no se rompe nada.

#[cfg(target_os = "windows")]
use xilem::winit::platform::windows::Color as WinitColor;
use crate::theme::{DesignStyle, MaterialTheme};
use crate::WindowOptions;

/// Aplica el material y los detalles de la ventana según el diseño del tema.
///
/// - Fluent → Mica (`DWMSBT_MAINWINDOW`).
/// - Liquid Glass / Material You → sin material (pintan su propio fondo).
#[cfg(target_os = "windows")]
pub fn aplicar_chrome<S>(options: WindowOptions<S>, theme: &MaterialTheme) -> WindowOptions<S> {
    use xilem::winit::platform::windows::{BackdropType, CornerPreference};
    use xilem::WindowOptionsExtWindows;

    // El modo del tema vs. el del sistema decide quién manda en la barra:
    // - coinciden → DWM pinta la barra con su material (Mica en Fluent);
    // - no coinciden → se pinta con los colores del tema, porque si no una app
    //   clara queda con una barra oscura. Con backdrop activo DWM ignora los
    //   colores de la barra, así que ahí hay que apagar el material.
    let coincide_con_sistema = theme.is_dark == crate::theme::is_system_dark();
    let con_mica = theme.design == DesignStyle::Fluent && coincide_con_sistema;

    let options = options
        .with_corner_preference(CornerPreference::Round)
        .with_system_backdrop(if con_mica {
            BackdropType::MainWindow
        } else {
            BackdropType::None
        });
    // Borde de 1px con el contorno del tema (Win11 usa un gris fijo que en oscuro
    // queda demasiado claro). `Color` de winit es u32 en formato 0x00BBGGRR.
    let c = theme.scheme.outline_variant;
    let borde = WinitColor::from_rgb(c.0, c.1, c.2);
    let options = options.with_border_color(Some(borde));

    if !con_mica {
        let fondo = theme.scheme.background;
        let texto = theme.scheme.on_surface;
        return options
            .with_title_background_color(Some(WinitColor::from_rgb(fondo.0, fondo.1, fondo.2)))
            .with_title_text_color(WinitColor::from_rgb(texto.0, texto.1, texto.2));
    }
    options
}

#[cfg(not(target_os = "windows"))]
pub fn aplicar_chrome<S>(options: WindowOptions<S>, _theme: &MaterialTheme) -> WindowOptions<S> {
    options
}

/// `true` si la plataforma pinta el fondo con un material del sistema (Mica).
pub fn usa_material_del_sistema(theme: &MaterialTheme) -> bool {
    cfg!(target_os = "windows") && theme.design == DesignStyle::Fluent
}

/// Fondo de la página. Con Mica, el material tiene que verse: la página se pinta
/// con el color de superficie pero translúcido, y las tarjetas encima con el
/// relleno de capa de Fluent.
pub fn fondo_pagina(theme: &MaterialTheme) -> xilem::Color {
    let s = &theme.scheme;
    if usa_material_del_sistema(theme) {
        // Fluent deja traslucir Mica: `SolidBackgroundFillColorBase` al 50%.
        s.background.with_alpha(0.55)
    } else {
        s.background.into()
    }
}
