// Forja GUI — Widgets de reconocimiento de gestos táctiles
//
// Proporciona widgets Masonry personalizados para detección de gestos:
// - SwipeWidget: swipe horizontal (para descartar elementos)
// - PanWidget: arrastre continuo (para scroll/drag personalizado)
// - PullToRefreshWidget: tirar hacia abajo para recargar
//
// Compatibles con el sistema Xilem vía Pod wrappers.

use std::collections::HashMap;
use std::time::Instant;

use crate::{
    accesskit::{Node, Role},
    vello::{
        kurbo::{Affine, Point, Rect, Size},
        peniko::color::AlphaColor,
        peniko::{Brush, Fill},
        Scene,
    },
    AccessCtx, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NewWidget, PaintCtx,
    PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerUpdate, PropertiesMut,
    PropertiesRef, RegisterCtx, Widget, WidgetPod,
};

// ═══════════════════════════════════════════════════════════════════
// GestureResult — resultado de un gesto reconocido
// ═══════════════════════════════════════════════════════════════════

/// Resultado de un gesto reconocido
#[derive(Debug, Clone)]
pub enum GestureResult {
    /// Swipe horizontal completado (direction: -1 left, +1 right, distance, velocity)
    Swipe {
        direction: f64,
        distance: f64,
        velocity: f64,
    },
    /// Pan/arrastre continuo (delta_x, delta_y, total_dx, total_dy)
    Pan {
        dx: f64,
        dy: f64,
        total_dx: f64,
        total_dy: f64,
    },
    /// Pull-to-refresh completado (distancia de arrastre hacia abajo)
    PullToRefresh { distance: f64 },
    /// Click simple
    Click,
}

// ═══════════════════════════════════════════════════════════════════
// GestureTracker — estado interno del tracker de gestos
// ═══════════════════════════════════════════════════════════════════

/// Estado interno del tracker de gestos
#[derive(Debug, Clone)]
pub struct GestureTracker {
    /// ¿Está capturando el puntero?
    pub active: bool,
    /// Posición inicial del gesto (coordenadas locales)
    pub start_x: f64,
    pub start_y: f64,
    /// Posición actual
    pub current_x: f64,
    pub current_y: f64,
    /// Posición del frame anterior (para velocidad)
    pub last_x: f64,
    pub last_y: f64,
    /// Tiempo de inicio
    pub start_time: Instant,
    /// Tiempo del frame anterior
    pub last_time: Instant,
    /// Desplazamiento acumulado
    pub total_dx: f64,
    pub total_dy: f64,
    /// ¿Se ha superado el umbral de swipe?
    pub threshold_crossed: bool,
}

impl GestureTracker {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            active: false,
            start_x: 0.0,
            start_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            last_x: 0.0,
            last_y: 0.0,
            start_time: now,
            last_time: now,
            total_dx: 0.0,
            total_dy: 0.0,
            threshold_crossed: false,
        }
    }

    pub fn start(&mut self, x: f64, y: f64) {
        let now = Instant::now();
        self.active = true;
        self.start_x = x;
        self.start_y = y;
        self.current_x = x;
        self.current_y = y;
        self.last_x = x;
        self.last_y = y;
        self.start_time = now;
        self.last_time = now;
        self.total_dx = 0.0;
        self.total_dy = 0.0;
        self.threshold_crossed = false;
    }

    pub fn update(&mut self, x: f64, y: f64) {
        let now = Instant::now();
        self.last_x = self.current_x;
        self.last_y = self.current_y;
        self.current_x = x;
        self.current_y = y;
        self.last_time = now;
        self.total_dx = self.current_x - self.start_x;
        self.total_dy = self.current_y - self.start_y;
    }

    pub fn end(&mut self) {
        self.active = false;
    }

    /// Velocidad actual en px/ms
    pub fn velocity(&self) -> f64 {
        let dt = self.last_time.saturating_duration_since(self.start_time);
        let ms = dt.as_secs_f64() * 1000.0;
        if ms <= 0.0 {
            return 0.0;
        }
        let dx = self.current_x - self.start_x;
        let dy = self.current_y - self.start_y;
        dx.hypot(dy) / ms
    }

    /// ¿El gesto superó un umbral de distancia?
    pub fn crossed_threshold(&self, threshold: f64) -> bool {
        self.total_dx.abs() > threshold || self.total_dy.abs() > threshold
    }
}

