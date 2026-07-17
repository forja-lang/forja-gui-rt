// Forja GUI — Widgets de gráficos vectoriales con Vello
//
// Reemplaza la emulación anterior (barras flex, puntos Unicode, leyendas de texto)
// por dibujo vectorial real con Vello Scene, usando kurbo para formas geométricas.
//
// Incluye View wrappers Xilem para integración directa en layout_a_view().

use std::marker::PhantomData;

use crate::accesskit::{Node, Role};
use crate::vello::kurbo::{Affine, BezPath, Circle, Line, Point, Rect, Size, Stroke};
use crate::vello::peniko::{self, Brush, Fill};
use crate::vello::Scene;
use crate::{
    AccessCtx, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NoAction, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx,
};
use crate::{MessageContext, MessageResult, Mut, Pod, View, ViewCtx, ViewMarker};
use std::f64::consts::PI;

// ═══════════════════════════════════════════════════════════════════
// DATA: Estructura de datos compartida para todos los charts
// ═══════════════════════════════════════════════════════════════════

/// Representación compacta de color RGBA para almacenar en widgets
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(r, g, b, a)
    }

    pub fn to_peniko_color(&self) -> peniko::Color {
        peniko::Color::from_rgba8(self.0, self.1, self.2, self.3)
    }

    pub fn to_brush(&self) -> Brush {
        Brush::Solid(self.to_peniko_color())
    }
}


/// Un punto individual de datos para cualquier chart
#[derive(Debug, Clone)]
pub struct ChartDataPoint {
    pub label: String,
    pub value: f64,
    pub color: Rgba,
}

impl ChartDataPoint {
    pub fn new(label: &str, value: f64, color: Rgba) -> Self {
        Self {
            label: label.to_string(),
            value,
            color,
        }
    }

    pub fn from_rgba8(label: &str, value: f64, r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            label: label.to_string(),
            value,
            color: Rgba(r, g, b, a),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helpers de dibujo vectorial
// ═══════════════════════════════════════════════════════════════════

/// Dibuja un arco aproximado usando pequeños segmentos de línea.
/// Esto evita la dependencia de `kurbo::Arc` o `BezPath::arc_to`.
fn add_arc_to_path(
    path: &mut BezPath,
    cx: f64,
    cy: f64,
    radius: f64,
    start_angle: f64,
    sweep_angle: f64,
    segments: usize,
) {
    let segments = segments.max(4);
    for i in 1..=segments {
        let t = i as f64 / segments as f64;
        let angle = start_angle + sweep_angle * t;
        let x = cx + angle.cos() * radius;
        let y = cy + angle.sin() * radius;
        path.line_to(Point::new(x, y));
    }
}

/// Dibuja un segmento de pastel (centro → arco → centro) usando BezPath con segmentos de línea.
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

    let mut path = BezPath::new();
    // Centro
    path.move_to(Point::new(cx, cy));
    // Primer punto en el arco
    let x0 = cx + start_angle.cos() * radius;
    let y0 = cy + start_angle.sin() * radius;
    path.line_to(Point::new(x0, y0));
    // Segmentos del arco
    let num_seg = (sweep_angle.abs() / (PI / 32.0)).ceil() as usize;
    add_arc_to_path(&mut path, cx, cy, radius, start_angle, sweep_angle, num_seg);
    // Cerrar al centro
    path.close_path();

    scene.fill(Fill::NonZero, Affine::IDENTITY, brush, None, &path);
}

/// Dibuja un arco grueso (anillo) usando BezPath con segmentos de línea.
/// Útil para el gauge y donut avanzado.
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
    let num_seg = (sweep_angle.abs() / (PI / 32.0)).ceil() as usize;
    let num_seg = num_seg.max(4);

    let mut path = BezPath::new();

    // Arco exterior (start → end)
    let x0_outer = cx + start_angle.cos() * outer_r;
    let y0_outer = cy + start_angle.sin() * outer_r;
    path.move_to(Point::new(x0_outer, y0_outer));
    add_arc_to_path(&mut path, cx, cy, outer_r, start_angle, sweep_angle, num_seg);

    // Línea hacia el interior
    let x1_inner = cx + end_angle.cos() * inner_r;
    let y1_inner = cy + end_angle.sin() * inner_r;
    path.line_to(Point::new(x1_inner, y1_inner));

    // Arco interior (end → start, inverso)
    add_arc_to_path(&mut path, cx, cy, inner_r, end_angle, -sweep_angle, num_seg);

