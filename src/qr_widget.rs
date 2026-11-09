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
    texto: String,
    /// Tamaño total del widget en píxeles lógicos
    tamano: f64,
    /// Módulos del QR: true = negro, false = blanco
    modules: Vec<bool>,
    /// El QR es una matriz module_count × module_count
    modulo_cantidad: usize,
}

impl QRWidget {
    /// Crea un nuevo QRWidget que codifica `text` en un QR de `size` píxeles.
    ///
    /// Usa nivel de corrección de errores M (~15%) por defecto.
    pub fn nuevo(texto: String, tamano: f64) -> Self {
        let tamano = tamano.max(50.0); // mínimo 50px

        // Generar QR con corrección de errores nivel M
        let code =
            match qrcode::QrCode::with_error_correction_level(texto.as_bytes(), qrcode::EcLevel::M)
            {
                Ok(code) => code,
                Err(_) => {
                    // Si falla (texto muy largo), usar nivel L
                    qrcode::QrCode::with_error_correction_level(
                        texto.as_bytes(),
                        qrcode::EcLevel::L,
                    )
                    .unwrap_or_else(|_| {
                        // Si sigue fallando, crear un QR dummy
                        qrcode::QrCode::new(b"error").unwrap()
                    })
                }
            };

        let modulo_cantidad = code.width();
        // qrcode 0.14 usa into_colors() para obtener Vec<Color>
        // Donde Color::Dark = true (módulo negro), Color::Light = false
        let modules: Vec<bool> = code
            .into_colors()
            .iter()
            .map(|c| *c == qrcode::types::Color::Dark)
            .collect();

        Self {
            texto,
            tamano,
            modules,
            modulo_cantidad,
        }
    }

    /// Actualiza el texto del QR y regenera los módulos
    pub fn fijar_texto(&mut self, texto: String) {
        if self.texto == texto {
            return;
        }
        self.texto = texto;
        let code =
            qrcode::QrCode::with_error_correction_level(self.texto.as_bytes(), qrcode::EcLevel::M)
                .unwrap_or_else(|_| {
                    qrcode::QrCode::with_error_correction_level(
                        self.texto.as_bytes(),
                        qrcode::EcLevel::L,
                    )
                    .unwrap_or_else(|_| qrcode::QrCode::new(b"error").unwrap())
                });
        self.modulo_cantidad = code.width();
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
        longitud_req: LenReq,
        _cross_longitud: Option<Length>,
    ) -> Length {
        // El QR tiene tamaño propio fijo: lo respeta, pero se achica si el
        // espacio disponible es menor (equivalente al `bc.constrain` anterior).
        let fijo = Length::px(self.tamano);
        match longitud_req {
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
        if self.modules.is_empty() || self.modulo_cantidad == 0 {
            return;
        }

        let cell_tamano = self.tamano / self.modulo_cantidad as f64;

        // Fondo blanco
        let bg_rect = Rect::new(0.0, 0.0, self.tamano, self.tamano);
        let white = peniko::Color::from_rgba8(255, 255, 255, 255);
        painter.fill_rect(bg_rect, white);

        // Módulos negros
        let black = peniko::Color::from_rgba8(0, 0, 0, 255);

        // Añadir margen de seguridad (quiet zone) de 4 módulos
        let quiet_zone = 4.0 * cell_tamano;

        for (i, module) in self.modules.iter().enumerate() {
            if *module {
                let row = i / self.modulo_cantidad;
                let col = i % self.modulo_cantidad;
                let x = quiet_zone + col as f64 * cell_tamano;
                let y = quiet_zone + row as f64 * cell_tamano;
                let rect = Rect::new(x, y, x + cell_tamano, y + cell_tamano);
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
        nodo: &mut xilem::masonry::accesskit::Node,
    ) {
        nodo.set_label(format!("Código QR: {}", self.texto));
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
pub struct QRVista<State> {
    texto: String,
    tamano: f64,
    phantom: PhantomData<fn() -> State>,
}

/// Crea una vista de código QR
pub fn qr_vista<State: 'static>(texto: &str, tamano: f64) -> QRVista<State> {
    QRVista {
        texto: texto.to_string(),
        tamano,
        phantom: PhantomData,
    }
}

impl<State: 'static> ViewMarker for QRVista<State> {}

impl<State: 'static> View<State, (), ViewCtx> for QRVista<State> {
    type Element = xilem::Pod<QRWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (xilem::Pod<QRWidget>, ()) {
        let widget = QRWidget::nuevo(self.texto.clone(), self.tamano);
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        anterior: &Self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, xilem::Pod<QRWidget>>,
        _app_state: &mut State,
    ) {
        if self.texto != anterior.texto || (self.tamano - anterior.tamano).abs() > 0.5 {
            // Reconstrucción completa manejada por Xilem
        }
    }

    fn teardown(
        &self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, xilem::Pod<QRWidget>>,
    ) {
    }

    fn message(
        &self,
        _vista_state: &mut (),
        _message: &mut crate::MessageContext,
        _element: Mut<'_, xilem::Pod<QRWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}
