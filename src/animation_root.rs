// Forja GUI — RaizAnimacion: raíz del motor de animaciones

use std::time::Instant;

use crate::{
    accesskit::{Node, Role},
    AccessCtx, Axis, ChildrenIds, LayoutCtx, LenReq, Length, MeasureCtx, NewWidget, NoAction,
    PaintCtx, Painter, Point, PropertiesRef, RegisterCtx, Size, Widget, WidgetPod,
};
use xilem::masonry::layout::LayoutSize;

/// Widget raíz que envuelve todo el árbol y tickea el motor de animaciones
/// en cada frame de pintura.
pub struct RaizAnimacion {
    hijo: Option<WidgetPod<dyn Widget>>,
    ultimo_frame: Instant,
}

impl RaizAnimacion {
    pub fn nuevo(hijo: impl Widget + 'static) -> Self {
        Self {
            hijo: Some(NewWidget::new(hijo).erased().to_pod()),
            ultimo_frame: Instant::now(),
        }
    }

    /// Calcula el delta time desde el último frame en milisegundos
    pub fn delta_ms(&mut self) -> f64 {
        let now = Instant::now();
        let delta = now.duration_since(self.ultimo_frame).as_secs_f64() * 1000.0;
        self.ultimo_frame = now;
        delta
    }
}

impl Widget for RaizAnimacion {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(ref mut hijo) = self.hijo {
            ctx.register_child(hijo);
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        cross_longitud: Option<Length>,
    ) -> Length {
        // Adopta el tamaño del hijo (mismo comportamiento que el `layout` viejo,
        // que devolvía el tamaño resultante del hijo).
        match self.hijo.as_mut() {
            Some(hijo) => {
                let context_tamano = LayoutSize::maybe(axis.cross(), cross_longitud);
                ctx.compute_length(
                    hijo,
                    longitud_req.into(),
                    context_tamano,
                    axis,
                    cross_longitud,
                )
            }
            None => Length::ZERO,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, tamano: Size) {
        if let Some(ref mut hijo) = self.hijo {
            ctx.run_layout(hijo, tamano);
            ctx.place_child(hijo, Point::ORIGIN);
        }
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
        let _delta_ms = self.delta_ms();
    }

    fn accessibility_role(&self) -> Role {
        Role::Window
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        nodo: &mut Node,
    ) {
        nodo.set_label("Forja Animation Root");
    }

    fn children_ids(&self) -> ChildrenIds {
        match &self.hijo {
            Some(hijo) => ChildrenIds::from_slice(&[hijo.id()]),
            None => ChildrenIds::from_slice(&[]),
        }
    }
}