// ═══════════════════════════════════════════════════════════════════
// SwipeWidget — wrapper que detecta swipe horizontal
// ═══════════════════════════════════════════════════════════════════

/// Widget wrapper que detecta swipe horizontal.
/// Emite `GestureResult::Swipe` cuando se completa un swipe.
pub struct SwipeWidget {
    child: WidgetPod<dyn Widget>,
    tracker: GestureTracker,
    /// Distancia mínima en px para considerar swipe
    threshold: f64,
    /// Desplazamiento visual actual durante el gesto
    offset_x: f64,
}

impl SwipeWidget {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
            tracker: GestureTracker::new(),
            threshold: 50.0,
            offset_x: 0.0,
        }
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }
}

impl Widget for SwipeWidget {
    type Action = GestureResult;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                state,
                ..
            }) => {
                ctx.capture_pointer();
                let pos = ctx.local_position(state.position);
                self.tracker.start(pos.x, pos.y);
                self.offset_x = 0.0;
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                if ctx.is_active() {
                    let pos = ctx.local_position(current.position);
                    self.tracker.update(pos.x, pos.y);
                    // Desplazar visualmente el hijo
                    self.offset_x = self.tracker.total_dx;
                    ctx.request_render();
                }
            }
            PointerEvent::Up(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                ..
            }) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                    self.tracker.end();
                    // Verificar si superó el umbral
                    if self.tracker.total_dx.abs() > self.threshold {
                        let direction = self.tracker.total_dx.signum();
                        let velocity = self.tracker.velocity();
                        ctx.submit_action::<GestureResult>(GestureResult::Swipe {
                            direction,
                            distance: self.tracker.total_dx.abs(),
                            velocity,
                        });
                    }
                    // Resetear offset visual
                    self.offset_x = 0.0;
                    ctx.request_render();
                }
            }
            PointerEvent::Cancel(_) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                    self.tracker.end();
                    self.offset_x = 0.0;
                    ctx.request_render();
                }
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.child, bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {
        // El framework pinta al hijo automáticamente
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// PanWidget — drag continuo
// ═══════════════════════════════════════════════════════════════════

/// Widget wrapper que detecta pan/arrastre continuo.
/// Emite `GestureResult::Pan` en CADA movimiento.
pub struct PanWidget {
    child: WidgetPod<dyn Widget>,
    tracker: GestureTracker,
    offset_x: f64,
    offset_y: f64,
}

impl PanWidget {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
            tracker: GestureTracker::new(),
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl Widget for PanWidget {
    type Action = GestureResult;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                state,
                ..
            }) => {
                ctx.capture_pointer();
                let pos = ctx.local_position(state.position);
                self.tracker.start(pos.x, pos.y);
                self.offset_x = 0.0;
                self.offset_y = 0.0;
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                if ctx.is_active() {
                    let pos = ctx.local_position(current.position);
                    self.tracker.update(pos.x, pos.y);
                    self.offset_x = self.tracker.total_dx;
                    self.offset_y = self.tracker.total_dy;
                    ctx.submit_action::<GestureResult>(GestureResult::Pan {
                        dx: self.tracker.current_x - self.tracker.last_x,
                        dy: self.tracker.current_y - self.tracker.last_y,
                        total_dx: self.tracker.total_dx,
                        total_dy: self.tracker.total_dy,
                    });
                    ctx.request_render();
                }
            }
            PointerEvent::Up(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                ..
            }) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                    self.tracker.end();
                    self.offset_x = 0.0;
                    self.offset_y = 0.0;
                    ctx.request_render();
                }
            }
            PointerEvent::Cancel(_) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                    self.tracker.end();
                    self.offset_x = 0.0;
                    self.offset_y = 0.0;
                    ctx.request_render();
                }
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.child, bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {
        // El framework aplica el transform automáticamente
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// PullToRefreshWidget — tirar hacia abajo para recargar
// ═══════════════════════════════════════════════════════════════════

/// Widget que implementa Pull-to-Refresh: tirar hacia abajo para recargar.
pub struct PullToRefreshWidget {
    child: WidgetPod<dyn Widget>,
    tracker: GestureTracker,
    /// Cuánto se ha tirado hacia abajo
    pull_distance: f64,
    /// Umbral para activar refresh
    threshold: f64,
    /// ¿Está refrescando?
    refreshing: bool,
    /// Progreso 0.0 a 1.0
    progress: f64,
}

impl PullToRefreshWidget {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
            tracker: GestureTracker::new(),
            pull_distance: 0.0,
            threshold: 80.0,
            refreshing: false,
            progress: 0.0,
        }
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Marcar como refrescando (llamar cuando se inicia la recarga)
    pub fn set_refreshing(&mut self, refreshing: bool) {
        self.refreshing = refreshing;
        if !refreshing {
            self.progress = 0.0;
            self.pull_distance = 0.0;
        }
    }
}

