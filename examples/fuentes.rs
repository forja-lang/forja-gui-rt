// Diagnóstico de fuentes: qué family resuelve cada `FontFamily` en este sistema.
//
// Ejecutar: cargo run --example fuentes -p forja-gui-rt
//
// Compara la fuente por defecto (SystemUi) contra familias nombradas. Sirve para
// saber si un tema puede pedir "Segoe UI" de verdad o si hay que empaquetarla.

use forja_gui_rt::layout::AsUnit as _;
use forja_gui_rt::{view, AnyWidgetView, EventLoop, Length, LogicalSize, WindowOptions, Xilem};
use xilem::masonry::parley::{FontFamily, FontFamilyName, GenericFamily};
use xilem::style::Style;

const TEXTO: &str = "Galería de componentes 2026 Agjl 123";

fn etiqueta(_nombre: &str, familia: FontFamily<'static>) -> Box<AnyWidgetView<()>> {
    Box::new(
        view::label(TEXTO.to_string())
            .text_size(20.0)
            .weight(forja_gui_rt::FontWeight::BOLD)
            .font(familia),
    )
}

fn familia(nombre: &str) -> FontFamily<'static> {
    FontFamily::Single(FontFamilyName::Named(nombre.to_string().into()))
}

fn main() -> Result<(), String> {
    let app = Xilem::new_simple(
        (),
        move |_state: &mut ()| -> Box<AnyWidgetView<()>> {
            // Sólo el texto de muestra: así todas las filas se pueden comparar
            // pixel a pixel (misma cadena, mismo origen).
            let filas: Vec<Box<AnyWidgetView<()>>> = vec![
                etiqueta(
                    "",
                    FontFamily::Single(FontFamilyName::Generic(GenericFamily::SystemUi)),
                ),
                etiqueta("", familia("Segoe UI")),
                etiqueta("", familia("Segoe UI Variable Text")),
                etiqueta("", familia("Roboto")),
                etiqueta("", familia("Open Sans")),
                etiqueta("", familia("NoExisteEstaFuenteXYZ")),
            ];
            Box::new(
                view::flex(forja_gui_rt::Axis::Vertical, (filas,))
                    .gap(Length::px(16.0))
                    .padding(24.0f64.px()),
            )
        },
        WindowOptions::new("Diagnostico de fuentes")
            .with_initial_inner_size(LogicalSize::new(920.0, 400.0)),
    );

    app.run_in(EventLoop::with_user_event())
        .map_err(|e| format!("Error en GUI: {}", e))
}
