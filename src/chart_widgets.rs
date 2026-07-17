// Forja GUI — Widgets de gráficos vectoriales con Vello
//
// Reemplaza la emulación anterior (barras flex, puntos Unicode, leyendas de texto)
// por dibujo vectorial real con Vello Scene, usando kurbo para formas geométricas.
//
// Cada widget implementa el trait Widget de Masonry y puede usarse directamente
// envuelto en un Pod<Widget> desde layout_a_view() en gui_nativa.rs.

use crate::accesskit::{Node, Role};
use crate::vello::kurbo::{Affine, Arc, BezPath, Circle, Line, Point, Rect, Size, Stroke, Vec2};
use crate::vello::peniko::{self, Brush, Fill};
use crate::vello::peniko::color::AlphaColor;
use crate::vello::Scene;
use crate::{
    AccessCtx, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NoAction, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx,
};
use std::f64::consts::PI;

// ═══════════════════════════════════════════════════════════════════
// DATA: Estructura de datos compartida para todos los charts
// ═══════════════════════════════════════════════════════════════════

/// Un punto individual de datos para cualquier chart
#[derive(Debug, Clone)]
pub struct ChartDataPoint {
    pub label: String,
    pub value: f64,
    pub color: AlphaColor,
}

impl ChartDataPoint {
    pub fn new(label: &str, value: f64, color: AlphaColor) -> Self {
        Self {
            label: label.to_string(),
            value,
            color,
        }
    }

    /// Crea un punto desde un color en formato RGBA u8
    pub fn from_rgba8(label: &str, value: f64, r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            label: label.to_string(),
            value,
            color: AlphaColor::from_rgba8(r, g, b, a),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helper: convertir un ratio 0..1 a un color de gradiente
// ═══════════════════════════════════════════════════════════════════

fn lerp_color(a: AlphaColor, b: AlphaColor, t: f64) -> AlphaColor {
    let t = t.clamp(0.0, 1.0);
    let [ar, ag, ab, aa] = a.components;
    let [br, bg, bb, ba] = b.components;
    AlphaColor::from_rgba8(
        (ar as f64 + (br as f64 - ar as f64) * t) as u8,
        (ag as f64 + (bg as f64 - ag as f64) * t) as u8,
        (ab as f64 + (bb as f64 - ab as f64) * t) as u8,
        (aa as f64 + (ba as f64 - aa as f64) * t) as u8,
    )
}

// ═══════════════════════════════════════════════════════════════════
// LINECHART WIDGET
// ═══════════════════════════════════════════════════════════════════

/// Widget de gráfico de líneas vectorial.
///
/// Dibuja una línea poligonal conectando puntos de datos, con puntos opcionales
/// en cada vértice y líneas de referencia horizontales.
pub struct LineChartWidget {
    data: Vec<ChartDataPoint>,
    show_points: bool,
    show_grid: bool,
    line_color: AlphaColor,
    line_width: f64,
}

impl LineChartWidget {
    pub fn new(data: Vec<ChartDataPoint>) -> Self {
        Self {
            data,
            show_points: true,
            show_grid: true,
            line_color: AlphaColor::from_rgba8(0x67, 0x50, 0xA4, 0xFF), // Material primary
            line_width: 3.0,
        }
    }

    pub fn with_line_color(mut self, color: AlphaColor) -> Self {
        self.line_color = color;
        self
    }

    pub fn with_line_width(mut self, width: f64) -> Self {
        self.line_width = width;
        self
    }

    pub fn with_show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    pub fn with_show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }
}

impl crate::Widget for LineChartWidget {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(Size::new(300.0, 200.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let padding = 30.0;
        let chart_w = size.width - padding * 2.0;
        let chart_h = size.height - padding * 2.0;

        if self.data.is_empty() || chart_w <= 0.0 || chart_h <= 0.0 {
            return;
        }

        // Encontrar valor máximo
        let max_val = self.data.iter().map(|d| d.value).fold(0.0_f64, f64::max).max(1.0);
        let count = self.data.len();
        let step_x = if count > 1 { chart_w / (count - 1) as f64 } else { chart_w / 2.0 };

        // ── Dibujar líneas de referencia (grid) ──
        if self.show_grid {
            let grid_color = Brush::Solid(AlphaColor::from_rgba8(0xE0, 0xE0, 0xE0, 0xFF));
            for i in 0..=4 {
                let y = padding + chart_h * (1.0 - i as f64 / 4.0);
                let line = Line::new(Point::new(padding, y), Point::new(padding + chart_w, y));
                scene.stroke(
                    &Stroke::new(1.0),
                    Affine::IDENTITY,
                    &grid_color,
                    None,
                    &line,
                );
            }
        }

        // ── Construir línea poligonal ──
        let mut path = BezPath::new();
        for (i, point) in self.data.iter().enumerate() {
            let x = padding + i as f64 * step_x;
            let y = padding + chart_h - (point.value / max_val * chart_h);
            if i == 0 {
                path.move_to(Point::new(x, y));
            } else {
                path.line_to(Point::new(x, y));
            }
        }

        // Dibujar línea
        let line_brush = Brush::Solid(self.line_color);
        scene.stroke(
            &Stroke::new(self.line_width),
            Affine::IDENTITY,
            &line_brush,
            None,
            &path,
        );

        // ── Dibujar puntos en los vértices ──
        if self.show_points {
            for (i, point) in self.data.iter().enumerate() {
                let x = padding + i as f64 * step_x;
                let y = padding + chart_h - (point.value / max_val * chart_h);
                let circle = Circle::new(Point::new(x, y), 4.0);
                let point_brush = Brush::Solid(point.color);
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &point_brush,
                    None,
                    &circle,
                );

                // Borde blanco alrededor del punto
                let border = Circle::new(Point::new(x, y), 4.0);
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    &Brush::Solid(AlphaColor::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)),
                    None,
                    &border,
                );
            }
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label("Gráfico de líneas");
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

// ═══════════════════════════════════════════════════════════════════
// BARCHART WIDGET
// ═══════════════════════════════════════════════════════════════════

/// Widget de gráfico de barras vectorial.
///
/// Dibuja barras rectangulares usando Vello, con soporte para apilado.
pub struct BarChartWidget {
    data: Vec<ChartDataPoint>,
    apilado: bool,
    bar_width: f64,
    show_values: bool,
}

impl BarChartWidget {
    pub fn new(data: Vec<ChartDataPoint>, apilado: bool) -> Self {
        Self {
            data,
            apilado,
            bar_width: 40.0,
            show_values: true,
        }
    }

