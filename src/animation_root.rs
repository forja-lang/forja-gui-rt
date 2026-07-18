// Forja GUI — AnimationRoot: raíz del motor de animaciones

use std::time::Instant;

use crate::{
    accesskit::{Node, Role},
    vello::{
        kurbo::{Point, Size},
        Scene,
    },
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NewWidget, NoAction, PaintCtx,
    PropertiesMut, PropertiesRef, RegisterCtx, Widget, WidgetPod,
};

/// Widget raíz que envuelve todo el árbol y tickea el motor de animaciones
/// en cada frame de pintura.
pub struct AnimationRoot {
    child: Option<WidgetPod<dyn Widget>>,
    last_frame: Instant,
}

impl AnimationRoot {
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            child: Some(NewWidget::new(child).erased().to_pod()),
            last_frame: Instant::now(),
        }
    }

    /// Calcula el delta time desde el último frame en milisegundos
    pub fn delta_ms(&mut self) -> f64 {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f64() * 1000.0;
        self.last_frame = now;
        delta
    }
}

impl Widget for AnimationRoot {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(ref mut child) = self.child {
            ctx.register_child(child);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        if let Some(ref mut child) = self.child {
            let size = ctx.run_layout(child, bc);
            ctx.place_child(child, Point::ORIGIN);
            size
        } else {
            bc.constrain(Size::ZERO)
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {
        let _delta_ms = self.delta_ms();
    }

    fn accessibility_role(&self) -> Role {
        Role::Window
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label("Forja Animation Root");
    }

    fn children_ids(&self) -> ChildrenIds {
        match &self.child {
            Some(child) => ChildrenIds::from_slice(&[child.id()]),
            None => ChildrenIds::from_slice(&[]),
        }
    }
}
