// Forja GUI Runtime (forja-gui-rt)
extern crate self as forja_gui_rt;
// Pre-compila xilem para que las apps GUI de Forja compilen en segundos.
// Este crate se compila UNA SOLA VEZ; las apps generadas solo enlazan contra él.

// Módulo de signals/streams reactivos (reemplaza HashMap)
pub use forja_gui_core::signals;
// Módulo de accesibilidad (TalkBack/VoiceOver)
pub mod accessibility;
pub mod gui_nativa;
pub(crate) mod widgets;
// Módulo de evaluación tree-walking completa del AST de Forja
pub mod evaluador;
// Módulo de depuración y trazabilidad del estado GUI
pub mod debug;
// Funciones helper centralizadas (extracción AST, temas/colores, callbacks, builders)
pub(crate) mod helpers;
pub use debug::*;

// Re-exportar todo lo que necesita el código generado por Forja
pub use winit::error::EventLoopError;
pub use xilem::core::{lens, memoize};
// Renombres de xilem_core (main): `map_message` → `map_message_result`,
// `MessageContext` → `MessageCtx`. Se aliasan al nombre histórico de Forja.
pub use xilem::core::map_message_result as map_message;
pub use xilem::core::{MessageCtx as MessageContext, MessageResult};
pub use xilem::masonry::layout::Length;
// Módulos de Masonry usados por los widgets personalizados del crate
// (re-exportados para evitar escribir `xilem::masonry::...` en cada archivo).
pub use xilem::masonry::{kurbo, layout};
pub use xilem::style::{Background, Style};
pub use xilem::view::{
    self, button, button_any_pointer, checkbox, flex, grid, image, indexed_stack, label, portal,
    progress_bar, prose, sized_box, slider, spinner, split, text_button, text_input, transformed,
    variable_label, virtual_scroll, zstack, CrossAxisAlignment, GridParams, MainAxisAlignment,
};
pub use xilem::winit;
pub use xilem::ViewCtx;
pub use xilem::{palette, Blob, Color, ImageBrush, ImageFormat};
pub use xilem::{
    AnyWidgetView, EventLoop, FontWeight, TextAlign, WidgetView, WindowOptions, Xilem,
};

// === Re-exportaciones para WindowSize y resize detection ===
pub use xilem::core::{Mut, View, ViewMarker};
pub use xilem::masonry::accesskit::{self, Node, Role};
pub use xilem::masonry::core::ChildrenIds;
pub use xilem::masonry::core::NewWidget;
pub use xilem::masonry::core::Widget;
pub use xilem::masonry::core::{
    AccessCtx, AccessEvent, ComposeCtx, EventCtx, LayoutCtx, MeasureCtx, NoAction, PaintCtx,
    PointerButtonEvent, PointerEvent, PointerId, PointerUpdate, PropertiesMut, PropertiesRef,
    RegisterCtx, TextEvent, Update, UpdateCtx, WidgetId, WidgetMut, WidgetPod,
};
/// LogicalSize: tamaño lógico de ventana (independiente de DPI)
pub use xilem::masonry::dpi::LogicalSize;
/// Dibujo: `imaging::Painter` reemplaza a `vello::Scene` en `Widget::paint`.
pub use xilem::masonry::imaging::{self, Painter};
pub use xilem::masonry::kurbo::{Affine, Axis, Point, Size};
pub use xilem::masonry::layout::LenReq;
pub use xilem::masonry::peniko;
pub use xilem::masonry::ui_events::pointer::PointerButton;
/// Re-exportamos módulos de Masonry necesarios para widgets personalizados
pub use xilem::masonry::widgets as masonry_widgets;
/// Window: acceso a la ventana nativa de winit
pub use xilem::winit::window::Window;
/// Pod: wrapper de widgets Masonry como WidgetView
pub use xilem::Pod;

