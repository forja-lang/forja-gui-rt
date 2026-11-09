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
    kurbo::{Affine, Point, Rect, Size},
    layout::{LayoutSize, LenReq, Length},
    peniko::color::AlphaColor,
    AccessCtx, Axis, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, Painter,
    PointerButton, PointerButtonEvent, PointerEvent, PointerId, PointerUpdate, PropertiesMut,
    PropertiesRef, RegisterCtx, Widget, WidgetPod,
};

// ═══════════════════════════════════════════════════════════════════
// GestureResult — resultado de un gesto reconocido
// ═══════════════════════════════════════════════════════════════════

/// Resultado de un gesto reconocido
#[derive(Debug, Clone)]
pub enum GestureResultado {
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
    pub inicio_x: f64,
    pub inicio_y: f64,
    /// Posición actual
    pub actual_x: f64,
    pub actual_y: f64,
    /// Posición del frame anterior (para velocidad)
    pub ultimo_x: f64,
    pub ultimo_y: f64,
    /// Tiempo de inicio
    pub inicio_time: Instant,
    /// Tiempo del frame anterior
    pub ultimo_time: Instant,
    /// Desplazamiento acumulado
    pub total_dx: f64,
    pub total_dy: f64,
    /// ¿Se ha superado el umbral de swipe?
    pub threshold_crossed: bool,
}

impl Default for GestureTracker {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl GestureTracker {
    pub fn nuevo() -> Self {
        let now = Instant::now();
        Self {
            active: false,
            inicio_x: 0.0,
            inicio_y: 0.0,
            actual_x: 0.0,
            actual_y: 0.0,
            ultimo_x: 0.0,
            ultimo_y: 0.0,
            inicio_time: now,
            ultimo_time: now,
            total_dx: 0.0,
            total_dy: 0.0,
            threshold_crossed: false,
        }
    }

    pub fn inicio(&mut self, x: f64, y: f64) {
        let now = Instant::now();
        self.active = true;
        self.inicio_x = x;
        self.inicio_y = y;
        self.actual_x = x;
        self.actual_y = y;
        self.ultimo_x = x;
        self.ultimo_y = y;
        self.inicio_time = now;
        self.ultimo_time = now;
        self.total_dx = 0.0;
        self.total_dy = 0.0;
        self.threshold_crossed = false;
    }

    pub fn update(&mut self, x: f64, y: f64) {
        let now = Instant::now();
        self.ultimo_x = self.actual_x;
        self.ultimo_y = self.actual_y;
        self.actual_x = x;
        self.actual_y = y;
        self.ultimo_time = now;
        self.total_dx = self.actual_x - self.inicio_x;
        self.total_dy = self.actual_y - self.inicio_y;
    }

    pub fn end(&mut self) {
        self.active = false;
    }

    /// Velocidad actual en px/ms
    pub fn velocity(&self) -> f64 {
        let dt = self.ultimo_time.saturating_duration_since(self.inicio_time);
        let ms = dt.as_secs_f64() * 1000.0;
        if ms <= 0.0 {
            return 0.0;
        }
        let dx = self.actual_x - self.inicio_x;
        let dy = self.actual_y - self.inicio_y;
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
    hijo: WidgetPod<dyn Widget>,
    tracker: GestureTracker,
    /// Distancia mínima en px para considerar swipe
    threshold: f64,
    /// Desplazamiento visual actual durante el gesto
    offset_x: f64,
}

impl SwipeWidget {
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: hijo.erased().to_pod(),
            tracker: GestureTracker::nuevo(),
            threshold: 50.0,
            offset_x: 0.0,
        }
    }

    pub fn con_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }
}