    path.close_path();
    scene.fill(Fill::NonZero, Affine::IDENTITY, brush, None, &path);
}

// ═══════════════════════════════════════════════════════════════════
// LINECHART WIDGET
// ═══════════════════════════════════════════════════════════════════

/// Widget de gráfico de líneas vectorial.
pub struct LineChartWidget {
    data: Vec<ChartDataPoint>,
    show_points: bool,
    show_grid: bool,
    line_color: Rgba,
    line_width: f64,
}

impl LineChartWidget {
    pub fn new(data: Vec<ChartDataPoint>) -> Self {
        Self {
            data,
            show_points: true,
            show_grid: true,
            line_color: Rgba::new(0x67, 0x50, 0xA4, 0xFF),
            line_width: 3.0,
        }
    }

    pub fn with_line_color(mut self, color: Rgba) -> Self {
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

        let max_val = self.data.iter().map(|d| d.value).fold(0.0_f64, f64::max).max(1.0);
        let count = self.data.len();
        let step_x = if count > 1 {
            chart_w / (count - 1) as f64
        } else {
            chart_w / 2.0
        };

        // ── Grid ──
        if self.show_grid {
            let grid_brush = Brush::Solid(peniko::Color::from_rgba8(0xE0, 0xE0, 0xE0, 0xFF));
            for i in 0..=4 {
                let y = padding + chart_h * (1.0 - i as f64 / 4.0);
                let line = Line::new(Point::new(padding, y), Point::new(padding + chart_w, y));
                scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, &grid_brush, None, &line);
            }
        }

        // ── Línea poligonal ──
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

        scene.stroke(
            &Stroke::new(self.line_width),
            Affine::IDENTITY,
            &self.line_color.to_brush(),
            None,
            &path,
        );

        // ── Puntos ──
        if self.show_points {
            for (i, point) in self.data.iter().enumerate() {
                let x = padding + i as f64 * step_x;
                let y = padding + chart_h - (point.value / max_val * chart_h);
                let circle = Circle::new(Point::new(x, y), 4.0);
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &point.color.to_brush(),
                    None,
                    &circle,
                );
                // Borde blanco
                let border = Circle::new(Point::new(x, y), 4.0);
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    &Brush::Solid(peniko::Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)),
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

pub struct BarChartWidget {
    data: Vec<ChartDataPoint>,
    _apilado: bool,
    bar_width: f64,
    _show_values: bool,
}

impl BarChartWidget {
    pub fn new(data: Vec<ChartDataPoint>, apilado: bool) -> Self {
        Self {
            data,
            _apilado: apilado,
            bar_width: 40.0,
            _show_values: true,
        }
    }

    pub fn with_bar_width(mut self, width: f64) -> Self {
        self.bar_width = width;
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
        let step_x = chart_w / count as f64;
        let bar_w = (step_x * 0.7).min(self.bar_width).max(4.0);

        // Eje base
        let axis_y = padding + chart_h;
        let axis = Line::new(Point::new(padding, axis_y), Point::new(padding + chart_w, axis_y));
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(peniko::Color::from_rgba8(0xCC, 0xCC, 0xCC, 0xFF)),
            None,
            &axis,
        );

        for (i, point) in self.data.iter().enumerate() {
            let bar_height = (point.value / max_val * chart_h).max(2.0);
            let x = padding + i as f64 * step_x + (step_x - bar_w) / 2.0;
            let y = padding + chart_h - bar_height;
            let rect = Rect::new(x, y, x + bar_w, padding + chart_h);

            scene.fill(Fill::NonZero, Affine::IDENTITY, &point.color.to_brush(), None, &rect);

            // Borde sutil
            scene.stroke(
                &Stroke::new(0.5),
                Affine::IDENTITY,
                &Brush::Solid(peniko::Color::from_rgba8(0x00, 0x00, 0x00, 0x20)),
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

pub struct PieChartWidget {
    data: Vec<ChartDataPoint>,
    donut: bool,
    donut_ratio: f64,
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

        let mut start_angle = -PI / 2.0;

        for point in &self.data {
            let sweep = (point.value / total) * 2.0 * PI;
            draw_pie_segment(scene, cx, cy, radius, start_angle, sweep, &point.color.to_brush());
            start_angle += sweep;
        }

        // Donut: círculo central
        if self.donut {
            let inner_r = radius * self.donut_ratio;
            let inner_circle = Circle::new(Point::new(cx, cy), inner_r);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &Brush::Solid(peniko::Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)),
                None,
                &inner_circle,
            );
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                &Brush::Solid(peniko::Color::from_rgba8(0xDD, 0xDD, 0xDD, 0xFF)),
                None,
                &inner_circle,
            );
        }

