//! Estrella pintada con `imaging::Painter`.
//!
//! Nace de un pendiente de GUI: el rating usaba `view::label("★")` y la fuente por
//! defecto no trae los glifos `★`/`☆`, así que se veían cuadraditos (tofu). Dibujar
//! el path de la estrella no depende de la fuente y toma el color del tema.

use std::f64::consts::PI;
use std::marker::PhantomData;

use xilem::core::{MessageResult, Mut, View, ViewMarker};
use xilem::masonry::core::Widget;
use xilem::masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef, RegisterCtx,
    UpdateCtx,
};
use xilem::masonry::imaging::Painter;
use xilem::masonry::kurbo::{Axis, BezPath, Point, Size, Stroke};
use xilem::masonry::layout::{LenReq, Length};
use xilem::Pod;

use crate::Color;
use crate::ViewCtx;

/// Widget que pinta una estrella de 5 puntas, llena o sólo el contorno.
pub struct StarWidget {
    /// Lado del cuadrado que ocupa la estrella, en px lógicos.
    size: f64,
    color: Color,
    filled: bool,
}

impl StarWidget {
    /// Crea la estrella con el tamaño y color del tema.
    pub fn new(size: f64, color: Color, filled: bool) -> Self {
        Self {
            size,
            color,
            filled,
        }
    }

    /// Path de una estrella de 5 puntas centrada en `centro`, con radio `radio`.
    fn path(centro: Point, radio: f64) -> BezPath {
        // 10 vértices alternando radio externo e interno; el interno ~0.4 da la
        // proporción de las estrellas de Material.
        let mut camino = BezPath::new();
        for i in 0..10 {
            let angulo = -PI / 2.0 + i as f64 * PI / 5.0;
            let r = if i % 2 == 0 { radio } else { radio * 0.4 };
            let p = Point::new(centro.x + r * angulo.cos(), centro.y + r * angulo.sin());
            if i == 0 {
                camino.move_to(p);
            } else {
                camino.line_to(p);
            }
        }
        camino.close_path();
        camino
    }
}

impl Widget for StarWidget {
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
        // Tamaño propio fijo, achicado si el espacio disponible es menor.
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
        let centro = Point::new(self.size / 2.0, self.size / 2.0);
        // Un poco de margen para que el contorno no se corte en los bordes.
        let radio = self.size / 2.0 - 1.0;
        if radio <= 0.0 {
            return;
        }
        let camino = Self::path(centro, radio);
        if self.filled {
            painter.fill(camino, self.color).draw();
        } else {
            painter.stroke(camino, &Stroke::new(1.4), self.color).draw();
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
        node.set_label(if self.filled {
            "Estrella"
        } else {
            "Estrella vacía"
        });
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

/// Wrapper Xilem para [`StarWidget`].
pub struct StarIconView<State> {
    size: f64,
    color: Color,
    filled: bool,
    phantom: PhantomData<fn() -> State>,
}

/// Crea una vista de estrella (llena o vacía) del tamaño y color indicados.
pub fn estrella<State: 'static>(size: f64, color: Color, filled: bool) -> StarIconView<State> {
    StarIconView {
        size,
        color,
        filled,
        phantom: PhantomData,
    }
}

impl<State: 'static> ViewMarker for StarIconView<State> {}

impl<State: 'static> View<State, (), ViewCtx> for StarIconView<State> {
    type Element = Pod<StarWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<StarWidget>, ()) {
        let widget = StarWidget::new(self.size, self.color, self.filled);
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<StarWidget>>,
        _app_state: &mut State,
    ) {
        // Los cambios de color/relleno se resuelven reconstruyendo la vista desde
        // el layout del widget padre; acá no hay estado propio que sincronizar.
        let _ = (prev, self);
    }

    fn teardown(
        &self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<StarWidget>>,
    ) {
    }

    fn message(
        &self,
        _view_state: &mut (),
        _message: &mut crate::MessageContext,
        _element: Mut<'_, Pod<StarWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_path_tiene_10_vertices_y_cierra() {
        let camino = StarWidget::path(Point::new(12.0, 12.0), 10.0);
        let segmentos = camino.elements().len();
        // 10 vértices → 1 move_to + 9 line_to + cierre
        assert_eq!(segmentos, 11, "segmentos del path: {segmentos}");
    }

    #[test]
    fn el_path_entra_en_el_cuadrado() {
        use xilem::masonry::kurbo::Shape as _;
        let camino = StarWidget::path(Point::new(12.0, 12.0), 10.0);
        let bbox = camino.bounding_box();
        assert!(bbox.x0 >= 2.0 && bbox.y0 >= 2.0, "bbox: {bbox:?}");
        assert!(bbox.x1 <= 22.0 && bbox.y1 <= 22.0, "bbox: {bbox:?}");
    }
}
