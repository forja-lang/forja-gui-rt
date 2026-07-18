// Forja GUI Runtime (forja-gui-rt)
extern crate self as forja_gui_rt;
// Pre-compila xilem para que las apps GUI de Forja compilen en segundos.
// Este crate se compila UNA SOLA VEZ; las apps generadas solo enlazan contra él.

// Módulo de signals/streams reactivos (reemplaza HashMap)
pub mod signals;
// Módulo de accesibilidad (TalkBack/VoiceOver)
pub mod accessibility;
pub mod gui_nativa;
// Módulo de evaluación tree-walking completa del AST de Forja
pub mod evaluador;

// Re-exportar todo lo que necesita el código generado por Forja
pub use winit::error::EventLoopError;
pub use xilem::core::{lens, map_message, memoize};
pub use xilem::core::{MessageContext, MessageResult};
pub use xilem::masonry::properties::types::Length;
pub use xilem::style::{Background, Style};
pub use xilem::view::{
    self, button, button_any_pointer, checkbox, flex, grid, image, indexed_stack, label, portal,
    progress_bar, prose, sized_box, slider, spinner, split, text_button, text_input, transformed,
    variable_label, virtual_scroll, zstack, Axis, CrossAxisAlignment, GridParams,
    MainAxisAlignment,
};
pub use xilem::winit;
pub use xilem::ViewCtx;
pub use xilem::{palette, Affine, Blob, Color, ImageBrush, ImageFormat};
pub use xilem::{
    AnyWidgetView, EventLoop, FontWeight, TextAlign, WidgetView, WindowOptions, Xilem,
};

// === Re-exportaciones para WindowSize y resize detection ===
pub use xilem::core::{Mut, View, ViewMarker};
pub use xilem::masonry::accesskit::{self, Node, Role};
pub use xilem::masonry::core::ChildrenIds;
pub use xilem::masonry::core::Widget;
pub use xilem::masonry::core::{
    AccessCtx, AccessEvent, ComposeCtx, EventCtx, LayoutCtx, NoAction, PaintCtx,
    PointerButtonEvent, PointerEvent, PointerId, PointerUpdate, PropertiesMut, PropertiesRef,
    RegisterCtx, TextEvent, Update, UpdateCtx, WidgetId, WidgetMut, WidgetPod,
};
pub use xilem::masonry::core::{BoxConstraints, NewWidget};
/// LogicalSize: tamaño lógico de ventana (independiente de DPI)
pub use xilem::masonry::dpi::LogicalSize;
pub use xilem::masonry::kurbo::{Point, Size};
pub use xilem::masonry::ui_events::pointer::PointerButton;
pub use xilem::masonry::vello::{self, Scene};
/// Re-exportamos módulos de Masonry necesarios para widgets personalizados
pub use xilem::masonry::widgets;
/// Window: acceso a la ventana nativa de winit
pub use xilem::winit::window::Window;
/// Pod: wrapper de widgets Masonry como WidgetView
pub use xilem::Pod;

// Módulo de tema Material You
pub mod theme;
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

// === Re-exportaciones de forja::ast para que el transpiler pueda generar código ===
pub use forja::ast::{BrazoMatch, BrazoSeleccionar, Contrato, Metodo, Parametro, VariableClase};
pub use forja::ast::{Declaracion, Expresion, Operador, OperadorUnario, Patron, Programa, Tipo};

// === Módulo de QR Code ===
pub mod qr_widget;
pub use qr_widget::{qr_view, QRView, QRWidget};

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
    AppStateNativo, Layout, ValorGUI,
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