        // Borde exterior
        let outer_circle = Circle::new(Point::new(cx, cy), radius);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(peniko::Color::from_rgba8(0x40, 0x40, 0x40, 0x30)),
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
        node.set_label(if self.donut {
            "Gráfico de donut"
        } else {
            "Gráfico de pastel"
        });
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

// ═══════════════════════════════════════════════════════════════════
// GAUGE CHART WIDGET
// ═══════════════════════════════════════════════════════════════════

pub struct GaugeChartWidget {
    value: f64,
    max_value: f64,
    threshold: f64,
    low_color: Rgba,
    high_color: Rgba,
    _show_label: bool,
}

impl GaugeChartWidget {
    pub fn new(value: f64, max_value: f64, threshold: f64) -> Self {
        Self {
            value,
            max_value: max_value.max(1.0),
            threshold,
            low_color: Rgba::new(0x4C, 0xAF, 0x50, 0xFF),
            high_color: Rgba::new(0xF4, 0x43, 0x36, 0xFF),
            _show_label: true,
        }
    }

    pub fn with_colors(mut self, low: Rgba, high: Rgba) -> Self {
        self.low_color = low;
        self.high_color = high;
        self
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

        let gauge_start = 0.75 * PI; // 135°
        let gauge_sweep = 1.5 * PI; // 270°

        // Arco de fondo
        draw_thick_arc(
            scene,
            cx,
            cy,
            radius,
            thickness,
            gauge_start,
            gauge_sweep,
            &Brush::Solid(peniko::Color::from_rgba8(0xE8, 0xE8, 0xE8, 0xFF)),
        );

        // Arco de progreso
        if progress > 0.0 {
            draw_thick_arc(
                scene,
                cx,
                cy,
                radius,
                thickness,
                gauge_start,
                gauge_sweep * progress,
                &color.to_brush(),
            );
        }

        // Aguja
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
            &Brush::Solid(peniko::Color::from_rgba8(0x33, 0x33, 0x33, 0xFF)),
            None,
            &needle,
        );

        // Círculo central
        let center = Circle::new(Point::new(cx, cy), 5.0);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(peniko::Color::from_rgba8(0x33, 0x33, 0x33, 0xFF)),
            None,
            &center,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            &Brush::Solid(peniko::Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)),
            None,
            &center,
        );

