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
pub use xilem::view::{
    self, button, button_any_pointer, checkbox, flex, grid, image, indexed_stack, label,
    portal, progress_bar, prose, sized_box, slider, spinner, split, text_button, text_input,
    transformed, variable_label, virtual_scroll, zstack, Axis, GridParams, MainAxisAlignment,
    CrossAxisAlignment,
};
pub use xilem::{
    AnyWidgetView, EventLoop, FontWeight, TextAlign, WidgetView, WindowOptions, Xilem,
};
pub use xilem::winit;
pub use winit::error::EventLoopError;
pub use xilem::{Affine, Blob, Color, ImageBrush, ImageFormat, palette};
pub use xilem::core::{lens, memoize, map_message};
pub use xilem::core::{MessageResult, MessageContext};
pub use xilem::ViewCtx;
pub use xilem::masonry::properties::types::Length;
pub use xilem::style::{Background, Style};

// === Re-exportaciones para WindowSize y resize detection ===
/// LogicalSize: tamaño lógico de ventana (independiente de DPI)
pub use xilem::masonry::dpi::LogicalSize;
/// Window: acceso a la ventana nativa de winit
pub use xilem::winit::window::Window;
/// Pod: wrapper de widgets Masonry como WidgetView
pub use xilem::Pod;
/// Re-exportamos módulos de Masonry necesarios para widgets personalizados
pub use xilem::masonry::widgets;
pub use xilem::masonry::core::Widget;
pub use xilem::masonry::core::{BoxConstraints, NewWidget};
pub use xilem::masonry::core::ChildrenIds;
pub use xilem::masonry::core::{
    WidgetPod, WidgetMut, NoAction, LayoutCtx, RegisterCtx, PaintCtx, AccessCtx,
    PropertiesMut, PropertiesRef, UpdateCtx, WidgetId, EventCtx, ComposeCtx,
    PointerEvent, PointerButtonEvent, PointerUpdate, AccessEvent, TextEvent,
    Update,
};
pub use xilem::masonry::ui_events::pointer::PointerButton;
pub use xilem::masonry::accesskit::{self, Node, Role};
pub use xilem::masonry::vello::{self, Scene};
pub use xilem::masonry::kurbo::{Point, Size};
pub use xilem::core::{View, ViewMarker, Mut};

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
    MaterialTheme,    // Tema completo
    ColorScheme,      // Esquema de color con roles
    RgbColor,         // Color RGB (convierte a xilem::Color via From)
    TypeScale,        // Escala tipográfica (15 estilos)
    TextStyle,        // Estilo de texto individual
    ShapeSystem,      // Sistema de formas (radios de borde)
    ShapeFamily,      // Familia de componentes para formas
    ElevationSystem,  // Sistema de elevación (sombras)
};
// Re-exportar tipos principales de iconos para uso directo
pub use icons::{
    MaterialIcon,
    IconStyle,
    icon_widget,
    icon_widget_styled,
};

// Re-exportar tipos principales de animación para uso directo
pub use theme::animation::{
    AnimationEngine,
    AnimatedValue,
    SpringAnimation,
    AnimationPresets,
    Animation,
    interpolate_color,
};

// Re-exportar tipos principales de signals/streams
pub use signals::{
    Signal,
    Stream,
    ValorReactivio,
    VariableStore,
    ReactiveCtx,
    read_var,
    write_var,
};

// Re-exportar tipos principales de accesibilidad
pub use accessibility::{
    AccessibilityManager,
    A11yData,
    Anuncio,
    PrioridadAnuncio,
    descripcion_accesible,
    material_role,
    estado_checkbox,
    estado_switch,
    estado_seleccion,
};

// Re-exportar tipos principales de widgets personalizados
pub use svg_icon_widget::{MaterialSvgIcon, SvgIconView, svg_icon};
pub use shadow_widget::{shadow_to_box_shadow, glow_box_shadow};

// === Módulos de animación ===
pub mod animation_root;
pub mod animated_widgets;

// Re-exportar tipos principales de animación
pub use animation_root::AnimationRoot;
pub use animated_widgets::{AnimatedOpacity, AnimatedSlide, AnimatedScale, RippleWidget};

// === Módulo de gestos táctiles ===
pub mod gesture_widgets;
pub use gesture_widgets::{SwipeWidget, PanWidget, PullToRefreshWidget, GestureResult, GestureTracker};

// === Módulo de pickers (DatePicker, TimePicker, ColorPicker) ===
pub mod pickers;

// === Re-exportaciones de forja::ast para que el transpiler pueda generar código ===
pub use forja::ast::{Expresion, Declaracion, Programa, Operador, Tipo, Patron, OperadorUnario};
pub use forja::ast::{Parametro, VariableClase, Metodo, Contrato, BrazoMatch, BrazoSeleccionar};

// === Módulo de QR Code ===
pub mod qr_widget;
pub use qr_widget::{QRWidget, QRView, qr_view};

// === Módulo de gráficos vectoriales con Vello (Fase 9) ===
pub mod chart_widgets;
pub use chart_widgets::{
    BarChartWidget, ChartDataPoint, GaugeChartWidget, LineChartWidget, PieChartWidget,
    SparklineWidget,
};

// === Re-exportaciones del GUI runtime ===
// Funciones principales para convertir AST → Layout → Widgets
pub use gui_nativa::{
    AppStateNativo,
    Layout,
    ValorGUI,
    expr_a_layout,
    layout_a_view,
    extraer_layout,
    build_and_run,
    inicializar_estado,
};

/// Convierte una expresión Forja (AST) a un Layout del runtime.
/// Útil para el código generado por el transpiler.
pub fn layout_from_expr(expr: &Expresion) -> Option<Layout> {
    gui_nativa::expr_a_layout(expr)
}
