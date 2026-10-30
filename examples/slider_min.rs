// Diagnóstico: ¿el slider de Xilem repinta mientras se arrastra?
//
// Ejecutar: cargo run --example slider_min -p forja-gui-rt
//
// Si el thumb sigue al mouse acá, el problema está en el árbol de Forja
// (memoize / reconstrucción). Si no, está en el widget de Xilem.

use forja_gui_rt::layout::AsUnit as _;
use forja_gui_rt::{view, AnyWidgetView, EventLoop, Length, LogicalSize, WindowOptions, Xilem};
use xilem::style::Style;

#[derive(Default)]
struct Estado {
    valor: f64,
}

fn main() -> Result<(), String> {
    let estado = Estado { valor: 30.0 };

    let app = Xilem::new_simple(
        estado,
        |estado: &mut Estado| -> Box<AnyWidgetView<Estado>> {
            let valor = estado.valor;
            Box::new(
                view::flex(
                    forja_gui_rt::Axis::Vertical,
                    (
                        view::label(format!("valor = {:.1}", valor)).text_size(28.0),
                        view::slider(0.0, 100.0, valor, |estado: &mut Estado, v: f64| {
                            estado.valor = v;
                        }),
                    ),
                )
                .gap(Length::px(24.0))
                .padding(32.0f64.px()),
            )
        },
        WindowOptions::new("Slider mínimo").with_initial_inner_size(LogicalSize::new(700.0, 260.0)),
    );

    app.run_in(EventLoop::with_user_event())
        .map_err(|e| format!("Error en GUI: {}", e))
}