        // Marcas de referencia
        for i in 0..=4 {
            let t = i as f64 / 4.0;
            let a = gauge_start + gauge_sweep * t;
            let inner_tick = radius - thickness - 4.0;
            let outer_tick = radius - thickness - 10.0;
            let p1 = Point::new(cx + a.cos() * inner_tick, cy + a.sin() * inner_tick);
            let p2 = Point::new(cx + a.cos() * outer_tick, cy + a.sin() * outer_tick);
            scene.stroke(
                &Stroke::new(1.5),
                Affine::IDENTITY,
                &Brush::Solid(peniko::Color::from_rgba8(0x66, 0x66, 0x66, 0xFF)),
                None,
                &Line::new(p1, p2),
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

pub struct SparklineWidget {
    data: Vec<f64>,
    line_color: Rgba,
    fill_color: Option<Rgba>,
    line_width: f64,
}

impl SparklineWidget {
    pub fn new(data: Vec<f64>, line_color: Rgba) -> Self {
        Self {
            data,
            line_color,
            fill_color: Some(Rgba(
                line_color.0,
                line_color.1,
                line_color.2,
                40, // 15% opacidad
            )),
            line_width: 2.0,
        }
    }

    pub fn with_fill(mut self, color: Option<Rgba>) -> Self {
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

        // Relleno bajo la línea
        if let Some(fill_color) = self.fill_color {
            let mut fill_path = BezPath::new();
            fill_path.move_to(Point::new(0.0, h));
            for (i, &v) in self.data.iter().enumerate() {
                let x = i as f64 * step_x;
                let y = h - (v / max_val * h);
                fill_path.line_to(Point::new(x, y));
            }
            fill_path.line_to(Point::new(w, h));
            fill_path.close_path();
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                &fill_color.to_brush(),
                None,
                &fill_path,
            );
        }

        // Línea
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

        scene.stroke(
            &Stroke::new(self.line_width),
            Affine::IDENTITY,
            &self.line_color.to_brush(),
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

// ═══════════════════════════════════════════════════════════════════
// XILEM VIEW WRAPPERS
// ═══════════════════════════════════════════════════════════════════
//
// Estos wrappers permiten usar los chart widgets desde layout_a_view()
// devolviendo Box<AnyWidgetView<AppStateNativo>>.

// ─── LineChartView ────────────────────────────────────────────────

pub struct LineChartView<State> {
    data: Vec<ChartDataPoint>,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> LineChartView<State> {
    pub fn new(data: Vec<ChartDataPoint>) -> Self {
        Self {
            data,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for LineChartView<State> {}

impl<State: 'static> View<State, (), ViewCtx> for LineChartView<State> {
    type Element = Pod<LineChartWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<LineChartWidget>, ()) {
        (ctx.create_pod(LineChartWidget::new(self.data.clone())), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<LineChartWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<LineChartWidget>>,
    ) {
    }

    fn message(
        &self,
        _view_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<LineChartWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

// ─── BarChartView ─────────────────────────────────────────────────

pub struct BarChartView<State> {
    data: Vec<ChartDataPoint>,
    apilado: bool,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> BarChartView<State> {
    pub fn new(data: Vec<ChartDataPoint>, apilado: bool) -> Self {
        Self {
            data,
            apilado,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for BarChartView<State> {}

impl<State: 'static> View<State, (), ViewCtx> for BarChartView<State> {
    type Element = Pod<BarChartWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<BarChartWidget>, ()) {
        (ctx.create_pod(BarChartWidget::new(self.data.clone(), self.apilado)), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<BarChartWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<BarChartWidget>>,
    ) {
    }

    fn message(
        &self,
        _view_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<BarChartWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

// ─── PieChartView ─────────────────────────────────────────────────

pub struct PieChartView<State> {
    data: Vec<ChartDataPoint>,
    donut: bool,
    donut_ratio: f64,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> PieChartView<State> {
    pub fn new(data: Vec<ChartDataPoint>, donut: bool, donut_ratio: f64) -> Self {
        Self {
            data,
            donut,
            donut_ratio,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for PieChartView<State> {}

impl<State: 'static> View<State, (), ViewCtx> for PieChartView<State> {
    type Element = Pod<PieChartWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<PieChartWidget>, ()) {
        let mut widget = PieChartWidget::new(self.data.clone());
        if self.donut {
            widget.set_donut(self.donut_ratio);
        }
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<PieChartWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<PieChartWidget>>,
    ) {
    }

    fn message(
        &self,
        _view_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<PieChartWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

// ─── GaugeChartView ───────────────────────────────────────────────

pub struct GaugeChartView<State> {
    value: f64,
    max_value: f64,
    threshold: f64,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> GaugeChartView<State> {
    pub fn new(value: f64, max_value: f64, threshold: f64) -> Self {
        Self {
            value,
            max_value,
            threshold,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for GaugeChartView<State> {}

impl<State: 'static> View<State, (), ViewCtx> for GaugeChartView<State> {
    type Element = Pod<GaugeChartWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<GaugeChartWidget>, ()) {
        let verde = Rgba::new(0x4C, 0xAF, 0x50, 0xFF);
        let rojo = Rgba::new(0xF4, 0x43, 0x36, 0xFF);
        let widget = GaugeChartWidget::new(self.value, self.max_value, self.threshold)
            .with_colors(verde, rojo);
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<GaugeChartWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<GaugeChartWidget>>,
    ) {
    }

    fn message(
        &self,
        _view_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<GaugeChartWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

// ─── SparklineView ────────────────────────────────────────────────

pub struct SparklineView<State> {
    data: Vec<f64>,
    color: Rgba,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> SparklineView<State> {
    pub fn new(data: Vec<f64>, color: Rgba) -> Self {
        Self {
            data,
            color,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for SparklineView<State> {}

impl<State: 'static> View<State, (), ViewCtx> for SparklineView<State> {
    type Element = Pod<SparklineWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<SparklineWidget>, ()) {
        (ctx.create_pod(SparklineWidget::new(self.data.clone(), self.color)), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<SparklineWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<SparklineWidget>>,
    ) {
    }

    fn message(
        &self,
        _view_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<SparklineWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}
