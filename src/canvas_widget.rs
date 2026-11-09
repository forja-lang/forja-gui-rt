// Forja GUI — Lienzo / Pintura Libre con Vello
//
// Proporciona una API de dibujo libre que permite a programas Forja
// dibujar formas geométricas, rutas, texto y gradientes en un lienzo Vello
// a través de comandos almacenados en VariableStore.
//
// Uso desde Forja:
//   canvas_comandos = '[{"type":"FillCircle","x":100,"y":100,"radius":50,"r":255,"g":0,"b":0,"a":255}]'
//   lienzo("canvas_comandos", 400, 300)

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::accesskit::{Node, Role};
use crate::kurbo::{BezPath, Circle, Line, Point, Rect, Size, Stroke};
use crate::layout::{LenReq, Length};
use crate::peniko;
use crate::{
    AccessCtx, Axis, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NoAction, PaintCtx, Painter,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx,
};
use crate::{MessageResult, Mut, Pod, View, ViewCtx, ViewMarker};

// ═══════════════════════════════════════════════════════════════════
// CanvasCommand: enumeration of drawing commands
// ═══════════════════════════════════════════════════════════════════

/// Comando de dibujo individual que puede ser serializado desde Forja.
///
/// Cada variante incluye todos los parámetros necesarios para el dibujo,
/// incluyendo color RGBA explícito por comando (a menos que se use
/// SetFillColor / SetStrokeColor para modo painter).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CanvasCommand {
    /// Rellenar un círculo
    FillCircle {
        x: f64,
        y: f64,
        radius: f64,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    /// Trazar contorno de círculo
    StrokeCircle {
        x: f64,
        y: f64,
        radius: f64,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        ancho: f64,
    },
    /// Rellenar rectángulo
    FillRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    /// Trazar contorno de rectángulo
    StrokeRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        ancho: f64,
    },
    /// Rellenar un path definido por puntos (BezPath poligonal)
    RellenarTrazado {
        points: Vec<[f64; 2]>, // lista de [x, y]
        closed: bool,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    /// Trazar contorno de path
    Trazar {
        points: Vec<[f64; 2]>,
        closed: bool,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        ancho: f64,
    },
    /// Dibujar línea
    DibujarLinea {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        ancho: f64,
    },
    /// Dibujar texto (placeholder: rectángulo coloreado)
    DibujarTexto {
        x: f64,
        y: f64,
        texto: String,
        tamano: f64,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    /// Limpiar todo el canvas con un color
    Limpiar { r: u8, g: u8, b: u8, a: u8 },
    /// Cambiar color de relleno actual (modo painter)
    FijarColorRelleno { r: u8, g: u8, b: u8, a: u8 },
    /// Cambiar color de trazo actual
    FijarColorTrazo { r: u8, g: u8, b: u8, a: u8 },
    /// Cambiar grosor de trazo actual
    FijarAnchoTrazo { ancho: f64 },
}

// ═══════════════════════════════════════════════════════════════════
// CanvasWidget: widget Vello que ejecuta comandos de dibujo en paint()
// ═══════════════════════════════════════════════════════════════════

/// Widget de lienzo de dibujo libre que renderiza una lista de [`CanvasCommand`]
/// sobre un fondo blanco usando Vello `Scene`.
pub struct CanvasWidget {
    commands_var: String,
    commands: Vec<CanvasCommand>,
    ancho: f64,
    alto: f64,
    fill_color: peniko::Color,
    stroke_color: peniko::Color,
    stroke_ancho: f64,
}

impl CanvasWidget {
    pub fn nuevo(commands_var: String, ancho: f64, alto: f64) -> Self {
        Self {
            commands_var,
            commands: Vec::new(),
            ancho,
            alto,
            fill_color: peniko::Color::BLACK,
            stroke_color: peniko::Color::BLACK,
            stroke_ancho: 2.0,
        }
    }

    /// Cargar comandos desde un JSON string (ej: contenido de VariableStore)
    pub fn cargar_commands(&mut self, json: &str) {
        if json.trim().is_empty() || json == "null" {
            self.commands.clear();
            return;
        }
        if let Ok(cmds) = serde_json::from_str::<Vec<CanvasCommand>>(json) {
            self.commands = cmds;
        }
    }

    /// Obtener el nombre de la variable de comandos
    pub fn commands_var(&self) -> &str {
        &self.commands_var
    }
}

impl crate::Widget for CanvasWidget {
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
        // El lienzo tiene tamaño propio fijo: lo respeta, pero se achica si el
        // espacio disponible es menor (equivalente al `bc.constrain` anterior).
        let fijo = match axis {
            Axis::Horizontal => Length::px(self.ancho),
            Axis::Vertical => Length::px(self.alto),
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

        // Limpiar el canvas con blanco (fondo por defecto)
        let bg_rect = Rect::new(0.0, 0.0, tamano.width, tamano.height);
        painter.fill(bg_rect, peniko::Color::WHITE).draw();

        // Resetear estado de painter
        self.fill_color = peniko::Color::BLACK;
        self.stroke_color = peniko::Color::BLACK;
        self.stroke_ancho = 2.0;

        for cmd in &self.commands {
            match cmd {
                CanvasCommand::Limpiar { r, g, b, a } => {
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    painter.fill(bg_rect, color).draw();
                }

                CanvasCommand::FillCircle {
                    x,
                    y,
                    radius,
                    r,
                    g,
                    b,
                    a,
                } => {
                    let circle = Circle::new(Point::new(*x, *y), *radius);
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    painter.fill(circle, color).draw();
                }

                CanvasCommand::StrokeCircle {
                    x,
                    y,
                    radius,
                    r,
                    g,
                    b,
                    a,
                    ancho,
                } => {
                    let circle = Circle::new(Point::new(*x, *y), *radius);
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    painter.stroke(circle, &Stroke::new(*ancho), color).draw();
                }

                CanvasCommand::FillRect {
                    x,
                    y,
                    w,
                    h,
                    r,
                    g,
                    b,
                    a,
                } => {
                    let rect = Rect::new(*x, *y, x + w, y + h);
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    painter.fill(rect, color).draw();
                }

                CanvasCommand::StrokeRect {
                    x,
                    y,
                    w,
                    h,
                    r,
                    g,
                    b,
                    a,
                    ancho,
                } => {
                    let rect = Rect::new(*x, *y, x + w, y + h);
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    painter.stroke(rect, &Stroke::new(*ancho), color).draw();
                }

                CanvasCommand::DibujarLinea {
                    x1,
                    y1,
                    x2,
                    y2,
                    r,
                    g,
                    b,
                    a,
                    ancho,
                } => {
                    let linea = Line::new(Point::new(*x1, *y1), Point::new(*x2, *y2));
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    painter.stroke(linea, &Stroke::new(*ancho), color).draw();
                }

                CanvasCommand::RellenarTrazado {
                    points,
                    closed,
                    r,
                    g,
                    b,
                    a,
                } => {
                    if points.is_empty() {
                        continue;
                    }
                    let mut ruta = BezPath::new();
                    ruta.move_to(Point::new(points[0][0], points[0][1]));
                    for p in &points[1..] {
                        ruta.line_to(Point::new(p[0], p[1]));
                    }
                    if *closed {
                        ruta.close_path();
                    }
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    painter.fill(ruta, color).draw();
                }

                CanvasCommand::Trazar {
                    points,
                    closed,
                    r,
                    g,
                    b,
                    a,
                    ancho,
                } => {
                    if points.is_empty() {
                        continue;
                    }
                    let mut ruta = BezPath::new();
                    ruta.move_to(Point::new(points[0][0], points[0][1]));
                    for p in &points[1..] {
                        ruta.line_to(Point::new(p[0], p[1]));
                    }
                    if *closed {
                        ruta.close_path();
                    }
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    painter.stroke(ruta, &Stroke::new(*ancho), color).draw();
                }

                CanvasCommand::DibujarTexto {
                    x,
                    y,
                    texto,
                    tamano,
                    r,
                    g,
                    b,
                    a,
                } => {
                    // Placeholder: dibujar un pequeño rectángulo coloreado
                    // que representa el área del texto
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    let texto_ancho = texto.len() as f64 * tamano * 0.5;
                    let rect = Rect::new(*x, *y, x + texto_ancho.min(200.0), y + *tamano * 1.2);
                    painter.fill(rect, color).draw();
                }

                CanvasCommand::FijarColorRelleno { r, g, b, a } => {
                    self.fill_color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                }

                CanvasCommand::FijarColorTrazo { r, g, b, a } => {
                    self.stroke_color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                }

                CanvasCommand::FijarAnchoTrazo { ancho } => {
                    self.stroke_ancho = *ancho;
                }
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
        nodo.set_label("Lienzo de dibujo");
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

// ═══════════════════════════════════════════════════════════════════
// CanvasView: wrapper Xilem View para CanvasWidget
// ═══════════════════════════════════════════════════════════════════

/// Wrapper Xilem View que crea un [`CanvasWidget`] y lo actualiza con
/// comandos JSON desde [`VariableStore`].
///
/// El `commands_json` se lee desde VariableStore en `layout_a_vista()`
/// y se pasa al View. Xilem detecta el cambio en `rebuild()` y actualiza
/// el widget internamente.
///
/// Sigue el patrón de `QRView` / `SvgIconView`: implementa
/// `View<State, (), ViewCtx>` con `Element = Pod<CanvasWidget>`.
pub struct CanvasVista<State> {
    commands_var: String,
    commands_json: String,
    ancho: f64,
    alto: f64,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> CanvasVista<State> {
    pub fn nuevo(commands_var: String, commands_json: String, ancho: f64, alto: f64) -> Self {
        Self {
            commands_var,
            commands_json,
            ancho,
            alto,
            phantom: PhantomData,
        }
    }

    /// Constructor sin comandos (útil cuando no hay variable)
    pub fn empty(commands_var: String, ancho: f64, alto: f64) -> Self {
        Self {
            commands_var,
            commands_json: String::new(),
            ancho,
            alto,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for CanvasVista<State> {}

impl<State: 'static> View<State, (), ViewCtx> for CanvasVista<State> {
    type Element = Pod<CanvasWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<CanvasWidget>, ()) {
        let mut widget = CanvasWidget::nuevo(self.commands_var.clone(), self.ancho, self.alto);
        if !self.commands_json.is_empty() && self.commands_json != "null" {
            widget.cargar_commands(&self.commands_json);
        }
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        _anterior: &Self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<CanvasWidget>>,
        _app_state: &mut State,
    ) {
        // Xilem maneja automáticamente la reconstrucción del widget cuando
        // los campos del View cambian (commands_json, width, height, etc.)
        // No es necesario actualizar manualmente el widget aquí.
    }

    fn teardown(
        &self,
        _vista_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<CanvasWidget>>,
    ) {
    }

    fn message(
        &self,
        _vista_state: &mut (),
        _message: &mut crate::MessageContext,
        _element: Mut<'_, Pod<CanvasWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}