impl Widget for PullToRefreshWidget {
    type Action = GestureResult;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if self.refreshing {
            return;
        }

        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                state,
                ..
            }) => {
                ctx.capture_pointer();
                let pos = ctx.local_position(state.position);
                self.tracker.start(pos.x, pos.y);
                self.pull_distance = 0.0;
                self.progress = 0.0;
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                if ctx.is_active() {
                    let pos = ctx.local_position(current.position);
                    self.tracker.update(pos.x, pos.y);
                    // Solo arrastrando hacia abajo (total_dy > 0)
                    if self.tracker.total_dy > 0.0 {
                        self.pull_distance = self.tracker.total_dy;
                        self.progress = (self.pull_distance / self.threshold).clamp(0.0, 1.0);
                    } else {
                        self.pull_distance = 0.0;
                        self.progress = 0.0;
                    }
                    ctx.request_render();
                }
            }
            PointerEvent::Up(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                ..
            }) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                    self.tracker.end();
                    if self.pull_distance >= self.threshold && !self.refreshing {
                        self.refreshing = true;
                        ctx.submit_action::<GestureResult>(GestureResult::PullToRefresh {
                            distance: self.pull_distance,
                        });
                    }
                    self.pull_distance = 0.0;
                    self.progress = 0.0;
                    ctx.request_render();
                }
            }
            PointerEvent::Cancel(_) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                    self.tracker.end();
                    self.pull_distance = 0.0;
                    self.progress = 0.0;
                    ctx.request_render();
                }
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.child, bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();

        // Pintar indicador de pull-to-refresh si hay progreso
        if self.progress > 0.0 || self.refreshing {
            let indicator_height = 40.0;
            let indicator_y = if self.refreshing {
                // Mostrar spinner en la parte superior
                -indicator_height
            } else {
                // Mostrar indicador según el progreso
                -indicator_height + (self.progress * indicator_height)
            };

            // Rectángulo de fondo del indicador
            let bg_rect = Rect::new(0.0, indicator_y, size.width, indicator_y + indicator_height);
            let bg_color = AlphaColor::from_rgba8(200, 200, 200, 180);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Brush::Solid(bg_color),
                None,
                &bg_rect,
            );

            // Barra de progreso (mostrar avance hacia el umbral)
            if !self.refreshing && self.progress > 0.0 {
                let bar_width = size.width * self.progress;
                let bar_rect = Rect::new(
                    0.0,
                    indicator_y + indicator_height - 4.0,
                    bar_width,
                    indicator_y + indicator_height,
                );
                let bar_color = AlphaColor::from_rgba8(100, 150, 255, 220);
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    Brush::Solid(bar_color),
                    None,
                    &bar_rect,
                );
            }
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helpers para acceder a children (usados por View wrappers)
// ═══════════════════════════════════════════════════════════════════

