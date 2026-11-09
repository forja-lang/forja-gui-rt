// Forja GUI — Widgets animados Masonry personalizados
//
// Proporciona contenedores animados para opacidad, desplazamiento (slide)
// y escala, más un widget de efecto ripple para botones.
// Compatibles con el sistema Xilem vía Pod.

use std::time::Instant;

use crate::{
    accesskit::{Node, Role},
    kurbo::{Circle, Point, Size},
    layout::{LayoutSize, LenReq, Length},
    peniko::color::AlphaColor,
    AccessCtx, Axis, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, Painter,
    PropertiesRef, RegisterCtx, Widget, WidgetMut, WidgetPod,
};

// ═══════════════════════════════════════════════════════════════════
// AnimatedOpacity
// ═══════════════════════════════════════════════════════════════════

/// Widget que envuelve un hijo y controla su opacidad visual
/// mediante el dibujado de una superposición semitransparente.
pub struct AnimatedOpacity {
    hijo: Option<WidgetPod<dyn Widget>>,
    opacity: f64,
    auto_hide: bool,
}

impl AnimatedOpacity {
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: Some(hijo.erased().to_pod()),
            opacity: 1.0,
            auto_hide: false,
        }
    }

    pub fn con_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    pub fn con_auto_hide(mut self) -> Self {
        self.auto_hide = true;
        self
    }
}

impl Widget for AnimatedOpacity {
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
        if self.auto_hide && self.opacity < 0.01 {
            return Length::ZERO;
        }
        // Adopta el tamaño del hijo (como el `layout` viejo, que devolvía el del hijo).
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
            // Si está oculto, el hijo se colapsa a cero (antes no se hacía layout de él).
            let hijo_tamano = if self.auto_hide && self.opacity < 0.01 {
                Size::ZERO
            } else {
                tamano
            };
            ctx.run_layout(hijo, hijo_tamano);
            ctx.place_child(hijo, Point::ORIGIN);
        }
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        if self.auto_hide && self.opacity < 0.01 {
            return;
        }
        if self.opacity < 0.999 && !self.auto_hide {
            let tamano = ctx.border_box().size();
            let alpha = (1.0 - self.opacity) as f32;
            let overlay_color = AlphaColor::from_rgba8(0, 0, 0, (alpha * 255.0) as u8);
            let rect = crate::kurbo::Rect::from_origin_size(Point::ORIGIN, tamano);
            painter.fill_rect(rect, overlay_color);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _nodo: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        match &self.hijo {
            Some(hijo) => ChildrenIds::from_slice(&[hijo.id()]),
            None => ChildrenIds::from_slice(&[]),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// AnimatedSlide
// ═══════════════════════════════════════════════════════════════════

/// Widget que desplaza a su hijo horizontal y/o verticalmente.
pub struct AnimatedSlide {
    hijo: Option<WidgetPod<dyn Widget>>,
    offset_x: f64,
    offset_y: f64,
}

impl AnimatedSlide {
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: Some(hijo.erased().to_pod()),
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    pub fn con_offset(mut self, x: f64, y: f64) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }
}

impl Widget for AnimatedSlide {
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
        // El framework aplica el transform automáticamente
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _nodo: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        match &self.hijo {
            Some(hijo) => ChildrenIds::from_slice(&[hijo.id()]),
            None => ChildrenIds::from_slice(&[]),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// AnimatedScale
// ═══════════════════════════════════════════════════════════════════

/// Widget que escala a su hijo horizontal y/o verticalmente.
pub struct AnimatedScale {
    hijo: Option<WidgetPod<dyn Widget>>,
    scale_x: f64,
    scale_y: f64,
}

impl AnimatedScale {
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: Some(hijo.erased().to_pod()),
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }

    pub fn con_scale(mut self, sx: f64, sy: f64) -> Self {
        self.scale_x = sx;
        self.scale_y = sy;
        self
    }
}

impl Widget for AnimatedScale {
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
        // El framework aplica el transform automáticamente
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _nodo: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        match &self.hijo {
            Some(hijo) => ChildrenIds::from_slice(&[hijo.id()]),
            None => ChildrenIds::from_slice(&[]),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// RippleWidget
// ═══════════════════════════════════════════════════════════════════

/// Widget que dibuja un efecto ripple (onda expansiva) de Material Design.
pub struct RippleWidget {
    hijo: Option<WidgetPod<dyn Widget>>,
    ripple_progress: f64,
    ripple_origin: (f64, f64),
    ripple_rgba: [u8; 4],
    duracion_ms: f64,
    anim_inicio: Option<Instant>,
    max_radius: f64,
}

impl RippleWidget {
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: Some(hijo.erased().to_pod()),
            ripple_progress: 0.0,
            ripple_origin: (0.0, 0.0),
            ripple_rgba: [255, 255, 255, 80],
            duracion_ms: 150.0,
            anim_inicio: None,
            max_radius: 0.0,
        }
    }

    pub fn con_color(mut self, r: u8, g: u8, b: u8, a: u8) -> Self {
        self.ripple_rgba = [r, g, b, a];
        self
    }

    pub fn trigger(this: &mut WidgetMut<'_, Self>) {
        let tamano = this.ctx.border_box().size();
        let cx = tamano.width / 2.0;
        let cy = tamano.height / 2.0;
        let max_r = (cx * cx + cy * cy).sqrt();
        this.widget.ripple_progress = 0.0;
        this.widget.ripple_origin = (cx, cy);
        this.widget.max_radius = max_r;
        this.widget.anim_inicio = Some(Instant::now());
        this.ctx.request_paint_only();
    }
}

impl Widget for RippleWidget {
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
        painter: &mut Painter<'_>,
    ) {
        if self.ripple_progress <= 0.0 || self.ripple_progress >= 1.0 {
            // Si la animación sigue activa, avanzar progreso
            if let Some(inicio) = self.anim_inicio {
                let elapsed = inicio.elapsed().as_secs_f64() * 1000.0;
                self.ripple_progress = (elapsed / self.duracion_ms).clamp(0.0, 1.0);
                if self.ripple_progress >= 1.0 {
                    self.anim_inicio = None;
                }
            }
            if self.ripple_progress <= 0.0 {
                return;
            }
        }

        let radius = self.max_radius * self.ripple_progress;
        let alpha = ((1.0 - self.ripple_progress) * 0.3) as f32;
        let [r, g, b, _] = self.ripple_rgba;
        let color = AlphaColor::from_rgba8(r, g, b, (alpha * 255.0) as u8);

        let circle = Circle::new(
            Point::new(self.ripple_origin.0, self.ripple_origin.1),
            radius,
        );

        painter.fill(circle, color).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _nodo: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        match &self.hijo {
            Some(hijo) => ChildrenIds::from_slice(&[hijo.id()]),
            None => ChildrenIds::from_slice(&[]),
        }
    }
}