    pub fn with_bar_width(mut self, width: f64) -> Self {
        self.bar_width = width;
        self
    }

    pub fn with_show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }
}

impl crate::Widget for BarChartWidget {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(Size::new(300.0, 200.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let padding = 30.0;
        let chart_w = size.width - padding * 2.0;
        let chart_h = size.height - padding * 2.0;

        if self.data.is_empty() || chart_w <= 0.0 || chart_h <= 0.0 {
            return;
        }

        let max_val = self.data.iter().map(|d| d.value).fold(0.0_f64, f64::max).max(1.0);
        let count = self.data.len();
        let total_bar_space = chart_w - (count as f64 * 4.0); // 4px gap between bars
        let bar_w = (total_bar_space / count as f64).min(self.bar_width).max(4.0);
        let step_x = chart_w / count as f64;

        // ── Dibujar eje base ──
        let axis_y = padding + chart_h;
        let axis = Line::new(Point::new(padding, axis_y), Point::new(padding + chart_w, axis_y));
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(AlphaColor::from_rgba8(0xCC, 0xCC, 0xCC, 0xFF)),
            None,
            &axis,
        );

        for (i, point) in self.data.iter().enumerate() {
            let bar_height = (point.value / max_val * chart_h).max(2.0);
            let x = padding + i as f64 * step_x + (step_x - bar_w) / 2.0;
            let y = padding + chart_h - bar_height;

            let rect = Rect::new(x, y, x + bar_w, padding + chart_h);
            let brush = Brush::Solid(point.color);

            // Dibujar barra con esquinas redondeadas (simuladas con rect simple)
            scene.fill(Fill::NonZero, Affine::IDENTITY, &brush, None, &rect);

            // Borde sutil
            scene.stroke(
                &Stroke::new(0.5),
                Affine::IDENTITY,
                &Brush::Solid(AlphaColor::from_rgba8(0x00, 0x00, 0x00, 0x20)),
                None,
                &rect,
            );
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label("Gráfico de barras");
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

// ═══════════════════════════════════════════════════════════════════
// PIECHART WIDGET (con soporte Donut)
// ═══════════════════════════════════════════════════════════════════

/// Widget de gráfico de pastel / donut vectorial.
///
/// Dibuja arcos circulares usando `kurbo::Arc` para cada segmento.
/// Si `donut` es true, dibuja un círculo hueco en el centro.
pub struct PieChartWidget {
    data: Vec<ChartDataPoint>,
    donut: bool,
    donut_ratio: f64, // 0.0-1.0, qué tan grande es el agujero central
}

impl PieChartWidget {
    pub fn new(data: Vec<ChartDataPoint>) -> Self {
        Self {
            data,
            donut: false,
            donut_ratio: 0.5,
        }
    }

    pub fn with_donut(mut self, ratio: f64) -> Self {
        self.donut = true;
        self.donut_ratio = ratio.clamp(0.1, 0.9);
        self
    }

    pub fn set_donut(&mut self, ratio: f64) {
        self.donut = true;
        self.donut_ratio = ratio.clamp(0.1, 0.9);
    }
}

impl PieChartWidget {
    /// Dibuja un segmento de arco relleno (pastel).
    /// Construye un path cerrado: centro → arco → centro.
    fn draw_pie_segment(
        scene: &mut Scene,
        cx: f64,
        cy: f64,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
        brush: &Brush,
    ) {
        if sweep_angle.abs() < 0.001 {
            return;
        }

        // Construir path de pastel (centro → arco → centro)
        let mut path = BezPath::new();
        path.move_to(Point::new(cx, cy));

        // Punto inicial del arco
        let x0 = cx + start_angle.cos() * radius;
        let y0 = cy + start_angle.sin() * radius;
        path.line_to(Point::new(x0, y0));

        // Si el sweep es > PI, necesitamos dos arcos
        let end_angle = start_angle + sweep_angle;
        if sweep_angle.abs() > PI {
            // Primer arco (mitad)
            let mid_angle = start_angle + sweep_angle / 2.0;
            let xm = cx + mid_angle.cos() * radius;
            let ym = cy + mid_angle.sin() * radius;
            path.arc_to(
                Vec2::new(radius, radius),
                Vec2::new(xm - x0, ym - y0).hypot().recip() * 0.0, // rotation
                &Point::new(xm, ym),
            );
            // Segundo arco
            let x1 = cx + end_angle.cos() * radius;
            let y1 = cy + end_angle.sin() * radius;
            path.arc_to(
                Vec2::new(radius, radius),
                0.0,
                &Point::new(x1, y1),
            );
        } else {
            // Arco simple
            let x1 = cx + end_angle.cos() * radius;
            let y1 = cy + end_angle.sin() * radius;
            path.arc_to(
                Vec2::new(radius, radius),
                0.0,
                &Point::new(x1, y1),
            );
        }

        // Cerrar al centro
        path.close_path();

        scene.fill(Fill::NonZero, Affine::IDENTITY, brush, None, &path);
    }
}

impl crate::Widget for PieChartWidget {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(Size::new(200.0, 200.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let cx = size.width / 2.0;
        let cy = size.height / 2.0;
        let radius = size.width.min(size.height) / 2.0 * 0.8;

        let total: f64 = self.data.iter().map(|d| d.value).sum();
        if total <= 0.0 || radius < 5.0 {
            return;
        }

        let mut start_angle = -PI / 2.0; // empezar desde arriba

        for point in &self.data {
            let sweep = (point.value / total) * 2.0 * PI;
            let brush = Brush::Solid(point.color);

            Self::draw_pie_segment(scene, cx, cy, radius, start_angle, sweep, &brush);

            start_angle += sweep;
        }

        // Si es donut, dibujar círculo en el centro (agujero)
        if self.donut {
            let inner_r = radius * self.donut_ratio;
            let inner_circle = Circle::new(Point::new(cx, cy), inner_r);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(AlphaColor::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)),
                None,
                &inner_circle,
            );

            // Borde del agujero para definición
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                &Brush::Solid(AlphaColor::from_rgba8(0xDD, 0xDD, 0xDD, 0xFF)),
                None,
                &inner_circle,
            );
        }

        // Borde exterior del pastel
        let outer_circle = Circle::new(Point::new(cx, cy), radius);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(AlphaColor::from_rgba8(0x40, 0x40, 0x40, 0x30)),
            None,
            &outer_circle,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label(if self.donut { "Gráfico de donut" } else { "Gráfico de pastel" });
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

// ═══════════════════════════════════════════════════════════════════
// GAUGE CHART WIDGET
// ═══════════════════════════════════════════════════════════════════

/// Widget de indicador tipo Gauge (velocímetro) vectorial.
///
/// Dibuja un arco parcial de 270° con color de relleno según umbral,
/// una aguja indicadora y un círculo central.
pub struct GaugeChartWidget {
    value: f64,
    max_value: f64,
    threshold: f64, // valor donde cambia de color
    low_color: AlphaColor,
    high_color: AlphaColor,
    show_label: bool,
}

impl GaugeChartWidget {
    pub fn new(value: f64, max_value: f64, threshold: f64) -> Self {
        Self {
            value,
            max_value: max_value.max(1.0),
            threshold,
            low_color: AlphaColor::from_rgba8(0x4C, 0xAF, 0x50, 0xFF),  // verde
            high_color: AlphaColor::from_rgba8(0xF4, 0x43, 0x36, 0xFF), // rojo
            show_label: true,
        }
    }

    pub fn with_colors(mut self, low: AlphaColor, high: AlphaColor) -> Self {
        self.low_color = low;
        self.high_color = high;
        self
    }

    pub fn with_show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }
}

impl GaugeChartWidget {
    /// Dibuja un arco grueso usando path de BezPath.
    fn draw_thick_arc(
        scene: &mut Scene,
        cx: f64,
        cy: f64,
        radius: f64,
        thickness: f64,
        start_angle: f64,
        sweep_angle: f64,
        brush: &Brush,
    ) {
        if sweep_angle.abs() < 0.001 {
            return;
        }

        let outer_r = radius;
        let inner_r = radius - thickness;

        let end_angle = start_angle + sweep_angle;

        // Puntos del arco exterior e interior
        let x0_outer = cx + start_angle.cos() * outer_r;
        let y0_outer = cy + start_angle.sin() * outer_r;
        let x1_outer = cx + end_angle.cos() * outer_r;
        let y1_outer = cy + end_angle.sin() * outer_r;

        let x0_inner = cx + start_angle.cos() * inner_r;
        let y0_inner = cy + start_angle.sin() * inner_r;
        let x1_inner = cx + end_angle.cos() * inner_r;
        let y1_inner = cy + end_angle.sin() * inner_r;

        let mut path = BezPath::new();
        // Arco exterior
        path.move_to(Point::new(x0_outer, y0_outer));
        path.arc_to(
            Vec2::new(outer_r, outer_r),
            0.0,
            &Point::new(x1_outer, y1_outer),
        );
        // Línea hacia el interior
        path.line_to(Point::new(x1_inner, y1_inner));
        // Arco interior (en dirección inversa)
        path.arc_to(
            Vec2::new(inner_r, inner_r),
            0.0,
            &Point::new(x0_inner, y0_inner),
        );
        path.close_path();

        scene.fill(Fill::NonZero, Affine::IDENTITY, brush, None, &path);
    }
}

impl crate::Widget for GaugeChartWidget {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(Size::new(200.0, 200.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let cx = size.width / 2.0;
        let cy = size.height * 0.65;
        let radius = size.width.min(size.height) / 2.0 * 0.55;
        let thickness = 14.0;

        if radius < 10.0 {
            return;
        }

        let progress = (self.value / self.max_value).clamp(0.0, 1.0);
        let color = if self.value > self.threshold {
            self.high_color
        } else {
            self.low_color
        };

        // Ángulos: el gauge abarca 270° empezando desde 135° (abajo-izquierda)
        let gauge_start = 0.75 * PI;   // 135°
        let gauge_sweep = 1.5 * PI;    // 270°

        // ── Arco de fondo (gris claro) ──
        Self::draw_thick_arc(
            scene,
            cx,
            cy,
            radius,
            thickness,
            gauge_start,
            gauge_sweep,
            &Brush::Solid(AlphaColor::from_rgba8(0xE8, 0xE8, 0xE8, 0xFF)),
        );

        // ── Arco de progreso ──
        if progress > 0.0 {
            Self::draw_thick_arc(
                scene,
                cx,
                cy,
                radius,
                thickness,
                gauge_start,
                gauge_sweep * progress,
                &Brush::Solid(color),
            );
        }

        // ── Aguja indicadora ──
        let angle = gauge_start + gauge_sweep * progress;
        let needle_len = radius * 0.65;
        let needle_end = Point::new(
            cx + angle.cos() * needle_len,
            cy + angle.sin() * needle_len,
        );
        let needle = Line::new(Point::new(cx, cy), needle_end);
        scene.stroke(
            &Stroke::new(2.5),
            Affine::IDENTITY,
            &Brush::Solid(AlphaColor::from_rgba8(0x33, 0x33, 0x33, 0xFF)),
            None,
            &needle,
        );

        // ── Círculo central ──
        let center = Circle::new(Point::new(cx, cy), 5.0);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(AlphaColor::from_rgba8(0x33, 0x33, 0x33, 0xFF)),
            None,
            &center,
        );

        // Borde blanco del círculo central
        let center_border = Circle::new(Point::new(cx, cy), 5.0);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(AlphaColor::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)),
            None,
            &center_border,
        );

        // ── Marcas de referencia (opcional) ──
        for i in 0..=4 {
            let t = i as f64 / 4.0;
            let a = gauge_start + gauge_sweep * t;
            let inner_tick = radius - thickness - 4.0;
            let outer_tick = radius - thickness - 10.0;
            let p1 = Point::new(cx + a.cos() * inner_tick, cy + a.sin() * inner_tick);
            let p2 = Point::new(cx + a.cos() * outer_tick, cy + a.sin() * outer_tick);
            let tick = Line::new(p1, p2);
            scene.stroke(
                &Stroke::new(1.5),
                Affine::IDENTITY,
                &Brush::Solid(AlphaColor::from_rgba8(0x66, 0x66, 0x66, 0xFF)),
                None,
                &tick,
            );
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label(format!("Indicador: {:.0} de {:.0}", self.value, self.max_value));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

// ═══════════════════════════════════════════════════════════════════
// SPARKLINE WIDGET
// ═══════════════════════════════════════════════════════════════════

/// Widget de minigráfico (Sparkline) vectorial.
///
/// Similar a LineChart pero sin ejes, etiquetas ni puntos: solo la línea
/// y opcionalmente un relleno degradado bajo la curva.
pub struct SparklineWidget {
    data: Vec<f64>,
    line_color: AlphaColor,
    fill_color: Option<AlphaColor>, // relleno bajo la línea
    line_width: f64,
}

impl SparklineWidget {
    pub fn new(data: Vec<f64>, line_color: AlphaColor) -> Self {
        Self {
            data,
            line_color,
            fill_color: Some(AlphaColor::from_rgba8(
                line_color.components[0],
                line_color.components[1],
                line_color.components[2],
                40, // 15% de opacidad
            )),
            line_width: 2.0,
        }
    }

    pub fn with_fill(mut self, color: Option<AlphaColor>) -> Self {
        self.fill_color = color;
        self
    }

    pub fn with_line_width(mut self, width: f64) -> Self {
        self.line_width = width;
        self
    }
}

impl crate::Widget for SparklineWidget {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(Size::new(120.0, 30.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let w = size.width;
        let h = size.height;

        if self.data.is_empty() || w <= 0.0 || h <= 0.0 {
            return;
        }

        let max_val = self.data.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
        let count = self.data.len();
        let step_x = w / (count - 1).max(1) as f64;

        // ── Construir línea poligonal ──
        let mut path = BezPath::new();
        for (i, &v) in self.data.iter().enumerate() {
            let x = i as f64 * step_x;
            let y = h - (v / max_val * h);
            if i == 0 {
                path.move_to(Point::new(x, y));
            } else {
                path.line_to(Point::new(x, y));
            }
        }

        // ── Relleno bajo la línea ──
        if let Some(fill_color) = self.fill_color {
            let mut fill_path = BezPath::new();
            // Empezar desde abajo a la izquierda
            fill_path.move_to(Point::new(0.0, h));
            for (i, &v) in self.data.iter().enumerate() {
                let x = i as f64 * step_x;
                let y = h - (v / max_val * h);
                fill_path.line_to(Point::new(x, y));
            }
            // Cerrar hasta abajo a la derecha
            fill_path.line_to(Point::new(w, h));
            fill_path.close_path();

            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(fill_color),
                None,
                &fill_path,
            );
        }

        // ── Línea ──
        let line_brush = Brush::Solid(self.line_color);
        scene.stroke(
            &Stroke::new(self.line_width),
            Affine::IDENTITY,
            &line_brush,
            None,
            &path,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_label("Minigráfico");
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}
