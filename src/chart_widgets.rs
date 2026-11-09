// Forja GUI — Widgets de gráficos vectoriales con Vello
//
// Reemplaza la emulación anterior (barras flex, puntos Unicode, leyendas de texto)
// por dibujo vectorial real con Vello Scene, usando kurbo para formas geométricas.
//
// Incluye View wrappers Xilem para integración directa en layout_a_vista().

use std::marker::PhantomData;

use crate::accesskit::{Node, Role};
use crate::imaging::Painter;
use crate::kurbo::{BezPath, Circle, Line, Point, Rect, Size, Stroke};
use crate::layout::{LenReq, Length};
use crate::peniko::{self, Brush};
use crate::{
    AccessCtx, Axis, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NoAction, PaintCtx,
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
    pub const fn nuevo(r: u8, g: u8, b: u8, a: u8) -> Self {
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
    pub etiqueta: String,
    pub valor: f64,
    pub color: Rgba,
}

impl ChartDataPoint {
    pub fn nuevo(etiqueta: &str, valor: f64, color: Rgba) -> Self {
        Self {
            etiqueta: etiqueta.to_string(),
            valor,
            color,
        }
    }

    pub fn from_rgba8(etiqueta: &str, valor: f64, r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            etiqueta: etiqueta.to_string(),
            valor,
            color: Rgba(r, g, b, a),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helpers de dibujo vectorial
// ═══════════════════════════════════════════════════════════════════

/// Dibuja un arco aproximado usando pequeños segmentos de línea.
/// Esto evita la dependencia de `kurbo::Arc` o `BezPath::arc_to`.
fn sumar_arc_to_ruta(
    ruta: &mut BezPath,
    cx: f64,
    cy: f64,
    radius: f64,
    inicio_angle: f64,
    angulo_barrido: f64,
    segments: usize,
) {
    let segments = segments.max(4);
    for i in 1..=segments {
        let t = i as f64 / segments as f64;
        let angle = inicio_angle + angulo_barrido * t;
        let x = cx + angle.cos() * radius;
        let y = cy + angle.sin() * radius;
        ruta.line_to(Point::new(x, y));
    }
}

/// Dibuja un segmento de pastel (centro → arco → centro) usando BezPath con segmentos de línea.
fn draw_pie_segment(
    painter: &mut Painter<'_>,
    cx: f64,
    cy: f64,
    radius: f64,
    inicio_angle: f64,
    angulo_barrido: f64,
    brush: &Brush,
) {
    if angulo_barrido.abs() < 0.001 {
        return;
    }

    let mut ruta = BezPath::new();
    // Centro
    ruta.move_to(Point::new(cx, cy));
    // Primer punto en el arco
    let x0 = cx + inicio_angle.cos() * radius;
    let y0 = cy + inicio_angle.sin() * radius;
    ruta.line_to(Point::new(x0, y0));
    // Segmentos del arco
    let num_seg = (angulo_barrido.abs() / (PI / 32.0)).ceil() as usize;
    sumar_arc_to_ruta(
        &mut ruta,
        cx,
        cy,
        radius,
        inicio_angle,
        angulo_barrido,
        num_seg,
    );
    // Cerrar al centro
    ruta.close_path();

    painter.fill(&ruta, brush).draw();
}

/// Dibuja un arco grueso (anillo) usando BezPath con segmentos de línea.
/// Útil para el gauge y donut avanzado.
/// Arco de un gauge: centro, radio, grosor y barrido (radianes).
struct Arco {
    cx: f64,
    cy: f64,
    radius: f64,
    thickness: f64,
    inicio_angle: f64,
    angulo_barrido: f64,
}

fn draw_thick_arc(painter: &mut Painter<'_>, arco: &Arco, brush: &Brush) {
    let Arco {
        cx,
        cy,
        radius,
        thickness,
        inicio_angle,
        angulo_barrido,
    } = *arco;
    if angulo_barrido.abs() < 0.001 {
        return;
    }

    let outer_r = radius;
    let inner_r = radius - thickness;
    let end_angle = inicio_angle + angulo_barrido;
    let num_seg = (angulo_barrido.abs() / (PI / 32.0)).ceil() as usize;
    let num_seg = num_seg.max(4);

    let mut ruta = BezPath::new();

    // Arco exterior (start → end)
    let x0_outer = cx + inicio_angle.cos() * outer_r;
    let y0_outer = cy + inicio_angle.sin() * outer_r;
    ruta.move_to(Point::new(x0_outer, y0_outer));
    sumar_arc_to_ruta(
        &mut ruta,
        cx,
        cy,
        outer_r,
        inicio_angle,
        angulo_barrido,
        num_seg,
    );

    // Línea hacia el interior
    let x1_inner = cx + end_angle.cos() * inner_r;
    let y1_inner = cy + end_angle.sin() * inner_r;
    ruta.line_to(Point::new(x1_inner, y1_inner));

    // Arco interior (end → start, inverso)
    sumar_arc_to_ruta(
        &mut ruta,
        cx,
        cy,
        inner_r,
        end_angle,
        -angulo_barrido,
        num_seg,
    );

    ruta.close_path();
    painter.fill(&ruta, brush).draw();
}

// ═══════════════════════════════════════════════════════════════════
// LINECHART WIDGET
// ═══════════════════════════════════════════════════════════════════

/// Widget de gráfico de líneas vectorial.
pub struct LineaChartWidget {
    data: Vec<ChartDataPoint>,
    show_points: bool,
    show_grid: bool,
    linea_color: Rgba,
    linea_ancho: f64,
}

impl LineaChartWidget {
    pub fn nuevo(data: Vec<ChartDataPoint>) -> Self {
        Self {
            data,
            show_points: true,
            show_grid: true,
            linea_color: Rgba::nuevo(0x67, 0x50, 0xA4, 0xFF),
            linea_ancho: 3.0,
        }
    }

    pub fn con_linea_color(mut self, color: Rgba) -> Self {
        self.linea_color = color;
        self
    }

    pub fn con_linea_ancho(mut self, ancho: f64) -> Self {
        self.linea_ancho = ancho;
        self
    }

    pub fn con_show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    pub fn con_show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }
}

impl crate::Widget for LineaChartWidget {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _evento: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        _cross_longitud: Option<Length>,
    ) -> Length {
        // Tamaño propio fijo, achicado si el espacio disponible es menor
        // (equivalente al `bc.constrain` anterior).
        let fijo = match axis {
            Axis::Horizontal => Length::px(300.0),
            Axis::Vertical => Length::px(200.0),
        };
        match longitud_req {
            LenReq::FitContent(space) => fijo.min(space),
            _ => fijo,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let tamano = ctx.border_box().size();
        let padding = 30.0;
        let chart_w = tamano.width - padding * 2.0;
        let chart_h = tamano.height - padding * 2.0;

        if self.data.is_empty() || chart_w <= 0.0 || chart_h <= 0.0 {
            return;
        }

        let max_val = self
            .data
            .iter()
            .map(|d| d.valor)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let cantidad = self.data.len();
        let step_x = if cantidad > 1 {
            chart_w / (cantidad - 1) as f64
        } else {
            chart_w / 2.0
        };

        // ── Grid ──
        if self.show_grid {
            let grid_brush = Brush::Solid(peniko::Color::from_rgba8(0xE0, 0xE0, 0xE0, 0xFF));
            for i in 0..=4 {
                let y = padding + chart_h * (1.0 - i as f64 / 4.0);
                let linea = Line::new(Point::new(padding, y), Point::new(padding + chart_w, y));
                painter.stroke(linea, &Stroke::new(1.0), &grid_brush).draw();
            }
        }

        // ── Línea poligonal ──
        let mut ruta = BezPath::new();
        for (i, point) in self.data.iter().enumerate() {
            let x = padding + i as f64 * step_x;
            let y = padding + chart_h - (point.valor / max_val * chart_h);
            if i == 0 {
                ruta.move_to(Point::new(x, y));
            } else {
                ruta.line_to(Point::new(x, y));
            }
        }

        painter
            .stroke(
                &ruta,
                &Stroke::new(self.linea_ancho),
                &self.linea_color.to_brush(),
            )
            .draw();

        // ── Puntos ──
        if self.show_points {
            for (i, point) in self.data.iter().enumerate() {
                let x = padding + i as f64 * step_x;
                let y = padding + chart_h - (point.valor / max_val * chart_h);
                let circle = Circle::new(Point::new(x, y), 4.0);
                painter.fill(circle, &point.color.to_brush()).draw();
                // Borde blanco
                let border = Circle::new(Point::new(x, y), 4.0);
                painter
                    .stroke(
                        border,
                        &Stroke::new(1.5),
                        &Brush::Solid(peniko::Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)),
                    )
                    .draw();
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
        nodo: &mut Node,
    ) {
        nodo.set_label("Gráfico de líneas");
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
    bar_ancho: f64,
    _show_values: bool,
}

impl BarChartWidget {
    pub fn nuevo(data: Vec<ChartDataPoint>, apilado: bool) -> Self {
        Self {
            data,
            _apilado: apilado,
            bar_ancho: 40.0,
            _show_values: true,
        }
    }

    pub fn con_bar_ancho(mut self, ancho: f64) -> Self {
        self.bar_ancho = ancho;
        self
    }
}

impl crate::Widget for BarChartWidget {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _evento: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        _cross_longitud: Option<Length>,
    ) -> Length {
        // Tamaño propio fijo, achicado si el espacio disponible es menor
        // (equivalente al `bc.constrain` anterior).
        let fijo = match axis {
            Axis::Horizontal => Length::px(300.0),
            Axis::Vertical => Length::px(200.0),
        };
        match longitud_req {
            LenReq::FitContent(space) => fijo.min(space),
            _ => fijo,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let tamano = ctx.border_box().size();
        let padding = 30.0;
        let chart_w = tamano.width - padding * 2.0;
        let chart_h = tamano.height - padding * 2.0;

        if self.data.is_empty() || chart_w <= 0.0 || chart_h <= 0.0 {
            return;
        }

        let max_val = self
            .data
            .iter()
            .map(|d| d.valor)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let cantidad = self.data.len();
        let step_x = chart_w / cantidad as f64;
        let bar_w = (step_x * 0.7).min(self.bar_ancho).max(4.0);

        // Eje base
        let axis_y = padding + chart_h;
        let axis = Line::new(
            Point::new(padding, axis_y),
            Point::new(padding + chart_w, axis_y),
        );
        painter
            .stroke(
                axis,
                &Stroke::new(1.0),
                &Brush::Solid(peniko::Color::from_rgba8(0xCC, 0xCC, 0xCC, 0xFF)),
            )
            .draw();

        for (i, point) in self.data.iter().enumerate() {
            let bar_alto = (point.valor / max_val * chart_h).max(2.0);
            let x = padding + i as f64 * step_x + (step_x - bar_w) / 2.0;
            let y = padding + chart_h - bar_alto;
            let rect = Rect::new(x, y, x + bar_w, padding + chart_h);

            painter.fill(rect, &point.color.to_brush()).draw();

            // Borde sutil
            painter
                .stroke(
                    rect,
                    &Stroke::new(0.5),
                    &Brush::Solid(peniko::Color::from_rgba8(0x00, 0x00, 0x00, 0x20)),
                )
                .draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        nodo: &mut Node,
    ) {
        nodo.set_label("Gráfico de barras");
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
    pub fn nuevo(data: Vec<ChartDataPoint>) -> Self {
        Self {
            data,
            donut: false,
            donut_ratio: 0.5,
        }
    }

    pub fn con_donut(mut self, ratio: f64) -> Self {
        self.donut = true;
        self.donut_ratio = ratio.clamp(0.1, 0.9);
        self
    }

    pub fn fijar_donut(&mut self, ratio: f64) {
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
        _evento: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        _cross_longitud: Option<Length>,
    ) -> Length {
        // Tamaño propio fijo, achicado si el espacio disponible es menor
        // (equivalente al `bc.constrain` anterior).
        let fijo = match axis {
            Axis::Horizontal => Length::px(200.0),
            Axis::Vertical => Length::px(200.0),
        };
        match longitud_req {
            LenReq::FitContent(space) => fijo.min(space),
            _ => fijo,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let tamano = ctx.border_box().size();
        let cx = tamano.width / 2.0;
        let cy = tamano.height / 2.0;
        let radius = tamano.width.min(tamano.height) / 2.0 * 0.8;

        let total: f64 = self.data.iter().map(|d| d.valor).sum();
        if total <= 0.0 || radius < 5.0 {
            return;
        }

        let mut inicio_angle = -PI / 2.0;

        for point in &self.data {
            let barrido = (point.valor / total) * 2.0 * PI;
            draw_pie_segment(
                painter,
                cx,
                cy,
                radius,
                inicio_angle,
                barrido,
                &point.color.to_brush(),
            );
            inicio_angle += barrido;
        }

        // Donut: círculo central
        if self.donut {
            let inner_r = radius * self.donut_ratio;
            let inner_circle = Circle::new(Point::new(cx, cy), inner_r);
            painter
                .fill(
                    inner_circle,
                    &Brush::Solid(peniko::Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)),
                )
                .draw();
            painter
                .stroke(
                    inner_circle,
                    &Stroke::new(1.0),
                    &Brush::Solid(peniko::Color::from_rgba8(0xDD, 0xDD, 0xDD, 0xFF)),
                )
                .draw();
        }

        // Borde exterior
        let outer_circle = Circle::new(Point::new(cx, cy), radius);
        painter
            .stroke(
                outer_circle,
                &Stroke::new(1.0),
                &Brush::Solid(peniko::Color::from_rgba8(0x40, 0x40, 0x40, 0x30)),
            )
            .draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        nodo: &mut Node,
    ) {
        nodo.set_label(if self.donut {
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
    valor: f64,
    max_valor: f64,
    threshold: f64,
    low_color: Rgba,
    high_color: Rgba,
    show_etiqueta: bool,
}

impl GaugeChartWidget {
    pub fn nuevo(valor: f64, max_valor: f64, threshold: f64) -> Self {
        Self {
            valor,
            max_valor: max_valor.max(1.0),
            threshold,
            low_color: Rgba::nuevo(0x4C, 0xAF, 0x50, 0xFF),
            high_color: Rgba::nuevo(0xF4, 0x43, 0x36, 0xFF),
            show_etiqueta: true,
        }
    }

    pub fn con_colors(mut self, low: Rgba, high: Rgba) -> Self {
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
        _evento: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        _cross_longitud: Option<Length>,
    ) -> Length {
        // Tamaño propio fijo, achicado si el espacio disponible es menor
        // (equivalente al `bc.constrain` anterior).
        let fijo = match axis {
            Axis::Horizontal => Length::px(200.0),
            Axis::Vertical => Length::px(200.0),
        };
        match longitud_req {
            LenReq::FitContent(space) => fijo.min(space),
            _ => fijo,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let tamano = ctx.border_box().size();
        let cx = tamano.width / 2.0;
        let cy = tamano.height * 0.65;
        let radius = tamano.width.min(tamano.height) / 2.0 * 0.55;
        let thickness = 14.0;

        if radius < 10.0 {
            return;
        }

        let progress = (self.valor / self.max_valor).clamp(0.0, 1.0);
        let color = if self.valor > self.threshold {
            self.high_color
        } else {
            self.low_color
        };

        let gauge_inicio = 0.75 * PI; // 135°
        let barrido_indicador = 1.5 * PI; // 270°

        // Arco de fondo
        draw_thick_arc(
            painter,
            &Arco {
                cx,
                cy,
                radius,
                thickness,
                inicio_angle: gauge_inicio,
                angulo_barrido: barrido_indicador,
            },
            &Brush::Solid(peniko::Color::from_rgba8(0xE8, 0xE8, 0xE8, 0xFF)),
        );

        // Arco de progreso
        if progress > 0.0 {
            draw_thick_arc(
                painter,
                &Arco {
                    cx,
                    cy,
                    radius,
                    thickness,
                    inicio_angle: gauge_inicio,
                    angulo_barrido: barrido_indicador * progress,
                },
                &color.to_brush(),
            );
        }

        // Aguja
        let angle = gauge_inicio + barrido_indicador * progress;
        let needle_longitud = radius * 0.65;
        let needle_end = Point::new(
            cx + angle.cos() * needle_longitud,
            cy + angle.sin() * needle_longitud,
        );
        let needle = Line::new(Point::new(cx, cy), needle_end);
        painter
            .stroke(
                needle,
                &Stroke::new(2.5),
                &Brush::Solid(peniko::Color::from_rgba8(0x33, 0x33, 0x33, 0xFF)),
            )
            .draw();

        // Círculo central
        let center = Circle::new(Point::new(cx, cy), 5.0);
        painter
            .fill(
                center,
                &Brush::Solid(peniko::Color::from_rgba8(0x33, 0x33, 0x33, 0xFF)),
            )
            .draw();
        painter
            .stroke(
                center,
                &Stroke::new(1.0),
                &Brush::Solid(peniko::Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF)),
            )
            .draw();

        // Marcas de referencia
        for i in 0..=4 {
            let t = i as f64 / 4.0;
            let a = gauge_inicio + barrido_indicador * t;
            let inner_tick = radius - thickness - 4.0;
            let outer_tick = radius - thickness - 10.0;
            let p1 = Point::new(cx + a.cos() * inner_tick, cy + a.sin() * inner_tick);
            let p2 = Point::new(cx + a.cos() * outer_tick, cy + a.sin() * outer_tick);
            painter
                .stroke(
                    Line::new(p1, p2),
                    &Stroke::new(1.5),
                    &Brush::Solid(peniko::Color::from_rgba8(0x66, 0x66, 0x66, 0xFF)),
                )
                .draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        nodo: &mut Node,
    ) {
        nodo.set_label(format!(
            "Indicador: {:.0} de {:.0}",
            self.valor, self.max_valor
        ));
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
    linea_color: Rgba,
    fill_color: Option<Rgba>,
    linea_ancho: f64,
}

impl SparklineWidget {
    pub fn nuevo(data: Vec<f64>, linea_color: Rgba) -> Self {
        Self {
            data,
            linea_color,
            fill_color: Some(Rgba(
                linea_color.0,
                linea_color.1,
                linea_color.2,
                40, // 15% opacidad
            )),
            linea_ancho: 2.0,
        }
    }

    pub fn con_fill(mut self, color: Option<Rgba>) -> Self {
        self.fill_color = color;
        self
    }

    pub fn con_linea_ancho(mut self, ancho: f64) -> Self {
        self.linea_ancho = ancho;
        self
    }
}

impl crate::Widget for SparklineWidget {
    type Action = NoAction;

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _evento: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        _cross_longitud: Option<Length>,
    ) -> Length {
        // Tamaño propio fijo, achicado si el espacio disponible es menor
        // (equivalente al `bc.constrain` anterior).
        let fijo = match axis {
            Axis::Horizontal => Length::px(120.0),
            Axis::Vertical => Length::px(30.0),
        };
        match longitud_req {
            LenReq::FitContent(space) => fijo.min(space),
            _ => fijo,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let tamano = ctx.border_box().size();
        let w = tamano.width;
        let h = tamano.height;

        if self.data.is_empty() || w <= 0.0 || h <= 0.0 {
            return;
        }

        let max_val = self.data.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
        let cantidad = self.data.len();
        let step_x = w / (cantidad - 1).max(1) as f64;

        // Relleno bajo la línea
        if let Some(fill_color) = self.fill_color {
            let mut fill_ruta = BezPath::new();
            fill_ruta.move_to(Point::new(0.0, h));
            for (i, &v) in self.data.iter().enumerate() {
                let x = i as f64 * step_x;
                let y = h - (v / max_val * h);
                fill_ruta.line_to(Point::new(x, y));
            }
            fill_ruta.line_to(Point::new(w, h));
            fill_ruta.close_path();
            painter.fill(&fill_ruta, &fill_color.to_brush()).draw();
        }

        // Línea
        let mut ruta = BezPath::new();
        for (i, &v) in self.data.iter().enumerate() {
            let x = i as f64 * step_x;
            let y = h - (v / max_val * h);
            if i == 0 {
                ruta.move_to(Point::new(x, y));
            } else {
                ruta.line_to(Point::new(x, y));
            }
        }

        painter
            .stroke(
                &ruta,
                &Stroke::new(self.linea_ancho),
                &self.linea_color.to_brush(),
            )
            .draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        nodo: &mut Node,
    ) {
        nodo.set_label("Minigráfico");
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

// ═══════════════════════════════════════════════════════════════════
// XILEM VIEW WRAPPERS
// ═══════════════════════════════════════════════════════════════════
//
// Estos wrappers permiten usar los chart widgets desde layout_a_vista()
// devolviendo Box<AnyWidgetView<AppStateNativo>>.

// ─── LineChartView ────────────────────────────────────────────────

pub struct LineaChartVista<State> {
    data: Vec<ChartDataPoint>,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> LineaChartVista<State> {
    pub fn nuevo(data: Vec<ChartDataPoint>) -> Self {
        Self {
            data,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for LineaChartVista<State> {}

impl<State: 'static> View<State, (), ViewCtx> for LineaChartVista<State> {
    type Element = Pod<LineaChartWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<LineaChartWidget>, ()) {
        (
            ctx.create_pod(LineaChartWidget::nuevo(self.data.clone())),
            (),
        )
    }

    fn rebuild(
        &self,
        _anterior: &Self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<LineaChartWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<LineaChartWidget>>,
    ) {
    }

    fn message(
        &self,
        _vista_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<LineaChartWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

// ─── BarChartView ─────────────────────────────────────────────────

pub struct BarChartVista<State> {
    data: Vec<ChartDataPoint>,
    apilado: bool,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> BarChartVista<State> {
    pub fn nuevo(data: Vec<ChartDataPoint>, apilado: bool) -> Self {
        Self {
            data,
            apilado,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for BarChartVista<State> {}

impl<State: 'static> View<State, (), ViewCtx> for BarChartVista<State> {
    type Element = Pod<BarChartWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<BarChartWidget>, ()) {
        (
            ctx.create_pod(BarChartWidget::nuevo(self.data.clone(), self.apilado)),
            (),
        )
    }

    fn rebuild(
        &self,
        _anterior: &Self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<BarChartWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<BarChartWidget>>,
    ) {
    }

    fn message(
        &self,
        _vista_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<BarChartWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

// ─── PieChartView ─────────────────────────────────────────────────

pub struct PieChartVista<State> {
    data: Vec<ChartDataPoint>,
    donut: bool,
    donut_ratio: f64,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> PieChartVista<State> {
    pub fn nuevo(data: Vec<ChartDataPoint>, donut: bool, donut_ratio: f64) -> Self {
        Self {
            data,
            donut,
            donut_ratio,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for PieChartVista<State> {}

impl<State: 'static> View<State, (), ViewCtx> for PieChartVista<State> {
    type Element = Pod<PieChartWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<PieChartWidget>, ()) {
        let mut widget = PieChartWidget::nuevo(self.data.clone());
        if self.donut {
            widget.fijar_donut(self.donut_ratio);
        }
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        _anterior: &Self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<PieChartWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<PieChartWidget>>,
    ) {
    }

    fn message(
        &self,
        _vista_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<PieChartWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

// ─── GaugeChartView ───────────────────────────────────────────────

pub struct GaugeChartVista<State> {
    valor: f64,
    max_valor: f64,
    threshold: f64,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> GaugeChartVista<State> {
    pub fn nuevo(valor: f64, max_valor: f64, threshold: f64) -> Self {
        Self {
            valor,
            max_valor,
            threshold,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for GaugeChartVista<State> {}

impl<State: 'static> View<State, (), ViewCtx> for GaugeChartVista<State> {
    type Element = Pod<GaugeChartWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<GaugeChartWidget>, ()) {
        let verde = Rgba::nuevo(0x4C, 0xAF, 0x50, 0xFF);
        let rojo = Rgba::nuevo(0xF4, 0x43, 0x36, 0xFF);
        let widget = GaugeChartWidget::nuevo(self.valor, self.max_valor, self.threshold)
            .con_colors(verde, rojo);
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        _anterior: &Self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<GaugeChartWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<GaugeChartWidget>>,
    ) {
    }

    fn message(
        &self,
        _vista_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<GaugeChartWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

// ─── SparklineView ────────────────────────────────────────────────

pub struct SparklineVista<State> {
    data: Vec<f64>,
    color: Rgba,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> SparklineVista<State> {
    pub fn nuevo(data: Vec<f64>, color: Rgba) -> Self {
        Self {
            data,
            color,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for SparklineVista<State> {}

impl<State: 'static> View<State, (), ViewCtx> for SparklineVista<State> {
    type Element = Pod<SparklineWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<SparklineWidget>, ()) {
        (
            ctx.create_pod(SparklineWidget::nuevo(self.data.clone(), self.color)),
            (),
        )
    }

    fn rebuild(
        &self,
        _anterior: &Self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<SparklineWidget>>,
        _app_state: &mut State,
    ) {
    }

    fn teardown(
        &self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<SparklineWidget>>,
    ) {
    }

    fn message(
        &self,
        _vista_state: &mut (),
        _message: &mut MessageContext,
        _element: Mut<'_, Pod<SparklineWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}