impl Widget for SwipeWidget {
    type Action = GestureResultado;

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
                self.tracker.inicio(pos.x, pos.y);
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
                        ctx.submit_action::<GestureResultado>(GestureResultado::Swipe {
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
            PointerEvent::Cancel(_) if ctx.is_active() => {
                ctx.release_pointer();
                self.tracker.end();
                self.offset_x = 0.0;
                ctx.request_render();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.hijo);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        cross_longitud: Option<Length>,
    ) -> Length {
        // Adopta el tamaño del hijo (igual que el `layout` viejo).
        let context_tamano = LayoutSize::maybe(axis.cross(), cross_longitud);
        ctx.compute_length(
            &mut self.hijo,
            longitud_req.into(),
            context_tamano,
            axis,
            cross_longitud,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, tamano: Size) {
        ctx.run_layout(&mut self.hijo, tamano);
        ctx.place_child(&mut self.hijo, Point::ORIGIN);
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
        // El framework pinta al hijo automáticamente
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
        ChildrenIds::from_slice(&[self.hijo.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// PanWidget — drag continuo
// ═══════════════════════════════════════════════════════════════════

/// Widget wrapper que detecta pan/arrastre continuo.
/// Emite `GestureResult::Pan` en CADA movimiento.
pub struct PanWidget {
    hijo: WidgetPod<dyn Widget>,
    tracker: GestureTracker,
    offset_x: f64,
    offset_y: f64,
}

impl PanWidget {
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: hijo.erased().to_pod(),
            tracker: GestureTracker::nuevo(),
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl Widget for PanWidget {
    type Action = GestureResultado;

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
                self.tracker.inicio(pos.x, pos.y);
                self.offset_x = 0.0;
                self.offset_y = 0.0;
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                if ctx.is_active() {
                    let pos = ctx.local_position(current.position);
                    self.tracker.update(pos.x, pos.y);
                    self.offset_x = self.tracker.total_dx;
                    self.offset_y = self.tracker.total_dy;
                    ctx.submit_action::<GestureResultado>(GestureResultado::Pan {
                        dx: self.tracker.actual_x - self.tracker.ultimo_x,
                        dy: self.tracker.actual_y - self.tracker.ultimo_y,
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
            PointerEvent::Cancel(_) if ctx.is_active() => {
                ctx.release_pointer();
                self.tracker.end();
                self.offset_x = 0.0;
                self.offset_y = 0.0;
                ctx.request_render();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.hijo);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        cross_longitud: Option<Length>,
    ) -> Length {
        // Adopta el tamaño del hijo (igual que el `layout` viejo).
        let context_tamano = LayoutSize::maybe(axis.cross(), cross_longitud);
        ctx.compute_length(
            &mut self.hijo,
            longitud_req.into(),
            context_tamano,
            axis,
            cross_longitud,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, tamano: Size) {
        ctx.run_layout(&mut self.hijo, tamano);
        ctx.place_child(&mut self.hijo, Point::ORIGIN);
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
        ChildrenIds::from_slice(&[self.hijo.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// PullToRefreshWidget — tirar hacia abajo para recargar
// ═══════════════════════════════════════════════════════════════════

/// Widget que implementa Pull-to-Refresh: tirar hacia abajo para recargar.
pub struct PullToRefreshWidget {
    hijo: WidgetPod<dyn Widget>,
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
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: hijo.erased().to_pod(),
            tracker: GestureTracker::nuevo(),
            pull_distance: 0.0,
            threshold: 80.0,
            refreshing: false,
            progress: 0.0,
        }
    }

    pub fn con_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Marcar como refrescando (llamar cuando se inicia la recarga)
    pub fn fijar_refreshing(&mut self, refreshing: bool) {
        self.refreshing = refreshing;
        if !refreshing {
            self.progress = 0.0;
            self.pull_distance = 0.0;
        }
    }
}

impl Widget for PullToRefreshWidget {
    type Action = GestureResultado;

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
                self.tracker.inicio(pos.x, pos.y);
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
                        ctx.submit_action::<GestureResultado>(GestureResultado::PullToRefresh {
                            distance: self.pull_distance,
                        });
                    }
                    self.pull_distance = 0.0;
                    self.progress = 0.0;
                    ctx.request_render();
                }
            }
            PointerEvent::Cancel(_) if ctx.is_active() => {
                ctx.release_pointer();
                self.tracker.end();
                self.pull_distance = 0.0;
                self.progress = 0.0;
                ctx.request_render();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.hijo);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        cross_longitud: Option<Length>,
    ) -> Length {
        // Adopta el tamaño del hijo (igual que el `layout` viejo).
        let context_tamano = LayoutSize::maybe(axis.cross(), cross_longitud);
        ctx.compute_length(
            &mut self.hijo,
            longitud_req.into(),
            context_tamano,
            axis,
            cross_longitud,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, tamano: Size) {
        ctx.run_layout(&mut self.hijo, tamano);
        ctx.place_child(&mut self.hijo, Point::ORIGIN);
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let tamano = ctx.border_box().size();

        // Pintar indicador de pull-to-refresh si hay progreso
        if self.progress > 0.0 || self.refreshing {
            let indicator_alto = 40.0;
            let indicator_y = if self.refreshing {
                // Mostrar spinner en la parte superior
                -indicator_alto
            } else {
                // Mostrar indicador según el progreso
                -indicator_alto + (self.progress * indicator_alto)
            };

            // Rectángulo de fondo del indicador
            let bg_rect = Rect::new(0.0, indicator_y, tamano.width, indicator_y + indicator_alto);
            let bg_color = AlphaColor::from_rgba8(200, 200, 200, 180);
            painter.fill_rect(bg_rect, bg_color);

            // Barra de progreso (mostrar avance hacia el umbral)
            if !self.refreshing && self.progress > 0.0 {
                let bar_ancho = tamano.width * self.progress;
                let bar_rect = Rect::new(
                    0.0,
                    indicator_y + indicator_alto - 4.0,
                    bar_ancho,
                    indicator_y + indicator_alto,
                );
                let bar_color = AlphaColor::from_rgba8(100, 150, 255, 220);
                painter.fill_rect(bar_rect, bar_color);
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
        _nodo: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.hijo.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helpers para acceder a children (usados por View wrappers)
// ═══════════════════════════════════════════════════════════════════

impl SwipeWidget {
    pub fn hijo_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.hijo)
    }
}

impl PanWidget {
    pub fn hijo_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.hijo)
    }
}

impl PullToRefreshWidget {
    pub fn hijo_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.hijo)
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
    pub fn nuevo() -> Self {
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
    pub fn detect_pinch(&self, anterior: &Self) -> Option<f64> {
        if self.active_pointers.len() < 2 || anterior.active_pointers.len() < 2 {
            return None;
        }

        let actual_dists = self.pointer_distances();
        let anterior_dists = anterior.pointer_distances();

        if actual_dists.is_empty() || anterior_dists.is_empty() {
            return None;
        }

        let curr_dist = actual_dists[0];
        let anterior_dist = anterior_dists[0];

        if anterior_dist > 0.0 {
            Some(curr_dist / anterior_dist)
        } else {
            None
        }
    }

    /// Detectar gesto de rotación. Retorna ángulo en radianes.
    pub fn detect_rotate(&self, anterior: &Self) -> Option<f64> {
        if self.active_pointers.len() < 2 || anterior.active_pointers.len() < 2 {
            return None;
        }

        let curr_angle = self.angle_between_pointers();
        let anterior_angle = anterior.angle_between_pointers();

        if let (Some(ca), Some(pa)) = (curr_angle, anterior_angle) {
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
        let cantidad = self.active_pointers.len();
        if cantidad == 0 {
            return None;
        }
        let sum: Point = self
            .active_pointers
            .values()
            .fold(Point::ZERO, |acc, p| Point::new(acc.x + p.x, acc.y + p.y));
        Some(Point::new(sum.x / cantidad as f64, sum.y / cantidad as f64))
    }
}

// ═══════════════════════════════════════════════════════════════════
// PinchZoomWidget — zoom por pellizco (pinch-to-zoom)
// ═══════════════════════════════════════════════════════════════════

/// Widget que aplica zoom por gesto de pellizco (pinch) a su hijo
pub struct PinchZoomWidget {
    hijo: WidgetPod<dyn Widget>,
    multi_touch: MultiTouchState,
    anterior_multi_touch: MultiTouchState,
    scale: f64,
    min_scale: f64,
    max_scale: f64,
}

impl PinchZoomWidget {
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: hijo.erased().to_pod(),
            multi_touch: MultiTouchState::nuevo(),
            anterior_multi_touch: MultiTouchState::nuevo(),
            scale: 1.0,
            min_scale: 0.5,
            max_scale: 3.0,
        }
    }

    pub fn con_min_scale(mut self, min: f64) -> Self {
        self.min_scale = min;
        self
    }

    pub fn con_max_scale(mut self, max: f64) -> Self {
        self.max_scale = max;
        self
    }

    fn obtener_pointer_id(event: &PointerEvent) -> Option<PointerId> {
        match event {
            PointerEvent::Down(btn) => btn.pointer.pointer_id,
            PointerEvent::Move(upd) => upd.pointer.pointer_id,
            PointerEvent::Up(btn) => btn.pointer.pointer_id,
            PointerEvent::Cancel(info) => info.pointer_id,
            _ => None,
        }
    }

    fn obtener_position(event: &PointerEvent) -> Option<Point> {
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
        let id = Self::obtener_pointer_id(event);
        let pos = Self::obtener_position(event);

        match event {
            PointerEvent::Down(_) => {
                if let (Some(id), Some(pos)) = (id, pos) {
                    self.anterior_multi_touch = self.multi_touch.clone();
                    self.multi_touch.update(id, pos);
                }
            }
            PointerEvent::Move { .. } => {
                if let (Some(id), Some(pos)) = (id, pos) {
                    self.anterior_multi_touch = self.multi_touch.clone();
                    self.multi_touch.update(id, pos);

                    if let Some(scale_factor) =
                        self.multi_touch.detect_pinch(&self.anterior_multi_touch)
                    {
                        self.scale =
                            (self.scale * scale_factor).clamp(self.min_scale, self.max_scale);
                        let tamano = ctx.border_box().size();
                        let center = Point::new(tamano.width / 2.0, tamano.height / 2.0);
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
        ctx.register_child(&mut self.hijo);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        cross_longitud: Option<Length>,
    ) -> Length {
        // Adopta el tamaño del hijo (igual que el `layout` viejo).
        let context_tamano = LayoutSize::maybe(axis.cross(), cross_longitud);
        ctx.compute_length(
            &mut self.hijo,
            longitud_req.into(),
            context_tamano,
            axis,
            cross_longitud,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, tamano: Size) {
        ctx.run_layout(&mut self.hijo, tamano);
        ctx.place_child(&mut self.hijo, Point::ORIGIN);
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
        // La transformación se aplica via ctx.set_transform() en on_pointer_event
    }

    fn accessibility_role(&self) -> crate::accesskit::Role {
        crate::accesskit::Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _nodo: &mut crate::accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.hijo.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// RotateWidget — rotación por gesto multi-touch
// ═══════════════════════════════════════════════════════════════════

/// Widget que aplica rotación por gesto multi-touch a su hijo
pub struct RotateWidget {
    hijo: WidgetPod<dyn Widget>,
    multi_touch: MultiTouchState,
    anterior_multi_touch: MultiTouchState,
    angle: f64,
}

impl RotateWidget {
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: hijo.erased().to_pod(),
            multi_touch: MultiTouchState::nuevo(),
            anterior_multi_touch: MultiTouchState::nuevo(),
            angle: 0.0,
        }
    }

    fn obtener_pointer_id(event: &PointerEvent) -> Option<PointerId> {
        PinchZoomWidget::obtener_pointer_id(event)
    }

    fn obtener_position(event: &PointerEvent) -> Option<Point> {
        PinchZoomWidget::obtener_position(event)
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
        let id = Self::obtener_pointer_id(event);
        let pos = Self::obtener_position(event);

        match event {
            PointerEvent::Down(_) => {
                if let (Some(id), Some(pos)) = (id, pos) {
                    self.anterior_multi_touch = self.multi_touch.clone();
                    self.multi_touch.update(id, pos);
                }
            }
            PointerEvent::Move { .. } => {
                if let (Some(id), Some(pos)) = (id, pos) {
                    self.anterior_multi_touch = self.multi_touch.clone();
                    self.multi_touch.update(id, pos);

                    if let Some(delta) = self.multi_touch.detect_rotate(&self.anterior_multi_touch)
                    {
                        self.angle += delta;
                        let tamano = ctx.border_box().size();
                        let center = Point::new(tamano.width / 2.0, tamano.height / 2.0);
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
        ctx.register_child(&mut self.hijo);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        cross_longitud: Option<Length>,
    ) -> Length {
        // Adopta el tamaño del hijo (igual que el `layout` viejo).
        let context_tamano = LayoutSize::maybe(axis.cross(), cross_longitud);
        ctx.compute_length(
            &mut self.hijo,
            longitud_req.into(),
            context_tamano,
            axis,
            cross_longitud,
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, tamano: Size) {
        ctx.run_layout(&mut self.hijo, tamano);
        ctx.place_child(&mut self.hijo, Point::ORIGIN);
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
        // La transformación se aplica via ctx.set_transform() en on_pointer_event
    }

    fn accessibility_role(&self) -> crate::accesskit::Role {
        crate::accesskit::Role::GenericContainer
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        _nodo: &mut crate::accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.hijo.id()])
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helpers para acceder a children de PinchZoomWidget y RotateWidget
// ═══════════════════════════════════════════════════════════════════

impl PinchZoomWidget {
    pub fn hijo_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.hijo)
    }
}

impl RotateWidget {
    pub fn hijo_mut<'a>(
        this: &'a mut crate::WidgetMut<'_, Self>,
    ) -> crate::WidgetMut<'a, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.hijo)
    }
}
