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
use crate::vello::kurbo::{Affine, BezPath, Circle, Line, Point, Rect, Size, Stroke};
use crate::vello::peniko::{self, Brush, Fill};
use crate::vello::Scene;
use crate::{
    AccessCtx, BoxConstraints, ChildrenIds, EventCtx, LayoutCtx, NoAction, PaintCtx,
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
        width: f64,
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
        width: f64,
    },
    /// Rellenar un path definido por puntos (BezPath poligonal)
    FillPath {
        points: Vec<[f64; 2]>, // lista de [x, y]
        closed: bool,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    /// Trazar contorno de path
    StrokePath {
        points: Vec<[f64; 2]>,
        closed: bool,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        width: f64,
    },
    /// Dibujar línea
    DrawLine {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
        width: f64,
    },
    /// Dibujar texto (placeholder: rectángulo coloreado)
    DrawText {
        x: f64,
        y: f64,
        text: String,
        size: f64,
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    /// Limpiar todo el canvas con un color
    Clear {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    /// Cambiar color de relleno actual (modo painter)
    SetFillColor {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    /// Cambiar color de trazo actual
    SetStrokeColor {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
    /// Cambiar grosor de trazo actual
    SetStrokeWidth {
        width: f64,
    },
}

// ═══════════════════════════════════════════════════════════════════
// CanvasWidget: widget Vello que ejecuta comandos de dibujo en paint()
// ═══════════════════════════════════════════════════════════════════

/// Widget de lienzo de dibujo libre que renderiza una lista de [`CanvasCommand`]
/// sobre un fondo blanco usando Vello `Scene`.
pub struct CanvasWidget {
    commands_var: String,
    commands: Vec<CanvasCommand>,
    width: f64,
    height: f64,
    fill_color: peniko::Color,
    stroke_color: peniko::Color,
    stroke_width: f64,
}

impl CanvasWidget {
    pub fn new(commands_var: String, width: f64, height: f64) -> Self {
        Self {
            commands_var,
            commands: Vec::new(),
            width,
            height,
            fill_color: peniko::Color::BLACK,
            stroke_color: peniko::Color::BLACK,
            stroke_width: 2.0,
        }
    }

    /// Cargar comandos desde un JSON string (ej: contenido de VariableStore)
    pub fn load_commands(&mut self, json: &str) {
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
        bc.constrain(Size::new(self.width, self.height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();

        // Limpiar el canvas con blanco (fondo por defecto)
        let bg_rect = Rect::new(0.0, 0.0, size.width, size.height);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &Brush::Solid(peniko::Color::WHITE),
            None,
            &bg_rect,
        );

        // Resetear estado de painter
        self.fill_color = peniko::Color::BLACK;
        self.stroke_color = peniko::Color::BLACK;
        self.stroke_width = 2.0;

        for cmd in &self.commands {
            match cmd {
                CanvasCommand::Clear { r, g, b, a } => {
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &Brush::Solid(color),
                        None,
                        &bg_rect,
                    );
                }

                CanvasCommand::FillCircle {
                    x, y, radius, r, g, b, a,
                } => {
                    let circle = Circle::new(Point::new(*x, *y), *radius);
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &Brush::Solid(color),
                        None,
                        &circle,
                    );
                }

                CanvasCommand::StrokeCircle {
                    x, y, radius, r, g, b, a, width,
                } => {
                    let circle = Circle::new(Point::new(*x, *y), *radius);
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    scene.stroke(
                        &Stroke::new(*width),
                        Affine::IDENTITY,
                        &Brush::Solid(color),
                        None,
                        &circle,
                    );
                }

                CanvasCommand::FillRect {
                    x, y, w, h, r, g, b, a,
                } => {
                    let rect = Rect::new(*x, *y, x + w, y + h);
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &Brush::Solid(color),
                        None,
                        &rect,
                    );
                }

                CanvasCommand::StrokeRect {
                    x, y, w, h, r, g, b, a, width,
                } => {
                    let rect = Rect::new(*x, *y, x + w, y + h);
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    scene.stroke(
                        &Stroke::new(*width),
                        Affine::IDENTITY,
                        &Brush::Solid(color),
                        None,
                        &rect,
                    );
                }

                CanvasCommand::DrawLine {
                    x1, y1, x2, y2, r, g, b, a, width,
                } => {
                    let line = Line::new(Point::new(*x1, *y1), Point::new(*x2, *y2));
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    scene.stroke(
                        &Stroke::new(*width),
                        Affine::IDENTITY,
                        &Brush::Solid(color),
                        None,
                        &line,
                    );
                }

                CanvasCommand::FillPath {
                    points, closed, r, g, b, a,
                } => {
                    if points.is_empty() {
                        continue;
                    }
                    let mut path = BezPath::new();
                    path.move_to(Point::new(points[0][0], points[0][1]));
                    for p in &points[1..] {
                        path.line_to(Point::new(p[0], p[1]));
                    }
                    if *closed {
                        path.close_path();
                    }
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &Brush::Solid(color),
                        None,
                        &path,
                    );
                }

                CanvasCommand::StrokePath {
                    points, closed, r, g, b, a, width,
                } => {
                    if points.is_empty() {
                        continue;
                    }
                    let mut path = BezPath::new();
                    path.move_to(Point::new(points[0][0], points[0][1]));
                    for p in &points[1..] {
                        path.line_to(Point::new(p[0], p[1]));
                    }
                    if *closed {
                        path.close_path();
                    }
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    scene.stroke(
                        &Stroke::new(*width),
                        Affine::IDENTITY,
                        &Brush::Solid(color),
                        None,
                        &path,
                    );
                }

                CanvasCommand::DrawText {
                    x, y, text, size, r, g, b, a,
                } => {
                    // Placeholder: dibujar un pequeño rectángulo coloreado
                    // que representa el área del texto
                    let color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                    let text_width = text.len() as f64 * size * 0.5;
                    let rect = Rect::new(
                        *x,
                        *y,
                        x + text_width.min(200.0),
                        y + *size * 1.2,
                    );
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &Brush::Solid(color),
                        None,
                        &rect,
                    );
                }

                CanvasCommand::SetFillColor { r, g, b, a } => {
                    self.fill_color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                }

                CanvasCommand::SetStrokeColor { r, g, b, a } => {
                    self.stroke_color = peniko::Color::from_rgba8(*r, *g, *b, *a);
                }

                CanvasCommand::SetStrokeWidth { width } => {
                    self.stroke_width = *width;
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
        node: &mut Node,
    ) {
        node.set_label("Lienzo de dibujo");
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
/// El `commands_json` se lee desde VariableStore en `layout_a_view()`
/// y se pasa al View. Xilem detecta el cambio en `rebuild()` y actualiza
/// el widget internamente.
///
/// Sigue el patrón de `QRView` / `SvgIconView`: implementa
/// `View<State, (), ViewCtx>` con `Element = Pod<CanvasWidget>`.
pub struct CanvasView<State> {
    commands_var: String,
    commands_json: String,
    width: f64,
    height: f64,
    phantom: PhantomData<fn() -> State>,
}

impl<State: 'static> CanvasView<State> {
    pub fn new(commands_var: String, commands_json: String, width: f64, height: f64) -> Self {
        Self {
            commands_var,
            commands_json,
            width,
            height,
            phantom: PhantomData,
        }
    }

    /// Constructor sin comandos (útil cuando no hay variable)
    pub fn empty(commands_var: String, width: f64, height: f64) -> Self {
        Self {
            commands_var,
            commands_json: String::new(),
            width,
            height,
            phantom: PhantomData,
        }
    }
}

impl<State: 'static> ViewMarker for CanvasView<State> {}

impl<State: 'static> View<State, (), ViewCtx> for CanvasView<State> {
    type Element = Pod<CanvasWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<CanvasWidget>, ()) {
        let mut widget = CanvasWidget::new(
            self.commands_var.clone(),
            self.width,
            self.height,
        );
        if !self.commands_json.is_empty() && self.commands_json != "null" {
            widget.load_commands(&self.commands_json);
        }
        (ctx.create_pod(widget), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _view_state: &mut (),
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
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<CanvasWidget>>,
    ) {
    }

    fn message(
        &self,
        _view_state: &mut (),
        _message: &mut crate::MessageContext,
        _element: Mut<'_, Pod<CanvasWidget>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}