impl SwipeWidget {
    pub fn child_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl PanWidget {
    pub fn child_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl PullToRefreshWidget {
    pub fn child_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

// ═══════════════════════════════════════════════════════════════════
// MultiTouchState — seguimiento de múltiples puntos táctiles
// ═══════════════════════════════════════════════════════════════════

/// Trackea múltiples puntos táctiles para detección de gestos multi-touch
#[derive(Debug, Clone, Default)]
pub struct MultiTouchState {
    /// Puntos activos: pointer_id → posición
    pub active_pointers: HashMap<PointerId, Point>,
}

impl MultiTouchState {
    pub fn new() -> Self {
        Self {
            active_pointers: HashMap::new(),
        }
    }

    /// Agregar o actualizar un punto táctil
    pub fn update(&mut self, pointer_id: PointerId, pos: Point) {
        self.active_pointers.insert(pointer_id, pos);
    }

    /// Eliminar un punto táctil
    pub fn remove(&mut self, pointer_id: &PointerId) {
        self.active_pointers.remove(pointer_id);
    }

    /// Detectar gesto de pellizco (pinch). Retorna factor de escala relativo.
    /// Se llama en cada Move, comparando con el frame anterior.
    pub fn detect_pinch(&self, prev: &Self) -> Option<f64> {
        if self.active_pointers.len() < 2 || prev.active_pointers.len() < 2 {
            return None;
        }

        let current_dists = self.pointer_distances();
        let prev_dists = prev.pointer_distances();

        if current_dists.is_empty() || prev_dists.is_empty() {
            return None;
        }

        let curr_dist = current_dists[0];
        let prev_dist = prev_dists[0];

        if prev_dist > 0.0 {
            Some(curr_dist / prev_dist)
        } else {
            None
        }
    }

    /// Detectar gesto de rotación. Retorna ángulo en radianes.
    pub fn detect_rotate(&self, prev: &Self) -> Option<f64> {
        if self.active_pointers.len() < 2 || prev.active_pointers.len() < 2 {
            return None;
        }

        let curr_angle = self.angle_between_pointers();
        let prev_angle = prev.angle_between_pointers();

        if let (Some(ca), Some(pa)) = (curr_angle, prev_angle) {
            Some(ca - pa)
        } else {
            None
        }
    }

    /// Distancias entre todos los pares de punteros
    fn pointer_distances(&self) -> Vec<f64> {
        let points: Vec<&Point> = self.active_pointers.values().collect();
        let mut dists = Vec::new();
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                let dx = points[i].x - points[j].x;
                let dy = points[i].y - points[j].y;
                dists.push((dx * dx + dy * dy).sqrt());
            }
        }
        dists
    }

    /// Ángulo entre los 2 primeros punteros
    fn angle_between_pointers(&self) -> Option<f64> {
        let points: Vec<&Point> = self.active_pointers.values().collect();
        if points.len() < 2 {
            return None;
        }
        let dx = points[1].x - points[0].x;
        let dy = points[1].y - points[0].y;
        Some(dy.atan2(dx))
    }

    /// Centroide de todos los puntos activos
    pub fn centroid(&self) -> Option<Point> {
        let count = self.active_pointers.len();
        if count == 0 {
            return None;
        }
        let sum: Point = self
            .active_pointers
            .values()
            .fold(Point::ZERO, |acc, p| Point::new(acc.x + p.x, acc.y + p.y));
        Some(Point::new(sum.x / count as f64, sum.y / count as f64))
    }
}

// ═══════════════════════════════════════════════════════════════════
// PinchZoomWidget — zoom por pellizco (pinch-to-zoom)
// ═══════════════════════════════════════════════════════════════════

/// Widget que aplica zoom por gesto de pellizco (pinch) a su hijo
pub struct PinchZoomWidget {
    child: WidgetPod<dyn Widget>,
    multi_touch: MultiTouchState,
    prev_multi_touch: MultiTouchState,
    scale: f64,
    min_scale: f64,
    max_scale: f64,
}

impl PinchZoomWidget {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
            multi_touch: MultiTouchState::new(),
            prev_multi_touch: MultiTouchState::new(),
            scale: 1.0,
            min_scale: 0.5,
            max_scale: 3.0,
        }
    }

    pub fn with_min_scale(mut self, min: f64) -> Self {
        self.min_scale = min;
        self
    }

    pub fn with_max_scale(mut self, max: f64) -> Self {
        self.max_scale = max;
        self
    }

    fn get_pointer_id(event: &PointerEvent) -> Option<PointerId> {
        match event {
            PointerEvent::Down(btn) => btn.pointer.pointer_id,
            PointerEvent::Move(upd) => upd.pointer.pointer_id,
            PointerEvent::Up(btn) => btn.pointer.pointer_id,
            PointerEvent::Cancel(info) => info.pointer_id,
            _ => None,
        }
    }

    fn get_position(event: &PointerEvent) -> Option<Point> {
        match event {
            PointerEvent::Down(btn) => Some(Point::new(btn.state.position.x, btn.state.position.y)),
            PointerEvent::Move(upd) => {
                Some(Point::new(upd.current.position.x, upd.current.position.y))
            }
            PointerEvent::Up(btn) => Some(Point::new(btn.state.position.x, btn.state.position.y)),
            _ => None,
        }
    }
}