// Propiedades por defecto de Masonry derivadas del tema (afecta a TODOS los
// widgets nativos: botones, campos, switches, sliders, labels).
pub mod theme_props;
// Material y detalles de la ventana nativa (Mica en Windows 11).
pub mod chrome;
// Configuración de la ventana declarada desde el código Forja.
pub mod window_config;
// Medición del costo de armar cada frame (FORJA_GUI_PERF=1).
pub mod perf;
// Módulo de tema Material You — extraído al crate `forja-theme` (F3-T2).
// Se re-exporta para preservar la API pública histórica `forja_gui_rt::theme::*`.
pub use forja_theme as theme;
// Módulo de iconos Material Design
pub mod icons;
// Widget de icono SVG Material Design con renderizado Vello
pub mod svg_icon_widget;
// Widget de sombra de elevación con renderizado Vello blur
pub mod shadow_widget;
// Re-exportar tipos específicos del theme (evitar conflictos de nombres con xilem,
// ej: FontWeight existe tanto en xilem como en theme::typography)
pub use theme::{
    ColorScheme,     // Esquema de color con roles
    DesignStyle,     // Diseño visual base (Material You, Liquid Glass, Fluent)
    ElevationSystem, // Sistema de elevación (sombras)
    MaterialTheme,   // Tema completo
    RgbColor,        // Color RGB (convierte a xilem::Color via From)
    ShapeFamily,     // Familia de componentes para formas
    ShapeSystem,     // Sistema de formas (radios de borde)
    TextStyle,       // Estilo de texto individual
    TypeScale,       // Escala tipográfica (15 estilos)
};
// Re-exportar tipos principales de iconos para uso directo
pub use icons::{icon_widget, icon_widget_styled, IconStyle, MaterialIcon};

// Re-exportar tipos principales de animación para uso directo
pub use theme::animation::{
    interpolate_color, AnimatedValue, Animation, AnimationEngine, AnimationPresets, SpringAnimation,
};

// Re-exportar tipos principales de signals/streams
pub use signals::{
    read_var, write_var, ReactiveCtx, Signal, Stream, ValorReactivio, VariableStore,
};

// Re-exportar tipos principales de accesibilidad
pub use accessibility::{
    descripcion_accesible, estado_checkbox, estado_seleccion, estado_switch, material_role,
    A11yData, AccessibilityManager, Anuncio, PrioridadAnuncio,
};

// Re-exportar tipos principales de widgets personalizados
pub use shadow_widget::{glow_box_shadow, shadow_to_box_shadow};
pub use svg_icon_widget::{svg_icon, MaterialSvgIcon, SvgIconView};

// === Módulos de animación ===
pub mod animated_widgets;
pub mod animation_root;

// Re-exportar tipos principales de animación
pub use animated_widgets::{AnimatedOpacity, AnimatedScale, AnimatedSlide, RippleWidget};
pub use animation_root::AnimationRoot;

// === Módulo de gestos táctiles ===
pub mod gesture_widgets;
pub use gesture_widgets::{
    GestureResult, GestureTracker, MultiTouchState, PanWidget, PinchZoomWidget,
    PullToRefreshWidget, RotateWidget, SwipeWidget,
};

// === Módulo de pickers (DatePicker, TimePicker, ColorPicker) ===
pub mod pickers;

// === Re-exportaciones de forja_syntax::ast para que el transpiler pueda generar código ===
pub use forja_syntax::ast::{
    BrazoMatch, BrazoSeleccionar, Contrato, Metodo, Parametro, VariableClase,
};
pub use forja_syntax::ast::{
    Declaracion, Expresion, Operador, OperadorUnario, Patron, Programa, Tipo,
};

// === Módulo de QR Code ===
pub mod qr_widget;
pub use qr_widget::{qr_view, QRView, QRWidget};
pub mod star_widget;
pub use star_widget::{estrella, StarIconView, StarWidget};

// === Módulo de gráficos vectoriales con Vello (Fase 9) ===
pub mod chart_widgets;
pub use chart_widgets::{
    BarChartView, BarChartWidget, ChartDataPoint, GaugeChartView, GaugeChartWidget, LineChartView,
    LineChartWidget, PieChartView, PieChartWidget, Rgba, SparklineView, SparklineWidget,
};

// === Módulo de canvas / pintura libre con Vello (Fase 10) ===
pub mod canvas_widget;
pub use canvas_widget::{CanvasCommand, CanvasView, CanvasWidget};

// === Re-exportaciones del GUI runtime ===
// Funciones principales para convertir AST → Layout → Widgets
pub use gui_nativa::{
    build_and_run, expr_a_layout, extraer_layout, inicializar_estado, layout_a_view,
    AppStateNativo, Layout, Orientation, ValorGUI,
};

/// Convierte una expresión Forja (AST) a un Layout del runtime.
/// Útil para el código generado por el transpiler.
pub fn layout_from_expr(expr: &Expresion) -> Option<Layout> {
    gui_nativa::expr_a_layout(expr)
}

/// Extrae el flag `--load-state=<json>` de los argumentos de línea de comandos.
/// Usado por el código generado por el transpiler para soportar hot reload.
pub fn initialize_from_args(args: &[String]) -> Option<String> {
    for arg in args {
        if let Some(state) = arg.strip_prefix("--load-state=") {
            return Some(state.to_string());
        }
    }
    None
}
