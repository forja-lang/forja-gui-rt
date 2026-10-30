// Forja GUI — Widget QR Code con renderizado Vello
//
// Genera códigos QR usando el crate `qrcode` y los renderiza
// como módulos negros usando Vello Scene.fill().

use crate::ViewCtx;
use std::marker::PhantomData;
use xilem::core::{MessageResult, Mut, View, ViewMarker};
use xilem::masonry::core::Widget;
use xilem::masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef, RegisterCtx,
    UpdateCtx,
};
use xilem::masonry::imaging::Painter;
use xilem::masonry::kurbo::{Axis, Rect, Size};
use xilem::masonry::layout::{LenReq, Length};
use xilem::masonry::peniko;

// ═══════════════════════════════════════════════════════════════════
// QRWidget — raw widget Masonry que pinta un código QR
// ═══════════════════════════════════════════════════════════════════

/// Widget Masonry personalizado que renderiza un código QR
/// usando Vello para dibujar los módulos negros sobre fondo blanco.
pub struct QRWidget {
    /// Texto a codificar
    text: String,
    /// Tamaño total del widget en píxeles lógicos
    size: f64,
    /// Módulos del QR: true = negro, false = blanco
    modules: Vec<bool>,
    /// El QR es una matriz module_count × module_count
    module_count: usize,
}

impl QRWidget {
    /// Crea un nuevo QRWidget que codifica `text` en un QR de `size` píxeles.
    ///
    /// Usa nivel de corrección de errores M (~15%) por defecto.
    pub fn new(text: String, size: f64) -> Self {
        let size = size.max(50.0); // mínimo 50px

        // Generar QR con corrección de errores nivel M
        let code = match qrcode::QrCode::with_error_correction_level(
            text.as_bytes(),
            qrcode::EcLevel::M,
        ) {
            Ok(code) => code,
            Err(_) => {
                // Si falla (texto muy largo), usar nivel L
                qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::L)
                    .unwrap_or_else(|_| {
                        // Si sigue fallando, crear un QR dummy
                        qrcode::QrCode::new(b"error").unwrap()
                    })
            }
        };

        let module_count = code.width();
        // qrcode 0.14 usa into_colors() para obtener Vec<Color>
        // Donde Color::Dark = true (módulo negro), Color::Light = false
        let modules: Vec<bool> = code
            .into_colors()
            .iter()
            .map(|c| *c == qrcode::types::Color::Dark)
            .collect();

        Self {
            text,
            size,
            modules,
            module_count,
        }
    }

    /// Actualiza el texto del QR y regenera los módulos
    pub fn set_text(&mut self, text: String) {
        if self.text == text {
            return;
        }
        self.text = text;
        let code =
            qrcode::QrCode::with_error_correction_level(self.text.as_bytes(), qrcode::EcLevel::M)
                .unwrap_or_else(|_| {
                    qrcode::QrCode::with_error_correction_level(
                        self.text.as_bytes(),
                        qrcode::EcLevel::L,
                    )
                    .unwrap_or_else(|_| qrcode::QrCode::new(b"error").unwrap())
                });
        self.module_count = code.width();
        self.modules = code
            .into_colors()
            .iter()
            .map(|c| *c == qrcode::types::Color::Dark)
            .collect();
    }
}

impl Widget for QRWidget {
    type Action = NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn property_changed(&mut self, _ctx: &mut UpdateCtx<'_>, _property_type: std::any::TypeId) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        // El QR tiene tamaño propio fijo: lo respeta, pero se achica si el
        // espacio disponible es menor (equivalente al `bc.constrain` anterior).
        let fijo = Length::px(self.size);
        match len_req {
            LenReq::FitContent(space) => fijo.min(space),
            _ => fijo,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        if self.modules.is_empty() || self.module_count == 0 {
            return;
        }

        let cell_size = self.size / self.module_count as f64;

        // Fondo blanco
        let bg_rect = Rect::new(0.0, 0.0, self.size, self.size);
        let white = peniko::Color::from_rgba8(255, 255, 255, 255);
        painter.fill_rect(bg_rect, white);

        // Módulos negros
        let black = peniko::Color::from_rgba8(0, 0, 0, 255);

        // Añadir margen de seguridad (quiet zone) de 4 módulos
        let quiet_zone = 4.0 * cell_size;

        for (i, module) in self.modules.iter().enumerate() {
            if *module {
                let row = i / self.module_count;
                let col = i % self.module_count;
                let x = quiet_zone + col as f64 * cell_size;
                let y = quiet_zone + row as f64 * cell_size;
                let rect = Rect::new(x, y, x + cell_size, y + cell_size);
                painter.fill_rect(rect, black);
            }
        }
    }

    fn accessibility_role(&self) -> xilem::masonry::accesskit::Role {
        xilem::masonry::accesskit::Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut xilem::masonry::accesskit::Node,
    ) {
        node.set_label(format!("Código QR: {}", self.text));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

// ═══════════════════════════════════════════════════════════════════
// QRView — wrapper Xilem View para integrar QRWidget
// ═══════════════════════════════════════════════════════════════════

/// Wrapper Xilem View que crea un QRWidget.
///
/// Sigue el patrón de SvgIconView: implementa View<State, (), ViewCtx>
/// con Element = Pod<QRWidget>.
pub struct QRView<State> {
    text: String,
    size: f64,
    phantom: PhantomData<fn() -> State>,
}

/// Crea una vista de código QR
pub fn qr_view<State: 'static>(text: &str, size: f64) -> QRView<State> {
    QRView {
        text: text.to_string(),
        size,
        phantom: PhantomData,
    }
}

impl<State: 'static> ViewMarker for QRView<State> {}

impl<State: 'static> View<State, (), ViewCtx> for QRView<State> {
    type Element = xilem::Pod<QRWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (xilem::Pod<QRWidget>, ()) {
        let widget = QRWidget::new(self.text.clone(), self.size);
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, xilem::Pod<QRWidget>>,
        _app_state: &mut State,
    ) {
        if self.text != prev.text || (self.size - prev.size).abs() > 0.5 {
            // Reconstrucción completa manejada por Xilem
        }
    }

    fn teardown(
        &self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, xilem::Pod<QRWidget>>,
    ) {
    }

    fn message(
        &self,
        _view_state: &mut (),
        _message: &mut crate::MessageContext,
        _element: Mut<'_, xilem::Pod<QRWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}