impl Widget for PinchZoomWidget {
    type Action = ();

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        let id = Self::get_pointer_id(event);
        let pos = Self::get_position(event);

        match event {
            PointerEvent::Down(_) => {
                if let (Some(id), Some(pos)) = (id, pos) {
                    self.prev_multi_touch = self.multi_touch.clone();
                    self.multi_touch.update(id, pos);
                }
            }
            PointerEvent::Move { .. } => {
                if let (Some(id), Some(pos)) = (id, pos) {
                    self.prev_multi_touch = self.multi_touch.clone();
                    self.multi_touch.update(id, pos);

                    if let Some(scale_factor) =
                        self.multi_touch.detect_pinch(&self.prev_multi_touch)
                    {
                        self.scale =
                            (self.scale * scale_factor).clamp(self.min_scale, self.max_scale);
                        let size = ctx.size();
                        let center = Point::new(size.width / 2.0, size.height / 2.0);
                        let transform = Affine::translate(center.to_vec2())
                            .then_scale(self.scale)
                            .then_translate(-center.to_vec2());
                        ctx.set_transform(transform);
                        ctx.request_render();
                    }
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
                if let Some(ref id) = id {
                    self.multi_touch.remove(id);
                    // Reset transform when last pointer is removed
                    if self.multi_touch.active_pointers.is_empty() {
                        ctx.set_transform(Affine::IDENTITY);
                        ctx.request_render();
                    }
                }
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.child, bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {
        // La transformación se aplica via ctx.set_transform() en on_pointer_event
    }

    fn accessibility_role(&self) -> crate::accesskit::Role {
        crate::accesskit::Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut crate::accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// RotateWidget — rotación por gesto multi-touch
// ═══════════════════════════════════════════════════════════════════

/// Widget que aplica rotación por gesto multi-touch a su hijo
pub struct RotateWidget {
    child: WidgetPod<dyn Widget>,
    multi_touch: MultiTouchState,
    prev_multi_touch: MultiTouchState,
    angle: f64,
}

impl RotateWidget {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: child.erased().to_pod(),
            multi_touch: MultiTouchState::new(),
            prev_multi_touch: MultiTouchState::new(),
            angle: 0.0,
        }
    }

    fn get_pointer_id(event: &PointerEvent) -> Option<PointerId> {
        PinchZoomWidget::get_pointer_id(event)
    }

    fn get_position(event: &PointerEvent) -> Option<Point> {
        PinchZoomWidget::get_position(event)
    }
}

impl Widget for RotateWidget {
    type Action = f64;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        let id = Self::get_pointer_id(event);
        let pos = Self::get_position(event);

        match event {
            PointerEvent::Down(_) => {
                if let (Some(id), Some(pos)) = (id, pos) {
                    self.prev_multi_touch = self.multi_touch.clone();
                    self.multi_touch.update(id, pos);
                }
            }
            PointerEvent::Move { .. } => {
                if let (Some(id), Some(pos)) = (id, pos) {
                    self.prev_multi_touch = self.multi_touch.clone();
                    self.multi_touch.update(id, pos);

                    if let Some(delta) = self.multi_touch.detect_rotate(&self.prev_multi_touch) {
                        self.angle += delta;
                        let size = ctx.size();
                        let center = Point::new(size.width / 2.0, size.height / 2.0);
                        let transform = Affine::translate(center.to_vec2())
                            .then_rotate(self.angle)
                            .then_translate(-center.to_vec2());
                        ctx.set_transform(transform);
                        ctx.submit_action::<f64>(self.angle);
                        ctx.request_render();
                    }
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
                if let Some(ref id) = id {
                    self.multi_touch.remove(id);
                }
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        let size = ctx.run_layout(&mut self.child, bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        size
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _scene: &mut Scene) {
        // La transformación se aplica via ctx.set_transform() en on_pointer_event
    }

    fn accessibility_role(&self) -> crate::accesskit::Role {
        crate::accesskit::Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _node: &mut crate::accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helpers para acceder a children de PinchZoomWidget y RotateWidget
// ═══════════════════════════════════════════════════════════════════

impl PinchZoomWidget {
    pub fn child_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}

impl RotateWidget {
    pub fn child_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}
