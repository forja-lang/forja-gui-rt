// Forja GUI Nativa — construye widgets xilem directamente desde el AST
// con soporte completo de tema Material You

use crate::helpers::*;
use crate::layout::AsUnit as _;
use crate::theme::animation::AnimationEngine;
use crate::theme::motion::{EASE_EMPHASIZED, EASE_STANDARD};
use crate::view;
use crate::Axis;
use crate::FontWeight;
use crate::Length;
use crate::*;
use crate::{map_message, MessageResult};
use crate::{
    MaterialTheme,   // Tema Material You completo
    RgbColor,        // Color RGB (convierte a xilem::Color vía From)
    VariableAlmacen, // Store reactivo de variables
};
use chrono::Datelike;
use forja_syntax::ast::*;
use std::collections::HashMap;
use std::time::Instant;
// icons module used implicitly through svg_icon() call

// El modelo de valores vive en `forja-interprete` (compartido con la web).
pub use forja_interprete::ValorGUI;


#[derive(Debug, Clone)]
pub struct AppStateNativo {
    pub almacen: VariableAlmacen,
    pub ventana_tamano: VentanaTamanoClase,
    pub ventana_ancho: f64,
    pub ventana_alto: f64,
    pub animation_engine: AnimationEngine,
    /// Previous visibility values keyed by variable name, for detecting transitions
    anim_anterior_visibility: HashMap<String, bool>,
    /// Animation start times for transitions keyed by variable name
    anim_inicio_time: HashMap<String, Instant>,
    /// Previous screen ID for navigator transitions
    pub(crate) pantalla_anterior: String,
    /// Animation start time for navigator screen transitions
    pub(crate) inicio_animacion_nav: Option<Instant>,
    pub orientation: Orientation,
    /// 5.7: Contexto reactivo para tracking de dependencias de widgets
    pub reactive_ctx: ReactiveCtx,
}

// ─── Conversión ValorGUI ↔ serde_json::Value ─────────────────────



impl AppStateNativo {
    pub fn nuevo() -> Self {
        AppStateNativo {
            almacen: VariableAlmacen::nuevo(),
            ventana_tamano: VentanaTamanoClase::Compact,
            ventana_ancho: 800.0,
            ventana_alto: 600.0,
            animation_engine: AnimationEngine::nuevo(),
            anim_anterior_visibility: HashMap::new(),
            anim_inicio_time: HashMap::new(),
            pantalla_anterior: String::new(),
            inicio_animacion_nav: None,
            orientation: Orientation::Portrait,
            reactive_ctx: ReactiveCtx::nuevo(),
        }
    }

    /// Detecta si una variable de visibilidad cambió de estado
    pub fn anim_visibility_changed(&mut self, var_name: &str, current_visible: bool) -> bool {
        let anterior = self
            .anim_anterior_visibility
            .get(var_name)
            .copied()
            .unwrap_or(false);
        let changed = anterior != current_visible;
        self.anim_anterior_visibility
            .insert(var_name.to_string(), current_visible);
        if changed {
            self.anim_inicio_time
                .insert(var_name.to_string(), Instant::now());
        }
        changed
    }

    /// Obtiene el progreso de animación (0.0 a 1.0) para una transición
    pub fn anim_progress(&self, var_name: &str, duracion_ms: f64) -> f64 {
        if let Some(inicio) = self.anim_inicio_time.get(var_name) {
            let elapsed = inicio.elapsed().as_secs_f64() * 1000.0;

            (elapsed / duracion_ms).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Obtiene el progreso de animación suavizado con easing emphasized
    pub fn anim_progress_eased(&self, var_name: &str, duracion_ms: f64) -> f64 {
        let raw = self.anim_progress(var_name, duracion_ms);
        EASE_EMPHASIZED.aplicar(raw)
    }

    pub fn nav_transition_inicio(&mut self, screen_id: &str) {
        if self.pantalla_anterior != screen_id {
            self.pantalla_anterior = screen_id.to_string();
            self.inicio_animacion_nav = Some(Instant::now());
        }
    }

    /// Anuncia un mensaje de accesibilidad (TalkBack)
    pub fn a11y_say(&self, mensaje: &str) {
        println!("  ♿ [TalkBack] {}", mensaje);
    }

    /// Anuncia foco en un widget
    pub fn a11y_focus(&self, widget_type: &str, etiqueta: &str, value: &str, state: &str) {
        let desc = descripcion_accesible(widget_type, etiqueta, value, state);
        self.a11y_say(&desc);
    }

    /// Actualiza el tamaño de ventana y la clase de tamaño
    pub fn actualizar_ventana_tamano(&mut self, width: f64, height: f64) {
        self.ventana_ancho = width;
        self.ventana_alto = height;
        self.ventana_tamano = VentanaTamanoClase::from_ancho(width);
        self.orientation = Orientation::from_dimensions(width, height);
    }

    pub fn leer(&self, nombre: &str) -> ValorGUI {
        // 5.7: Registrar lectura para reactive tracking
        self.reactive_ctx.track_dependency(nombre);
        self.almacen
            .obtener(nombre)
            .map(ValorGUI::from)
            .unwrap_or(ValorGUI::Nulo)
    }

    pub fn escribir(&mut self, nombre: &str, valor: ValorGUI) {
        let json_val: serde_json::Value = valor.into();
        self.almacen.fijar(nombre, json_val);
    }

    /// Serializa todas las variables de estado a JSON.
    /// Usado por hot reload para preservar estado entre reinicios.
    pub fn guardar_estado(&self) -> String {
        let snapshot = self.almacen.snapshot();
        serde_json::to_string(&snapshot).unwrap_or_default()
    }

    /// Carga estado desde un JSON, restaurando las variables guardadas.
    /// Usado por hot reload para restaurar estado tras recompilar.
    pub fn cargar_estado(&mut self, json: &str) {
        if let Ok(mapa) =
            serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(json)
        {
            for (clave, value) in mapa {
                self.almacen.fijar(&clave, value);
            }
        }
    }
}

impl Default for AppStateNativo {
    fn default() -> Self {
        Self::nuevo()
    }
}

// ─── Window Size Class (Material Design 3) ─────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    Portrait,
    Landscape,
}

impl Orientation {
    pub fn from_dimensions(width: f64, height: f64) -> Self {
        if width >= height {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VentanaTamanoClase {
    Compact,  // < 600dp
    Medium,   // 600-840dp
    Expanded, // > 840dp
}

impl VentanaTamanoClase {
    pub fn from_ancho(width: f64) -> Self {
        if width < 600.0 {
            VentanaTamanoClase::Compact
        } else if width < 840.0 {
            VentanaTamanoClase::Medium
        } else {
            VentanaTamanoClase::Expanded
        }
    }
}

// ─── Variantes de botón Material Design 3 ────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonVariant {
    Filled,
    Tonal,
    Outlined,
    Text,
    Elevated,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FabTamano {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IconButtonVariant {
    Standard,
    Filled,
    Tonal,
    Outlined,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChipVariant {
    Assist,
    Filter,
    Entrada,
    Suggestion,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextoCampoVariant {
    Filled,
    Outlined,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CardVariant {
    Filled,
    Elevated,
    Outlined,
    Selectable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SheetVariant {
    Standard,
    Modal,
    Expanded,
}

// ─── Navegación Material You ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NavElemento {
    pub icono: String,
    pub etiqueta: String,
    pub badge: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IconAction {
    pub icono: String,
    pub callback: String,
}

// ─── Navigator (sistema de navegación por pantallas) ──────────────

/// Una pantalla dentro del Navigator
#[derive(Debug)]
pub struct NavigatorScreen {
    /// Identificador único de la pantalla
    pub id: String,
    /// Título mostrado en la barra de navegación/tabs
    pub titulo: String,
    /// Icono opcional (para NavigationBar/Tabs)
    pub icono: Option<String>,
    /// Contenido de la pantalla (pre-evaluado, puede ser Spacer si es función)
    pub(crate) contenido: Box<Layout>,
    /// Badge opcional (notificaciones)
    pub badge: Option<String>,
    /// Nombre de la función que genera el contenido (para evaluación diferida)
    pub(crate) content_fn: Option<String>,
}

/// Tipo de navegación visual del Navigator
#[derive(Debug, Clone, PartialEq)]
pub enum NavigatorTipo {
    /// Solo las pantallas, sin barra de navegación
    None,
    /// NavigationBar inferior (mobile style)
    BottomBar,
    /// NavigationRail lateral
    Rail,
    /// Pestañas superiores
    Tabs,
    /// Cajón lateral (Drawer)
    Drawer,
}

/// Animación de transición entre pantallas
#[derive(Debug, Clone, PartialEq)]
pub enum NavigatorAnim {
    /// Sin animación
    None,
    /// Fundido
    Fade,
    /// Deslizamiento horizontal
    Slide,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopAppBarVariant {
    Small,
    Medium,
    Large,
}

// ─── Avatar variant ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AvatarVariant {
    Text,
    Icon,
    Image,
}

// ─── Layout (representación intermedia) ───────────────────────────

#[derive(Debug)]
pub enum Layout {
    // ─── Layouts básicos ──────────────────────────────────────────
    Column {
        children: Vec<Layout>,
        gap: f64,
        alignment: String,
    },
    CenteredColumn(Vec<Layout>),
    Row {
        children: Vec<Layout>,
        gap: f64,
        alignment: String,
    },
    PilaZ(Vec<Layout>),
    Portal(Box<Layout>),
    Container {
        hijo: Box<Layout>,
        max_ancho: f64,
    },
    Label {
        texto: String,
        es_variable: bool,
    },
    VariableLabel {
        variable: String,
    },
    Title(String),
    ColoredLabel {
        texto: String,
        color: String,
    },
    // ─── Variantes de tema Material You ───────────────────────────
    /// Proveedor de tema: cambia el color semilla para los hijos
    ThemeProvider {
        hijo: Box<Layout>,
        theme: String,
    },
    /// Aplica un color del esquema al hijo (primary, secondary, etc.)
    ColoredBox {
        hijo: Box<Layout>,
        color_role: String,
    },
    /// Etiqueta con estilo tipográfico predefinido (display_large, body_medium, etc.)
    StyledLabel {
        texto: String,
        style: String,
    },
    /// Aplica una familia de forma (border-radius) al hijo
    ShapedBox {
        hijo: Box<Layout>,
        forma_family: String,
    },
    /// Aplica elevación (sombra) al hijo, con familia de forma opcional
    ElevatedBox {
        hijo: Box<Layout>,
        level: u8,
        forma_family: String,
    },
    /// Aplica un estado visual al hijo (hover, pressed, etc.)
    StateLayerBox {
        hijo: Box<Layout>,
        state: String,
    },
    /// Layout responsivo con 3 variantes
    ResponsiveLayout {
        compact: Box<Layout>,
        medium: Box<Layout>,
        expanded: Box<Layout>,
    },
    /// Aplica padding alrededor del hijo
    Padding {
        hijo: Box<Layout>,
        amount: f64,
    },
    /// Hace que el hijo se expanda para llenar el espacio disponible
    Expanded {
        hijo: Box<Layout>,
    },
    /// Centra al hijo en el eje transversal
    Centered {
        hijo: Box<Layout>,
    },
    Button {
        texto: String,
        callback: String,
    },
    EntradaTexto {
        variable: String,
        multiline: bool,
        placeholder: String,
    },
    ProgressBar {
        variable: String,
    },
    Slider {
        variable: String,
        min: f64,
        max: f64,
    },
    Checkbox {
        variable: String,
    },
    Prose(String),
    Spinner,
    Separator,
    Spacer(f64),
    // ─── Layout responsive avanzado ───────────────────────────────
    /// Flex layout con gap, axis y wrap configurables
    FlexLayout {
        children: Vec<Layout>,
        axis: String,
        gap: f64,
        envolver: bool,
    },
    /// Flow layout con gap y wrap automático
    FlowLayout {
        children: Vec<Layout>,
        gap: f64,
    },
    /// Grid dinámico: columnas ajustables según el tamaño de pantalla
    Grid {
        children: Vec<Layout>,
        columnas: usize,
        gap: f64,
    },
    /// Caja con relación de aspecto fija
    AspectRatioBox {
        hijo: Box<Layout>,
        ratio: f64,
    },
    // ─── Botones Material Design 3 ──────────────────────────────────
    MaterialButton {
        texto: String,
        callback: String,
        variant: ButtonVariant,
        icono: Option<String>,
        disabled: bool,
    },
    FAB {
        icono: String,
        callback: String,
        tamano: FabTamano,
        texto_extendido: Option<String>,
    },
    IconButton {
        icono: String,
        callback: String,
        variant: IconButtonVariant,
        seleccionado: bool,
    },
    SegmentedButton {
        opciones: Vec<String>,
        seleccionados: Vec<bool>,
        callback: String,
        multiple: bool,
    },
    Chip {
        texto: String,
        callback: String,
        variant: ChipVariant,
        activo: bool,
        on_quitar: Option<String>,
    },

    // ─── Inputs Material Design 3 ────────────────────────────────
    CampoTextoMaterial {
        variable: String,
        etiqueta: String,
        placeholder: String,
        variant: TextoCampoVariant,
        multiline: bool,
        error: String,
        counter: bool,
    },
    CampoContrasenaMaterial {
        variable: String,
        etiqueta: String,
        visible: bool,
    },
    CampoNumeroMaterial {
        variable: String,
        etiqueta: String,
        min: f64,
        max: f64,
        decimales: i32,
    },
    CampoBusquedaMaterial {
        variable: String,
        placeholder: String,
    },
    MaterialDropdown {
        opciones: Vec<String>,
        seleccionada: usize,
        placeholder: String,
    },
    MaterialSelect {
        opciones: Vec<String>,
        seleccionada: usize,
        etiqueta: String,
    },
    MaterialAutocomplete {
        opciones: Vec<String>,
        variable: String,
    },
    MaterialRadioGroup {
        nombre: String,
        opciones: Vec<String>,
        seleccion: usize,
        callback: String,
        direction: String,
    },
    MaterialSwitch {
        etiqueta: String,
        variable: String,
    },
    MaterialSliderDiscrete {
        variable: String,
        min: f64,
        max: f64,
        steps: i32,
    },
    MaterialSliderRange {
        variable_inicio: String,
        variable_fin: String,
        min: f64,
        max: f64,
    },
    MaterialChipGroup {
        chips: Vec<String>,
        seleccion: Vec<bool>,
        callback: String,
        multiple: bool,
    },
    MaterialDatePicker {
        variable: String,
    },
    MaterialTimePicker {
        variable: String,
    },
    ColorPicker {
        variable: String,
    },

    // ─── Tarjetas, Listas y Tablas ──────────────────────────────
    MaterialCard {
        hijo: Box<Layout>,
        variant: CardVariant,
        on_click: Option<String>,
        seleccionado: bool,
    },
    ElementoListaMaterial {
        leading: Option<Box<Layout>>,
        titulo: String,
        subtitulo: Option<String>,
        trailing: Option<Box<Layout>>,
        on_click: Option<String>,
    },
    ListaMaterial {
        items: Vec<Layout>,
        dividers: bool,
    },
    ControlListaMaterial {
        items: Vec<Layout>,
        control_tipo: String,
        variables: Vec<String>,
    },
    SeleccionListaMaterial {
        items: Vec<Layout>,
        seleccion: Vec<bool>,
        callback: String,
        multiple: bool,
    },
    /// Lista dinámica: renderiza los items de una variable (JSON array)
    /// usando una función Forja como callback de render (lista_dinamica).
    ListaDinamica {
        variable: String,
        elemento_fn: String,
    },
    /// Texto dinámico con estilo: la expresión se evalúa en tiempo de render
    /// usando el store (soporta concatenación + etiqueta_dinamica + helpers).
    DynamicText {
        expr: Box<Expresion>,
        params: HashMap<String, ValorGUI>,
        style: String,
    },
    /// Etiqueta dinámica: igual que DynamicText pero sin estilo tipográfico.
    DynamicLabel {
        expr: Box<Expresion>,
        params: HashMap<String, ValorGUI>,
    },
    TablaDatosMaterial {
        columnas: Vec<String>,
        filas: Vec<Vec<String>>,
        ordenable: bool,
        seleccionable: bool,
        col_orden: usize,
        orden_asc: bool,
    },
    MaterialSurface {
        hijo: Box<Layout>,
        color_role: String,
    },
    MaterialScaffold {
        top: Option<Box<Layout>>,
        body: Box<Layout>,
        bottom: Option<Box<Layout>>,
        fab: Option<Box<Layout>>,
    },
    /// Barra de progreso lineal con valor directo (para items de listas
    /// donde el valor viene de un mapa, ej: barra_progreso(item["porcentaje"])).
    ValorProgresoLineal(f64),

    // ─── Feedback y Superposiciones ──────────────────────────────

    // Dialogs
    DialogOverlay {
        dialog: Box<Layout>,
        visible: String, // nombre variable bool
    },
    DialogAlert {
        titulo: String,
        mensaje: String,
        confirmar_texto: String,
        cancelar_texto: String,
        on_confirm: String,
        on_cancel: String,
    },
    DialogCustom {
        titulo: String,
        hijo: Box<Layout>,
        on_cerrar: String,
    },

    // Bottom Sheets
    BottomSheet {
        hijo: Box<Layout>,
        variant: SheetVariant, // Standard | Modal | Expanded
        visible: String,
        on_dismiss: Option<String>,
    },

    // Snackbar
    Snackbar {
        mensaje: String,
        accion_texto: Option<String>,
        accion_callback: Option<String>,
        duracion: f64, // en ms
        visible: String,
    },

    // Tooltip
    Tooltip {
        hijo: Box<Layout>,
        texto: String,
    },

    // Menu
    Menu {
        items: Vec<String>,
        on_select: String,
        visible: String,
    },
    ContextMenu {
        items: Vec<String>,
        on_select: String,
        visible: String,
    },

    // === NAVEGACIÓN ===
    /// Navigator: sistema de navegación por pantallas con push/pop
    Navigator {
        screens: Vec<NavigatorScreen>,
        /// Variable que almacena el ID de la pantalla actual
        actual_variable: String,
        /// Variable para pila de navegación (historial)
        history_var: String,
        /// Tipo de navegación visual
        nav_tipo: NavigatorTipo,
        /// Animación de transición
        anim: NavigatorAnim,
        /// Callback opcional para cuando cambia la pantalla
        on_change: Option<String>,
    },
    NavigationBar {
        items: Vec<NavElemento>,
        seleccion: usize,
        /// Variable del `.fa` que guarda el índice seleccionado (2º argumento);
        /// sin esto la barra escribía en una variable con el nombre del callback
        /// y la página nunca cambiaba.
        variable_seleccion: Option<String>,
        on_change: String,
    },
    NavigationRail {
        items: Vec<NavElemento>,
        seleccion: usize,
        variable_seleccion: Option<String>,
        on_change: String,
        extended: bool, // muestra labels
    },
    NavigationDrawer {
        items: Vec<NavElemento>,
        seleccion: usize,
        variable_seleccion: Option<String>,
        on_change: String,
        modal: bool,
        visible: String,
    },
    TopAppBar {
        titulo: String,
        acciones: Vec<IconAction>,
        menu_visible: bool,
        variant: TopAppBarVariant, // Small | Medium | Large
    },
    BottomAppBar {
        acciones: Vec<IconAction>,
        fab: Option<Box<Layout>>,
    },
    Tabs {
        tabs: Vec<String>,
        seleccion: usize,
        on_change: String,
        scrollable: bool,
    },
    BarraBusqueda {
        placeholder: String,
        on_buscar: String,
        variable: String,
    },
    VistaBusqueda {
        query: String,
        resultados: Vec<Layout>,
        visible: String,
    },

    // ═══════════════════════════════════════════════════════════════════
    // INDICADORES (Fase 7)
    // ═══════════════════════════════════════════════════════════════════
    /// Barra de progreso lineal
    LinearProgress {
        variable: String,
        indeterminado: bool,
    },
    /// Círculo de progreso
    CircularProgress {
        variable: String,
        tamano: f64,
        indeterminado: bool,
    },
    /// Badge (distintivo) sobre un hijo
    Badge {
        hijo: Box<Layout>,
        valor: Option<String>,
        dot: bool,
    },
    /// Skeleton (placeholder de carga)
    Skeleton {
        ancho: f64,
        alto: f64,
        tipo: String,
    },
    /// Empty state (estado vacío)
    EmptyState {
        icono: String,
        mensaje: String,
        accion_texto: Option<String>,
        accion_cb: Option<String>,
    },
    /// Error state (estado de error)
    ErrorState {
        mensaje: String,
        on_retry: Option<String>,
    },

    // ═══════════════════════════════════════════════════════════════════
    // AVATARES (Fase 7)
    // ═══════════════════════════════════════════════════════════════════
    /// Avatar individual
    Avatar {
        texto: String,
        variant: AvatarVariant,
        tamaño: f64,
    },
    /// Grupo de avatares
    AvatarGroup {
        avatares: Vec<String>,
        max: usize,
    },

    // ═══════════════════════════════════════════════════════════════════
    // MOTION (Fase 8)
    // ═══════════════════════════════════════════════════════════════════
    /// Transición Fade (muestra/oculta con opacidad)
    FadeTransition {
        hijo: Box<Layout>,
        visible: String,
        duracion: f64,
    },
    /// Efecto ripple (onda al hacer clic)
    RippleEffect {
        hijo: Box<Layout>,
        color: String,
    },

    // ═══════════════════════════════════════════════════════════════════
    // INTERACCIONES (Pull-to-refresh, Swipe-to-dismiss)
    // ═══════════════════════════════════════════════════════════════════
    /// Pull-to-refresh: envuelve un hijo con capacidad de recargar al tirar hacia abajo
    PullToRefresh {
        hijo: Box<Layout>,
        callback: String,
        refreshing: String,
    },
    /// Swipe-to-dismiss: deslizar para descartar con opción de deshacer
    SwipeToDismiss {
        hijo: Box<Layout>,
        on_dismiss: String,
        etiqueta: String,
        dismissed: String,
    },

    /// Pinch-to-zoom: zoom por pellizco multi-touch
    PinchZoom {
        hijo: Box<Layout>,
        min_scale: f64,
        max_scale: f64,
    },
    /// Rotate: rotación por gesto multi-touch
    Rotate {
        hijo: Box<Layout>,
    },

    // ═══════════════════════════════════════════════════════════════════
    // GRÁFICOS (Fase 9)
    // ═══════════════════════════════════════════════════════════════════
    /// Gráfico de líneas
    GraficoLineas {
        datos: Vec<f64>,
        color: String,
        etiquetas: Vec<String>,
    },
    /// Gráfico de barras
    BarChart {
        datos: Vec<f64>,
        colores: Vec<String>,
        etiquetas: Vec<String>,
        apilado: bool,
    },
    /// Gráfico de pastel / donut
    PieChart {
        datos: Vec<f64>,
        etiquetas: Vec<String>,
        donut: bool,
    },
    /// Gráfico indicador tipo Gauge
    GaugeChart {
        valor: f64,
        min: f64,
        max: f64,
        color: String,
    },
    /// Mini gráfico (Sparkline)
    Sparkline {
        datos: Vec<f64>,
        color: String,
    },

    // ═══════════════════════════════════════════════════════════════════
    // AVANZADOS (Fase 9)
    // ═══════════════════════════════════════════════════════════════════
    /// Calificación por estrellas
    StarRating {
        valor: usize,
        /// Variable de la que se lee el valor en cada render (si el `.fa` pasó
        /// un identificador en vez de un literal).
        variable: Option<String>,
        max: usize,
        callback: String,
    },
    /// Asistente de pasos (Stepper)
    Stepper {
        pasos: Vec<String>,
        actual: usize,
        callback: String,
    },
    /// Migas de pan (Breadcrumbs)
    Breadcrumbs {
        items: Vec<String>,
        separador: String,
    },
    /// Calendario
    Calendar {
        mes: i32,
        año: i32,
        seleccionado: Option<String>,
        callback: String,
    },
    /// Visor de Markdown
    MarkdownViewer {
        texto: String,
    },
    /// Generador de QR
    QRCode {
        texto: String,
        tamaño: f64,
    },
    /// Selector de archivos
    SelectorArchivo {
        tipos: Vec<String>,
        multiple: bool,
        callback: String,
    },

    // ═══════════════════════════════════════════════════════════════════
    // EXPRESSIVE (Fase 10)
    // ═══════════════════════════════════════════════════════════════════
    /// Tarjeta con efecto vidrio (Glassmorphism)
    GlassCard {
        hijo: Box<Layout>,
        blur: f64,
        opacity: f64,
    },
    /// Caja con gradiente
    GradientBox {
        hijo: Box<Layout>,
        colores: Vec<String>,
        direccion: String,
    },
    /// Botón morphing (icono → texto)
    MorphingButton {
        icono: String,
        texto_extendido: String,
        callback: String,
    },
    /// Fondo expresivo animado
    ExpressiveBackground {
        colores: Vec<String>,
        animado: bool,
    },
    /// Borde con brillo (Glow)
    GlowBorder {
        hijo: Box<Layout>,
        color: String,
        ancho: f64,
    },

    /// Lienzo de dibujo libre (Canvas / Pintura)
    ///
    /// Lee comandos de dibujo desde una variable JSON en VariableStore.
    /// Los comandos se especifican como un array JSON del tipo [`CanvasCommand`].
    Canvas {
        commands_var: String,
        width: f64,
        height: f64,
    },

    // ═══════════════════════════════════════════════════════════════════
    // ICONOS MATERIAL (Fase Iconos)
    // ═══════════════════════════════════════════════════════════════════
    /// Icono Material Design vectorial
    MaterialIconLayout {
        nombre: String, // nombre del icono: "home", "favorite"
        tamaño: f64,    // tamaño en píxeles (ej: 24, 32, 48)
        color: String,  // color (hex o role del tema)
        estilo: String, // "filled", "outlined", "rounded", "sharp", "twotone"
    },
}

// ─── AST → Layout ─────────────────────────────────────────────────

/// Extrae el layout del AST (recursivo: soporta desplazable, pila, etc.)
pub fn extraer_layout(decls: &[Declaracion], almacen: &mut VariableAlmacen) -> Layout {
    for decl in decls {
        if let Declaracion::Funcion { nombre, cuerpo, .. } = decl {
            if nombre == "main" {
                let children = layouts_de_bloque(cuerpo, almacen, decls);
                // Una sola sentencia de nivel superior → ese layout tal cual (así
                // las apps que ya envuelven todo en `columna(...)` no cambian).
                // Varias → se concatenan en una columna implícita.
                let layout = if children.len() == 1 {
                    children.into_iter().next().unwrap()
                } else {
                    Layout::Column {
                        children,
                        gap: 0.0,
                        alignment: "start".to_string(),
                    }
                };
                return layout;
            }
        }
    }
    Layout::Column {
        children: vec![],
        gap: 0.0,
        alignment: "start".to_string(),
    }
}

/// Recorre un bloque de sentencias y devuelve los layouts que produce, en orden.
///
/// - Llamadas y expresiones sueltas se mapean con [`expr_a_layout`].
/// - `si/sino` se resuelve con el estado inicial (el mismo con el que arranca la
///   app) y se sigue sólo por la rama elegida: el layout se extrae una vez, no
///   se re-evalúa cuando la UI cambia de página.
/// - Lo que no produce layout (declaraciones de variables, bucles, etc.) se ignora.
fn layouts_de_bloque(
    cuerpo: &[Declaracion],
    almacen: &mut VariableAlmacen,
    decls: &[Declaracion],
) -> Vec<Layout> {
    cuerpo
        .iter()
        .flat_map(|d| expandir_declaracion_layout(d, almacen, decls, 0))
        .filter_map(|expr| expr_a_layout(&expr))
        .collect()
}

/// Cuántas llamadas a funciones de usuario se inlinean como máximo.
const MAXIMO_LAYOUT_EN_LINEA: u8 = 8;

/// Busca el cuerpo de una función de usuario (distinta de `main`).
fn cuerpo_de_funcion<'a>(nombre: &str, decls: &'a [Declaracion]) -> Option<&'a Vec<Declaracion>> {
    decls.iter().find_map(|d| match d {
        Declaracion::Funcion {
            nombre: n, cuerpo, ..
        } if n == nombre => Some(cuerpo),
        _ => None,
    })
}

/// Convierte una sentencia en las expresiones de layout que produce, resolviendo
/// `si/sino` con el estado actual e inlineando llamadas a funciones de usuario.
///
/// Las ramas y los cuerpos inlineados son sentencias, así que se expanden
/// recursivamente; el resultado es una lista plana de expresiones de widgets.
fn expandir_declaracion_layout(
    decl: &Declaracion,
    almacen: &mut VariableAlmacen,
    decls: &[Declaracion],
    depth: u8,
) -> Vec<Expresion> {
    match decl {
        // Convertir Declaracion::LlamadaFuncion → Expresion::LlamadaFuncion
        // para usar expr_a_layout que ya maneja todos los wrappers
        Declaracion::LlamadaFuncion { nombre, argumentos } => expandir_expresion_layout(
            &Expresion::LlamadaFuncion {
                nombre: nombre.clone(),
                argumentos: argumentos.clone(),
            },
            almacen,
            decls,
            depth,
        ),
        Declaracion::Expresion(expr) => expandir_expresion_layout(expr, almacen, decls, depth),
        // Las funciones que arman layouts suelen devolverlo con `retornar`:
        // el valor es la expresión de layout (y en una función de valor, el
        // texto literal que después lee `extraer_texto`).
        Declaracion::Retornar { valor: Some(expr) } => {
            expandir_expresion_layout(expr, almacen, decls, depth)
        }
        Declaracion::Si {
            condicion,
            bloque_verdadero,
            bloque_falso,
        } => expandir_expresion_layout(
            &Expresion::Si {
                condicion: condicion.clone(),
                bloque_verdadero: bloque_verdadero.clone(),
                bloque_falso: bloque_falso.clone(),
            },
            almacen,
            decls,
            depth,
        ),
        _ => Vec::new(),
    }
}

/// Expande una expresión de layout a una lista plana, en este orden:
///
/// 1. `si (…) { … } sino { … }` → se evalúa la condición con el estado actual y
///    se expanden las sentencias de la rama elegida.
/// 2. Llamada a una función de usuario (nombre que no es widget) → se inlinea el
///    cuerpo, con tope [`MAXIMO_LAYOUT_EN_LINEA`] para no colgarse con recursión.
/// 3. Cualquier otra expresión se devuelve tal cual, expandiendo sus argumentos.
fn expandir_expresion_layout(
    expr: &Expresion,
    almacen: &mut VariableAlmacen,
    decls: &[Declaracion],
    depth: u8,
) -> Vec<Expresion> {
    match expr {
        Expresion::Si {
            condicion,
            bloque_verdadero,
            bloque_falso,
        } => {
            let condicion_verdadera =
                crate::evaluador::evaluar_condicion(condicion, &crate::evaluador::AlmacenGui(almacen), decls).unwrap_or(true);
            let rama = if condicion_verdadera {
                Some(bloque_verdadero)
            } else {
                bloque_falso.as_ref()
            };
            match rama {
                Some(bloque) => bloque
                    .iter()
                    .flat_map(|d| expandir_declaracion_layout(d, almacen, decls, depth))
                    .collect(),
                None => Vec::new(),
            }
        }
        Expresion::LlamadaFuncion { nombre, argumentos } => {
            let argumentos: Vec<Expresion> = argumentos
                .iter()
                .flat_map(|a| expandir_expresion_layout(a, almacen, decls, depth))
                .collect();
            // Función de usuario: puede ser de **valor** (`titulo_categoria(pagina)`)
            // o armar un **layout** (`seccion_botones(...)`). Se intenta primero
            // evaluarla: si devuelve un valor, se reemplaza la llamada por el
            // literal; si no (las llamadas a widgets no son evaluables), se inlinea
            // el cuerpo, que es lo que espera `expr_a_layout`.
            if depth < MAXIMO_LAYOUT_EN_LINEA {
                if let Some(cuerpo) = cuerpo_de_funcion(nombre, decls) {
                    let args: Result<Vec<ValorGUI>, String> = argumentos
                        .iter()
                        .map(|a| crate::evaluador::evaluar_valor(a, &crate::evaluador::AlmacenGui(almacen), decls))
                        .collect();
                    if let Ok(args) = args {
                        if let Ok(valor) =
                            crate::evaluador::ejecutar_funcion(nombre, &args, decls, &crate::evaluador::AlmacenGui(almacen))
                        {
                            return vec![valor_a_expresion(&valor)];
                        }
                    }
                    return cuerpo
                        .iter()
                        .flat_map(|d| expandir_declaracion_layout(d, almacen, decls, depth + 1))
                        .collect();
                }
            }
            vec![Expresion::LlamadaFuncion {
                nombre: nombre.clone(),
                argumentos,
            }]
        }
        Expresion::Arreglo(items) => vec![Expresion::Arreglo(
            items
                .iter()
                .flat_map(|a| expandir_expresion_layout(a, almacen, decls, depth))
                .collect(),
        )],
        Expresion::Grupo(inner) => {
            let mut expandido = expandir_expresion_layout(inner, almacen, decls, depth);
            match expandido.len() {
                0 => Vec::new(),
                1 => vec![Expresion::Grupo(Box::new(expandido.remove(0)))],
                _ => expandido,
            }
        }
        otra => vec![otra.clone()],
    }
}

/// Convierte el valor devuelto por una función de valor en la expresión
/// literal equivalente, para inyectarla en el árbol de layout.
fn valor_a_expresion(valor: &ValorGUI) -> Expresion {
    match valor {
        ValorGUI::Texto(s) => Expresion::LiteralTexto(s.clone()),
        ValorGUI::Entero(n) => Expresion::LiteralNumero(*n),
        ValorGUI::Decimal(d) => Expresion::LiteralDecimal(*d),
        ValorGUI::Booleano(b) => Expresion::LiteralBooleano(*b),
        ValorGUI::Nulo => Expresion::LiteralNulo,
        // Mapas/estructuras no tienen literal en el AST: se muestran como texto.
        otro => Expresion::LiteralTexto(otro.to_display()),
    }
}

/// Nombre de la variable que se pasa como argumento (p. ej. la selección de una
/// barra de navegación: `riel_navegacion(items, pagina, &cb)`).
fn variable_de(args: &[Expresion], index: usize) -> Option<String> {
    match args.get(index) {
        Some(Expresion::Identificador { nombre, .. }) => Some(nombre.clone()),
        _ => None,
    }
}

fn procesar_args(args: &[Expresion]) -> Vec<Layout> {
    // Los hijos se pueden pasar en forma variádica (`columna(a, b)`) o como un
    // único arreglo (`columna([a, b])`), que es lo que usan varios ejemplos.
    if let [Expresion::Arreglo(items)] = args {
        return items.iter().filter_map(expr_a_layout).collect();
    }
    args.iter().filter_map(expr_a_layout).collect()
}

// ─── Helpers para extraer argumentos → movidos a crate::helpers ─────

// ─── AST → Layout ─────────────────────────────────────────────────

/// 5.8: Tipo para la función de procesamiento de hijos.
/// Permite que `parse_layout_call` use tanto `procesar_args` como `procesar_args_item`.
type ChildProcessor =
    fn(&[Expresion], &HashMap<String, ValorGUI>, &VariableAlmacen, &[Declaracion]) -> Vec<Layout>;

/// 5.8: Wrapper que adapta `procesar_args` al signature de `ChildProcessor`.
/// Disponible para futura migración de `expr_a_layout` a usar `parse_layout_call`.
#[allow(dead_code)]
fn procesar_args_wrapper(
    args: &[Expresion],
    _params: &HashMap<String, ValorGUI>,
    _almacen: &VariableAlmacen,
    _declaraciones: &[Declaracion],
) -> Vec<Layout> {
    procesar_args(args)
}

/// 5.8: Extrae gap y alignment de argumentos (parámetros 2 y 3).
fn extract_gap_alignment(argumentos: &[Expresion]) -> (f64, String) {
    let gap = argumentos
        .get(1)
        .and_then(|a| match a {
            Expresion::LiteralNumero(n) => Some(*n as f64),
            Expresion::LiteralDecimal(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(0.0);
    let alignment = argumentos
        .get(2)
        .map(|a| match a {
            Expresion::LiteralTexto(s) => s.clone(),
            _ => "start".to_string(),
        })
        .unwrap_or_else(|| "start".to_string());
    (gap, alignment)
}

/// 5.8: Extrae children de un Expresion::Arreglo usando el process_children dado.
fn extract_children_from_array(
    arg: &Expresion,
    params: &HashMap<String, ValorGUI>,
    almacen: &VariableAlmacen,
    declaraciones: &[Declaracion],
    procesar_children: ChildProcessor,
) -> Vec<Layout> {
    match arg {
        Expresion::Arreglo(exprs) => procesar_children(exprs, params, almacen, declaraciones),
        _ => Vec::new(),
    }
}

/// 5.8: Función compartida para parsear llamadas de función → Layout.
/// Maneja los casos comunes entre `expr_a_layout` y `expr_a_layout_item`.
/// Retorna Some(Layout) si es un caso compartido, None si el caller debe manejarlo.
fn parsear_layout_llamada(
    nombre: &str,
    argumentos: &[Expresion],
    params: &HashMap<String, ValorGUI>,
    almacen: &VariableAlmacen,
    declaraciones: &[Declaracion],
    procesar_children: ChildProcessor,
) -> Option<Layout> {
    match nombre {
        "columna" | "gui_columna" => Some(Layout::Column {
            children: procesar_children(argumentos, params, almacen, declaraciones),
            gap: 0.0,
            alignment: "start".to_string(),
        }),
        "fila" | "gui_fila" => Some(Layout::Row {
            children: procesar_children(argumentos, params, almacen, declaraciones),
            gap: 0.0,
            alignment: "start".to_string(),
        }),
        "columna_con_gap" | "column_with_gap" => {
            let children = argumentos
                .first()
                .map(|a| {
                    extract_children_from_array(
                        a,
                        params,
                        almacen,
                        declaraciones,
                        procesar_children,
                    )
                })
                .unwrap_or_default();
            let (gap, alignment) = extract_gap_alignment(argumentos);
            Some(Layout::Column {
                children,
                gap,
                alignment,
            })
        }
        "fila_con_gap" | "row_with_gap" => {
            let children = argumentos
                .first()
                .map(|a| {
                    extract_children_from_array(
                        a,
                        params,
                        almacen,
                        declaraciones,
                        procesar_children,
                    )
                })
                .unwrap_or_default();
            let (gap, alignment) = extract_gap_alignment(argumentos);
            Some(Layout::Row {
                children,
                gap,
                alignment,
            })
        }
        "espacio" | "spacer" => {
            let tamano = argumentos
                .first()
                .and_then(|a| match a {
                    Expresion::LiteralNumero(n) => Some(*n as f64),
                    _ => None,
                })
                .unwrap_or(10.0);
            Some(Layout::Spacer(tamano))
        }
        "flujo" | "flow_layout" | "flow" => {
            let children = argumentos
                .first()
                .and_then(|a| match a {
                    Expresion::Arreglo(exprs) => {
                        Some(procesar_children(exprs, params, almacen, declaraciones))
                    }
                    _ => None,
                })
                .unwrap_or_default();
            let gap = argumentos
                .get(1)
                .and_then(|a| match a {
                    Expresion::LiteralNumero(n) => Some(*n as f64),
                    Expresion::LiteralDecimal(f) => Some(*f),
                    _ => None,
                })
                .unwrap_or(0.0);
            Some(Layout::FlowLayout { children, gap })
        }
        "grid" | "cuadricula" | "cuadrícula" => {
            let children = argumentos
                .first()
                .and_then(|a| match a {
                    Expresion::Arreglo(exprs) => {
                        Some(procesar_children(exprs, params, almacen, declaraciones))
                    }
                    _ => None,
                })
                .unwrap_or_default();
            let columnas = argumentos
                .get(1)
                .and_then(|a| match a {
                    Expresion::LiteralNumero(n) => Some(*n as usize),
                    _ => None,
                })
                .unwrap_or(2);
            let gap = argumentos
                .get(2)
                .and_then(|a| match a {
                    Expresion::LiteralNumero(n) => Some(*n as f64),
                    Expresion::LiteralDecimal(f) => Some(*f),
                    _ => None,
                })
                .unwrap_or(8.0);
            Some(Layout::Grid {
                children,
                columnas,
                gap,
            })
        }
        "boton" | "button" | "btn" => {
            let texto = extraer_texto(argumentos, 0);
            let callback = extraer_callback(argumentos, 1);
            Some(Layout::MaterialButton {
                texto,
                callback,
                variant: ButtonVariant::Filled,
                icono: None,
                disabled: false,
            })
        }
        _ => None, // No es un caso compartido; el caller maneja el resto
    }
}

pub fn expr_a_layout(expr: &Expresion) -> Option<Layout> {
    match expr {
        Expresion::LlamadaFuncion { nombre, argumentos } => {
            match nombre.as_str() {
                "escribir" | "etiqueta" | "label" | "text" => {
                    if let Some(arg) = argumentos.first() {
                        match arg {
                            Expresion::Identificador { nombre: v, .. } => Some(Layout::Label {
                                texto: v.clone(),
                                es_variable: true,
                            }),
                            Expresion::LiteralTexto(s) => Some(Layout::Label {
                                texto: s.clone(),
                                es_variable: false,
                            }),
                            _ => Some(Layout::DynamicLabel {
                                expr: Box::new(arg.clone()),
                                params: HashMap::new(),
                            }),
                        }
                    } else {
                        Some(Layout::Spacer(0.0))
                    }
                }
                "etiqueta_titulo" | "titulo" | "title" => {
                    let texto = argumentos
                        .first()
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    Some(Layout::Title(texto))
                }
                "etiqueta_color" | "texto_color" | "colored_label" => {
                    let texto = argumentos
                        .first()
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    let color = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => "defecto".to_string(),
                        })
                        .unwrap_or_else(|| "defecto".to_string());
                    Some(Layout::ColoredLabel { texto, color })
                }
                "etiqueta_dinamica" | "varlabel" => {
                    let variable = argumentos
                        .first()
                        .map(|a| match a {
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    Some(Layout::VariableLabel { variable })
                }
                "boton" | "button" | "btn" => {
                    let texto = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::MaterialButton {
                        texto,
                        callback,
                        variant: ButtonVariant::Filled,
                        icono: None,
                        disabled: false,
                    })
                }
                "entrada_texto" | "text_input" | "input" => {
                    let variable = argumentos
                        .first()
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    let placeholder = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    Some(Layout::EntradaTexto {
                        variable,
                        multiline: false,
                        placeholder,
                    })
                }
                "area_texto" | "textarea" => {
                    let variable = argumentos
                        .first()
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    let placeholder = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    Some(Layout::EntradaTexto {
                        variable,
                        multiline: true,
                        placeholder,
                    })
                }
                "gui_barra_progreso" | "progress_bar" | "progress" => {
                    let variable = argumentos
                        .first()
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    Some(Layout::ProgressBar { variable })
                }
                "deslizante" | "gui_deslizante" | "slider" => {
                    let variable = argumentos
                        .first()
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    let min = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            _ => None,
                        })
                        .unwrap_or(0.0);
                    let max = argumentos
                        .get(2)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            _ => None,
                        })
                        .unwrap_or(100.0);
                    Some(Layout::Slider { variable, min, max })
                }
                "casilla" | "gui_casilla" | "checkbox" | "check" => {
                    let variable = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            _ => String::new(),
                        })
                        .or_else(|| {
                            argumentos.first().map(|a| match a {
                                Expresion::LiteralTexto(s) => s.clone(),
                                Expresion::Identificador { nombre: s, .. } => s.clone(),
                                _ => String::new(),
                            })
                        })
                        .unwrap_or_default();
                    Some(Layout::Checkbox { variable })
                }
                "texto_enriquecido" | "prose" => {
                    let texto = argumentos
                        .first()
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    Some(Layout::Prose(texto))
                }
                "cargando" | "spinner" => Some(Layout::Spinner),
                "separador" | "divider" => Some(Layout::Separator),
                "espacio" | "spacer" => {
                    let tamano = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            _ => None,
                        })
                        .unwrap_or(10.0);
                    Some(Layout::Spacer(tamano))
                }
                "columna" | "gui_columna" => Some(Layout::Column {
                    children: procesar_args(argumentos),
                    gap: 0.0,
                    alignment: "start".to_string(),
                }),
                "columna_centrada" | "centered_column" | "center_col" => {
                    Some(Layout::CenteredColumn(procesar_args(argumentos)))
                }
                "fila" | "gui_fila" => Some(Layout::Row {
                    children: procesar_args(argumentos),
                    gap: 0.0,
                    alignment: "start".to_string(),
                }),
                "pila" | "gui_pila" | "zstack" => Some(Layout::PilaZ(procesar_args(argumentos))),
                "desplazable" | "gui_desplazable" | "scroll" => argumentos
                    .first()
                    .and_then(expr_a_layout)
                    .map(|hijo| Layout::Portal(Box::new(hijo))),
                "contenedor" | "container" | "caja" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let max_ancho = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            _ => None,
                        })
                        .unwrap_or(400.0);
                    hijo.map(|c| Layout::Container {
                        hijo: Box::new(c),
                        max_ancho,
                    })
                }
                // ─── Configuración de la ventana ────────────────────
                // `ventana(...)` no dibuja nada: se lee del AST al crear la
                // ventana (ver `window_config`). Devolver `None` la saca del árbol.
                "ventana" | "configurar_ventana" | "window" | "configurar_ventana_gui" => None,

                // ─── Funciones de tema Material You ────────────────
                // Proveedor de tema: cambia el color semilla
                "tema_material" | "theme_provider" => {
                    let seed = argumentos
                        .first()
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => "#6750A4".to_string(),
                        })
                        .unwrap_or_else(|| "#6750A4".to_string());
                    let hijo = argumentos.get(1).and_then(expr_a_layout);
                    hijo.map(|c| Layout::ThemeProvider {
                        hijo: Box::new(c),
                        theme: seed,
                    })
                }
                // ─── Diseños predeterminados (claro/oscuro) ──────────
                // tema_fluent(hijo) / tema_fluent_oscuro(hijo)
                "tema_fluent" | "fluent" | "tema_windows11" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::ThemeProvider {
                        hijo: Box::new(c),
                        theme: "fluent:light".to_string(),
                    })
                }
                "tema_fluent_oscuro" | "fluent_oscuro" | "tema_windows11_oscuro" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::ThemeProvider {
                        hijo: Box::new(c),
                        theme: "fluent:dark".to_string(),
                    })
                }
                // tema_liquid_glass(hijo) / tema_liquid_glass_oscuro(hijo)
                "tema_liquid_glass" | "tema_vidrio" | "liquid_glass" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::ThemeProvider {
                        hijo: Box::new(c),
                        theme: "liquid_glass:light".to_string(),
                    })
                }
                "tema_liquid_glass_oscuro" | "tema_vidrio_oscuro" | "liquid_glass_oscuro" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::ThemeProvider {
                        hijo: Box::new(c),
                        theme: "liquid_glass:dark".to_string(),
                    })
                }
                // Colores del tema (envuelven un hijo con un color role)
                "color_primario" | "color_primary" => {
                    envolver_con_color_role(argumentos, "primary")
                }
                "color_sobre_primario" | "color_on_primary" => {
                    envolver_con_color_role(argumentos, "on_primary")
                }
                "color_primario_contenedor" | "color_primary_container" => {
                    envolver_con_color_role(argumentos, "primary_container")
                }
                "color_sobre_primario_contenedor" | "color_on_primary_container" => {
                    envolver_con_color_role(argumentos, "on_primary_container")
                }
                "color_secundario" | "color_secondary" => {
                    envolver_con_color_role(argumentos, "secondary")
                }
                "color_sobre_secundario" | "color_on_secondary" => {
                    envolver_con_color_role(argumentos, "on_secondary")
                }
                "color_secundario_contenedor" | "color_secondary_container" => {
                    envolver_con_color_role(argumentos, "secondary_container")
                }
                "color_sobre_secundario_contenedor" | "color_on_secondary_container" => {
                    envolver_con_color_role(argumentos, "on_secondary_container")
                }
                "color_terciario" | "color_tertiary" => {
                    envolver_con_color_role(argumentos, "tertiary")
                }
                "color_sobre_terciario" | "color_on_tertiary" => {
                    envolver_con_color_role(argumentos, "on_tertiary")
                }
                "color_error" | "color_error_role" => envolver_con_color_role(argumentos, "error"),
                "color_sobre_error" | "color_on_error" => {
                    envolver_con_color_role(argumentos, "on_error")
                }
                "color_superficie" | "color_surface" => {
                    envolver_con_color_role(argumentos, "surface")
                }
                "color_sobre_superficie" | "color_on_surface" => {
                    envolver_con_color_role(argumentos, "on_surface")
                }
                "color_fondo" | "color_background" => {
                    envolver_con_color_role(argumentos, "background")
                }
                "color_sobre_fondo" | "color_on_background" => {
                    envolver_con_color_role(argumentos, "on_background")
                }
                "color_perfil" | "color_outline" => envolver_con_color_role(argumentos, "outline"),

                // Estilos tipográficos (15 estilos Material You)
                "texto_grande" | "display_large" => styled_etiqueta(argumentos, "display_large"),
                "texto_mediano" | "display_medium" => styled_etiqueta(argumentos, "display_medium"),
                "texto_pequeño" | "texto_pequeno" | "display_small" => {
                    styled_etiqueta(argumentos, "display_small")
                }
                "titular_grande" | "headline_large" => {
                    styled_etiqueta(argumentos, "headline_large")
                }
                "titular_mediano" | "headline_medium" => {
                    styled_etiqueta(argumentos, "headline_medium")
                }
                "titular_pequeño" | "headline_small" => {
                    styled_etiqueta(argumentos, "headline_small")
                }
                "encabezado_grande" | "title_large" => styled_etiqueta(argumentos, "title_large"),
                "encabezado_mediano" | "title_medium" => {
                    styled_etiqueta(argumentos, "title_medium")
                }
                "encabezado_pequeño" | "title_small" => styled_etiqueta(argumentos, "title_small"),
                "cuerpo_grande" | "body_large" => styled_etiqueta(argumentos, "body_large"),
                "cuerpo_mediano" | "body_medium" => styled_etiqueta(argumentos, "body_medium"),
                "cuerpo_pequeño" | "body_small" => styled_etiqueta(argumentos, "body_small"),
                "etiqueta_grande" | "label_large" => styled_etiqueta(argumentos, "label_large"),
                "etiqueta_mediana" | "label_medium" => styled_etiqueta(argumentos, "label_medium"),
                "etiqueta_pequeña" | "label_small" => styled_etiqueta(argumentos, "label_small"),

                // Formas (border-radius)
                "esquinas_pequeñas" | "shape_small" => envolver_con_forma(argumentos, "small"),
                "esquinas_medianas" | "shape_medium" => envolver_con_forma(argumentos, "medium"),
                "esquinas_grandes" | "shape_large" => envolver_con_forma(argumentos, "large"),
                "esquinas_completas" | "shape_full" => envolver_con_forma(argumentos, "full"),

                // Layout avanzado
                "relleno" | "padding" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let amount = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            _ => None,
                        })
                        .unwrap_or(8.0);
                    hijo.map(|c| Layout::Padding {
                        hijo: Box::new(c),
                        amount,
                    })
                }
                "expansor" | "expanded" => argumentos
                    .first()
                    .and_then(expr_a_layout)
                    .map(|c| Layout::Expanded { hijo: Box::new(c) }),
                "centrado" | "centered" | "center" => argumentos
                    .first()
                    .and_then(expr_a_layout)
                    .map(|c| Layout::Centered { hijo: Box::new(c) }),
                "sombra" | "shadow" | "elevated" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let level = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as u8),
                            _ => None,
                        })
                        .unwrap_or(1);
                    let forma_family = argumentos
                        .get(2)
                        .and_then(|a| match a {
                            Expresion::LiteralTexto(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "surface".to_string());
                    hijo.map(|c| Layout::ElevatedBox {
                        hijo: Box::new(c),
                        level,
                        forma_family,
                    })
                }
                "adaptable" | "responsive" => {
                    let compact = argumentos.first().and_then(expr_a_layout);
                    let medium = argumentos.get(1).and_then(expr_a_layout);
                    let expanded = argumentos.get(2).and_then(expr_a_layout);
                    match (compact, medium, expanded) {
                        (Some(c), Some(m), Some(e)) => Some(Layout::ResponsiveLayout {
                            compact: Box::new(c),
                            medium: Box::new(m),
                            expanded: Box::new(e),
                        }),
                        (Some(c), _, _) => Some(c), // fallback: solo compact
                        _ => None,
                    }
                }

                // ─── Layout responsive avanzado ────────────────
                // columna_con_gap(hijos, gap, alinear)
                // hijos se pasa como tupla: (hijo1, hijo2, ...)
                "columna_con_gap" | "column_with_gap" => {
                    let children = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(procesar_args(exprs)),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let gap = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(0.0);
                    let alignment = argumentos
                        .get(2)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => "start".to_string(),
                        })
                        .unwrap_or_else(|| "start".to_string());
                    Some(Layout::Column {
                        children,
                        gap,
                        alignment,
                    })
                }

                // fila_con_gap(hijos, gap, alinear)
                "fila_con_gap" | "row_with_gap" => {
                    let children = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(procesar_args(exprs)),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let gap = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(0.0);
                    let alignment = argumentos
                        .get(2)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => "start".to_string(),
                        })
                        .unwrap_or_else(|| "start".to_string());
                    Some(Layout::Row {
                        children,
                        gap,
                        alignment,
                    })
                }

                // flujo(hijos, gap) — Flow layout con wrap
                "flujo" | "flow_layout" | "flow" => {
                    let children = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(procesar_args(exprs)),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let gap = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(0.0);
                    Some(Layout::FlowLayout { children, gap })
                }

                // grid(hijos, columnas, gap) — Grid dinámico
                // `grilla`/`cuadrícula` son los nombres documentados en el stdlib.
                "grid" | "grilla" | "cuadricula" | "cuadrícula" => {
                    let children = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(procesar_args(exprs)),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let columnas = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as usize),
                            _ => None,
                        })
                        .unwrap_or(2);
                    let gap = argumentos
                        .get(2)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(8.0);
                    Some(Layout::Grid {
                        children,
                        columnas,
                        gap,
                    })
                }

                // caja_relativa(hijo, proporcion) — Aspect ratio
                "caja_relativa" | "aspect_ratio" | "aspectratio" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let ratio = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(1.0);
                    hijo.map(|c| Layout::AspectRatioBox {
                        hijo: Box::new(c),
                        ratio,
                    })
                }

                // flex_layout(hijos, axis, gap, wrap) — Flex layout configurable
                "flex_layout" | "flex" => {
                    let children = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(procesar_args(exprs)),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let axis = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => "vertical".to_string(),
                        })
                        .unwrap_or_else(|| "vertical".to_string());
                    let gap = argumentos
                        .get(2)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(0.0);
                    let envolver = argumentos
                        .get(3)
                        .and_then(|a| match a {
                            Expresion::LiteralBooleano(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(false);
                    Some(Layout::FlexLayout {
                        children,
                        axis,
                        gap,
                        envolver,
                    })
                }

                // ─── Botones Material Design 3 ──────────────────────

                // Variantes principales (5)
                "boton_relleno" | "filled_button" => {
                    let texto = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::MaterialButton {
                        texto,
                        callback,
                        variant: ButtonVariant::Filled,
                        icono: None,
                        disabled: false,
                    })
                }
                "boton_tonal" | "tonal_button" => {
                    let texto = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::MaterialButton {
                        texto,
                        callback,
                        variant: ButtonVariant::Tonal,
                        icono: None,
                        disabled: false,
                    })
                }
                "boton_perfilado" | "outlined_button" => {
                    let texto = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::MaterialButton {
                        texto,
                        callback,
                        variant: ButtonVariant::Outlined,
                        icono: None,
                        disabled: false,
                    })
                }
                "boton_texto" | "text_button_cmd" => {
                    let texto = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::MaterialButton {
                        texto,
                        callback,
                        variant: ButtonVariant::Text,
                        icono: None,
                        disabled: false,
                    })
                }
                "boton_elevado" | "elevated_button" => {
                    let texto = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::MaterialButton {
                        texto,
                        callback,
                        variant: ButtonVariant::Elevated,
                        icono: None,
                        disabled: false,
                    })
                }

                // FAB (Floating Action Button)
                "fab" => {
                    let icono = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::FAB {
                        icono,
                        callback,
                        tamano: FabTamano::Medium,
                        texto_extendido: None,
                    })
                }
                "fab_pequeño" | "fab_pequeno" | "fab_small" => {
                    let icono = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::FAB {
                        icono,
                        callback,
                        tamano: FabTamano::Small,
                        texto_extendido: None,
                    })
                }
                "fab_grande" | "fab_large" => {
                    let icono = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::FAB {
                        icono,
                        callback,
                        tamano: FabTamano::Large,
                        texto_extendido: None,
                    })
                }
                "fab_extendido" | "fab_extended" => {
                    let texto = extraer_texto(argumentos, 0);
                    let icono = extraer_texto(argumentos, 1);
                    let callback = extraer_callback(argumentos, 2);
                    Some(Layout::FAB {
                        icono,
                        callback,
                        tamano: FabTamano::Medium,
                        texto_extendido: Some(texto),
                    })
                }

                // Icon Buttons
                "boton_icono" | "icon_button" => {
                    let icono = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::IconButton {
                        icono,
                        callback,
                        variant: IconButtonVariant::Standard,
                        seleccionado: false,
                    })
                }
                "boton_icono_relleno" | "icon_button_filled" => {
                    let icono = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::IconButton {
                        icono,
                        callback,
                        variant: IconButtonVariant::Filled,
                        seleccionado: false,
                    })
                }
                "boton_icono_tonal" | "icon_button_tonal" => {
                    let icono = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::IconButton {
                        icono,
                        callback,
                        variant: IconButtonVariant::Tonal,
                        seleccionado: false,
                    })
                }
                "boton_icono_perfilado" | "icon_button_outlined" => {
                    let icono = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::IconButton {
                        icono,
                        callback,
                        variant: IconButtonVariant::Outlined,
                        seleccionado: false,
                    })
                }

                // Segmented Buttons
                "segmentado" | "segmented_button" => {
                    let opciones = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(
                                exprs
                                    .iter()
                                    .filter_map(|e| match e {
                                        Expresion::LiteralTexto(s) => Some(s.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let seleccion_idx = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as usize),
                            _ => None,
                        })
                        .unwrap_or(0);
                    let callback = extraer_callback(argumentos, 2);
                    let mut seleccionados = vec![false; opciones.len()];
                    if seleccion_idx < seleccionados.len() {
                        seleccionados[seleccion_idx] = true;
                    }
                    Some(Layout::SegmentedButton {
                        opciones,
                        seleccionados,
                        callback,
                        multiple: false,
                    })
                }
                "segmentado_multiple" | "segmented_button_multiple" => {
                    let opciones = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(
                                exprs
                                    .iter()
                                    .filter_map(|e| match e {
                                        Expresion::LiteralTexto(s) => Some(s.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let seleccionados = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(
                                exprs
                                    .iter()
                                    .filter_map(|e| match e {
                                        Expresion::LiteralBooleano(b) => Some(*b),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let callback = extraer_callback(argumentos, 2);
                    Some(Layout::SegmentedButton {
                        opciones,
                        seleccionados,
                        callback,
                        multiple: true,
                    })
                }

                // Chips
                "subconjunto_asistente" | "chip_assist" => {
                    let texto = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::Chip {
                        texto,
                        callback,
                        variant: ChipVariant::Assist,
                        activo: false,
                        on_quitar: None,
                    })
                }
                "subconjunto_filtro" | "chip_filter" => {
                    let texto = extraer_texto(argumentos, 0);
                    let activo = extraer_booleano(argumentos, 1);
                    let callback = extraer_callback(argumentos, 2);
                    Some(Layout::Chip {
                        texto,
                        callback,
                        variant: ChipVariant::Filter,
                        activo,
                        on_quitar: None,
                    })
                }
                "subconjunto_entrada" | "chip_input" => {
                    let texto = extraer_texto(argumentos, 0);
                    let cb_fn = extraer_callback(argumentos, 1);
                    let cb_clone = cb_fn.clone();
                    Some(Layout::Chip {
                        texto,
                        callback: cb_fn,
                        variant: ChipVariant::Entrada,
                        activo: true,
                        on_quitar: Some(cb_clone),
                    })
                }
                "subconjunto_sugerencia" | "chip_suggestion" => {
                    let texto = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::Chip {
                        texto,
                        callback,
                        variant: ChipVariant::Suggestion,
                        activo: false,
                        on_quitar: None,
                    })
                }

                // ─── Inputs Material Design 3 ──────────────────────────

                // Text Fields
                "campo_texto" | "campo_texto_error" => {
                    let variable = extraer_texto(argumentos, 0);
                    let etiqueta = extraer_texto(argumentos, 1);
                    let error = if nombre == "campo_texto_error" {
                        extraer_texto(argumentos, 2)
                    } else {
                        String::new()
                    };
                    Some(Layout::CampoTextoMaterial {
                        variable,
                        etiqueta,
                        placeholder: String::new(),
                        variant: TextoCampoVariant::Filled,
                        multiline: false,
                        error,
                        counter: false,
                    })
                }
                "campo_perfilado" => {
                    let variable = extraer_texto(argumentos, 0);
                    let etiqueta = extraer_texto(argumentos, 1);
                    Some(Layout::CampoTextoMaterial {
                        variable,
                        etiqueta,
                        placeholder: String::new(),
                        variant: TextoCampoVariant::Outlined,
                        multiline: false,
                        error: String::new(),
                        counter: false,
                    })
                }
                "campo_email" => {
                    let variable = extraer_texto(argumentos, 0);
                    let etiqueta = extraer_texto(argumentos, 1);
                    Some(Layout::CampoTextoMaterial {
                        variable,
                        etiqueta,
                        placeholder: "email@ejemplo.com".to_string(),
                        variant: TextoCampoVariant::Outlined,
                        multiline: false,
                        error: String::new(),
                        counter: false,
                    })
                }
                "campo_telefono" => {
                    let variable = extraer_texto(argumentos, 0);
                    let etiqueta = extraer_texto(argumentos, 1);
                    Some(Layout::CampoTextoMaterial {
                        variable,
                        etiqueta,
                        placeholder: "+54 11 1234-5678".to_string(),
                        variant: TextoCampoVariant::Outlined,
                        multiline: false,
                        error: String::new(),
                        counter: false,
                    })
                }
                "campo_url" => {
                    let variable = extraer_texto(argumentos, 0);
                    let etiqueta = extraer_texto(argumentos, 1);
                    Some(Layout::CampoTextoMaterial {
                        variable,
                        etiqueta,
                        placeholder: "https://".to_string(),
                        variant: TextoCampoVariant::Outlined,
                        multiline: false,
                        error: String::new(),
                        counter: false,
                    })
                }
                "campo_contraseña" | "campo_contrasena" => {
                    let variable = extraer_texto(argumentos, 0);
                    let etiqueta = extraer_texto(argumentos, 1);
                    Some(Layout::CampoContrasenaMaterial {
                        variable,
                        etiqueta,
                        visible: false,
                    })
                }
                "campo_numero" => {
                    let variable = extraer_texto(argumentos, 0);
                    let etiqueta = extraer_texto(argumentos, 1);
                    let min = extraer_f64(argumentos, 2);
                    let max = extraer_f64(argumentos, 3);
                    Some(Layout::CampoNumeroMaterial {
                        variable,
                        etiqueta,
                        min,
                        max,
                        decimales: 0,
                    })
                }
                "campo_busqueda" | "campo_search" => {
                    let variable = extraer_texto(argumentos, 0);
                    Some(Layout::CampoBusquedaMaterial {
                        variable,
                        placeholder: "Buscar...".to_string(),
                    })
                }

                // Dropdown & Select
                "contraer_desplegable" | "dropdown" => {
                    let opciones = extraer_array_strings(argumentos, 0);
                    let seleccionada = extraer_usize(argumentos, 1);
                    Some(Layout::MaterialDropdown {
                        opciones,
                        seleccionada,
                        placeholder: "Seleccionar...".to_string(),
                    })
                }
                "menu_seleccion" | "select_menu" => {
                    let opciones = extraer_array_strings(argumentos, 0);
                    let seleccionada = extraer_usize(argumentos, 1);
                    let etiqueta = extraer_texto(argumentos, 2);
                    Some(Layout::MaterialSelect {
                        opciones,
                        seleccionada,
                        etiqueta,
                    })
                }
                "autocompletar" | "autocomplete" => {
                    let opciones = extraer_array_strings(argumentos, 0);
                    let variable = extraer_texto(argumentos, 1);
                    Some(Layout::MaterialAutocomplete { opciones, variable })
                }

                // Radio Group
                "grupo_radio" | "radio_group" => {
                    let opciones = extraer_array_strings(argumentos, 0);
                    let seleccion = extraer_usize(argumentos, 1);
                    let callback = extraer_callback(argumentos, 2);
                    let direction = extraer_texto(argumentos, 3);
                    Some(Layout::MaterialRadioGroup {
                        nombre: String::new(),
                        opciones,
                        seleccion,
                        callback,
                        direction: if direction.is_empty() {
                            "vertical".to_string()
                        } else {
                            direction
                        },
                    })
                }

                // Switch
                "interruptor" | "switch_widget" => {
                    let etiqueta = extraer_texto(argumentos, 0);
                    let variable = extraer_texto(argumentos, 1);
                    Some(Layout::MaterialSwitch { etiqueta, variable })
                }

                // Sliders
                "deslizante_discreto" | "discrete_slider" => {
                    let variable = extraer_texto(argumentos, 0);
                    let min = extraer_f64(argumentos, 1);
                    let max = extraer_f64(argumentos, 2);
                    let steps = extraer_usize(argumentos, 3) as i32;
                    Some(Layout::MaterialSliderDiscrete {
                        variable,
                        min,
                        max,
                        steps,
                    })
                }
                "deslizante_rango" | "range_slider" => {
                    let variable_inicio = extraer_texto(argumentos, 0);
                    let variable_fin = extraer_texto(argumentos, 1);
                    let min = extraer_f64(argumentos, 2);
                    let max = extraer_f64(argumentos, 3);
                    Some(Layout::MaterialSliderRange {
                        variable_inicio,
                        variable_fin,
                        min,
                        max,
                    })
                }

                // Chip Group
                "grupo_subconjuntos" | "chip_group" => {
                    let chips = extraer_array_strings(argumentos, 0);
                    let seleccion = extraer_array_bool(argumentos, 1);
                    let callback = extraer_callback(argumentos, 2);
                    Some(Layout::MaterialChipGroup {
                        chips,
                        seleccion,
                        callback,
                        multiple: false,
                    })
                }

                // Date & Time Pickers
                "selector_fecha" | "date_picker" => {
                    let variable = extraer_texto(argumentos, 0);
                    Some(Layout::MaterialDatePicker { variable })
                }
                "selector_hora" | "time_picker" => {
                    let variable = extraer_texto(argumentos, 0);
                    Some(Layout::MaterialTimePicker { variable })
                }
                "selector_color" | "color_picker" => {
                    let variable = extraer_texto(argumentos, 0);
                    Some(Layout::ColorPicker { variable })
                }

                // ─── Tarjetas (Cards) ─────────────────────────────
                "tarjeta" | "card" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::MaterialCard {
                        hijo: Box::new(c),
                        variant: CardVariant::Filled,
                        on_click: None,
                        seleccionado: false,
                    })
                }
                "tarjeta_elevada" | "elevated_card" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::MaterialCard {
                        hijo: Box::new(c),
                        variant: CardVariant::Elevated,
                        on_click: None,
                        seleccionado: false,
                    })
                }
                "tarjeta_perfilada" | "outlined_card" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::MaterialCard {
                        hijo: Box::new(c),
                        variant: CardVariant::Outlined,
                        on_click: None,
                        seleccionado: false,
                    })
                }
                "tarjeta_seleccionable" | "selectable_card" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let _variable = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    hijo.map(|c| Layout::MaterialCard {
                        hijo: Box::new(c),
                        variant: CardVariant::Selectable,
                        on_click: None,
                        seleccionado: false,
                    })
                }

                // ─── List Items ──────────────────────────────────
                "elemento_lista" | "list_item" => {
                    let leading = argumentos.first().and_then(expr_a_layout).map(Box::new);
                    let titulo = extraer_texto(argumentos, 1);
                    let trailing = argumentos.get(2).and_then(expr_a_layout).map(Box::new);
                    Some(Layout::ElementoListaMaterial {
                        leading,
                        titulo,
                        subtitulo: None,
                        trailing,
                        on_click: None,
                    })
                }
                "elemento_lista_doble" | "two_line_list_item" => {
                    let leading = argumentos.first().and_then(expr_a_layout).map(Box::new);
                    let titulo = extraer_texto(argumentos, 1);
                    let subtitulo = Some(extraer_texto(argumentos, 2));
                    let trailing = argumentos.get(3).and_then(expr_a_layout).map(Box::new);
                    Some(Layout::ElementoListaMaterial {
                        leading,
                        titulo,
                        subtitulo,
                        trailing,
                        on_click: None,
                    })
                }
                "lista" | "list_widget" => {
                    let items = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(procesar_args(exprs)),
                            _ => None,
                        })
                        .unwrap_or_default();
                    Some(Layout::ListaMaterial {
                        items,
                        dividers: false,
                    })
                }
                "lista_con_dividores" | "list_with_dividers" => {
                    let items = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(procesar_args(exprs)),
                            _ => None,
                        })
                        .unwrap_or_default();
                    Some(Layout::ListaMaterial {
                        items,
                        dividers: true,
                    })
                }
                "lista_control" | "list_control" => {
                    let items = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(procesar_args(exprs)),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let control_tipo = extraer_texto(argumentos, 1);
                    let variables = extraer_array_strings(argumentos, 2);
                    Some(Layout::ControlListaMaterial {
                        items,
                        control_tipo,
                        variables,
                    })
                }
                "lista_seleccion" | "list_selection" => {
                    let items = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(procesar_args(exprs)),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let seleccion = extraer_array_bool(argumentos, 1);
                    let callback = extraer_callback(argumentos, 2);
                    Some(Layout::SeleccionListaMaterial {
                        items,
                        seleccion,
                        callback,
                        multiple: false,
                    })
                }
                // ─── Lista dinámica: renderiza items desde una variable ──
                // lista_dinamica("variable_lista", &render_item)
                "lista_dinamica" | "dynamic_list" => {
                    let variable = extraer_texto(argumentos, 0);
                    let elemento_fn = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::Referencia { expr, .. } => match expr.as_ref() {
                                Expresion::Identificador { nombre: n, .. } => n.clone(),
                                _ => String::new(),
                            },
                            Expresion::Identificador { nombre: n, .. } => n.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    Some(Layout::ListaDinamica {
                        variable,
                        elemento_fn,
                    })
                }

                // ─── Data Tables ─────────────────────────────────
                "tabla_datos" | "data_table" => {
                    let columnas = extraer_array_strings(argumentos, 0);
                    let filas = extraer_array_arrays_strings(argumentos, 1);
                    Some(Layout::TablaDatosMaterial {
                        columnas,
                        filas,
                        ordenable: false,
                        seleccionable: false,
                        col_orden: 0,
                        orden_asc: true,
                    })
                }
                "tabla_ordenable" | "sortable_table" => {
                    let columnas = extraer_array_strings(argumentos, 0);
                    let filas = extraer_array_arrays_strings(argumentos, 1);
                    Some(Layout::TablaDatosMaterial {
                        columnas,
                        filas,
                        ordenable: true,
                        seleccionable: false,
                        col_orden: 0,
                        orden_asc: true,
                    })
                }
                "tabla_seleccion" | "selectable_table" => {
                    let columnas = extraer_array_strings(argumentos, 0);
                    let filas = extraer_array_arrays_strings(argumentos, 1);
                    Some(Layout::TablaDatosMaterial {
                        columnas,
                        filas,
                        ordenable: false,
                        seleccionable: true,
                        col_orden: 0,
                        orden_asc: true,
                    })
                }

                // ─── Surfaces ────────────────────────────────────
                "superficie" | "surface_widget" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::MaterialSurface {
                        hijo: Box::new(c),
                        color_role: "surface".to_string(),
                    })
                }
                "superficie_tonal" | "tonal_surface" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::MaterialSurface {
                        hijo: Box::new(c),
                        color_role: "tonal".to_string(),
                    })
                }
                "andamio" | "scaffold" => {
                    let top = argumentos.first().and_then(expr_a_layout).map(Box::new);
                    let body = argumentos.get(1).and_then(expr_a_layout);
                    let bottom = argumentos.get(2).and_then(expr_a_layout).map(Box::new);
                    body.map(|b| Layout::MaterialScaffold {
                        top,
                        body: Box::new(b),
                        bottom,
                        fab: None,
                    })
                }

                // ─── FEEDBACK: Diálogos ──────────────────────────────
                "dialogo_alerta" | "dialog_alert" => {
                    let titulo = extraer_texto(argumentos, 0);
                    let mensaje = extraer_texto(argumentos, 1);
                    Some(Layout::DialogAlert {
                        titulo,
                        mensaje,
                        confirmar_texto: "Aceptar".to_string(),
                        cancelar_texto: String::new(),
                        on_confirm: String::new(),
                        on_cancel: String::new(),
                    })
                }
                "dialogo_confirmacion" | "dialog_confirm" => {
                    let titulo = extraer_texto(argumentos, 0);
                    let mensaje = extraer_texto(argumentos, 1);
                    let on_confirm = extraer_callback(argumentos, 2);
                    let on_cancel = extraer_callback(argumentos, 3);
                    Some(Layout::DialogAlert {
                        titulo,
                        mensaje,
                        confirmar_texto: "Confirmar".to_string(),
                        cancelar_texto: "Cancelar".to_string(),
                        on_confirm,
                        on_cancel,
                    })
                }
                "dialogo_personalizado" | "dialog_custom" => {
                    let titulo = extraer_texto(argumentos, 0);
                    let hijo = argumentos.get(1).and_then(expr_a_layout);
                    hijo.map(|c| Layout::DialogCustom {
                        titulo,
                        hijo: Box::new(c),
                        on_cerrar: String::new(),
                    })
                }
                "dialogo_completo" | "dialog_full" => {
                    let titulo = extraer_texto(argumentos, 0);
                    let hijo = argumentos.get(1).and_then(expr_a_layout);
                    hijo.map(|c| Layout::DialogCustom {
                        titulo,
                        hijo: Box::new(c),
                        on_cerrar: String::new(),
                    })
                }

                // ─── FEEDBACK: Bottom Sheets ─────────────────────────
                "hoja_inferior" | "bottom_sheet" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let visible = extraer_texto(argumentos, 1);
                    hijo.map(|c| Layout::BottomSheet {
                        hijo: Box::new(c),
                        variant: SheetVariant::Standard,
                        visible,
                        on_dismiss: None,
                    })
                }
                "hoja_inferior_modal" | "modal_sheet" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let visible = extraer_texto(argumentos, 1);
                    hijo.map(|c| Layout::BottomSheet {
                        hijo: Box::new(c),
                        variant: SheetVariant::Modal,
                        visible,
                        on_dismiss: None,
                    })
                }
                "hoja_inferior_grande" | "expanded_sheet" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let visible = extraer_texto(argumentos, 1);
                    hijo.map(|c| Layout::BottomSheet {
                        hijo: Box::new(c),
                        variant: SheetVariant::Expanded,
                        visible,
                        on_dismiss: None,
                    })
                }

                // ─── FEEDBACK: Snackbar ──────────────────────────────
                "notificación" | "snackbar" | "notification" => {
                    let mensaje = extraer_texto(argumentos, 0);
                    Some(Layout::Snackbar {
                        mensaje,
                        accion_texto: None,
                        accion_callback: None,
                        duracion: 3000.0,
                        visible: String::new(),
                    })
                }
                "notificación_accion" | "snackbar_action" => {
                    let mensaje = extraer_texto(argumentos, 0);
                    let accion_texto = Some(extraer_texto(argumentos, 1));
                    let accion_callback = {
                        let cb = extraer_callback(argumentos, 2);
                        if cb.is_empty() {
                            None
                        } else {
                            Some(cb)
                        }
                    };
                    Some(Layout::Snackbar {
                        mensaje,
                        accion_texto,
                        accion_callback,
                        duracion: 4000.0,
                        visible: String::new(),
                    })
                }

                // ─── FEEDBACK: Tooltip ───────────────────────────────
                "información" | "tooltip" | "info" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let texto = extraer_texto(argumentos, 1);
                    hijo.map(|c| Layout::Tooltip {
                        hijo: Box::new(c),
                        texto,
                    })
                }

                // ─── FEEDBACK: Menús ─────────────────────────────────
                "menú_desplegable" | "menu_desplegable" | "dropdown_menu" => {
                    let items = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(
                                exprs
                                    .iter()
                                    .filter_map(|e| match e {
                                        Expresion::LiteralTexto(s) => Some(s.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            Expresion::LiteralTexto(s) => Some(vec![s.clone()]),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let _seleccion = extraer_usize(argumentos, 1);
                    let on_select = extraer_callback(argumentos, 2);
                    Some(Layout::Menu {
                        items,
                        on_select,
                        visible: "true".to_string(),
                    })
                }
                "menú_contexto" | "menu_contexto" | "context_menu" => {
                    let items = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(
                                exprs
                                    .iter()
                                    .filter_map(|e| match e {
                                        Expresion::LiteralTexto(s) => Some(s.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            Expresion::LiteralTexto(s) => Some(vec![s.clone()]),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let _seleccion = extraer_usize(argumentos, 1);
                    let on_select = extraer_callback(argumentos, 2);
                    Some(Layout::ContextMenu {
                        items,
                        on_select,
                        visible: "true".to_string(),
                    })
                }

                // ═══════════════════════════════════════════════════════
                // Navegación Material You
                // ═══════════════════════════════════════════════════════

                // ─── Navigator (navegación por pantallas) ──────────
                "navegador" | "navigator" => {
                    let screens = extraer_navigator_screens(argumentos, 0);
                    let actual_variable = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => "pantalla_actual".to_string(),
                        })
                        .unwrap_or_else(|| "pantalla_actual".to_string());
                    let actual_variable_clonar = actual_variable.clone();
                    let nav_tipo_texto = argumentos
                        .get(2)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => "ninguno".to_string(),
                        })
                        .unwrap_or_else(|| "ninguno".to_string());
                    let nav_tipo = match nav_tipo_texto.to_lowercase().as_str() {
                        "barra" | "bottom" | "inferior" => NavigatorTipo::BottomBar,
                        "riel" | "rail" | "lateral" => NavigatorTipo::Rail,
                        "pestañas" | "tabs" | "tab" => NavigatorTipo::Tabs,
                        "cajon" | "drawer" => NavigatorTipo::Drawer,
                        _ => NavigatorTipo::None,
                    };
                    // Arg 3 opcional: callback on_change (ej: &cambiar_pantalla)
                    let on_change = argumentos
                        .get(3)
                        .map(|_| extraer_callback(argumentos, 3))
                        .filter(|s| !s.is_empty());
                    // Arg 4 opcional: tipo de animación ("fade", "slide", "ninguno")
                    let anim_tipo = argumentos
                        .get(4)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.to_lowercase(),
                            _ => "ninguno".to_string(),
                        })
                        .unwrap_or_else(|| "ninguno".to_string());
                    let anim = match anim_tipo.as_str() {
                        "fade" => NavigatorAnim::Fade,
                        "slide" => NavigatorAnim::Slide,
                        _ => NavigatorAnim::None,
                    };
                    if screens.is_empty() {
                        Some(Layout::Spacer(0.0))
                    } else {
                        Some(Layout::Navigator {
                            screens,
                            actual_variable: actual_variable_clonar,
                            history_var: format!("{}_historial", actual_variable),
                            nav_tipo,
                            anim,
                            on_change,
                        })
                    }
                }

                // ─── Pantalla individual (para usar dentro de navigator) ──
                "pantalla" | "screen" => {
                    // Las pantallas individuales se procesan dentro de extraer_navigator_screens
                    // Como llamada directa, devolvemos placeholder
                    let titulo = extraer_texto(argumentos, 0);
                    Some(Layout::StyledLabel {
                        texto: format!("[Pantalla: {}]", titulo),
                        style: "body_medium".to_string(),
                    })
                }

                // ─── NavigationBar ───────────────────────────────────
                "barra_navegacion" | "navigation_bar" => {
                    let items = extraer_nav_items(argumentos, 0);
                    let seleccion = extraer_usize(argumentos, 1);
                    let on_change = extraer_callback(argumentos, 2);
                    Some(Layout::NavigationBar {
                        items,
                        seleccion,
                        variable_seleccion: variable_de(argumentos, 1),
                        on_change,
                    })
                }

                // ─── NavigationRail ──────────────────────────────────
                "riel_navegacion" | "navigation_rail" => {
                    let items = extraer_nav_items(argumentos, 0);
                    let seleccion = extraer_usize(argumentos, 1);
                    let on_change = extraer_callback(argumentos, 2);
                    let extended = extraer_booleano(argumentos, 3);
                    Some(Layout::NavigationRail {
                        items,
                        seleccion,
                        variable_seleccion: variable_de(argumentos, 1),
                        on_change,
                        extended,
                    })
                }

                // ─── NavigationDrawer ────────────────────────────────
                "cajon_navegacion" | "navigation_drawer" => {
                    let items = extraer_nav_items(argumentos, 0);
                    let seleccion = extraer_usize(argumentos, 1);
                    let on_change = extraer_callback(argumentos, 2);
                    Some(Layout::NavigationDrawer {
                        items,
                        seleccion,
                        variable_seleccion: variable_de(argumentos, 1),
                        on_change,
                        modal: false,
                        visible: String::new(),
                    })
                }

                // ─── NavigationDrawer Modal ──────────────────────────
                "cajon_modal" | "modal_drawer" => {
                    let items = extraer_nav_items(argumentos, 0);
                    let seleccion = extraer_usize(argumentos, 1);
                    let on_change = extraer_callback(argumentos, 2);
                    let visible = extraer_texto(argumentos, 3);
                    Some(Layout::NavigationDrawer {
                        items,
                        seleccion,
                        variable_seleccion: variable_de(argumentos, 1),
                        on_change,
                        modal: true,
                        visible,
                    })
                }

                // ─── TopAppBar ───────────────────────────────────────
                "barra_superior" | "top_app_bar" => {
                    let titulo = extraer_texto(argumentos, 0);
                    let acciones = extraer_icon_actions(argumentos, 1);
                    Some(Layout::TopAppBar {
                        titulo,
                        acciones,
                        menu_visible: false,
                        variant: TopAppBarVariant::Small,
                    })
                }
                "barra_superior_media" | "top_app_bar_medium" => {
                    let titulo = extraer_texto(argumentos, 0);
                    Some(Layout::TopAppBar {
                        titulo,
                        acciones: vec![],
                        menu_visible: false,
                        variant: TopAppBarVariant::Medium,
                    })
                }
                "barra_superior_grande" | "top_app_bar_large" => {
                    let titulo = extraer_texto(argumentos, 0);
                    Some(Layout::TopAppBar {
                        titulo,
                        acciones: vec![],
                        menu_visible: false,
                        variant: TopAppBarVariant::Large,
                    })
                }

                // ─── BottomAppBar ────────────────────────────────────
                "barra_inferior" | "bottom_app_bar" => {
                    let acciones = extraer_icon_actions(argumentos, 0);
                    Some(Layout::BottomAppBar {
                        acciones,
                        fab: None,
                    })
                }

                // ─── Tabs ────────────────────────────────────────────
                "pestañas" | "tabs_widget" => {
                    let tabs = extraer_array_strings(argumentos, 0);
                    let seleccion = extraer_usize(argumentos, 1);
                    let on_change = extraer_callback(argumentos, 2);
                    Some(Layout::Tabs {
                        tabs,
                        seleccion,
                        on_change,
                        scrollable: false,
                    })
                }
                "pestañas_desplazables" | "scrollable_tabs" => {
                    let tabs = extraer_array_strings(argumentos, 0);
                    let seleccion = extraer_usize(argumentos, 1);
                    let on_change = extraer_callback(argumentos, 2);
                    Some(Layout::Tabs {
                        tabs,
                        seleccion,
                        on_change,
                        scrollable: true,
                    })
                }

                // ─── SearchBar ───────────────────────────────────────
                "barra_busqueda" | "search_bar_widget" => {
                    let placeholder = extraer_texto(argumentos, 0);
                    let variable = extraer_texto(argumentos, 1);
                    Some(Layout::BarraBusqueda {
                        placeholder,
                        on_buscar: String::new(),
                        variable,
                    })
                }

                // ─── item_navegacion (helper, se procesa en extraer_nav_items) ──
                "item_navegacion" | "nav_item" => {
                    // item_navegacion se procesa dentro de extraer_nav_items,
                    // pero como llamada directa devolvemos un placeholder
                    let icono = extraer_texto(argumentos, 0);
                    let etiqueta = extraer_texto(argumentos, 1);
                    Some(Layout::StyledLabel {
                        texto: format!("{} {}", icono, etiqueta),
                        style: "label_medium".to_string(),
                    })
                }

                // ═══════════════════════════════════════════════════════
                // INDICADORES (Fase 7)
                // ═══════════════════════════════════════════════════════

                // ─── LinearProgress ──────────────────────────────────
                "barra_progreso" | "progress_bar_linear" => {
                    let variable = extraer_texto(argumentos, 0);
                    Some(Layout::LinearProgress {
                        variable,
                        indeterminado: false,
                    })
                }
                "barra_progreso_indeterminada" | "progress_bar_indeterminate" => {
                    Some(Layout::LinearProgress {
                        variable: String::new(),
                        indeterminado: true,
                    })
                }

                // ─── CircularProgress ────────────────────────────────
                "circulo_progreso" | "circular_progress" => {
                    let variable = extraer_texto(argumentos, 0);
                    let tamano = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(40.0);
                    Some(Layout::CircularProgress {
                        variable,
                        tamano,
                        indeterminado: false,
                    })
                }
                "circulo_progreso_indeterminado" | "circular_progress_indeterminate" => {
                    let tamano = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(40.0);
                    Some(Layout::CircularProgress {
                        variable: String::new(),
                        tamano,
                        indeterminado: true,
                    })
                }

                // ─── Badge ───────────────────────────────────────────
                "distintivo" | "badge" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let valor = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => Some(s.clone()),
                            Expresion::Identificador { nombre: s, .. } => Some(s.clone()),
                            Expresion::LiteralNumero(n) => Some(n.to_string()),
                            _ => None,
                        })
                        .unwrap_or(None);
                    hijo.map(|c| Layout::Badge {
                        hijo: Box::new(c),
                        valor,
                        dot: false,
                    })
                }
                "distintivo_punto" | "badge_dot" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::Badge {
                        hijo: Box::new(c),
                        valor: None,
                        dot: true,
                    })
                }

                // ─── Skeleton ───────────────────────────────────────
                "esqueleto" | "skeleton" => {
                    let ancho = extraer_f64(argumentos, 0);
                    let alto = extraer_f64(argumentos, 1);
                    let tipo = extraer_texto(argumentos, 2);
                    Some(Layout::Skeleton {
                        ancho,
                        alto,
                        tipo: if tipo.is_empty() {
                            "linea".to_string()
                        } else {
                            tipo
                        },
                    })
                }
                "esqueleto_tarjeta" | "skeleton_card" => Some(Layout::Skeleton {
                    ancho: 300.0,
                    alto: 180.0,
                    tipo: "tarjeta".to_string(),
                }),
                "esqueleto_linea" | "skeleton_line" => Some(Layout::Skeleton {
                    ancho: 200.0,
                    alto: 16.0,
                    tipo: "linea".to_string(),
                }),

                // ─── EmptyState ─────────────────────────────────────
                "estado_vacio" | "empty_state" => {
                    let icono = extraer_texto(argumentos, 0);
                    let mensaje = extraer_texto(argumentos, 1);
                    let accion_texto = argumentos
                        .get(2)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or(None);
                    let accion_cb = argumentos
                        .get(3)
                        .map(|a| match a {
                            Expresion::Identificador { nombre: s, .. } => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or(None);
                    Some(Layout::EmptyState {
                        icono,
                        mensaje,
                        accion_texto,
                        accion_cb,
                    })
                }

                // ─── ErrorState ─────────────────────────────────────
                "estado_error" | "error_state" => {
                    let mensaje = extraer_texto(argumentos, 0);
                    let on_retry = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::Identificador { nombre: s, .. } => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or(None);
                    Some(Layout::ErrorState { mensaje, on_retry })
                }

                // ═══════════════════════════════════════════════════════
                // AVATARES (Fase 7)
                // ═══════════════════════════════════════════════════════

                // ─── Avatar ─────────────────────────────────────────
                "avatar" | "avatar_text" => {
                    let texto = extraer_texto(argumentos, 0);
                    let tamaño = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(40.0);
                    Some(Layout::Avatar {
                        texto,
                        variant: AvatarVariant::Text,
                        tamaño,
                    })
                }
                "avatar_icono" | "avatar_icon" => {
                    let icono = extraer_texto(argumentos, 0);
                    let tamaño = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(40.0);
                    Some(Layout::Avatar {
                        texto: icono,
                        variant: AvatarVariant::Icon,
                        tamaño,
                    })
                }
                "avatar_imagen" | "avatar_image" => {
                    let ruta = extraer_texto(argumentos, 0);
                    let tamaño = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(40.0);
                    Some(Layout::Avatar {
                        texto: ruta,
                        variant: AvatarVariant::Image,
                        tamaño,
                    })
                }

                // ─── AvatarGroup ─────────────────────────────────────
                "grupo_avatar" | "avatar_group" => {
                    let avatares = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::Arreglo(exprs) => Some(
                                exprs
                                    .iter()
                                    .filter_map(|e| match e {
                                        Expresion::LiteralTexto(s) => Some(s.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let max = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as usize),
                            _ => None,
                        })
                        .unwrap_or(3);
                    Some(Layout::AvatarGroup { avatares, max })
                }

                // ═══════════════════════════════════════════════════════
                // MOTION (Fase 8)
                // ═══════════════════════════════════════════════════════

                // ─── FadeTransition ──────────────────────────────────
                "transición" | "fade_transition" | "transition" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let visible = extraer_texto(argumentos, 1);
                    let duracion = argumentos
                        .get(2)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(0.3);
                    hijo.map(|c| Layout::FadeTransition {
                        hijo: Box::new(c),
                        visible,
                        duracion,
                    })
                }

                // ─── RippleEffect ────────────────────────────────────
                "efecto_onda" | "ripple_effect" | "ripple" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let color = extraer_texto(argumentos, 1);
                    hijo.map(|c| Layout::RippleEffect {
                        hijo: Box::new(c),
                        color: if color.is_empty() {
                            "primary".to_string()
                        } else {
                            color
                        },
                    })
                }

                // ─── PullToRefresh ──────────────────────────────────
                "pull_to_refresh" | "pulltorefresh" | "tirar_recargar" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let callback = extraer_callback(argumentos, 1);
                    let refreshing = argumentos
                        .get(2)
                        .map(|a| match a {
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    hijo.map(|c| Layout::PullToRefresh {
                        hijo: Box::new(c),
                        callback,
                        refreshing: if refreshing.is_empty() {
                            "refreshing".to_string()
                        } else {
                            refreshing
                        },
                    })
                }

                // ─── SwipeToDismiss ─────────────────────────────────
                "swipe_to_dismiss" | "swipetodismiss" | "deslizar_descartar" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let on_dismiss = extraer_callback(argumentos, 1);
                    let etiqueta = extraer_texto(argumentos, 2);
                    let dismissed = argumentos
                        .get(3)
                        .map(|a| match a {
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    hijo.map(|c| Layout::SwipeToDismiss {
                        hijo: Box::new(c),
                        on_dismiss: if on_dismiss.is_empty() {
                            String::new()
                        } else {
                            on_dismiss
                        },
                        etiqueta: if etiqueta.is_empty() {
                            "Descartar".to_string()
                        } else {
                            etiqueta
                        },
                        dismissed: if dismissed.is_empty() {
                            "dismissed".to_string()
                        } else {
                            dismissed
                        },
                    })
                }

                // ─── PinchZoom ────────────────────────────────────
                "zoom_pellizco" | "pinch_zoom" | "zoom" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let min_scale = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            _ => None,
                        })
                        .unwrap_or(0.5);
                    let max_scale = argumentos
                        .get(2)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            _ => None,
                        })
                        .unwrap_or(3.0);
                    hijo.map(|c| Layout::PinchZoom {
                        hijo: Box::new(c),
                        min_scale,
                        max_scale,
                    })
                }

                // ─── Rotate ───────────────────────────────────────
                "rotar" | "rotate" | "girar" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    hijo.map(|c| Layout::Rotate { hijo: Box::new(c) })
                }

                // ═══════════════════════════════════════════════════════
                // GRÁFICOS (Fase 9)
                // ═══════════════════════════════════════════════════════

                // ─── LineChart ────────────────────────────────────────
                "gráfico_linea" | "grafico_linea" | "line_chart" | "chart_line" => {
                    let datos = extraer_array_f64(argumentos, 0);
                    let etiquetas = extraer_array_strings(argumentos, 1);
                    let color = extraer_texto(argumentos, 2);
                    Some(Layout::GraficoLineas {
                        datos,
                        color: if color.is_empty() {
                            "primary".to_string()
                        } else {
                            color
                        },
                        etiquetas,
                    })
                }

                // ─── BarChart ─────────────────────────────────────────
                "gráfico_barras" | "grafico_barras" | "bar_chart" | "chart_bar" => {
                    let datos = extraer_array_f64(argumentos, 0);
                    let etiquetas = extraer_array_strings(argumentos, 1);
                    let colores = extraer_array_strings(argumentos, 2);
                    let apilado = extraer_booleano(argumentos, 3);
                    Some(Layout::BarChart {
                        datos,
                        colores,
                        etiquetas,
                        apilado,
                    })
                }

                // ─── PieChart ─────────────────────────────────────────
                "gráfico_pastel" | "grafico_pastel" | "pie_chart" | "chart_pie" => {
                    let datos = extraer_array_f64(argumentos, 0);
                    let etiquetas = extraer_array_strings(argumentos, 1);
                    Some(Layout::PieChart {
                        datos,
                        etiquetas,
                        donut: false,
                    })
                }

                // ─── Donut ────────────────────────────────────────────
                "gráfico_donut" | "grafico_donut" | "donut_chart" | "chart_donut" => {
                    let datos = extraer_array_f64(argumentos, 0);
                    let etiquetas = extraer_array_strings(argumentos, 1);
                    Some(Layout::PieChart {
                        datos,
                        etiquetas,
                        donut: true,
                    })
                }

                // ─── GaugeChart ───────────────────────────────────────
                "gráfico_indicador" | "grafico_indicador" | "gauge_chart" | "gauge" => {
                    let valor = extraer_f64(argumentos, 0);
                    let min = extraer_f64(argumentos, 1);
                    let max = extraer_f64(argumentos, 2);
                    let color = extraer_texto(argumentos, 3);
                    Some(Layout::GaugeChart {
                        valor,
                        min: if min == 0.0 && max == 0.0 { 0.0 } else { min },
                        max: if max == 0.0 { 100.0 } else { max },
                        color: if color.is_empty() {
                            "primary".to_string()
                        } else {
                            color
                        },
                    })
                }

                // ─── Sparkline ────────────────────────────────────────
                "minigráfico" | "minigrafico" | "sparkline" | "mini_chart" => {
                    let datos = extraer_array_f64(argumentos, 0);
                    let color = extraer_texto(argumentos, 1);
                    Some(Layout::Sparkline {
                        datos,
                        color: if color.is_empty() {
                            "primary".to_string()
                        } else {
                            color
                        },
                    })
                }

                // ═══════════════════════════════════════════════════════
                // AVANZADOS (Fase 9)
                // ═══════════════════════════════════════════════════════

                // ─── StarRating ───────────────────────────────────────
                "calificación" | "calificacion" | "star_rating" | "rating" => {
                    let valor = extraer_usize(argumentos, 0);
                    let max = extraer_usize(argumentos, 1);
                    let callback = extraer_callback(argumentos, 2);
                    // `calificación(puntuacion, …)`: si el valor es una variable
                    // se guarda su nombre y se lee en cada render.
                    let variable = argumentos.first().and_then(|a| match a {
                        Expresion::Identificador { nombre, .. } => Some(nombre.clone()),
                        _ => None,
                    });
                    Some(Layout::StarRating {
                        valor,
                        variable,
                        max: if max == 0 { 5 } else { max },
                        callback,
                    })
                }

                // ─── Stepper ──────────────────────────────────────────
                "asistente_pasos" | "stepper" | "step_wizard" => {
                    let pasos = extraer_array_strings(argumentos, 0);
                    let actual = extraer_usize(argumentos, 1);
                    let callback = extraer_callback(argumentos, 2);
                    Some(Layout::Stepper {
                        pasos,
                        actual,
                        callback,
                    })
                }

                // ─── Breadcrumbs ──────────────────────────────────────
                "migaja_de_pan" | "breadcrumbs" | "breadcrumb" | "migajas" => {
                    let items = extraer_array_strings(argumentos, 0);
                    let separador = extraer_texto(argumentos, 1);
                    Some(Layout::Breadcrumbs {
                        items,
                        separador: if separador.is_empty() {
                            "›".to_string()
                        } else {
                            separador
                        },
                    })
                }

                // ─── Calendar ─────────────────────────────────────────
                "calendario" | "calendar" | "month_calendar" => {
                    let mes = extraer_usize(argumentos, 0) as i32;
                    let año = extraer_usize(argumentos, 1) as i32;
                    let callback = extraer_callback(argumentos, 2);
                    Some(Layout::Calendar {
                        mes: if mes == 0 { 1 } else { mes },
                        año: if año == 0 { 2025 } else { año },
                        seleccionado: None,
                        callback,
                    })
                }

                // ─── MarkdownViewer ───────────────────────────────────
                "visor_markdown" | "markdown_viewer" | "markdown" | "ver_markdown" => {
                    let texto = extraer_texto(argumentos, 0);
                    Some(Layout::MarkdownViewer { texto })
                }

                // ─── QRCode ───────────────────────────────────────────
                "visor_qr" | "qr_code" | "qr" | "codigo_qr" => {
                    let texto = extraer_texto(argumentos, 0);
                    let tamaño = extraer_f64(argumentos, 1);
                    Some(Layout::QRCode {
                        texto,
                        tamaño: if tamaño <= 0.0 { 128.0 } else { tamaño },
                    })
                }

                // ─── FilePicker ───────────────────────────────────────
                "selector_archivo" | "file_picker" | "file_selector" | "seleccionar_archivo" => {
                    let tipos = extraer_array_strings(argumentos, 0);
                    let multiple = extraer_booleano(argumentos, 1);
                    let callback = extraer_callback(argumentos, 2);
                    Some(Layout::SelectorArchivo {
                        tipos,
                        multiple,
                        callback,
                    })
                }

                // ═══════════════════════════════════════════════════════
                // EXPRESSIVE (Fase 10)
                // ═══════════════════════════════════════════════════════

                // ─── GlassCard ────────────────────────────────────────
                "tarjeta_vidrio" | "glass_card" | "glassmorphism" | "glass" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let blur = extraer_f64(argumentos, 1);
                    let opacity = extraer_f64(argumentos, 2);
                    hijo.map(|c| Layout::GlassCard {
                        hijo: Box::new(c),
                        blur: if blur <= 0.0 { 20.0 } else { blur },
                        opacity: if opacity <= 0.0 {
                            0.65
                        } else {
                            opacity.min(1.0)
                        },
                    })
                }

                // ─── GradientBox ──────────────────────────────────────
                "gradiente_lineal" | "gradiente_radial" | "gradient_box" | "gradient" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let colores = extraer_array_strings(argumentos, 1);
                    let direccion = extraer_texto(argumentos, 2);
                    hijo.map(|c| Layout::GradientBox {
                        hijo: Box::new(c),
                        colores,
                        direccion: if direccion.is_empty() {
                            "horizontal".to_string()
                        } else {
                            direccion
                        },
                    })
                }

                // ─── MorphingButton ───────────────────────────────────
                "boton_morphing" | "morphing_button" | "morph_button" => {
                    let icono = extraer_texto(argumentos, 0);
                    let texto_extendido = extraer_texto(argumentos, 1);
                    let callback = extraer_callback(argumentos, 2);
                    Some(Layout::MorphingButton {
                        icono,
                        texto_extendido,
                        callback,
                    })
                }

                // ─── ExpressiveBackground ─────────────────────────────
                "fondo_expresivo" | "expressive_background" | "bg_expressive" => {
                    let colores = extraer_array_strings(argumentos, 0);
                    let animado = extraer_booleano(argumentos, 1);
                    Some(Layout::ExpressiveBackground { colores, animado })
                }

                // ─── GlowBorder ───────────────────────────────────────
                "efecto_brillo" | "glow_border" | "glow" | "brillo" => {
                    let hijo = argumentos.first().and_then(expr_a_layout);
                    let color = extraer_texto(argumentos, 1);
                    let ancho = extraer_f64(argumentos, 2);
                    hijo.map(|c| Layout::GlowBorder {
                        hijo: Box::new(c),
                        color: if color.is_empty() {
                            "primary".to_string()
                        } else {
                            color
                        },
                        ancho: if ancho <= 0.0 { 2.0 } else { ancho },
                    })
                }

                // ─── Canvas / Lienzo ────────────────────────────────────
                "lienzo" | "canvas" | "lona" => {
                    let commands_var = extraer_texto(argumentos, 0);
                    let width = extraer_f64(argumentos, 1);
                    let height = extraer_f64(argumentos, 2);
                    Some(Layout::Canvas {
                        commands_var,
                        width: if width <= 0.0 { 300.0 } else { width },
                        height: if height <= 0.0 { 200.0 } else { height },
                    })
                }

                // ═══════════════════════════════════════════════════════════
                // ICONOS MATERIAL DESIGN
                // ═══════════════════════════════════════════════════════════

                // icono_material(nombre, tamaño, color)
                // icono_material(nombre, tamaño, color, estilo)
                "icono_material" | "material_icon" => {
                    let nombre = extraer_texto(argumentos, 0);
                    let tamaño = extraer_f64(argumentos, 1);
                    let color = extraer_texto(argumentos, 2);
                    let estilo = extraer_texto(argumentos, 3);
                    Some(Layout::MaterialIconLayout {
                        nombre,
                        tamaño: if tamaño <= 0.0 { 24.0 } else { tamaño },
                        color: if color.is_empty() {
                            "primary".to_string()
                        } else {
                            color
                        },
                        estilo: if estilo.is_empty() {
                            "filled".to_string()
                        } else {
                            estilo
                        },
                    })
                }

                // icono_relleno(nombre, tamaño) — estilo filled
                "icono_relleno" | "icon_filled" => {
                    let nombre = extraer_texto(argumentos, 0);
                    let tamaño = extraer_f64(argumentos, 1);
                    Some(Layout::MaterialIconLayout {
                        nombre,
                        tamaño: if tamaño <= 0.0 { 24.0 } else { tamaño },
                        color: "primary".to_string(),
                        estilo: "filled".to_string(),
                    })
                }

                // icono_perfilado(nombre, tamaño) — estilo outlined
                "icono_perfilado" | "icon_outlined" => {
                    let nombre = extraer_texto(argumentos, 0);
                    let tamaño = extraer_f64(argumentos, 1);
                    Some(Layout::MaterialIconLayout {
                        nombre,
                        tamaño: if tamaño <= 0.0 { 24.0 } else { tamaño },
                        color: "primary".to_string(),
                        estilo: "outlined".to_string(),
                    })
                }

                // icono_redondo(nombre, tamaño) — estilo rounded
                "icono_redondo" | "icon_rounded" => {
                    let nombre = extraer_texto(argumentos, 0);
                    let tamaño = extraer_f64(argumentos, 1);
                    Some(Layout::MaterialIconLayout {
                        nombre,
                        tamaño: if tamaño <= 0.0 { 24.0 } else { tamaño },
                        color: "primary".to_string(),
                        estilo: "rounded".to_string(),
                    })
                }

                // icono_agudo(nombre, tamaño) — estilo sharp
                "icono_agudo" | "icon_sharp" => {
                    let nombre = extraer_texto(argumentos, 0);
                    let tamaño = extraer_f64(argumentos, 1);
                    Some(Layout::MaterialIconLayout {
                        nombre,
                        tamaño: if tamaño <= 0.0 { 24.0 } else { tamaño },
                        color: "primary".to_string(),
                        estilo: "sharp".to_string(),
                    })
                }

                // icono_dos_tonos(nombre, tamaño) — estilo twotone
                "icono_dos_tonos" | "icon_twotone" => {
                    let nombre = extraer_texto(argumentos, 0);
                    let tamaño = extraer_f64(argumentos, 1);
                    Some(Layout::MaterialIconLayout {
                        nombre,
                        tamaño: if tamaño <= 0.0 { 24.0 } else { tamaño },
                        color: "primary".to_string(),
                        estilo: "twotone".to_string(),
                    })
                }

                _ => {
                    eprintln!(
                        "[Forja GUI Warning] Función de widget no reconocida en layout: '{}'",
                        nombre
                    );
                    Some(Layout::Label {
                        texto: format!("[Widget Desconocido: {}]", nombre),
                        es_variable: false,
                    })
                }
            }
        }
        Expresion::LiteralTexto(s) => Some(Layout::Label {
            texto: s.clone(),
            es_variable: false,
        }),
        _ => None,
    }
}

// ─── Helpers de tema Material You ──────────────────────────────────

/// Envuelve un child con un color role del tema
fn envolver_con_color_role(args: &[Expresion], role: &str) -> Option<Layout> {
    args.first()
        .and_then(expr_a_layout)
        .map(|c| Layout::ColoredBox {
            hijo: Box::new(c),
            color_role: role.to_string(),
        })
}

/// Crea una etiqueta con estilo tipográfico predefinido
fn styled_etiqueta(args: &[Expresion], style: &str) -> Option<Layout> {
    if let Some(a) = args.first() {
        match a {
            Expresion::LiteralTexto(s) => {
                return Some(Layout::StyledLabel {
                    texto: s.clone(),
                    style: style.to_string(),
                });
            }
            Expresion::Identificador { nombre: s, .. } => {
                return Some(Layout::StyledLabel {
                    texto: s.clone(),
                    style: style.to_string(),
                });
            }
            _ => {
                // Texto dinámico: concatenación, etiqueta_dinamica, Index, helpers...
                return Some(Layout::DynamicText {
                    expr: Box::new(a.clone()),
                    params: HashMap::new(),
                    style: style.to_string(),
                });
            }
        }
    }
    Some(Layout::StyledLabel {
        texto: String::new(),
        style: style.to_string(),
    })
}

/// Evalúa una expresión Forja a un valor (con soporte de parámetros locales,
/// acceso a mapas, concatenación y llamadas a funciones del AST).
/// Usado para renderizar listas dinámicas y textos dinámicos.
pub(crate) fn expr_a_valor(
    expr: &Expresion,
    params: &HashMap<String, ValorGUI>,
    almacen: &VariableAlmacen,
    declaraciones: &[Declaracion],
) -> Option<ValorGUI> {
    match expr {
        Expresion::LiteralTexto(s) => Some(ValorGUI::Texto(s.clone())),
        Expresion::LiteralNumero(n) => Some(ValorGUI::Entero(*n)),
        Expresion::LiteralDecimal(f) => Some(ValorGUI::Decimal(*f)),
        Expresion::LiteralBooleano(b) => Some(ValorGUI::Booleano(*b)),
        Expresion::Identificador { nombre, .. } => {
            if let Some(v) = params.get(nombre) {
                return Some(v.clone());
            }
            almacen.obtener(nombre).map(|v| ValorGUI::from_serde(&v))
        }
        Expresion::Index { objeto, indice } => {
            let obj = expr_a_valor(objeto, params, almacen, declaraciones)?;
            let idx = expr_a_valor(indice, params, almacen, declaraciones)?;
            match obj {
                ValorGUI::Mapa(m) => {
                    let clave = match idx {
                        ValorGUI::Texto(s) => s,
                        ValorGUI::Entero(i) => i.to_string(),
                        other => other.to_string(),
                    };
                    m.get(&clave).cloned()
                }
                ValorGUI::Texto(s) => {
                    // Objeto o array serializado (JSON)
                    let clave = idx.to_string();
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                        match v {
                            serde_json::Value::Object(obj) => {
                                if let Some(item) = obj.get(&clave) {
                                    return Some(ValorGUI::from_serde(item));
                                }
                            }
                            serde_json::Value::Array(arr) => {
                                if let Ok(pos) = clave.parse::<usize>() {
                                    if let Some(item) = arr.get(pos) {
                                        return Some(ValorGUI::from_serde(item));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    None
                }
                _ => None,
            }
        }
        Expresion::Binaria {
            izquierda,
            operador,
            derecha,
        } => {
            if *operador == Operador::Suma {
                let a = expr_a_texto(izquierda, params, almacen, declaraciones);
                let b = expr_a_texto(derecha, params, almacen, declaraciones);
                Some(ValorGUI::Texto(a + &b))
            } else {
                let a = expr_a_valor(izquierda, params, almacen, declaraciones)?;
                let b = expr_a_valor(derecha, params, almacen, declaraciones)?;
                let fa = a.to_f64();
                let fb = b.to_f64();
                let res = match operador {
                    Operador::Suma => fa + fb,
                    Operador::Resta => fa - fb,
                    Operador::Multiplicacion => fa * fb,
                    Operador::Division => {
                        if fb == 0.0 {
                            0.0
                        } else {
                            fa / fb
                        }
                    }
                    _ => return None,
                };
                Some(ValorGUI::Decimal(res))
            }
        }
        Expresion::LlamadaFuncion { nombre, argumentos } => {
            // etiqueta_dinamica(variable) → leer del store
            if nombre == "etiqueta_dinamica" || nombre == "varlabel" {
                if let Some(arg) = argumentos.first() {
                    let var_name = expr_a_texto(arg, params, almacen, declaraciones);
                    if !var_name.is_empty() {
                        if let Some(v) = almacen.obtener(&var_name) {
                            return Some(ValorGUI::from_serde(&v));
                        }
                    }
                }
                return None;
            }
            // Convertir argumentos y ejecutar función (helper o del AST)
            let mut args = Vec::new();
            for a in argumentos {
                match expr_a_valor(a, params, almacen, declaraciones) {
                    Some(v) => args.push(v),
                    None => return None,
                }
            }
            let mut almacen_clonar = almacen.clone();
            crate::evaluador::ejecutar_funcion(nombre, &args, declaraciones, &crate::evaluador::AlmacenGui(&almacen_clonar))
                .ok()
        }
        _ => None,
    }
}

/// Evalúa una expresión a texto (para etiquetas dinámicas).
pub(crate) fn expr_a_texto(
    expr: &Expresion,
    params: &HashMap<String, ValorGUI>,
    almacen: &VariableAlmacen,
    declaraciones: &[Declaracion],
) -> String {
    expr_a_valor(expr, params, almacen, declaraciones)
        .map(|v| v.to_display())
        .unwrap_or_default()
}

/// Convierte una expresión de texto (o layout) a un string simple para
/// titulo/subtitulo de listas: extrae el primer texto de una columna.
fn texto_elemento_columna(
    expr: &Expresion,
    params: &HashMap<String, ValorGUI>,
    almacen: &VariableAlmacen,
    declaraciones: &[Declaracion],
) -> String {
    match expr {
        Expresion::LlamadaFuncion { nombre, argumentos } => {
            if nombre == "columna" || nombre == "columna_con_gap" {
                if let Some(Expresion::Arreglo(hijos)) = argumentos.first() {
                    let mut textos = Vec::new();
                    for h in hijos {
                        if let Expresion::LlamadaFuncion {
                            nombre: hn,
                            argumentos: ha,
                        } = h
                        {
                            if hn == "etiqueta"
                                || hn == "texto_pequeno"
                                || hn == "texto_pequeño"
                                || hn == "texto_mediano"
                                || hn == "titular_mediano"
                                || hn == "titular_grande"
                                || hn == "encabezado_mediano"
                            {
                                if let Some(t) = ha.first() {
                                    textos.push(expr_a_texto(t, params, almacen, declaraciones));
                                }
                            }
                        }
                    }
                    return textos.join(" | ");
                }
            }
            // etiqueta(x), texto_pequeno(x) directo
            if let Some(t) = argumentos.first() {
                return expr_a_texto(t, params, almacen, declaraciones);
            }
            String::new()
        }
        _ => expr_a_texto(expr, params, almacen, declaraciones),
    }
}

/// Evalúa el layout de un item de lista dinámica, resolviendo accesos a mapa
/// (item["clave"]), funciones helper (formatear_fecha, formatear_monto) y
/// etiquetas dinámicas usando los parámetros del item.
pub(crate) fn expresion_a_layout_elemento(
    expr: &Expresion,
    params: &HashMap<String, ValorGUI>,
    almacen: &VariableAlmacen,
    declaraciones: &[Declaracion],
) -> Option<Layout> {
    match expr {
        Expresion::LlamadaFuncion { nombre, argumentos } => {
            // 5.8: Intentar parse_layout_call para casos compartidos primero
            if let Some(layout) = parsear_layout_llamada(
                nombre,
                argumentos,
                params,
                almacen,
                declaraciones,
                procesar_args_elemento,
            ) {
                return Some(layout);
            }
            // Casos específicos de expr_a_layout_item
            match nombre.as_str() {
                "elemento_lista_doble" | "two_line_list_item" => {
                    let leading = argumentos
                        .first()
                        .and_then(|a| {
                            expresion_a_layout_elemento(a, params, almacen, declaraciones)
                        })
                        .map(Box::new);
                    let titulo = texto_elemento_columna(
                        argumentos.get(1).unwrap_or(argumentos.first()?),
                        params,
                        almacen,
                        declaraciones,
                    );
                    let subtitulo = argumentos
                        .get(2)
                        .map(|a| texto_elemento_columna(a, params, almacen, declaraciones));
                    let trailing = argumentos
                        .get(3)
                        .and_then(|a| {
                            expresion_a_layout_elemento(a, params, almacen, declaraciones)
                        })
                        .map(Box::new);
                    Some(Layout::ElementoListaMaterial {
                        leading,
                        titulo,
                        subtitulo,
                        trailing,
                        on_click: None,
                    })
                }
                "icono_material" | "material_icon" => {
                    let nombre_icono = argumentos
                        .first()
                        .map(|a| expr_a_texto(a, params, almacen, declaraciones))
                        .unwrap_or_default();
                    Some(Layout::MaterialIconLayout {
                        nombre: nombre_icono,
                        tamaño: 24.0,
                        color: "on_surface_variant".to_string(),
                        estilo: "filled".to_string(),
                    })
                }
                "circulo_color" => {
                    let color = argumentos
                        .first()
                        .map(|a| expr_a_texto(a, params, almacen, declaraciones))
                        .unwrap_or_default();
                    Some(Layout::ColoredLabel {
                        texto: String::new(),
                        color,
                    })
                }
                "etiqueta" | "label" | "text" | "escribir" => {
                    if let Some(arg) = argumentos.first() {
                        let txt = expr_a_texto(arg, params, almacen, declaraciones);
                        if txt.is_empty() {
                            Some(Layout::Spacer(0.0))
                        } else {
                            Some(Layout::Label {
                                texto: txt,
                                es_variable: false,
                            })
                        }
                    } else {
                        Some(Layout::Spacer(0.0))
                    }
                }
                "texto_pequeno" | "texto_pequeño" | "texto_mediano" | "texto_grande"
                | "titular_mediano" | "titular_grande" | "titular_pequeño"
                | "encabezado_mediano" | "encabezado_grande" | "cuerpo_mediano" => {
                    let style = match nombre.as_str() {
                        "texto_pequeno" | "texto_pequeño" => "display_small",
                        "texto_mediano" => "display_medium",
                        "texto_grande" => "display_large",
                        "titular_mediano" => "headline_medium",
                        "titular_grande" => "headline_large",
                        "titular_pequeño" => "headline_small",
                        "encabezado_mediano" => "title_medium",
                        "encabezado_grande" => "title_large",
                        _ => "body_medium",
                    };
                    let txt = argumentos
                        .first()
                        .map(|a| expr_a_texto(a, params, almacen, declaraciones))
                        .unwrap_or_default();
                    Some(Layout::StyledLabel {
                        texto: txt,
                        style: style.to_string(),
                    })
                }
                "tarjeta" | "card" => argumentos
                    .first()
                    .and_then(|a| expresion_a_layout_elemento(a, params, almacen, declaraciones))
                    .map(|c| Layout::MaterialCard {
                        hijo: Box::new(c),
                        variant: CardVariant::Elevated,
                        on_click: None,
                        seleccionado: false,
                    }),
                "tarjeta_seleccionable" | "selectable_card" => argumentos
                    .first()
                    .and_then(|a| expresion_a_layout_elemento(a, params, almacen, declaraciones))
                    .map(|c| Layout::MaterialCard {
                        hijo: Box::new(c),
                        variant: CardVariant::Outlined,
                        on_click: extraer_callback(argumentos, 1).into(),
                        seleccionado: false,
                    }),
                "barra_progreso" | "progress" | "progress_bar" => {
                    let val = argumentos
                        .first()
                        .and_then(|a| expr_a_valor(a, params, almacen, declaraciones))
                        .map(|v| v.to_f64())
                        .unwrap_or(0.0);
                    Some(Layout::ValorProgresoLineal(val))
                }
                _ => expr_a_layout(expr),
            }
        }
        _ => expr_a_layout(expr),
    }
}

/// procesar_args con contexto de item (para listas dinámicas).
fn procesar_args_elemento(
    args: &[Expresion],
    params: &HashMap<String, ValorGUI>,
    almacen: &VariableAlmacen,
    declaraciones: &[Declaracion],
) -> Vec<Layout> {
    args.iter()
        .filter_map(|a| expresion_a_layout_elemento(a, params, almacen, declaraciones))
        .collect()
}

/// Envuelve un child con una familia de forma
fn envolver_con_forma(args: &[Expresion], family: &str) -> Option<Layout> {
    args.first()
        .and_then(expr_a_layout)
        .map(|c| Layout::ShapedBox {
            hijo: Box::new(c),
            forma_family: family.to_string(),
        })
}

// ─── Funciones de tema/color → movidas a crate::helpers ──────────────

// ─── Layout → xilem widgets (con tema Material You) ──────────────

/// 5.10: Estima el ancho de un Layout basado en su tipo (para wrap de FlowLayout).
/// Retorna None si no se puede estimar (fallback: 100px).
fn estimate_hijo_ancho(layout: &Layout) -> Option<f64> {
    match layout {
        Layout::MaterialButton { texto, .. } => Some((texto.len() as f64) * 8.0 + 32.0),
        Layout::Chip { texto, .. } => Some((texto.len() as f64) * 7.0 + 24.0),
        Layout::Label { texto, .. } => Some((texto.len() as f64) * 8.0),
        Layout::MaterialCard { .. } => Some(300.0),
        Layout::Container { max_ancho, .. } => Some(*max_ancho),
        Layout::ElementoListaMaterial { .. } => Some(280.0),
        Layout::Row { gap, .. } => Some(200.0 + gap),
        Layout::Column { gap, .. } => Some(200.0 + gap),
        _ => None,
    }
}

/// Convierte Layout a xilem usando AnyWidgetView para type erasure.
/// Recibe el tema actual como referencia para aplicar colores, tipografía y formas.
pub fn layout_a_vista<'a>(
    layout: &'a Layout,
    data: &'a mut AppStateNativo,
    _prog: &'a [Declaracion],
    theme: &'a MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    if crate::perf::activo() {
        crate::perf::contar_nodo();
    }
    match layout {
        // ─── Layouts existentes (compatibilidad hacia atrás) ──────
        Layout::Column {
            children,
            gap,
            alignment,
        } => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in children {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            let ma = parsear_alignment(alignment);
            // Ancho dinámico: en portrait usa el ancho completo, en landscape limita a 720dp máximo
            let ancho = match data.orientation {
                Orientation::Portrait => (data.ventana_ancho - 16.0).max(200.0),
                Orientation::Landscape => (data.ventana_ancho - 16.0).clamp(200.0, 720.0),
            };
            Box::new(
                view::sized_box(
                    view::flex(Axis::Vertical, (widgets,))
                        .main_axis_alignment(ma)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .gap(Length::px(*gap)),
                )
                .width(Length::px(ancho))
                .padding(16.0.px()),
            )
        }
        Layout::CenteredColumn(hijos) => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in hijos {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            // Ancho dinámico: en portrait usa el ancho completo, en landscape limita a 720dp máximo
            let ancho = match data.orientation {
                Orientation::Portrait => (data.ventana_ancho - 16.0).max(200.0),
                Orientation::Landscape => (data.ventana_ancho - 16.0).clamp(200.0, 720.0),
            };
            Box::new(
                view::sized_box(
                    view::flex(Axis::Vertical, (widgets,))
                        .main_axis_alignment(MainAxisAlignment::Center)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .dims(xilem::masonry::properties::Dimensions::height(
                            xilem::masonry::layout::Dim::Stretch,
                        )),
                )
                .width(Length::px(ancho))
                .padding(16.0.px()),
            )
        }
        Layout::Row {
            children,
            gap,
            alignment,
        } => {
            // 5.9: Fila con soporte de Flex para tarjetas y list items.
            // NOTA: FlexExt::flex() no funciona con Box<AnyWidgetView> (type erasure),
            // ya que Pod<DynWidget> no implementa AnyElement<FlexElement>.
            // Se maneja con tuplas de tamaño fijo para los casos más comunes (1-3 hijos).
            let ma = parsear_alignment(alignment);
            let flex_eligible = children.iter().all(|h| {
                matches!(
                    h,
                    Layout::MaterialCard { .. } | Layout::ElementoListaMaterial { .. }
                )
            });
            if flex_eligible && !children.is_empty() && children.len() <= 3 {
                // Casos con 1-3 hijos flex: usar tuplas con FlexExt
                match children.len() {
                    1 => {
                        let a = layout_a_vista(&children[0], data, _prog, theme);
                        let row =
                            view::flex(Axis::Horizontal, (xilem::view::FlexExt::flex(a, 1.0),))
                                .main_axis_alignment(ma)
                                .gap(Length::px(*gap));
                        return Box::new(row) as Box<AnyWidgetView<AppStateNativo>>;
                    }
                    2 => {
                        let a = layout_a_vista(&children[0], data, _prog, theme);
                        let b = layout_a_vista(&children[1], data, _prog, theme);
                        let row = view::flex(
                            Axis::Horizontal,
                            (
                                xilem::view::FlexExt::flex(a, 1.0),
                                xilem::view::FlexExt::flex(b, 1.0),
                            ),
                        )
                        .main_axis_alignment(ma)
                        .gap(Length::px(*gap));
                        return Box::new(row) as Box<AnyWidgetView<AppStateNativo>>;
                    }
                    3 => {
                        let a = layout_a_vista(&children[0], data, _prog, theme);
                        let b = layout_a_vista(&children[1], data, _prog, theme);
                        let c = layout_a_vista(&children[2], data, _prog, theme);
                        let row = view::flex(
                            Axis::Horizontal,
                            (
                                xilem::view::FlexExt::flex(a, 1.0),
                                xilem::view::FlexExt::flex(b, 1.0),
                                xilem::view::FlexExt::flex(c, 1.0),
                            ),
                        )
                        .main_axis_alignment(ma)
                        .gap(Length::px(*gap));
                        return Box::new(row) as Box<AnyWidgetView<AppStateNativo>>;
                    }
                    _ => {}
                }
            }
            // Caso general: sin flex
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in children {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            Box::new(
                view::flex(Axis::Horizontal, (widgets,))
                    .main_axis_alignment(ma)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(Length::px(*gap)),
            )
        }
        Layout::PilaZ(hijos) => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in hijos {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            Box::new(view::zstack((widgets,)))
        }
        Layout::Portal(hijo) => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // El área desplazable ocupa el ancho de la ventana (menos un margen
            // para el scrollbar). El contenido interno se estira con cross Fill.
            let ancho = (data.ventana_ancho - 18.0).max(200.0);
            Box::new(view::portal(
                view::sized_box(inner)
                    .width(Length::px(ancho))
                    .padding(16.0.px()),
            ))
        }
        Layout::Container { hijo, max_ancho } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            Box::new(view::sized_box(inner).width(Length::px(*max_ancho)))
        }
        Layout::Label { texto, es_variable } => {
            if *es_variable {
                let var_name = texto.clone();
                let txt = data.leer(&var_name).to_string();
                let gen = data.almacen.generation(&var_name);
                let color: forja_gui_rt::Color =
                    obtener_color_role(&theme.scheme, "on_surface").into();
                Box::new(memoize(
                    (gen, txt, color),
                    move |(_, text, clr): &(u64, String, forja_gui_rt::Color)| {
                        view::label(text.clone()).color(*clr)
                    },
                ))
            } else {
                let txt = texto.clone();
                let color = color_segun_monto(&theme.scheme, &txt);
                Box::new(view::label(txt).color(color))
            }
        }
        Layout::Title(texto) => {
            // Usar estilo headline_medium del tema
            let style = obtener_texto_style(&theme.typography, "headline_medium");
            let color: forja_gui_rt::Color = obtener_color_role(&theme.scheme, "on_surface").into();
            Box::new(
                view::label(texto.clone())
                    .text_size(style.font_tamano as f32)
                    .weight(FontWeight::BOLD)
                    .color(color),
            )
        }
        Layout::ColoredLabel { texto, color } => {
            // Roles del tema primero (primary, error, on_surface_variant, ...):
            // antes caía en blanco fijo y el texto desaparecía en modo claro.
            let c: Color = crate::theme_props::color_de_rol(theme, color)
                .map(Into::into)
                .or_else(|| color_desde_nombre(color))
                .unwrap_or_else(|| theme.scheme.on_surface.into());
            Box::new(view::label(texto.clone()).color(c))
        }
        Layout::VariableLabel { variable } => {
            let var_name = variable.clone();
            // Leer valor Y generación para memoize reactivo
            let txt = data.leer(&var_name).to_string();
            let gen = data.almacen.generation(&var_name);
            // Usar memoize para que este widget solo se reconstruya
            // cuando la generación de la variable cambie
            Box::new(memoize((gen, txt), move |(_, text): &(u64, String)| {
                view::variable_label(text.clone())
            }))
        }
        Layout::Button { texto, callback } => {
            let cb = callback.clone();
            let t = texto.clone();
            let prog = _prog.to_vec();
            Box::new(view::text_button(t, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            }))
            // Nota: xilem 0.4 text_button no expone color de fondo directamente;
            // el color primario se aplica donde la API lo permita
        }
        Layout::EntradaTexto {
            variable,
            multiline: _,
            placeholder,
        } => {
            let var_name = variable.clone();
            let val = data.leer(&var_name).to_string();
            let gen = data.almacen.generation(&var_name);
            let ph = placeholder.clone();
            let tc: forja_gui_rt::Color = theme.scheme.on_surface.into();
            Box::new(memoize(
                (gen, val.clone(), var_name.clone(), ph.clone(), tc),
                move |(_, text, vn, pl, tc): &(
                    u64,
                    String,
                    String,
                    String,
                    forja_gui_rt::Color,
                )| {
                    let vn = vn.clone();
                    let pl = pl.clone();
                    let mut ti = view::text_input(
                        text.clone(),
                        move |data: &mut AppStateNativo, nuevo_val: String| {
                            data.escribir(&vn, ValorGUI::Texto(nuevo_val));
                        },
                    )
                    .text_color(*tc);
                    if !pl.is_empty() {
                        ti = ti.placeholder(pl.as_str());
                    }
                    map_message(
                        ti,
                        |_data: &mut AppStateNativo, resultado: MessageResult<()>| match resultado {
                            MessageResult::Action(()) => MessageResult::Nop,
                            other => other,
                        },
                    )
                },
            ))
        }
        Layout::ProgressBar { variable } => {
            let var_name = variable.clone();
            let val_str = data.leer(&var_name).to_string();
            let num: f64 = val_str.parse().unwrap_or(0.0);
            let gen = data.almacen.generation(&var_name);
            Box::new(memoize((gen, num), move |(_, n): &(u64, f64)| {
                view::progress_bar(Some(*n))
            }))
        }
        Layout::Slider { variable, min, max } => {
            let var_name = variable.clone();
            let val = data.leer(&var_name).to_f64();
            let gen = data.almacen.generation(&var_name);
            let mn = *min;
            let mx = *max;
            if depuracion_activo() {
                eprintln!(
                    "[slider] {}: valor={} rango={}..{} gen={}",
                    var_name, val, mn, mx, gen
                );
            }
            Box::new(memoize(
                (gen, val, mn, mx, var_name.clone()),
                move |(_, v, mn2, mx2, vn): &(u64, f64, f64, f64, String)| {
                    let vn = vn.clone();
                    view::slider(
                        *mn2,
                        *mx2,
                        *v,
                        move |data: &mut AppStateNativo, nuevo_val: f64| {
                            data.escribir(&vn, ValorGUI::Decimal(nuevo_val));
                        },
                    )
                },
            ))
        }
        Layout::Checkbox { variable } => {
            let var_name = variable.clone();
            let txt = variable.clone();
            let checked = data.leer(&var_name).to_bool();
            let gen = data.almacen.generation(&var_name);
            Box::new(memoize(
                (gen, checked, txt, var_name.clone()),
                move |(_, chk, label_txt, vn): &(u64, bool, String, String)| {
                    let vn = vn.clone();
                    let casilla = view::checkbox(
                        label_txt.clone(),
                        *chk,
                        move |data: &mut AppStateNativo, new_checked: bool| {
                            data.escribir(&vn, ValorGUI::Booleano(new_checked));
                        },
                    );
                    // En una columna el checkbox se estira a todo el ancho y el
                    // borde del widget cruza la pantalla; en una fila toma su
                    // ancho natural (caja + etiqueta).
                    view::flex(Axis::Horizontal, (casilla,))
                        .main_axis_alignment(MainAxisAlignment::Start)
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                },
            ))
        }
        Layout::Prose(texto) => Box::new(view::prose(texto.clone())),
        // El `spinner` de xilem main pide todo el espacio disponible cuando se le
        // mide con `LenReq::FitContent`, así que hay que acotarlo a mano: el
        // indicador Material mide 24 dp (antes, en 0.4, el widget se achicaba solo).
        Layout::Spinner => {
            Box::new(view::sized_box(view::spinner()).dims((Length::px(24.0), Length::px(24.0))))
        }
        Layout::Separator => {
            let color: forja_gui_rt::Color = obtener_color_role(&theme.scheme, "outline").into();
            // `Dim::Stretch` + alto fijo: una línea de 1 px a todo el ancho. Un
            // `.height(..)` suelto dejaría el ancho en `auto` (0 px) y no se vería.
            Box::new(
                view::sized_box(view::label(String::new()))
                    .dims((crate::layout::Dim::Stretch, Length::px(1.0)))
                    .background(Background::Color(color)),
            )
        }
        Layout::Spacer(tamano) => {
            let t = *tamano;
            Box::new(
                view::sized_box(view::label(String::new())).dims((Length::px(t), Length::px(t))),
            )
        }

        // ─── Nuevos: variantes de tema Material You ───────────────

        // ThemeProvider: cambia el tema para los hijos
        // Acepta color semilla (#RRGGBB) o diseño nombrado ("fluent", "liquid_glass", con
        // sufijo ":dark"/"oscuro" para forzar modo oscuro)
        Layout::ThemeProvider { hijo, theme: seed } => {
            let nuevo_theme = MaterialTheme::named(seed, theme.es_dark);
            let inner = layout_a_vista(hijo, data, _prog, &nuevo_theme);
            // Cada diseño tiene su propio fondo (Mica en Fluent, vidrio en Liquid
            // Glass): lo pintamos acá para que se vea también si el wrapper no es
            // el nodo raíz.
            let bg: Color = nuevo_theme.scheme.background.into();
            Box::new(view::sized_box(inner).background(Background::Color(bg)))
        }

        // ColoredBox: aplica un color del esquema como fondo
        Layout::ColoredBox { hijo, color_role } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let bg: Color = obtener_color_role(&theme.scheme, color_role).into();
            Box::new(view::sized_box(inner).background(Background::Color(bg)))
        }

        // StyledLabel: etiqueta con estilo tipográfico predefinido
        Layout::StyledLabel { texto, style } => {
            let texto_style = obtener_texto_style(&theme.typography, style);
            let color = color_segun_monto(&theme.scheme, texto);
            Box::new(
                view::label(texto.clone())
                    .text_size(texto_style.font_tamano as f32)
                    .weight(texto_style.weight.to_xilem_weight())
                    .color(color),
            )
        }

        // DynamicText: texto evaluado en tiempo de render (concatenación,
        // etiqueta_dinamica, helpers del AST) con estilo tipográfico.
        Layout::DynamicText {
            expr,
            params,
            style,
        } => {
            let texto_style = obtener_texto_style(&theme.typography, style);
            let txt = expr_a_texto(expr, params, &data.almacen, _prog);
            let color = color_segun_monto(&theme.scheme, &txt);
            let gen = data.almacen.global_generation();
            let fs = texto_style.font_tamano as f32;
            let wt = texto_style.weight.to_xilem_weight();
            Box::new(memoize(
                (gen, txt, color, fs, wt),
                move |(_, t, clr, size, w): &(
                    u64,
                    String,
                    forja_gui_rt::Color,
                    f32,
                    xilem::FontWeight,
                )| {
                    view::label(t.clone())
                        .text_size(*size)
                        .weight(*w)
                        .color(*clr)
                },
            ))
        }

        // DynamicLabel: etiqueta dinámica sin estilo tipográfico.
        Layout::DynamicLabel { expr, params } => {
            let txt = expr_a_texto(expr, params, &data.almacen, _prog);
            let color = color_segun_monto(&theme.scheme, &txt);
            let gen = data.almacen.global_generation();
            Box::new(memoize(
                (gen, txt, color),
                move |(_, t, clr): &(u64, String, forja_gui_rt::Color)| {
                    view::label(t.clone()).color(*clr)
                },
            ))
        }

        // ShapedBox: aplica border-radius al hijo
        Layout::ShapedBox { hijo, forma_family } => {
            let radius = obtener_forma_radius(&theme.shapes, forma_family);
            let inner = layout_a_vista(hijo, data, _prog, theme);
            Box::new(view::sized_box(inner).corner_radius(radius.px()))
        }

        // 5.5: ElevatedBox — aplica corner_radius como indicador visual de elevación.
        // xilem 0.4: BoxShadow no está disponible en sized_box; se documenta la restricción.
        Layout::ElevatedBox {
            hijo,
            level: _level,
            forma_family,
        } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let forma_radius = obtener_forma_radius(&theme.shapes, forma_family);
            // NOTA: shadow_to_box_shadow(&shadow_cfg) calcula la sombra correctamente
            // pero xilem 0.4 no expone box-shadow en WidgetPod. Aplicar solo corner_radius.
            Box::new(view::sized_box(inner).corner_radius(forma_radius.px()))
        }

        // 5.6: StateLayerBox — overlay semi-transparente según el estado visual
        Layout::StateLayerBox { hijo, state } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let overlay_alpha: f64 = match state.as_str() {
                "hover" => 0.08,
                "pressed" => 0.12,
                "focused" => 0.12,
                "disabled" => 0.38,
                _ => 0.0,
            };
            if overlay_alpha > 0.0 {
                let overlay_color = RgbColor(0, 0, 0).con_alpha(overlay_alpha);
                Box::new(view::zstack((Box::new(
                    view::sized_box(inner).background(Background::Color(overlay_color)),
                )
                    as Box<AnyWidgetView<AppStateNativo>>,)))
            } else {
                inner
            }
        }

        // ResponsiveLayout: 3 variantes según WindowSizeClass
        Layout::ResponsiveLayout {
            compact,
            medium,
            expanded,
        } => match data.ventana_tamano {
            VentanaTamanoClase::Expanded => layout_a_vista(expanded, data, _prog, theme),
            VentanaTamanoClase::Medium => layout_a_vista(medium, data, _prog, theme),
            VentanaTamanoClase::Compact => layout_a_vista(compact, data, _prog, theme),
        },

        // Padding: añade espacio alrededor del hijo
        Layout::Padding { hijo, amount } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            Box::new(view::sized_box(inner).padding((*amount).px()))
        }

        // Expanded: llena el espacio disponible
        Layout::Expanded { hijo } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // Envolver en flex vertical con must_fill para expandir el espacio disponible
            Box::new(view::flex(Axis::Vertical, (inner,)).dims(
                xilem::masonry::properties::Dimensions::height(
                    xilem::masonry::layout::Dim::Stretch,
                ),
            ))
        }

        // Centered: centra el hijo en el eje transversal
        Layout::Centered { hijo } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let widgets = vec![inner];
            Box::new(
                view::flex(Axis::Vertical, (widgets,))
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .dims(xilem::masonry::properties::Dimensions::height(
                        xilem::masonry::layout::Dim::Stretch,
                    )),
            )
        }

        // ─── Layout responsive avanzado ────────────────────────

        // FlexLayout: flex con axis, gap y wrap configurables
        Layout::FlexLayout {
            children,
            axis,
            gap,
            envolver: _,
        } => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in children {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            let ax = match axis.to_lowercase().as_str() {
                "horizontal" | "h" | "fila" => Axis::Horizontal,
                _ => Axis::Vertical,
            };
            // Nota: xilem 0.4 no soporta wrap nativamente; se implementará
            // cuando la API lo exponga. Por ahora se usa flex con gap.
            Box::new(view::flex(ax, (widgets,)).gap(Length::px(*gap)))
        }

        // 5.11: Grid dinámico — widget grid real de Masonry.
        // Columnas FRACTION: ocupan todo el ancho disponible (responsivo al resize).
        // Filas Auto: cada fila toma la altura de su contenido (autosize).
        Layout::Grid {
            children,
            columnas,
            gap,
        } => {
            if children.is_empty() {
                return Box::new(view::sized_box(view::label("")).height(Length::px(0.0)));
            }
            // Determinar columnas según orientación y tamaño
            let effective_cols = match data.orientation {
                Orientation::Portrait => {
                    if data.ventana_ancho < 400.0 {
                        1
                    } else if data.ventana_ancho < 600.0 {
                        (*columnas).min(2)
                    } else {
                        *columnas
                    }
                }
                Orientation::Landscape => *columnas,
            }
            .max(1);
            let n_rows = children.len().div_ceil(effective_cols);

            let mut items = Vec::with_capacity(children.len());
            for (i, item) in children.iter().enumerate() {
                let widget = layout_a_vista(item, data, _prog, theme);
                items.push(view::grid_item(
                    widget,
                    GridParams::pos((i % effective_cols) as u16, (i / effective_cols) as u16),
                ));
            }

            Box::new(
                view::grid(items)
                    .columns(view::repeat_tracks(
                        effective_cols,
                        view::GridTrackSize::FRACTION,
                    ))
                    .rows(view::repeat_tracks(n_rows, view::GridTrackSize::Auto))
                    .gap(Length::px(*gap)),
            )
        }

        // 5.10: FlowLayout con wrap real — divide hijos en filas según ancho disponible
        Layout::FlowLayout { children, gap } => {
            if children.is_empty() {
                return Box::new(view::sized_box(view::label("")).height(Length::px(0.0)));
            }
            // Estimar anchos de cada hijo y renderizarlos
            let rendered: Vec<(Box<AnyWidgetView<AppStateNativo>>, f64)> = children
                .iter()
                .map(|h| {
                    let w = layout_a_vista(h, data, _prog, theme);
                    let hijo_ancho = estimate_hijo_ancho(h).unwrap_or(100.0);
                    (w, hijo_ancho)
                })
                .collect();

            let available_ancho = (data.ventana_ancho - 48.0).max(200.0);
            let mut rows: Vec<Vec<Box<AnyWidgetView<AppStateNativo>>>> = Vec::new();
            let mut actual_row: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            let mut actual_ancho = 0.0;

            for (widget, width) in rendered {
                if actual_row.is_empty() || actual_ancho + width + *gap <= available_ancho {
                    actual_row.push(widget);
                    actual_ancho += width + *gap;
                } else {
                    rows.push(actual_row);
                    actual_row = vec![widget];
                    actual_ancho = width;
                }
            }
            if !actual_row.is_empty() {
                rows.push(actual_row);
            }

            let row_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = rows
                .into_iter()
                .map(|row| {
                    Box::new(
                        view::flex(Axis::Horizontal, (row,))
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .gap(Length::px(*gap)),
                    ) as Box<AnyWidgetView<AppStateNativo>>
                })
                .collect();

            Box::new(view::flex(Axis::Vertical, (row_widgets,)).gap(Length::px(*gap)))
        }

        // ─── Botones Material Design 3 ──────────────────────────────
        // NOTA: En xilem 0.4, Button no soporta ContentColor ni FontSize directamente.
        // Usamos view::button(view::label(...).color(...).text_size(...), callback)
        // para poder estilizar el texto, y .background()/.corner_radius()/.border_color()
        // en el Button wrapper para el contenedor.

        // ─── MaterialButton (5 variantes) ───────────────────────────
        Layout::MaterialButton {
            texto,
            callback,
            variant,
            icono: _,
            disabled: _,
        } => {
            let scheme = &theme.scheme;
            let etiqueta_style = obtener_texto_style(&theme.typography, "label_large");
            match variant {
                ButtonVariant::Filled => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.on_primary.into(),
                        bg: Some(scheme.primary.into()),
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Tonal => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.on_secondary_container.into(),
                        bg: Some(scheme.secondary_container.into()),
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Outlined => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.primary.into(),
                        bg: None,
                        border: Some(scheme.outline.into()),
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Text => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.primary.into(),
                        bg: None,
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Elevated => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.primary.into(),
                        bg: Some(scheme.surface.into()),
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
            }
        }

        // ─── FAB (Floating Action Button) ───────────────────────────
        Layout::FAB {
            icono,
            callback,
            tamano,
            texto_extendido,
        } => crate::widgets::material_buttons::render_fab(
            icono,
            callback,
            tamano,
            texto_extendido,
            data,
            _prog,
            theme,
        ),

        // ─── IconButton (4 variantes) ──────────────────────────────
        Layout::IconButton {
            icono,
            callback,
            variant,
            seleccionado: _,
        } => crate::widgets::material_buttons::render_icon_button(
            icono, callback, variant, data, _prog, theme,
        ),

        // ─── SegmentedButton ────────────────────────────────────────
        Layout::SegmentedButton {
            opciones,
            seleccionados,
            callback,
            multiple: _,
        } => crate::widgets::material_buttons::render_segmented_button(
            opciones,
            seleccionados,
            callback,
            data,
            _prog,
            theme,
        ),

        // ─── Chip (4 variantes) ────────────────────────────────────
        Layout::Chip {
            texto,
            callback,
            variant,
            activo,
            on_quitar: _,
        } => {
            let scheme = &theme.scheme;
            let etiqueta_style = obtener_texto_style(&theme.typography, "label_small");
            match variant {
                ChipVariant::Assist | ChipVariant::Suggestion => crear_chip_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.on_surface.into(),
                        bg: None,
                        border: Some(scheme.outline.into()),
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    8.0,
                ),
                ChipVariant::Filter => {
                    if *activo {
                        crear_chip_button(
                            texto,
                            ColoresBoton {
                                fg: scheme.on_secondary_container.into(),
                                bg: Some(scheme.secondary_container.into()),
                                border: None,
                            },
                            etiqueta_style,
                            callback,
                            _prog,
                            8.0,
                        )
                    } else {
                        crear_chip_button(
                            texto,
                            ColoresBoton {
                                fg: scheme.on_surface.into(),
                                bg: None,
                                border: Some(scheme.outline.into()),
                            },
                            etiqueta_style,
                            callback,
                            _prog,
                            8.0,
                        )
                    }
                }
                ChipVariant::Entrada => crear_chip_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.on_secondary_container.into(),
                        bg: Some(scheme.secondary_container.into()),
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    8.0,
                ),
            }
        }

        // ─── Inputs Material Design 3 ──────────────────────────────────

        // ─── MaterialTextField ─────────────────────────────────────────
        Layout::CampoTextoMaterial {
            variable,
            etiqueta,
            placeholder,
            variant,
            multiline: _,
            error,
            counter: _,
        } => crate::widgets::material_inputs::render_texto_campo(
            variable,
            etiqueta,
            placeholder,
            variant,
            error,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        // ─── MaterialPasswordField ─────────────────────────────────────
        Layout::CampoContrasenaMaterial {
            variable,
            etiqueta,
            visible: _,
        } => crate::widgets::material_inputs::render_contrasena_campo(
            variable, etiqueta, data, _prog, theme,
        ),

        // ─── MaterialNumberField ───────────────────────────────────────
        Layout::CampoNumeroMaterial {
            variable,
            etiqueta,
            min,
            max,
            decimales: _,
        } => crate::widgets::material_inputs::render_numero_campo(
            variable, etiqueta, min, max, data, _prog, theme,
        ),

        // ─── MaterialSearchField ───────────────────────────────────────
        Layout::CampoBusquedaMaterial {
            variable,
            placeholder,
        } => crate::widgets::material_inputs::render_buscar_campo(
            variable,
            placeholder,
            data,
            _prog,
            theme,
        ),

        // ─── MaterialDropdown ──────────────────────────────────────────
        Layout::MaterialDropdown {
            opciones,
            seleccionada,
            placeholder,
        } => crate::widgets::material_inputs::render_dropdown(
            opciones,
            seleccionada,
            placeholder,
            data,
            _prog,
            theme,
        ),

        // ─── MaterialSelect ────────────────────────────────────────────
        Layout::MaterialSelect {
            opciones,
            seleccionada,
            etiqueta,
        } => crate::widgets::material_inputs::render_select(
            opciones,
            seleccionada,
            etiqueta,
            data,
            _prog,
            theme,
        ),

        // ─── MaterialAutocomplete ──────────────────────────────────────
        Layout::MaterialAutocomplete {
            opciones: _,
            variable,
        } => crate::widgets::material_inputs::render_autocomplete(variable, data, _prog, theme),

        // ─── MaterialRadioGroup ────────────────────────────────────────
        Layout::MaterialRadioGroup {
            nombre: _,
            opciones,
            seleccion,
            callback,
            direction,
        } => crate::widgets::material_inputs::render_radio_group(
            opciones, seleccion, callback, direction, data, _prog, theme,
        ),

        // ─── MaterialSwitch ────────────────────────────────────────────
        Layout::MaterialSwitch { etiqueta, variable } => {
            crate::widgets::material_inputs::render_switch(etiqueta, variable, data, _prog, theme)
        }

        // ─── MaterialSliderDiscrete ────────────────────────────────────
        Layout::MaterialSliderDiscrete {
            variable,
            min,
            max,
            steps: _,
        } => crate::widgets::material_inputs::render_slider_discrete(
            variable, min, max, data, _prog, theme,
        ),

        // ─── MaterialSliderRange ───────────────────────────────────────
        Layout::MaterialSliderRange {
            variable_inicio,
            variable_fin,
            min,
            max,
        } => crate::widgets::material_inputs::render_slider_range(
            variable_inicio,
            variable_fin,
            min,
            max,
            data,
            _prog,
            theme,
        ),

        // ─── MaterialChipGroup ─────────────────────────────────────────
        Layout::MaterialChipGroup {
            chips,
            seleccion,
            callback,
            multiple: _,
        } => crate::widgets::material_inputs::render_chip_group(
            chips, seleccion, callback, data, _prog, theme,
        ),

        // ─── MaterialDatePicker ────────────────────────────────────────
        Layout::MaterialDatePicker { variable } => {
            crate::pickers::date_picker_vista(variable, data, &theme.scheme, theme)
        }

        // ─── MaterialTimePicker ────────────────────────────────────────
        Layout::MaterialTimePicker { variable } => {
            crate::pickers::time_picker_vista(variable, data, &theme.scheme, theme)
        }

        // ─── ColorPicker ───────────────────────────────────────────────
        Layout::ColorPicker { variable } => {
            crate::pickers::color_picker_vista(variable, data, &theme.scheme, theme)
        }

        // AspectRatioBox: caja con relación de aspecto fija
        Layout::AspectRatioBox { hijo, ratio } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // Calcular alto basado en el ancho disponible y el ratio
            let available_ancho = (data.ventana_ancho - 48.0).max(200.0);
            let height = available_ancho / ratio;
            Box::new(view::sized_box(inner).dims((Length::px(available_ancho), Length::px(height))))
        }

        // ─── Tarjetas, Listas y Tablas ──────────────────────────────
        Layout::MaterialCard {
            hijo,
            variant,
            on_click: _,
            seleccionado,
        } => crate::widgets::material_cards::render_card(
            hijo,
            variant,
            seleccionado,
            data,
            _prog,
            theme,
        ),

        Layout::ElementoListaMaterial {
            leading,
            titulo,
            subtitulo,
            trailing,
            on_click: _,
        } => crate::widgets::material_cards::render_lista_elemento(
            leading, titulo, subtitulo, trailing, data, _prog, theme,
        ),

        Layout::ListaMaterial { items, dividers } => {
            crate::widgets::material_cards::render_lista(items, dividers, data, _prog, theme)
        }

        Layout::ControlListaMaterial {
            items,
            control_tipo: _,
            variables: _,
        } => crate::widgets::material_cards::render_lista_control(items, data, _prog, theme),

        Layout::SeleccionListaMaterial {
            items,
            seleccion: _,
            callback: _,
            multiple: _,
        } => crate::widgets::material_cards::render_lista_selection(items, data, _prog, theme),

        // ─── DynamicList: renderiza items desde una variable ────────
        // 5.4: NOTA sobre virtualización: view::virtual_scroll() de xilem 0.4 requiere
        // un closure que retorne un WidgetView concreto (no type-erased). Dado que
        // render_dynamic_list usa expr_a_layout_item → Box<AnyWidgetView>, la virtualización
        // directa no es compatible sin un wrapper intermedio. Se documenta para futura
        // migración cuando xilem soporte virtual_scroll con type erasure.
        Layout::ListaDinamica {
            variable,
            elemento_fn,
        } => crate::widgets::material_cards::render_dynamic_lista(
            variable,
            elemento_fn,
            data,
            _prog,
            theme,
        ),

        Layout::TablaDatosMaterial {
            columnas,
            filas,
            ordenable,
            seleccionable: _,
            col_orden,
            orden_asc,
        } => crate::widgets::material_cards::render_data_tabla(
            columnas,
            filas,
            ordenable,
            col_orden,
            orden_asc,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        Layout::MaterialSurface { hijo, color_role } => {
            crate::widgets::material_cards::render_surface(hijo, color_role, data, _prog, theme)
        }

        Layout::MaterialScaffold {
            top,
            body,
            bottom,
            fab: _,
        } => crate::widgets::material_cards::render_scaffold(top, body, bottom, data, _prog, theme),

        // ═══════════════════════════════════════════════════════════════
        // FEEDBACK Y SUPERPOSICIONES (Material Design 3)
        // ═══════════════════════════════════════════════════════════════

        // ─── DialogOverlay ───────────────────────────────────────────
        // Diálogo superpuesto con overlay semitransparente + fade+scale animado
        Layout::DialogOverlay { dialog, visible } => {
            let show = data.leer(visible).to_string() == "true";
            // Detectar cambio y animar progreso
            data.anim_visibility_changed(visible, show);
            let progress = data.anim_progress(visible, 200.0);
            let eased = EASE_EMPHASIZED.aplicar(progress);

            if show || progress > 0.0 {
                let inner = layout_a_vista(dialog, data, _prog, theme);
                // Overlay con opacidad animada (0 → 0.32)
                let overlay_alpha = eased * 0.32;
                let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(overlay_alpha);
                // Diálogo con escala animada (0.95 → 1.0)
                let dialog_scale = 0.95 + eased * 0.05;

                Box::new(view::zstack((
                    view::sized_box(view::label(String::new()))
                        .background(Background::Color(overlay_color)),
                    view::transformed(view::sized_box(inner).width(Length::px(312.0)))
                        .scale_non_uniform(dialog_scale, dialog_scale),
                )))
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── DialogAlert ─────────────────────────────────────────────
        // Tarjeta de diálogo con título, mensaje y botones
        Layout::DialogAlert {
            titulo,
            mensaje,
            confirmar_texto,
            cancelar_texto,
            on_confirm,
            on_cancel,
        } => {
            let scheme = &theme.scheme;
            let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(0.32);
            let surface_bg: Color = scheme.surface.into();
            let fg: Color = scheme.on_surface.into();
            let fg_var: Color = scheme.on_surface_variant.into();
            let primary_color: Color = scheme.primary.into();
            let prog = _prog.to_vec();
            let cb_confirm = on_confirm.clone();
            let cb_cancel = on_cancel.clone();

            // Botón Confirmar
            let confirm_btn = {
                let cb = cb_confirm.clone();
                let p = prog.clone();
                view::button(
                    view::label(confirmar_texto.clone())
                        .text_size(14.0)
                        .weight(FontWeight::MEDIUM)
                        .color(primary_color),
                    move |data: &mut AppStateNativo| {
                        if !cb.is_empty() {
                            ejecutar_callback_y_actualizar(&cb, data, &p);
                        }
                    },
                )
            };

            // Botón Cancelar (solo si hay texto)
            let tiene_cancel = !cancelar_texto.is_empty();
            let cancel_btn = if tiene_cancel {
                let cb = cb_cancel.clone();
                let p = prog.clone();
                Some(view::button(
                    view::label(cancelar_texto.clone())
                        .text_size(14.0)
                        .weight(FontWeight::MEDIUM)
                        .color(primary_color),
                    move |data: &mut AppStateNativo| {
                        if !cb.is_empty() {
                            ejecutar_callback_y_actualizar(&cb, data, &p);
                        }
                    },
                ))
            } else {
                None
            };

            // Fila de botones
            let button_row: Box<AnyWidgetView<AppStateNativo>> = if tiene_cancel {
                Box::new(
                    view::flex(
                        Axis::Horizontal,
                        (
                            cancel_btn.unwrap(),
                            view::sized_box(view::label(String::new())).width(Length::px(8.0)),
                            confirm_btn,
                        ),
                    )
                    .main_axis_alignment(MainAxisAlignment::End),
                )
            } else {
                Box::new(
                    view::flex(Axis::Horizontal, (confirm_btn,))
                        .main_axis_alignment(MainAxisAlignment::End),
                )
            };

            // Título
            let title_etiqueta = {
                let title_style = obtener_texto_style(&theme.typography, "title_large");
                view::label(titulo.clone())
                    .text_size(title_style.font_tamano as f32)
                    .weight(FontWeight::BOLD)
                    .color(fg)
            };

            // Mensaje
            let mensaje_etiqueta = {
                let body_style = obtener_texto_style(&theme.typography, "body_medium");
                view::label(mensaje.clone())
                    .text_size(body_style.font_tamano as f32)
                    .color(fg_var)
            };

            // Card del diálogo
            let dialog_card = view::flex(
                Axis::Vertical,
                (
                    title_etiqueta,
                    view::sized_box(view::label(String::new())).height(Length::px(8.0)),
                    mensaje_etiqueta,
                    view::sized_box(view::label(String::new())).height(Length::px(16.0)),
                    button_row,
                ),
            );

            // Card del diálogo con padding interior
            let dialog_inner = view::sized_box(dialog_card)
                .background(Background::Color(surface_bg))
                .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px())
                .padding(24.0.px());

            // Contenedor externo con ancho máximo
            let dialog_container = view::sized_box(dialog_inner).width(Length::px(312.0));

            // Overlay completo
            Box::new(view::zstack((
                view::sized_box(view::label(String::new()))
                    .background(Background::Color(overlay_color)),
                dialog_container,
            )))
        }

        // ─── DialogCustom ────────────────────────────────────────────
        // Diálogo personalizado con hijo arbitrario
        Layout::DialogCustom {
            titulo,
            hijo,
            on_cerrar: _,
        } => {
            let scheme = &theme.scheme;
            let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(0.32);
            let surface_bg: Color = scheme.surface.into();
            let fg: Color = scheme.on_surface.into();

            let title_style = obtener_texto_style(&theme.typography, "title_large");
            let title_etiqueta = view::label(titulo.clone())
                .text_size(title_style.font_tamano as f32)
                .weight(FontWeight::BOLD)
                .color(fg);

            let inner = layout_a_vista(hijo, data, _prog, theme);

            let dialog_card = view::flex(
                Axis::Vertical,
                (
                    title_etiqueta,
                    view::sized_box(view::label(String::new())).height(Length::px(8.0)),
                    inner,
                ),
            );

            let dialog_container = view::sized_box(dialog_card)
                .background(Background::Color(surface_bg))
                .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px())
                .padding(24.0.px());

            Box::new(view::zstack((
                view::sized_box(view::label(String::new()))
                    .background(Background::Color(overlay_color)),
                dialog_container,
            )))
        }

        // ─── BottomSheet ─────────────────────────────────────────────
        // Panel que emerge desde abajo (Standard, Modal, Expanded)
        // Animación: slide-up con fade del overlay
        Layout::BottomSheet {
            hijo,
            variant,
            visible,
            on_dismiss: _,
        } => {
            let show = data.leer(visible).to_string() == "true";
            // Detectar cambio y animar
            data.anim_visibility_changed(visible, show);
            let progress = data.anim_progress(visible, 250.0);
            let eased = EASE_EMPHASIZED.aplicar(progress);

            if show || progress > 0.0 {
                let scheme = &theme.scheme;
                let inner = layout_a_vista(hijo, data, _prog, theme);
                let sheet_bg: Color = scheme.surface.into();
                // Slide-up: offset_y desde 300px hasta 0
                let sheet_offset_y = 300.0 * (1.0 - eased);

                match variant {
                    SheetVariant::Standard => {
                        // Sin overlay, solo panel con slide-up
                        let sheet = view::transformed(
                            view::sized_box(inner)
                                .background(Background::Color(sheet_bg))
                                .corner_radius(
                                    obtener_forma_radius(&theme.shapes, "extra_large").px(),
                                ),
                        )
                        .translate((0.0, sheet_offset_y));
                        Box::new(view::zstack((
                            view::sized_box(view::label(String::new())),
                            Box::new(sheet) as Box<AnyWidgetView<AppStateNativo>>,
                        )))
                    }
                    SheetVariant::Modal => {
                        // Overlay con fade + panel con slide-up
                        let overlay_alpha = eased * 0.32;
                        let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(overlay_alpha);
                        let sheet = view::transformed(
                            view::sized_box(inner)
                                .background(Background::Color(sheet_bg))
                                .corner_radius(
                                    obtener_forma_radius(&theme.shapes, "extra_large").px(),
                                ),
                        )
                        .translate((0.0, sheet_offset_y));
                        Box::new(view::zstack((
                            view::sized_box(view::label(String::new()))
                                .background(Background::Color(overlay_color)),
                            Box::new(sheet) as Box<AnyWidgetView<AppStateNativo>>,
                        )))
                    }
                    SheetVariant::Expanded => {
                        // Overlay con fade + panel expandido con slide-up
                        let overlay_alpha = eased * 0.32;
                        let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(overlay_alpha);
                        let sheet = view::transformed(
                            view::sized_box(inner)
                                .background(Background::Color(sheet_bg))
                                .corner_radius(
                                    obtener_forma_radius(&theme.shapes, "extra_large").px(),
                                ),
                        )
                        .translate((0.0, sheet_offset_y));
                        Box::new(view::zstack((
                            view::sized_box(view::label(String::new()))
                                .background(Background::Color(overlay_color)),
                            Box::new(sheet) as Box<AnyWidgetView<AppStateNativo>>,
                        )))
                    }
                }
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── Snackbar ────────────────────────────────────────────────
        // Barra inferior de notificación temporal
        Layout::Snackbar {
            mensaje,
            accion_texto,
            accion_callback,
            duracion: _,
            visible,
        } => {
            let show = data.leer(visible).to_string() == "true";
            if show {
                let scheme = &theme.scheme;
                let bg: Color = scheme.inverse_surface.into();
                let fg: Color = scheme.inverse_on_surface.into();
                let action_fg: Color = scheme.inverse_primary.into();
                let prog = _prog.to_vec();

                // Texto del mensaje
                let body_style = obtener_texto_style(&theme.typography, "body_medium");
                let mensaje_etiqueta = view::label(mensaje.clone())
                    .text_size(body_style.font_tamano as f32)
                    .color(fg);

                // Botón de acción (opcional)
                let tiene_action = accion_texto.is_some() && accion_callback.is_some();
                let action_btn: Option<Box<AnyWidgetView<AppStateNativo>>> = if tiene_action {
                    let at = accion_texto.clone().unwrap_or_default();
                    let cb = accion_callback.clone().unwrap_or_default();
                    let p = prog.clone();
                    Some(Box::new(view::button(
                        view::label(at.clone())
                            .text_size(14.0)
                            .weight(FontWeight::MEDIUM)
                            .color(action_fg),
                        move |data: &mut AppStateNativo| {
                            if !cb.is_empty() {
                                ejecutar_callback_y_actualizar(&cb, data, &p);
                            }
                        },
                    )) as Box<AnyWidgetView<AppStateNativo>>)
                } else {
                    None
                };

                // Construir fila: mensaje + botón
                let mut row_children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                row_children.push(Box::new(mensaje_etiqueta) as Box<AnyWidgetView<AppStateNativo>>);
                if let Some(btn) = action_btn {
                    row_children.push(Box::new(
                        view::sized_box(view::label(String::new())).width(Length::px(16.0)),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                    row_children.push(btn);
                }

                let row = view::flex(Axis::Horizontal, (row_children,)).gap(Length::px(8.0));

                let snackbar = view::sized_box(row)
                    .background(Background::Color(bg))
                    .corner_radius(obtener_forma_radius(&theme.shapes, "small").px())
                    .padding(16.0.px());

                Box::new(snackbar)
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── Tooltip ─────────────────────────────────────────────────
        // Pequeña etiqueta de ayuda contextual
        Layout::Tooltip { hijo, texto } => {
            let scheme = &theme.scheme;
            let tooltip_bg: Color = scheme.on_surface.into();
            let tooltip_fg: Color = scheme.surface.into();

            let inner = layout_a_vista(hijo, data, _prog, theme);
            let etiqueta_style = obtener_texto_style(&theme.typography, "label_small");
            let tooltip_etiqueta = view::label(texto.clone())
                .text_size(etiqueta_style.font_tamano as f32)
                .color(tooltip_fg);

            let tooltip_box = view::sized_box(tooltip_etiqueta)
                .background(Background::Color(tooltip_bg))
                .corner_radius(obtener_forma_radius(&theme.shapes, "small").px())
                .padding(8.0.px());

            // Mostrar tooltip debajo del contenido
            Box::new(view::flex(Axis::Vertical, (inner, tooltip_box)).gap(Length::px(4.0)))
        }

        // ─── Menu ────────────────────────────────────────────────────
        // Menú desplegable con lista de opciones
        Layout::Menu {
            items,
            on_select,
            visible,
        } => {
            let show = data.leer(visible).to_string() == "true";
            if show && !items.is_empty() {
                let scheme = &theme.scheme;
                let surface_bg: Color = scheme.surface.into();
                let fg: Color = scheme.on_surface.into();
                let prog = _prog.to_vec();
                let cb = on_select.clone();

                let mut menu_items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let cb_inner = cb.clone();
                    let t = item.clone();
                    let p = prog.clone();
                    let elemento_etiqueta = view::label(t.clone()).text_size(14.0).color(fg);
                    let elemento_btn =
                        view::button(elemento_etiqueta, move |data: &mut AppStateNativo| {
                            if !cb_inner.is_empty() {
                                ejecutar_callback_y_actualizar(&cb_inner, data, &p);
                            }
                        });
                    menu_items.push(Box::new(elemento_btn) as Box<AnyWidgetView<AppStateNativo>>);
                    if i < items.len() - 1 {
                        menu_items.push(Box::new(
                            view::sized_box(view::label(String::new())).height(Length::px(1.0)),
                        )
                            as Box<AnyWidgetView<AppStateNativo>>);
                    }
                }

                let menu_box =
                    view::sized_box(view::flex(Axis::Vertical, (menu_items,)).gap(Length::px(0.0)))
                        .background(Background::Color(surface_bg))
                        .corner_radius(obtener_forma_radius(&theme.shapes, "small").px());

                Box::new(menu_box)
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── ContextMenu ─────────────────────────────────────────────
        // Menú contextual con lista de opciones
        Layout::ContextMenu {
            items,
            on_select,
            visible,
        } => {
            let show = data.leer(visible).to_string() == "true";
            if show && !items.is_empty() {
                let scheme = &theme.scheme;
                let surface_bg: Color = scheme.surface.into();
                let fg: Color = scheme.on_surface.into();
                let prog = _prog.to_vec();
                let cb = on_select.clone();

                let mut menu_items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let cb_inner = cb.clone();
                    let t = item.clone();
                    let p = prog.clone();
                    let elemento_etiqueta = view::label(t.clone()).text_size(14.0).color(fg);
                    let elemento_btn =
                        view::button(elemento_etiqueta, move |data: &mut AppStateNativo| {
                            if !cb_inner.is_empty() {
                                ejecutar_callback_y_actualizar(&cb_inner, data, &p);
                            }
                        });
                    menu_items.push(Box::new(elemento_btn) as Box<AnyWidgetView<AppStateNativo>>);
                    if i < items.len() - 1 {
                        menu_items.push(Box::new(
                            view::sized_box(view::label(String::new())).height(Length::px(1.0)),
                        )
                            as Box<AnyWidgetView<AppStateNativo>>);
                    }
                }

                let menu_box =
                    view::sized_box(view::flex(Axis::Vertical, (menu_items,)).gap(Length::px(0.0)))
                        .background(Background::Color(surface_bg))
                        .corner_radius(obtener_forma_radius(&theme.shapes, "small").px());

                Box::new(menu_box)
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ═══════════════════════════════════════════════════════════
        // Navegación Material You
        // ═══════════════════════════════════════════════════════════

        // ─── Navigator (navegación por pantallas) ────────────────────
        Layout::Navigator {
            screens,
            actual_variable,
            history_var: _,
            nav_tipo,
            anim,
            on_change,
        } => crate::widgets::navigation::render_navigator(
            screens,
            actual_variable,
            nav_tipo,
            anim,
            on_change,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        // ─── NavigationBar ───────────────────────────────────────────
        Layout::NavigationBar {
            items,
            seleccion,
            variable_seleccion,
            on_change,
        } => crate::widgets::navigation::render_navigation_bar(
            items,
            seleccion,
            variable_seleccion.as_deref(),
            on_change,
            data,
            _prog,
            theme,
        ),

        // ─── NavigationRail ──────────────────────────────────────────
        Layout::NavigationRail {
            items,
            seleccion,
            variable_seleccion,
            on_change,
            extended,
        } => crate::widgets::navigation::render_navigation_rail(
            items,
            seleccion,
            variable_seleccion.as_deref(),
            on_change,
            extended,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        // ─── NavigationDrawer ────────────────────────────────────────
        Layout::NavigationDrawer {
            items,
            seleccion,
            variable_seleccion,
            on_change,
            modal,
            visible,
        } => crate::widgets::navigation::render_navigation_drawer(
            items,
            seleccion,
            variable_seleccion.as_deref(),
            on_change,
            modal,
            visible,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        // ─── TopAppBar ───────────────────────────────────────────────
        Layout::TopAppBar {
            titulo,
            acciones,
            menu_visible: _,
            variant: _,
        } => crate::widgets::navigation::render_top_app_bar(titulo, acciones, data, _prog, theme),

        // ─── BottomAppBar ────────────────────────────────────────────
        Layout::BottomAppBar { acciones, fab } => {
            crate::widgets::navigation::render_bottom_app_bar(acciones, fab, data, _prog, theme)
        }

        // ─── Tabs ────────────────────────────────────────────────────
        Layout::Tabs {
            tabs,
            seleccion,
            on_change,
            scrollable: _,
        } => {
            crate::widgets::navigation::render_tabs(tabs, seleccion, on_change, data, _prog, theme)
        }

        // ─── SearchBar ───────────────────────────────────────────────
        Layout::BarraBusqueda {
            placeholder,
            on_buscar: _,
            variable,
        } => {
            crate::widgets::navigation::render_buscar_bar(placeholder, variable, data, _prog, theme)
        }

        // ─── SearchView ──────────────────────────────────────────────
        Layout::VistaBusqueda {
            query: _,
            resultados,
            visible,
        } => {
            crate::widgets::navigation::render_buscar_vista(resultados, visible, data, _prog, theme)
        }

        // ═══════════════════════════════════════════════════════════════
        // INDICADORES (Fase 7)
        // ═══════════════════════════════════════════════════════════════

        // ─── LinearProgress ──────────────────────────────────────────
        Layout::LinearProgress {
            variable,
            indeterminado,
        } => crate::widgets::indicators::render_linear_progress(
            variable,
            indeterminado,
            data,
            _prog,
            theme,
        ),

        // ─── LinearProgressValue: barra con valor directo (items de lista) ──
        Layout::ValorProgresoLineal(val) => {
            crate::widgets::indicators::render_linear_progress_valor(val, theme)
        }

        // ─── CircularProgress ────────────────────────────────────────
        Layout::CircularProgress {
            variable,
            tamano,
            indeterminado,
        } => crate::widgets::indicators::render_circular_progress(
            variable,
            tamano,
            indeterminado,
            data,
            _prog,
            theme,
        ),

        // ─── Badge ──────────────────────────────────────────────────
        Layout::Badge { hijo, valor, dot } => {
            crate::widgets::indicators::render_badge(hijo, valor, dot, data, _prog, theme)
        }

        // ─── Skeleton ───────────────────────────────────────────────
        Layout::Skeleton { ancho, alto, tipo } => {
            crate::widgets::indicators::render_skeleton(ancho, alto, tipo, theme)
        }

        // ─── EmptyState ─────────────────────────────────────────────
        Layout::EmptyState {
            icono,
            mensaje,
            accion_texto,
            accion_cb,
        } => crate::widgets::indicators::render_empty_state(
            icono,
            mensaje,
            accion_texto,
            accion_cb,
            data,
            _prog,
            theme,
        ),

        // ─── ErrorState ─────────────────────────────────────────────
        Layout::ErrorState { mensaje, on_retry } => {
            crate::widgets::indicators::render_error_state(mensaje, on_retry, data, _prog, theme)
        }

        // ═══════════════════════════════════════════════════════════════
        // AVATARES (Fase 7)
        // ═══════════════════════════════════════════════════════════════

        // ─── Avatar ─────────────────────────────────────────────────
        Layout::Avatar {
            texto,
            variant,
            tamaño,
        } => crate::widgets::indicators::render_avatar(texto, variant, tamaño, theme),

        // ─── AvatarGroup ────────────────────────────────────────────
        Layout::AvatarGroup { avatares, max } => {
            crate::widgets::indicators::render_avatar_group(avatares, max, theme)
        }

        // ═══════════════════════════════════════════════════════════════
        // MOTION (Fase 8)
        // ═══════════════════════════════════════════════════════════════

        // ─── FadeTransition ──────────────────────────────────────────
        // Fundido de entrada/salida con opacidad animada
        Layout::FadeTransition {
            hijo,
            visible,
            duracion,
        } => {
            let show = data.leer(visible).to_string() == "true";
            data.anim_visibility_changed(visible, show);
            let d = if *duracion > 0.0 { *duracion } else { 200.0 };
            let progress = data.anim_progress(visible, d);
            let eased = EASE_STANDARD.aplicar(progress);

            if show || progress > 0.0 {
                let inner = layout_a_vista(hijo, data, _prog, theme);
                // Fade mediante opacidad del overlay
                let overlay_alpha = 1.0 - eased;
                if overlay_alpha < 0.01 {
                    inner
                } else {
                    Box::new(view::sized_box(inner).background(Background::Color(
                        RgbColor(0, 0, 0).con_alpha(overlay_alpha),
                    )))
                }
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── RippleEffect ───────────────────────────────────────────
        // Efecto de onda expansiva Material Design
        // NOTA: La integración completa requiere un View Xilem personalizado.
        // El RippleWidget de animated_widgets estará disponible para uso
        // directo con Pod cuando se requiera desde código Masonry nativo.
        Layout::RippleEffect { hijo, color: _ } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            Box::new(
                view::sized_box(inner)
                    .corner_radius(obtener_forma_radius(&theme.shapes, "extra_small").px()),
            )
        }

        // ─── PullToRefresh ────────────────────────────────────────────
        Layout::PullToRefresh {
            hijo,
            callback,
            refreshing,
        } => {
            let es_refreshing = data.leer(refreshing).to_bool();
            let prog = _prog.to_vec();
            let cb = callback.clone();
            let ref_var = refreshing.clone();

            let inner = layout_a_vista(hijo, data, _prog, theme);

            if es_refreshing {
                // Si está refrescando, mostrar spinner real + texto
                let fg: Color = theme.scheme.on_surface.into();
                let refresh_row = view::flex(
                    Axis::Horizontal,
                    (vec![
                        Box::new(
                            view::sized_box(view::spinner())
                                .dims((Length::px(18.0), Length::px(18.0))),
                        ) as Box<AnyWidgetView<AppStateNativo>>,
                        Box::new(view::label("Actualizando...").text_size(14.0).color(fg)),
                    ],),
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(Length::px(8.0));
                let header = Box::new(view::sized_box(refresh_row).padding(8.0.px()))
                    as Box<AnyWidgetView<AppStateNativo>>;
                let column_children: Vec<Box<AnyWidgetView<AppStateNativo>>> = vec![header, inner];
                Box::new(view::portal(
                    view::flex(Axis::Vertical, (column_children,)).gap(Length::px(4.0)),
                ))
            } else {
                // Envolver en PullToRefreshView con detección de gesto real
                let view = PullToRefreshVista::nuevo(inner, cb, ref_var, prog);
                Box::new(view)
            }
        }

        // ─── SwipeToDismiss ───────────────────────────────────────────
        Layout::SwipeToDismiss {
            hijo,
            on_dismiss,
            etiqueta,
            dismissed,
        } => {
            let es_dismissed = data.leer(dismissed).to_bool();
            let prog = _prog.to_vec();
            let cb = on_dismiss.clone();
            let dismiss_var = dismissed.clone();
            let action_texto = etiqueta.clone();

            if es_dismissed {
                // Ya descartado: mostrar opción de deshacer
                let scheme = &theme.scheme;
                let fg: Color = scheme.on_surface.into();
                let btn_fg: Color = scheme.primary.into();
                let undo_etiqueta = view::label("Elemento descartado").text_size(14.0).color(fg);
                let undo_btn = view::button(
                    view::label(action_texto.clone())
                        .text_size(14.0)
                        .weight(FontWeight::MEDIUM)
                        .color(btn_fg),
                    move |data: &mut AppStateNativo| {
                        data.escribir(&dismiss_var, ValorGUI::Booleano(false));
                    },
                );
                let row_children: Vec<Box<AnyWidgetView<AppStateNativo>>> = vec![
                    Box::new(undo_etiqueta) as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(view::sized_box(view::label(String::new())).width(Length::px(8.0)))
                        as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(undo_btn) as Box<AnyWidgetView<AppStateNativo>>,
                ];
                Box::new(
                    view::sized_box(
                        view::flex(Axis::Horizontal, (row_children,)).gap(Length::px(4.0)),
                    )
                    .padding(12.0.px())
                    .corner_radius(obtener_forma_radius(&theme.shapes, "small").px()),
                )
            } else {
                // Mostrar contenido envuelto en SwipeToDismissView con gesto real
                let inner = layout_a_vista(hijo, data, _prog, theme);
                let view = SwipeToDismissVista::nuevo(inner, cb, dismiss_var, action_texto, prog);
                Box::new(view)
            }
        }

        // ─── PinchZoom ──────────────────────────────────────────────
        Layout::PinchZoom {
            hijo,
            min_scale,
            max_scale,
        } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let view = PinchZoomVista::nuevo(inner, *min_scale, *max_scale);
            Box::new(view)
        }

        // ─── Rotate ─────────────────────────────────────────────────
        Layout::Rotate { hijo } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let view = RotateVista::nuevo(inner);
            Box::new(view)
        }

        // ═══════════════════════════════════════════════════════════════
        // GRÁFICOS VECTORIALES (Fase 9) — Vello Scene
        // ═══════════════════════════════════════════════════════════════
        //
        // Reemplaza la emulación anterior (barras flex, puntos Unicode,
        // leyendas de texto) por dibujo vectorial real con Vello Scene.
        // Usa widgets Masonry personalizados con View wrappers Xilem.

        // ─── LineChart ────────────────────────────────────────────────
        Layout::GraficoLineas {
            datos,
            color,
            etiquetas,
        } => {
            let c = color_role_to_rgba(&theme.scheme, color);
            let puntos: Vec<ChartDataPoint> = datos
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let etiqueta = etiquetas.get(i).cloned().unwrap_or_default();
                    ChartDataPoint::nuevo(&etiqueta, v, c)
                })
                .collect();
            Box::new(LineaChartVista::<AppStateNativo>::nuevo(puntos))
        }

        // ─── BarChart ────────────────────────────────────────────────
        Layout::BarChart {
            datos,
            colores,
            etiquetas,
            apilado,
        } => {
            let colores_ciclo = [
                theme.scheme.primary,
                theme.scheme.secondary,
                theme.scheme.tertiary,
                theme.scheme.error,
                theme.scheme.primary_container,
                theme.scheme.secondary_container,
            ];
            let puntos: Vec<ChartDataPoint> = datos
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let c_str = colores.get(i).cloned().unwrap_or_default();
                    let c = if c_str.is_empty() {
                        let rgb = colores_ciclo[i % colores_ciclo.len()];
                        Rgba::nuevo(rgb.0, rgb.1, rgb.2, 255)
                    } else {
                        color_role_to_rgba(&theme.scheme, &c_str)
                    };
                    let etiqueta = etiquetas.get(i).cloned().unwrap_or_default();
                    ChartDataPoint::nuevo(&etiqueta, v, c)
                })
                .collect();
            Box::new(BarChartVista::<AppStateNativo>::nuevo(puntos, *apilado))
        }

        // ─── PieChart / Donut ─────────────────────────────────────────
        Layout::PieChart {
            datos,
            etiquetas,
            donut,
        } => {
            let colores_pie = [
                theme.scheme.primary,
                theme.scheme.secondary,
                theme.scheme.tertiary,
                theme.scheme.error,
                theme.scheme.primary_container,
                theme.scheme.secondary_container,
                theme.scheme.tertiary_container,
                theme.scheme.error_container,
            ];
            let puntos: Vec<ChartDataPoint> = datos
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let rgb = colores_pie[i % colores_pie.len()];
                    let c = Rgba::nuevo(rgb.0, rgb.1, rgb.2, 255);
                    let etiqueta = etiquetas.get(i).cloned().unwrap_or_default();
                    ChartDataPoint::nuevo(&etiqueta, v, c)
                })
                .collect();
            Box::new(PieChartVista::<AppStateNativo>::nuevo(puntos, *donut, 0.5))
        }

        // ─── GaugeChart ──────────────────────────────────────────────
        Layout::GaugeChart {
            valor,
            min: _,
            max,
            color: _,
        } => {
            let threshold = *max * 0.75;
            Box::new(GaugeChartVista::<AppStateNativo>::nuevo(
                *valor, *max, threshold,
            ))
        }

        // ─── Sparkline ───────────────────────────────────────────────
        Layout::Sparkline { datos, color } => {
            let c = color_role_to_rgba(&theme.scheme, color);
            Box::new(SparklineVista::<AppStateNativo>::nuevo(datos.clone(), c))
        }

        // ═══════════════════════════════════════════════════════════════
        // AVANZADOS (Fase 9)
        // ═══════════════════════════════════════════════════════════════

        // ─── StarRating ──────────────────────────────────────────────
        Layout::StarRating {
            valor,
            variable,
            max,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb = callback.clone();
            let valor_actual = match variable.as_deref() {
                Some(nombre) => data.leer(nombre).to_f64().max(0.0) as usize,
                None => *valor,
            };
            let stars: Vec<Box<AnyWidgetView<AppStateNativo>>> = (0..*max)
                .map(|i| {
                    let llena = i < valor_actual;
                    let cb_name = cb.clone();
                    let p = prog.clone();
                    let star_color: Color = if llena {
                        scheme.primary.into()
                    } else {
                        scheme.on_surface_variant.into()
                    };
                    // La estrella se pinta como path: la fuente por defecto no
                    // trae los glifos ★/☆ y se veían cuadraditos.
                    let btn = view::button(
                        crate::star_widget::estrella::<AppStateNativo>(24.0, star_color, llena),
                        move |data: &mut AppStateNativo| {
                            data.escribir("rating", ValorGUI::Entero((i + 1) as i64));
                            if !cb_name.is_empty() {
                                ejecutar_callback_con_valor(
                                    &cb_name,
                                    Some(ValorGUI::Texto((i + 1).to_string())),
                                    data,
                                    &p,
                                );
                            }
                        },
                    );
                    Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>
                })
                .collect();
            Box::new(view::flex(Axis::Horizontal, (stars,)).gap(Length::px(4.0)))
        }

        // ─── Stepper ─────────────────────────────────────────────────
        Layout::Stepper {
            pasos,
            actual,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb = callback.clone();
            let steps: Vec<Box<AnyWidgetView<AppStateNativo>>> = pasos
                .iter()
                .enumerate()
                .map(|(i, paso)| {
                    let active = i == *actual;
                    let completed = i < *actual;
                    let paso_valor = paso.clone();
                    let num_color: Color = if active {
                        scheme.on_primary.into()
                    } else if completed {
                        scheme.on_primary_container.into()
                    } else {
                        scheme.on_surface_variant.into()
                    };
                    let bg_color = if active {
                        scheme.primary.into()
                    } else if completed {
                        scheme.primary_container.into()
                    } else {
                        scheme.surface_variant.into()
                    };
                    let numero_texto = if completed {
                        "✓".to_string()
                    } else {
                        (i + 1).to_string()
                    };
                    let num: Box<AnyWidgetView<AppStateNativo>> = if completed {
                        Box::new(view::sized_box(crate::svg_icon::<AppStateNativo>(
                            "check",
                            16.0,
                            scheme.on_primary_container,
                            crate::IconStyle::Filled,
                        )))
                    } else {
                        Box::new(
                            view::label(numero_texto)
                                .text_size(14.0)
                                .weight(FontWeight::BOLD)
                                .color(num_color),
                        )
                    };
                    let circle = view::sized_box(num)
                        .dims((Length::px(32.0), Length::px(32.0)))
                        .background(Background::Color(bg_color))
                        .corner_radius(obtener_forma_radius(&theme.shapes, "small").px());
                    let etiqueta = view::label(paso.clone())
                        .text_size(12.0)
                        .color(scheme.on_surface.into());
                    let cb_name = cb.clone();
                    let p = prog.clone();
                    let btn = view::button(
                        view::flex(
                            Axis::Vertical,
                            (
                                Box::new(circle) as Box<AnyWidgetView<AppStateNativo>>,
                                Box::new(etiqueta) as Box<AnyWidgetView<AppStateNativo>>,
                            ),
                        )
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .gap(Length::px(4.0)),
                        move |data: &mut AppStateNativo| {
                            data.escribir("step", ValorGUI::Entero(i as i64));
                            if !cb_name.is_empty() {
                                ejecutar_callback_con_valor(
                                    &cb_name,
                                    Some(ValorGUI::Texto(paso_valor.clone())),
                                    data,
                                    &p,
                                );
                            }
                        },
                    );
                    Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>
                })
                .collect();
            Box::new(
                view::flex(Axis::Horizontal, (steps,))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(Length::px(16.0)),
            )
        }

        // ─── Breadcrumbs ─────────────────────────────────────────────
        Layout::Breadcrumbs { items, separador } => {
            let scheme = &theme.scheme;
            let crumbs: Vec<Box<AnyWidgetView<AppStateNativo>>> = items
                .iter()
                .enumerate()
                .flat_map(|(i, item)| {
                    let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                    if i > 0 {
                        let sep = view::label(separador.clone())
                            .text_size(16.0)
                            .color(scheme.on_surface_variant.into());
                        widgets.push(Box::new(sep) as Box<AnyWidgetView<AppStateNativo>>);
                    }
                    let es_ultimo = i == items.len() - 1;
                    let lbl = view::label(item.clone())
                        .text_size(14.0)
                        .weight(if es_ultimo {
                            FontWeight::BOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .color(if es_ultimo {
                            scheme.on_surface.into()
                        } else {
                            scheme.primary.into()
                        });
                    widgets.push(Box::new(lbl) as Box<AnyWidgetView<AppStateNativo>>);
                    widgets
                })
                .collect();
            Box::new(
                view::flex(Axis::Horizontal, (crumbs,))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(Length::px(4.0)),
            )
        }

        // ─── Calendar (con chrono) ──────────────────────────────────────
        Layout::Calendar {
            mes,
            año,
            seleccionado,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb_name = callback.clone();
            let mes_val = *mes;
            let año_val = *año;
            let fecha_sel = seleccionado.clone().unwrap_or_default();

            // Usar chrono para calcular datos reales del mes
            let primero_date = chrono::NaiveDate::from_ymd_opt(año_val, mes_val as u32, 1)
                .unwrap_or_else(|| {
                    let today = chrono::Local::now().naive_local().date();
                    chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap()
                });
            let days_in_month = if mes_val == 12 {
                chrono::NaiveDate::from_ymd_opt(año_val + 1, 1, 1)
                    .and_then(|d| d.pred_opt())
                    .map(|d| d.day())
                    .unwrap_or(31)
            } else {
                chrono::NaiveDate::from_ymd_opt(año_val, mes_val as u32 + 1, 1)
                    .and_then(|d| d.pred_opt())
                    .map(|d| d.day())
                    .unwrap_or(30)
            };
            let primero_weekday = primero_date.weekday().num_days_from_monday() as usize; // 0=Lun, 6=Dom
            let num_dias = days_in_month as usize;

            // Nombres de meses
            let nombres_mes = [
                "Enero",
                "Febrero",
                "Marzo",
                "Abril",
                "Mayo",
                "Junio",
                "Julio",
                "Agosto",
                "Septiembre",
                "Octubre",
                "Noviembre",
                "Diciembre",
            ];
            let mes_nombre = nombres_mes.get(mes_val as usize - 1).unwrap_or(&"");
            let title = view::label(format!("{} {}", mes_nombre, año_val))
                .text_size(16.0)
                .weight(FontWeight::BOLD)
                .color(scheme.on_surface.into());

            // Botones de navegación de meses
            let _anterior_cb = cb_name.clone();
            let _prog_anterior = prog.clone();
            let anterior_btn = view::button(
                view::label("◀")
                    .text_size(14.0)
                    .color(scheme.primary.into()),
                move |data: &mut AppStateNativo| {
                    // La navegación requiere reconstruir la vista; se maneja
                    // cambiando las variables de estado. Por ahora placeholder.
                    data.escribir("calendar_accion", ValorGUI::Texto("prev".to_string()));
                },
            );
            let _next_cb = cb_name.clone();
            let _prog_next = prog.clone();
            let next_btn = view::button(
                view::label("▶")
                    .text_size(14.0)
                    .color(scheme.primary.into()),
                move |data: &mut AppStateNativo| {
                    data.escribir("calendar_accion", ValorGUI::Texto("next".to_string()));
                },
            );
            let header_row = view::flex(
                Axis::Horizontal,
                (
                    Box::new(anterior_btn) as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(title) as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(next_btn) as Box<AnyWidgetView<AppStateNativo>>,
                ),
            )
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(Length::px(12.0));

            // Días de la semana header (L M M J V S D)
            let dias_semana = ["L", "M", "M", "J", "V", "S", "D"];
            let header_dias: Vec<Box<AnyWidgetView<AppStateNativo>>> = dias_semana
                .iter()
                .map(|d| {
                    Box::new(
                        view::sized_box(
                            view::label(d.to_string())
                                .text_size(11.0)
                                .weight(FontWeight::BOLD)
                                .color(scheme.on_surface_variant.into()),
                        )
                        .dims((Length::px(36.0), Length::px(36.0))),
                    ) as Box<AnyWidgetView<AppStateNativo>>
                })
                .collect();
            let day_header = view::flex(Axis::Horizontal, (header_dias,)).gap(Length::px(2.0));

            // Generar grilla de días con datos reales
            let total_cells = primero_weekday + num_dias;
            let num_rows = total_cells.div_ceil(7);
            let mut all_rows: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            all_rows.push(Box::new(header_row) as Box<AnyWidgetView<AppStateNativo>>);
            all_rows.push(Box::new(day_header) as Box<AnyWidgetView<AppStateNativo>>);

            for row in 0..num_rows {
                let mut day_cells: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                for col in 0..7 {
                    let cell_idx = row * 7 + col;
                    if cell_idx < primero_weekday || cell_idx >= primero_weekday + num_dias {
                        day_cells.push(Box::new(
                            view::sized_box(view::label(String::new()))
                                .dims((Length::px(36.0), Length::px(36.0))),
                        )
                            as Box<AnyWidgetView<AppStateNativo>>);
                    } else {
                        let dia = (cell_idx - primero_weekday + 1) as u32;
                        let cb = cb_name.clone();
                        let p = prog.clone();
                        let fecha_str = format!("{:04}-{:02}-{:02}", año_val, mes_val, dia);
                        let es_selected = fecha_sel == fecha_str;

                        if es_selected {
                            let lbl = view::label(dia.to_string())
                                .text_size(12.0)
                                .weight(FontWeight::BOLD)
                                .color(scheme.on_surface.into());
                            day_cells.push(Box::new(
                                view::sized_box(lbl)
                                    .dims((Length::px(36.0), Length::px(36.0)))
                                    .background(Background::Color(scheme.primary_container.into()))
                                    .corner_radius(
                                        obtener_forma_radius(&theme.shapes, "full").px(),
                                    ),
                            )
                                as Box<AnyWidgetView<AppStateNativo>>);
                        } else {
                            let btn = view::button(
                                view::label(dia.to_string())
                                    .text_size(12.0)
                                    .color(scheme.on_surface.into()),
                                move |data: &mut AppStateNativo| {
                                    data.escribir("fecha", ValorGUI::Texto(fecha_str.clone()));
                                    if !cb.is_empty() {
                                        ejecutar_callback_con_valor(
                                            &cb,
                                            Some(ValorGUI::Texto(fecha_str.clone())),
                                            data,
                                            &p,
                                        );
                                    }
                                },
                            );
                            day_cells.push(Box::new(
                                view::sized_box(btn).dims((Length::px(36.0), Length::px(36.0))),
                            )
                                as Box<AnyWidgetView<AppStateNativo>>);
                        }
                    }
                }
                all_rows.push(Box::new(
                    view::flex(Axis::Horizontal, (day_cells,)).gap(Length::px(2.0)),
                ) as Box<AnyWidgetView<AppStateNativo>>);
            }

            Box::new(
                view::flex(Axis::Vertical, (all_rows,))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(Length::px(4.0)),
            )
        }

        // ─── MarkdownViewer ──────────────────────────────────────────
        Layout::MarkdownViewer { texto } => {
            let scheme = &theme.scheme;
            // Renderizado simplificado: reemplaza # y ** para simular formato
            let lineas: Vec<&str> = texto.split('\n').collect();
            let mut children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for line in lineas {
                let txt = line.to_string();
                if txt.starts_with("# ") {
                    let content = txt.trim_start_matches("# ");
                    children.push(Box::new(
                        view::label(content.to_string())
                            .text_size(24.0)
                            .weight(FontWeight::BOLD)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else if txt.starts_with("## ") {
                    let content = txt.trim_start_matches("## ");
                    children.push(Box::new(
                        view::label(content.to_string())
                            .text_size(20.0)
                            .weight(FontWeight::BOLD)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else if txt.starts_with("### ") {
                    let content = txt.trim_start_matches("### ");
                    children.push(Box::new(
                        view::label(content.to_string())
                            .text_size(16.0)
                            .weight(FontWeight::BOLD)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else if txt.starts_with("- ") || txt.starts_with("* ") {
                    let content = txt.trim_start_matches("- ").trim_start_matches("* ");
                    children.push(Box::new(
                        view::label(format!("  • {}", content))
                            .text_size(14.0)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else if txt.trim().is_empty() {
                    children.push(Box::new(
                        view::sized_box(view::label(String::new())).height(Length::px(8.0)),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else {
                    // Bold y cursiva simulados
                    let processed = txt.replace("**", "").replace("*", "").replace("__", "");
                    children.push(Box::new(
                        view::label(processed)
                            .text_size(14.0)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                }
            }
            Box::new(view::flex(Axis::Vertical, (children,)).gap(Length::px(4.0)))
        }

        // ─── QRCode (real) ───────────────────────────────────────────
        Layout::QRCode { texto, tamaño } => {
            let t = *tamaño;
            let content = if texto.len() > 30 {
                format!("{}…", &texto[..30])
            } else {
                texto.clone()
            };
            // Usar QRView/widget real con qrcode crate + Vello
            let qr = crate::qr_widget::qr_vista::<AppStateNativo>(texto, t);
            let lbl = view::label(content)
                .text_size(10.0)
                .color(theme.scheme.on_surface.into());
            Box::new(
                view::flex(
                    Axis::Vertical,
                    (
                        Box::new(qr) as Box<AnyWidgetView<AppStateNativo>>,
                        Box::new(lbl) as Box<AnyWidgetView<AppStateNativo>>,
                    ),
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(Length::px(8.0)),
            )
        }

        // ─── FilePicker (con rfd) ─────────────────────────────────────
        Layout::SelectorArchivo {
            tipos,
            multiple: _,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb = callback.clone();
            // Clonar a owned: el closure del botón debe ser 'static y no puede
            // capturar la referencia 'a del layout.
            let tipos_filtro = tipos.clone();
            let btn = view::button(
                view::flex(
                    Axis::Horizontal,
                    (
                        Box::new(crate::svg_icon::<AppStateNativo>(
                            "folder",
                            18.0,
                            scheme.on_surface,
                            crate::IconStyle::Filled,
                        )) as Box<AnyWidgetView<AppStateNativo>>,
                        Box::new(
                            view::label("Seleccionar archivo")
                                .text_size(14.0)
                                .weight(FontWeight::MEDIUM),
                        ) as Box<AnyWidgetView<AppStateNativo>>,
                    ),
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(Length::px(8.0)),
                move |data: &mut AppStateNativo| {
                    // Ruta elegida, o `None` si el usuario cancela el diálogo.
                    // Usar rfd para diálogo nativo de selección de archivos
                    #[cfg(not(target_os = "android"))]
                    let archivo_elegido: Option<String> = {
                        let mut dialog = rfd::FileDialog::new();
                        if !tipos_filtro.is_empty() {
                            dialog = dialog.add_filter(
                                "Archivos",
                                &tipos_filtro
                                    .iter()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<&str>>(),
                            );
                        }
                        dialog
                            .set_directory(".")
                            .pick_file()
                            .map(|path| path.to_string_lossy().to_string())
                    };
                    #[cfg(target_os = "android")]
                    let archivo_elegido: Option<String> = {
                        // Sin diálogo nativo en Android: simulamos la selección.
                        // Si se configuraron filtros, usar el primero como extensión
                        // (uso legítimo de tipos_filtro en ambos cfg).
                        Some(if tipos_filtro.is_empty() {
                            "archivo_seleccionado.txt".to_string()
                        } else {
                            format!("archivo_seleccionado.{}", tipos_filtro[0].replace('.', ""))
                        })
                    };
                    if let Some(ruta) = archivo_elegido {
                        data.escribir("archivo", ValorGUI::Texto(ruta.clone()));
                        if !cb.is_empty() {
                            ejecutar_callback_con_valor(
                                &cb,
                                Some(ValorGUI::Texto(ruta)),
                                data,
                                &prog,
                            );
                        }
                    }
                },
            );
            Box::new(
                view::sized_box(btn)
                    .corner_radius(obtener_forma_radius(&theme.shapes, "small").px())
                    .background(Background::Color(scheme.primary_container.into()))
                    .border_color(scheme.primary.into())
                    .border_width(1.0.px()),
            )
        }

        // ═══════════════════════════════════════════════════════════════
        // EXPRESSIVE (Fase 10)
        // ═══════════════════════════════════════════════════════════════

        // ─── GlassCard ───────────────────────────────────────────────
        Layout::GlassCard {
            hijo,
            blur: _,
            opacity,
        } => {
            let _scheme = &theme.scheme;
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // Glassmorphism simulado con alpha blending
            // with_alpha() ya retorna xilem::Color (alias Color)
            let bg_color = RgbColor(255, 255, 255).con_alpha(*opacity);
            let border_color = RgbColor(255, 255, 255).con_alpha(0.3);
            Box::new(
                view::sized_box(inner)
                    .background(Background::Color(bg_color))
                    .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px())
                    .border_color(border_color)
                    .border_width(1.0.px()),
            )
        }

        // ─── GradientBox ─────────────────────────────────────────────
        Layout::GradientBox {
            hijo,
            colores,
            direccion,
        } => {
            let scheme = &theme.scheme;
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // Gradiente real con Background::Gradient de Masonry
            let gradient = if colores.len() >= 2 {
                // Convertir colores a xilem::Color (AlphaColor<Srgb>)
                let color_vec: Vec<Color> = colores
                    .iter()
                    .map(|c| {
                        let rgb = parsear_color(c).unwrap_or(scheme.primary);
                        rgb.into()
                    })
                    .collect();
                // Crear color stops equiespaciados: [(0.0, c1), (1.0, c2), ...]
                let total = color_vec.len();
                let stops: Vec<(f32, Color)> = color_vec
                    .into_iter()
                    .enumerate()
                    .map(|(i, clr)| {
                        let offset = if total > 1 {
                            i as f32 / (total - 1) as f32
                        } else {
                            0.0
                        };
                        (offset, clr)
                    })
                    .collect();
                match direccion.to_lowercase().as_str() {
                    "radial" => {
                        xilem::style::Gradient::new_radial(crate::layout::UnitPoint::CENTER)
                            .with_stops(stops.as_slice())
                    }
                    "sweep" => xilem::style::Gradient::new_full_sweep(
                        crate::layout::UnitPoint::CENTER,
                        0.0,
                    )
                    .with_stops(stops.as_slice()),
                    _ => xilem::style::Gradient::new_linear(std::f64::consts::FRAC_PI_4)
                        .with_stops(stops.as_slice()),
                }
            } else if colores.len() == 1 {
                let c = parsear_color(&colores[0]).unwrap_or(scheme.primary);
                let color: Color = c.into();
                // Gradiente lineal simple con un solo color (fondo sólido)
                xilem::style::Gradient::new_linear(0.0)
                    .with_stops([(0.0, color), (1.0, color)].as_slice())
            } else {
                let c: Color = scheme.primary.into();
                xilem::style::Gradient::new_linear(0.0).with_stops([(0.0, c), (1.0, c)].as_slice())
            };
            Box::new(
                view::sized_box(inner)
                    .background(Background::Gradient(gradient))
                    .corner_radius(obtener_forma_radius(&theme.shapes, "container").px()),
            )
        }

        // ─── MorphingButton ──────────────────────────────────────────
        Layout::MorphingButton {
            icono,
            texto_extendido,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb = callback.clone();
            let icon = icono.clone();
            let txt = texto_extendido.clone();
            let btn = view::button(
                view::flex(
                    Axis::Horizontal,
                    (
                        Box::new(view::label(icon).text_size(20.0))
                            as Box<AnyWidgetView<AppStateNativo>>,
                        Box::new(view::label(txt).text_size(14.0).weight(FontWeight::MEDIUM))
                            as Box<AnyWidgetView<AppStateNativo>>,
                    ),
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(Length::px(8.0)),
                move |data: &mut AppStateNativo| {
                    if !cb.is_empty() {
                        ejecutar_callback_y_actualizar(&cb, data, &prog);
                    }
                },
            );
            Box::new(
                view::sized_box(btn)
                    .corner_radius(obtener_forma_radius(&theme.shapes, "button").px())
                    .background(Background::Color(scheme.primary_container.into()))
                    .border_color(scheme.primary.into())
                    .border_width(2.0.px()),
            )
        }

        // ─── ExpressiveBackground ────────────────────────────────────
        Layout::ExpressiveBackground {
            colores,
            animado: _,
        } => {
            let scheme = &theme.scheme;
            let bg_color = if !colores.is_empty() {
                let c = parsear_color(&colores[0]).unwrap_or(scheme.primary);
                c.into()
            } else {
                scheme.primary_container.into()
            };
            // Layout de fondo expansivo con gradiente simulado (múltiples capas)
            let base = view::sized_box(view::label(String::new()))
                .dims((Length::px(300.0), Length::px(120.0)))
                .background(Background::Color(bg_color))
                .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px());
            if colores.len() > 1 {
                let c2 = parsear_color(&colores[1]).unwrap_or(scheme.secondary);
                let overlay = view::sized_box(view::label(String::new()))
                    .dims((Length::px(300.0), Length::px(60.0)))
                    .background(Background::Color(c2.into()))
                    .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px());
                Box::new(view::zstack((
                    Box::new(base) as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(overlay) as Box<AnyWidgetView<AppStateNativo>>,
                )))
            } else {
                Box::new(base)
            }
        }

        // ─── GlowBorder ──────────────────────────────────────────────
        Layout::GlowBorder { hijo, color, ancho } => {
            let scheme = &theme.scheme;
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let glow_color: Color = obtener_color_role(scheme, color).into();
            Box::new(
                view::sized_box(inner)
                    .border_color(glow_color)
                    .border_width((*ancho).px())
                    .corner_radius(obtener_forma_radius(&theme.shapes, "container").px()),
            )
        }

        // ─── Canvas / Lienzo de dibujo libre ───────────────────────────
        Layout::Canvas {
            commands_var,
            width,
            height,
        } => {
            // Leer comandos de la variable si existe
            let commands_json = data.leer(commands_var).to_string();
            let view = if commands_json != "null" && !commands_json.is_empty() {
                crate::canvas_widget::CanvasVista::<AppStateNativo>::nuevo(
                    commands_var.clone(),
                    commands_json,
                    *width,
                    *height,
                )
            } else {
                crate::canvas_widget::CanvasVista::<AppStateNativo>::empty(
                    commands_var.clone(),
                    *width,
                    *height,
                )
            };
            Box::new(view)
        }

        // ═══════════════════════════════════════════════════════════════════
        // ICONOS MATERIAL DESIGN — renderizado SVG con Vello
        // ═══════════════════════════════════════════════════════════════════

        // MaterialIconLayout: icono vectorial con tamaño, color y estilo
        // Renderiza paths SVG de Material Design directamente en el Scene de Vello,
        // usando el view wrapper SvgIconView.
        Layout::MaterialIconLayout {
            nombre,
            tamaño,
            color,
            estilo,
        } => {
            let scheme = &theme.scheme;
            // El color sale del tema activo: primero roles del esquema (primary,
            // on_surface_variant, ...), después hex o nombre de color estándar.
            // `parse_color` solo se usa como último recurso porque devuelve
            // valores fijos de Material You.
            let icon_color: RgbColor = color_de_rol(scheme, color)
                .or_else(|| parsear_color(color))
                .unwrap_or(scheme.primary);
            let icon_tamano = *tamaño;
            let icon_estilo = IconStyle::desde_str(estilo);

            // Si el nombre no está en el catálogo, mostrar el nombre en texto:
            // un icono inexistente tiene que verse, no desaparecer.
            if crate::icons::catalog::by_name(nombre).is_none() {
                return Box::new(
                    view::label(format!("[{}]", nombre))
                        .text_size((icon_tamano * 0.5) as f32)
                        .color(icon_color.into()),
                );
            }

            // Crear el view wrapper del icono SVG (renderiza paths reales de Vello)
            Box::new(svg_icon::<AppStateNativo>(
                nombre,
                icon_tamano,
                icon_color,
                icon_estilo,
            ))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Window Width Observer — detección de resize vía widget personalizado
// ═══════════════════════════════════════════════════════════════════

use std::cell::Cell;
use std::marker::PhantomData;

// Almacena el ancho y alto actuales de la ventana para que la función de vista lo lea.
thread_local! {
    static ANCHO_VENTANA_TL: Cell<f64> = const { Cell::new(800.0) };
    static ALTO_VENTANA_TL: Cell<f64> = const { Cell::new(600.0) };
}

/// Obtiene el ancho de ventana almacenado en el thread-local.
pub fn actual_ventana_ancho() -> f64 {
    ANCHO_VENTANA_TL.with(|w| w.get())
}

/// Obtiene el alto de ventana almacenado en el thread-local.
pub fn actual_ventana_alto() -> f64 {
    ALTO_VENTANA_TL.with(|h| h.get())
}

/// Widget Masonry que observa el ancho disponible durante el layout
/// y lo almacena en el thread-local para que la vista Xilem lo lea.
pub struct VentanaAnchoProbe {
    hijo: Option<WidgetPod<dyn Widget>>,
}

impl VentanaAnchoProbe {
    /// Crea un nuevo probe que envuelve al widget hijo.
    pub fn nuevo(hijo: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            hijo: Some(hijo.erased().to_pod()),
        }
    }

    /// Obtiene una referencia mutable al widget hijo.
    pub fn hijo_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        let hijo = this
            .widget
            .hijo
            .as_mut()
            .expect("WidthProbe always has child");
        this.ctx.get_mut(hijo)
    }
}

impl Widget for VentanaAnchoProbe {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(ref mut hijo) = self.hijo {
            ctx.register_child(hijo);
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        longitud_req: LenReq,
        cross_longitud: Option<Length>,
    ) -> Length {
        // Adopta el tamaño del hijo (igual que el `layout` viejo); sin hijo usa
        // el tamaño de ventana por defecto (800x600) que antes imponía `bc.constrain`.
        match self.hijo.as_mut() {
            Some(hijo) => {
                let context_tamano = crate::layout::LayoutSize::maybe(axis.cross(), cross_longitud);
                ctx.compute_length(
                    hijo,
                    longitud_req.into(),
                    context_tamano,
                    axis,
                    cross_longitud,
                )
            }
            None => {
                let fijo = match axis {
                    Axis::Horizontal => 800.0.px(),
                    Axis::Vertical => 600.0.px(),
                };
                match longitud_req {
                    LenReq::FitContent(space) => fijo.min(space),
                    _ => fijo,
                }
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // Guardar el ancho y alto resueltos en el thread-local
        ANCHO_VENTANA_TL.with(|w| w.set(size.width));
        ALTO_VENTANA_TL.with(|h| h.set(size.height));

        // Layout del hijo ocupando todo el espacio disponible
        if let Some(hijo) = self.hijo.as_mut() {
            ctx.run_layout(hijo, size);
            ctx.place_child(hijo, Point::ORIGIN);
        }
    }

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        _painter: &mut Painter<'_>,
    ) {
        // Este widget envuelve la raíz: se pinta una vez por frame, así que sirve
        // para medir los fps reales (FORJA_GUI_PERF=1).
        if let Some(reporte) = crate::perf::marcar_frame() {
            eprintln!("{}", reporte);
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
        if let Some(hijo) = &self.hijo {
            ChildrenIds::from_slice(&[hijo.id()])
        } else {
            ChildrenIds::from_slice(&[])
        }
    }
}

// ─── View wrapper que inserta el WindowWidthProbe en el árbol ─────

/// Wrapper de Xilem View que envuelve la vista raíz con `WindowWidthProbe`
/// para detectar resize automáticamente.
pub struct AnchoObservedVista<V, State> {
    inner: V,
    phantom: PhantomData<fn() -> State>,
}

/// Envuelve cualquier `WidgetView` con detección de resize.
pub fn observe_ancho<State: 'static, V: WidgetView<State>>(
    inner: V,
) -> AnchoObservedVista<V, State> {
    AnchoObservedVista {
        inner,
        phantom: PhantomData,
    }
}

impl<V: WidgetView<State>, State: 'static> ViewMarker for AnchoObservedVista<V, State> {}

impl<V: WidgetView<State>, State: 'static> View<State, (), ViewCtx>
    for AnchoObservedVista<V, State>
{
    type Element = Pod<VentanaAnchoProbe>;
    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: &mut State,
    ) -> (Pod<VentanaAnchoProbe>, V::ViewState) {
        let (hijo, child_state) = self.inner.build(ctx, app_state);
        let widget = VentanaAnchoProbe::nuevo(hijo.new_widget);
        (ctx.create_pod(widget), child_state)
    }

    fn rebuild(
        &self,
        anterior: &Self,
        view_state: &mut V::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Pod<VentanaAnchoProbe>>,
        app_state: &mut State,
    ) {
        let mut hijo = VentanaAnchoProbe::hijo_mut(&mut element);
        self.inner
            .rebuild(&anterior.inner, view_state, ctx, hijo.downcast(), app_state);
    }

    fn teardown(
        &self,
        view_state: &mut V::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Pod<VentanaAnchoProbe>>,
    ) {
        let mut hijo = VentanaAnchoProbe::hijo_mut(&mut element);
        self.inner.teardown(view_state, ctx, hijo.downcast());
    }

    fn message(
        &self,
        view_state: &mut V::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Pod<VentanaAnchoProbe>>,
        app_state: &mut State,
    ) -> MessageResult<()> {
        let mut hijo = VentanaAnchoProbe::hijo_mut(&mut element);
        self.inner
            .message(view_state, message, hijo.downcast(), app_state)
    }
}

// WidthObservedView implementa WidgetView automáticamente vía el blanket impl de xilem
// (requiere View<..., Element=Pod<W>> + Send + Sync, que WidthObservedView cumple)

// ─── Punto de entrada ─────────────────────────────────────────────

/// Tema con el que se pinta la ventana: si la raíz del layout es un `tema_*`,
/// ese diseño manda también en el fondo de la ventana (no sólo en los hijos).
fn tema_de_raiz(layout: &Layout, base: &MaterialTheme) -> MaterialTheme {
    match layout {
        Layout::ThemeProvider { theme, .. } => MaterialTheme::named(theme, base.es_dark),
        _ => base.clone(),
    }
}

/// `true` si se pidió salida de depuración (`FORJA_GUI_DEBUG=1`).
///
/// Por defecto el runtime no imprime nada: en una app compilada esos logs sólo
/// ensucian la consola y retrasan el arranque.
fn depuracion_activo() -> bool {
    std::env::var("FORJA_GUI_DEBUG")
        .map(|v| v != "0")
        .unwrap_or(false)
}

/// Arma las `WindowOptions` con lo declarado en `ventana(...)`.
fn opciones_ventana(
    titulo: String,
    config: &crate::window_config::VentanaConfig,
    ancho: f64,
    alto: f64,
) -> WindowOptions<AppStateNativo> {
    let mut opciones = WindowOptions::new(titulo)
        .with_initial_inner_size(LogicalSize::new(ancho, alto))
        .with_resizable(config.redimensionable)
        .with_decorations(config.decoraciones);
    if let (Some(w), Some(h)) = (config.ancho_minimo, config.alto_minimo) {
        opciones = opciones.with_min_inner_size(LogicalSize::new(w, h));
    }
    if let Some(icono) = config.cargar_icono() {
        opciones = opciones.with_initial_window_icon(Some(icono));
    }
    opciones
}

/// Construye y ejecuta la ventana GUI nativa.
///
/// Acepta un `theme` opcional; si es `None`, usa `MaterialTheme::light()`.
///
/// El tamaño inicial de ventana es 1200×800 (suficiente para Expanded).
/// En cada render, se lee el ancho real desde el widget WindowWidthProbe
/// para actualizar `window_size` dinámicamente según el resize.
///
/// `auto_theme`: si es `true`, detecta automáticamente el tema del sistema.
/// `load_state`: estado serializado opcional para restaurar variables
///   (usado por hot reload para preservar estado entre reinicios).
pub fn construir_y_ejecutar(
    programa: &Programa,
    cargar_state: Option<&str>,
    theme: Option<MaterialTheme>,
    auto_theme: bool,
) -> Result<(), String> {
    // Lo antes posible: si el binario es de consola, despegarse para que no
    // aparezca la ventana negra antes de la GUI.
    crate::window_config::ocultar_consola_si_corresponde();

    let theme = if auto_theme {
        MaterialTheme::system("#6750A4")
    } else {
        theme.unwrap_or_else(MaterialTheme::light)
    };

    let mut state = AppStateNativo::nuevo();
    inicializar_estado(&programa.declaraciones, &mut state);
    // Restaurar estado guardado (hot reload) si se proporcionó
    if let Some(json) = cargar_state {
        state.cargar_estado(json);
    }
    // Árbol de arranque: sólo para el log (el layout se re-extrae en cada render,
    // así `si/sino` ve el estado actual y la navegación cambia de página).
    let layout_inicial = extraer_layout(&programa.declaraciones, &mut state.almacen);
    if depuracion_activo() {
        crate::debug::imprimir_layout_arbol(&layout_inicial);
    }
    // Si la app declara un diseño en la raíz (`tema_fluent(...)`, `tema_liquid_glass(...)`),
    // ese diseño manda también en el fondo de la ventana y el título.
    let tema_ventana = tema_de_raiz(&layout_inicial, &theme);
    let prog = programa.declaraciones.clone();

    // `ventana(...)` declarada en el código Forja.
    let config = crate::window_config::extraer_config_ventana(&programa.declaraciones);
    let titulo = config.titulo.clone().unwrap_or_else(|| {
        format!(
            "Forja GUI — {:?} {}",
            tema_ventana.design,
            if tema_ventana.es_dark {
                "oscuro"
            } else {
                "claro"
            }
        )
    });
    if depuracion_activo() {
        println!(
            "  🪟 Ventana «{}» {}x{} (icono: {:?})",
            titulo, config.ancho, config.alto, config.icono
        );
        println!(
            "  {} Diseño {:?} — modo {} (seed: {})",
            if tema_ventana.es_dark {
                "🌙"
            } else {
                "☀️"
            },
            tema_ventana.design,
            if tema_ventana.es_dark {
                "oscuro"
            } else {
                "claro"
            },
            tema_ventana.seed_color
        );
    }

    // Inicializar window_width con un valor por defecto (se actualizará en cada render)
    let initial_ancho: f64 = config.ancho;
    let initial_alto: f64 = config.alto;
    state.actualizar_ventana_tamano(initial_ancho, initial_alto);
    ANCHO_VENTANA_TL.with(|w| w.set(initial_ancho));
    ALTO_VENTANA_TL.with(|h| h.set(initial_alto));
    if depuracion_activo() {
        println!(
            "  📐 Clase de tamaño inicial: {:?} ({:.0}px)",
            state.ventana_tamano, initial_ancho
        );
    }

    // Crear la app con WindowOptions que fija el tamaño inicial de la ventana
    let app = Xilem::new_simple(
        state,
        move |data: &mut AppStateNativo| -> Box<AnyWidgetView<AppStateNativo>> {
            // Leer el ancho actual desde el thread-local (actualizado por WindowWidthProbe)
            let actual_ancho = actual_ventana_ancho();
            let actual_alto = actual_ventana_alto();
            data.actualizar_ventana_tamano(actual_ancho, actual_alto);
            let medir = crate::perf::activo();
            let t0 = std::time::Instant::now();
            let layout = extraer_layout(&prog, &mut data.almacen);
            let t1 = std::time::Instant::now();
            // El diseño de la raíz se re-evalúa en cada render (por si un `si`
            // cambia de tema según el estado).
            let tema = tema_de_raiz(&layout, &theme);
            let raiz_vista = layout_a_vista(&layout, data, &prog, &tema);
            if medir {
                if let Some(reporte) =
                    crate::perf::registrar(t1 - t0, std::time::Instant::now() - t1)
                {
                    eprintln!("{}", reporte);
                }
            }
            let raiz_bg = crate::chrome::fondo_pagina(&tema);
            let raiz_con_bg = view::sized_box(raiz_vista)
                .dims((
                    Length::px(actual_ancho.max(200.0)),
                    Length::px(actual_alto.max(200.0)),
                ))
                .background(Background::Color(raiz_bg));
            Box::new(observe_ancho(raiz_con_bg))
        },
        crate::chrome::aplicar_chrome(
            opciones_ventana(titulo, &config, initial_ancho, initial_alto),
            &tema_ventana,
        ),
    )
    // Los widgets nativos (botones, campos, switches, sliders) toman sus colores
    // del property set de Masonry, que por defecto es oscuro.
    .with_default_properties(crate::theme_props::default_properties(&tema_ventana));

    app.run_in(EventLoop::with_user_event())
        .map_err(|e| format!("Error en GUI: {}", e))
}

#[cfg(target_os = "android")]
pub fn construir_y_ejecutar_android(
    programa: &Programa,
    cargar_state: Option<&str>,
    theme: Option<MaterialTheme>,
    auto_theme: bool,
    android_app: winit::platform::android::activity::AndroidApp,
) -> Result<(), String> {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let theme = if auto_theme {
        MaterialTheme::system("#6750A4")
    } else {
        theme.unwrap_or_else(MaterialTheme::light)
    };
    // El closure de render se queda con `theme`; la copia sobrevive para el
    // property set de los widgets nativos.
    let tema_ventana = theme.clone();

    let mut state = AppStateNativo::nuevo();
    inicializar_estado(&programa.declaraciones, &mut state);
    // Restaurar estado guardado (hot reload) si se proporcionó
    if let Some(json) = cargar_state {
        state.cargar_estado(json);
    }
    // Árbol de arranque: sólo para el log (el layout se re-extrae en cada render,
    // así `si/sino` ve el estado actual y la navegación cambia de página).
    crate::debug::imprimir_layout_arbol(&extraer_layout(
        &programa.declaraciones,
        &mut state.almacen,
    ));
    let prog = programa.declaraciones.clone();

    let initial_ancho: f64 = 1200.0;
    let initial_alto: f64 = 800.0;
    state.actualizar_ventana_tamano(initial_ancho, initial_alto);
    ANCHO_VENTANA_TL.with(|w| w.set(initial_ancho));
    ALTO_VENTANA_TL.with(|h| h.set(initial_alto));

    let app = Xilem::new_simple(
        state,
        move |data: &mut AppStateNativo| -> Box<AnyWidgetView<AppStateNativo>> {
            let actual_ancho = actual_ventana_ancho();
            let actual_alto = actual_ventana_alto();
            data.actualizar_ventana_tamano(actual_ancho, actual_alto);
            let layout = extraer_layout(&prog, &mut data.almacen);
            let tema = tema_de_raiz(&layout, &theme);
            let raiz_vista = layout_a_vista(&layout, data, &prog, &tema);
            let raiz_bg = crate::chrome::fondo_pagina(&tema);
            let raiz_con_bg = view::sized_box(raiz_vista)
                .dims((
                    Length::px(actual_ancho.max(200.0)),
                    Length::px(actual_alto.max(200.0)),
                ))
                .background(Background::Color(raiz_bg));
            Box::new(observe_ancho(raiz_con_bg))
        },
        WindowOptions::new("Forja GUI — diseño de la app")
            .with_initial_inner_size(LogicalSize::new(initial_ancho, 800.0)),
    );

    let app = app.with_default_properties(crate::theme_props::default_properties(&tema_ventana));
    let mut evento_bucle = EventLoop::with_user_event();
    evento_bucle.with_android_app(android_app);

    app.run_in(evento_bucle)
        .map_err(|e| format!("Error en GUI Android: {}", e))
}

// ─── Callback: ejecutar funciones Forja (delegado al evaluador) ──

/// Ejecuta una función Forja usando el evaluador tree-walking completo.
/// Retorna el valor de retorno de la función o un mensaje de error.
pub fn ejecutar_callback_forja(
    nombre_fn: &str,
    state: &AppStateNativo,
    programa: &[Declaracion],
) -> ValorGUI {
    let mut almacen = state.almacen.clone();
    match crate::evaluador::ejecutar_funcion(nombre_fn, &[], programa, &crate::evaluador::AlmacenGui(&almacen)) {
        Ok(val) => val,
        Err(e) => ValorGUI::Texto(format!("Error: {}", e)),
    }
}

// ejecutar_callback_y_actualizar → movido a crate::helpers

/// Inicializa el estado de la GUI evaluando las variables de la función `main`.
pub fn inicializar_estado(decls: &[Declaracion], state: &mut AppStateNativo) {
    crate::evaluador::inicializar_estado(decls, &crate::evaluador::AlmacenGui(&state.almacen));
}

// ═══════════════════════════════════════════════════════════════════
// View wrappers para gestos (PullToRefreshView, SwipeToDismissView)
// ═══════════════════════════════════════════════════════════════════
//
// Estos wrappers permiten usar los widgets Masonry de gestos (de
// gesture_widgets.rs) como vistas Xilem, siguiendo el mismo patrón
// que el Button view de xilem.

use crate::GestureResultado as GestureAction;
use crate::PinchZoomWidget;
use crate::PullToRefreshWidget as PullRefreshWidget;
use crate::RotateWidget;
use crate::SwipeWidget as SwipeGestureWidget;
use crate::{Pod, ViewCtx, WidgetView};
use xilem::core::{Mut, View, ViewId, ViewMarker, ViewPathTracker};

// ─── ViewId para child routing ──────────────────────────────────
const GESTURE_CHILD_VIEW_ID: ViewId = ViewId::new(0);

// ═══════════════════════════════════════════════════════════════════
// PullToRefreshView
// ═══════════════════════════════════════════════════════════════════

/// View de Xilem que envuelve un hijo en un PullToRefreshWidget.
/// Detecta gesto de tirar hacia abajo y ejecuta un callback Forja.
pub struct PullToRefreshVista<V> {
    hijo: V,
    callback: String,
    refreshing: String,
    program: Vec<Declaracion>,
}

impl<V> PullToRefreshVista<V> {
    pub fn nuevo(hijo: V, callback: String, refreshing: String, program: Vec<Declaracion>) -> Self {
        Self {
            hijo,
            callback,
            refreshing,
            program,
        }
    }
}

impl<V> ViewMarker for PullToRefreshVista<V> {}

impl<V> View<AppStateNativo, (), ViewCtx> for PullToRefreshVista<V>
where
    V: WidgetView<AppStateNativo, ()>,
{
    type Element = Pod<PullRefreshWidget>;
    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: &mut AppStateNativo,
    ) -> (Self::Element, Self::ViewState) {
        let (hijo, child_state) = ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::build(&self.hijo, ctx, app_state)
        });
        let widget = PullRefreshWidget::nuevo(hijo.new_widget);
        (
            ctx.with_action_widget(|ctx| ctx.create_pod(widget)),
            child_state,
        )
    }

    fn rebuild(
        &self,
        anterior: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::rebuild(
                &self.hijo,
                &anterior.hijo,
                view_state,
                ctx,
                PullRefreshWidget::hijo_mut(&mut element).downcast(),
                app_state,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::teardown(
                &self.hijo,
                view_state,
                ctx,
                PullRefreshWidget::hijo_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        vista_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) -> crate::MessageResult<()> {
        match message.take_first() {
            Some(GESTURE_CHILD_VIEW_ID) => View::<AppStateNativo, (), _>::message(
                &self.hijo,
                vista_state,
                message,
                PullRefreshWidget::hijo_mut(&mut element).downcast(),
                app_state,
            ),
            None => {
                match message.take_message::<GestureAction>() {
                    Some(boxed) => match *boxed {
                        GestureAction::PullToRefresh { .. } => {
                            // Marcar como refrescando y ejecutar callback
                            if !self.callback.is_empty() {
                                app_state.escribir(&self.refreshing, ValorGUI::Booleano(true));
                                ejecutar_callback_y_actualizar(
                                    &self.callback,
                                    app_state,
                                    &self.program,
                                );
                            }
                            crate::MessageResult::Nop
                        }
                        _ => crate::MessageResult::Stale,
                    },
                    None => crate::MessageResult::Stale,
                }
            }
            _ => crate::MessageResult::Stale,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// SwipeToDismissView
// ═══════════════════════════════════════════════════════════════════

/// View de Xilem que envuelve un hijo en un SwipeWidget.
/// Detecta swipe horizontal para descartar elementos.
pub struct SwipeToDismissVista<V> {
    hijo: V,
    on_dismiss: String,
    dismissed: String,
    etiqueta: String,
    program: Vec<Declaracion>,
}

impl<V> SwipeToDismissVista<V> {
    pub fn nuevo(
        hijo: V,
        on_dismiss: String,
        dismissed: String,
        etiqueta: String,
        program: Vec<Declaracion>,
    ) -> Self {
        Self {
            hijo,
            on_dismiss,
            dismissed,
            etiqueta,
            program,
        }
    }
}

impl<V> ViewMarker for SwipeToDismissVista<V> {}

impl<V> View<AppStateNativo, (), ViewCtx> for SwipeToDismissVista<V>
where
    V: WidgetView<AppStateNativo, ()>,
{
    type Element = Pod<SwipeGestureWidget>;
    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: &mut AppStateNativo,
    ) -> (Self::Element, Self::ViewState) {
        let (hijo, child_state) = ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::build(&self.hijo, ctx, app_state)
        });
        let widget = SwipeGestureWidget::nuevo(hijo.new_widget).con_threshold(100.0);
        (
            ctx.with_action_widget(|ctx| ctx.create_pod(widget)),
            child_state,
        )
    }

    fn rebuild(
        &self,
        anterior: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::rebuild(
                &self.hijo,
                &anterior.hijo,
                view_state,
                ctx,
                SwipeGestureWidget::hijo_mut(&mut element).downcast(),
                app_state,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::teardown(
                &self.hijo,
                view_state,
                ctx,
                SwipeGestureWidget::hijo_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        vista_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) -> crate::MessageResult<()> {
        match message.take_first() {
            Some(GESTURE_CHILD_VIEW_ID) => View::<AppStateNativo, (), _>::message(
                &self.hijo,
                vista_state,
                message,
                SwipeGestureWidget::hijo_mut(&mut element).downcast(),
                app_state,
            ),
            None => {
                match message.take_message::<GestureAction>() {
                    Some(boxed) => match *boxed {
                        GestureAction::Swipe { .. } => {
                            // 5.1: Usar campo label para debugging durante swipe
                            if !self.etiqueta.is_empty() {
                                eprintln!(
                                    "[SwipeToDismiss] '{}' — deslizando para descartar",
                                    &self.etiqueta
                                );
                            }
                            // Marcar como descartado y ejecutar callback
                            if !self.on_dismiss.is_empty() {
                                app_state.escribir(&self.dismissed, ValorGUI::Booleano(true));
                                ejecutar_callback_y_actualizar(
                                    &self.on_dismiss,
                                    app_state,
                                    &self.program,
                                );
                            }
                            crate::MessageResult::Nop
                        }
                        _ => crate::MessageResult::Stale,
                    },
                    None => crate::MessageResult::Stale,
                }
            }
            _ => crate::MessageResult::Stale,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// PinchZoomView
// ═══════════════════════════════════════════════════════════════════

/// View de Xilem que envuelve un hijo en un PinchZoomWidget.
/// Aplica zoom por gesto de pellizco (pinch-to-zoom).
pub struct PinchZoomVista<V> {
    hijo: V,
    min_scale: f64,
    max_scale: f64,
}

impl<V> PinchZoomVista<V> {
    pub fn nuevo(hijo: V, min_scale: f64, max_scale: f64) -> Self {
        Self {
            hijo,
            min_scale,
            max_scale,
        }
    }
}

impl<V> ViewMarker for PinchZoomVista<V> {}

impl<V> View<AppStateNativo, (), ViewCtx> for PinchZoomVista<V>
where
    V: WidgetView<AppStateNativo, ()>,
{
    type Element = Pod<PinchZoomWidget>;
    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: &mut AppStateNativo,
    ) -> (Self::Element, Self::ViewState) {
        let (hijo, child_state) = ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::build(&self.hijo, ctx, app_state)
        });
        let widget = PinchZoomWidget::nuevo(hijo.new_widget)
            .con_min_scale(self.min_scale)
            .con_max_scale(self.max_scale);
        (
            ctx.with_action_widget(|ctx| ctx.create_pod(widget)),
            child_state,
        )
    }

    fn rebuild(
        &self,
        anterior: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::rebuild(
                &self.hijo,
                &anterior.hijo,
                view_state,
                ctx,
                PinchZoomWidget::hijo_mut(&mut element).downcast(),
                app_state,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::teardown(
                &self.hijo,
                view_state,
                ctx,
                PinchZoomWidget::hijo_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        vista_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) -> crate::MessageResult<()> {
        match message.take_first() {
            Some(GESTURE_CHILD_VIEW_ID) => View::<AppStateNativo, (), _>::message(
                &self.hijo,
                vista_state,
                message,
                PinchZoomWidget::hijo_mut(&mut element).downcast(),
                app_state,
            ),
            None => crate::MessageResult::Nop,
            _ => crate::MessageResult::Stale,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// RotateView
// ═══════════════════════════════════════════════════════════════════

/// View de Xilem que envuelve un hijo en un RotateWidget.
/// Aplica rotación por gesto multi-touch.
pub struct RotateVista<V> {
    hijo: V,
}

impl<V> RotateVista<V> {
    pub fn nuevo(hijo: V) -> Self {
        Self { hijo }
    }
}

impl<V> ViewMarker for RotateVista<V> {}

impl<V> View<AppStateNativo, (), ViewCtx> for RotateVista<V>
where
    V: WidgetView<AppStateNativo, ()>,
{
    type Element = Pod<RotateWidget>;
    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: &mut AppStateNativo,
    ) -> (Self::Element, Self::ViewState) {
        let (hijo, child_state) = ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::build(&self.hijo, ctx, app_state)
        });
        let widget = RotateWidget::nuevo(hijo.new_widget);
        (
            ctx.with_action_widget(|ctx| ctx.create_pod(widget)),
            child_state,
        )
    }

    fn rebuild(
        &self,
        anterior: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::rebuild(
                &self.hijo,
                &anterior.hijo,
                view_state,
                ctx,
                RotateWidget::hijo_mut(&mut element).downcast(),
                app_state,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::teardown(
                &self.hijo,
                view_state,
                ctx,
                RotateWidget::hijo_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        vista_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) -> crate::MessageResult<()> {
        match message.take_first() {
            Some(GESTURE_CHILD_VIEW_ID) => View::<AppStateNativo, (), _>::message(
                &self.hijo,
                vista_state,
                message,
                RotateWidget::hijo_mut(&mut element).downcast(),
                app_state,
            ),
            None => crate::MessageResult::Nop,
            _ => crate::MessageResult::Stale,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests de extracción de layout (sin abrir ventana)
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests_layout {
    use super::*;
    use forja_syntax::{lexer::Lexer, parser::Parser};

    fn layout_de(src: &str) -> Layout {
        let mut lexer = Lexer::nuevo(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = Parser::nuevo(tokens);
        let programa = parser.parsear().expect("parse");
        let mut almacen = VariableAlmacen::nuevo();
        extraer_layout(&programa.declaraciones, &mut almacen)
    }

    #[test]
    fn grid_toma_columnas_y_gap_posicionales() {
        let layout = layout_de(
            r#"
importar gui
funcion main() {
    grilla([etiqueta("a"), etiqueta("b"), etiqueta("c")], 3, 12)
}
"#,
        );
        match layout {
            Layout::Grid {
                children,
                columnas,
                gap,
            } => {
                assert_eq!(children.len(), 3);
                assert_eq!(columnas, 3);
                assert_eq!(gap, 12.0);
            }
            otro => panic!("se esperaba Grid, llegó {:?}", otro),
        }
    }

    #[test]
    fn grid_acepta_alias_grid_y_cuadricula() {
        for nombre in ["grid", "cuadricula", "cuadrícula"] {
            let src = format!(
                "importar gui\nfuncion main() {{ {}([etiqueta(\"x\")], 2, 8) }}\n",
                nombre
            );
            assert!(
                matches!(layout_de(&src), Layout::Grid { columnas: 2, .. }),
                "alias '{}' no produjo Grid",
                nombre
            );
        }
    }

    #[test]
    fn temas_nombrados_llegan_al_layout() {
        for (funcion, esperado) in [
            ("tema_fluent", "fluent:light"),
            ("tema_fluent_oscuro", "fluent:dark"),
            ("tema_liquid_glass", "liquid_glass:light"),
            ("tema_liquid_glass_oscuro", "liquid_glass:dark"),
        ] {
            let src = format!(
                "importar gui\nfuncion main() {{ {}(columna(etiqueta(\"x\"))) }}\n",
                funcion
            );
            match layout_de(&src) {
                Layout::ThemeProvider { theme, .. } => assert_eq!(theme, esperado),
                otro => panic!("{} no produjo ThemeProvider: {:?}", funcion, otro),
            }
        }
    }
}

#[cfg(test)]
mod tests_ejemplo_galeria {
    use super::*;
    use forja_syntax::{lexer::Lexer, parser::Parser};

    /// La galería de ejemplos debe extraer un árbol completo: tema → columna →
    /// grid de 6 tarjetas. Si un nombre de componente deja de existir en el
    /// parser, ese widget desaparece del árbol y este test falla.
    #[test]
    fn galeria_extrae_tema_grid_y_tarjetas() {
        let src = include_str!("../../../ejemplos/gui/208_gui_galeria.fa");
        let mut lexer = Lexer::nuevo(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = Parser::nuevo(tokens);
        let programa = parser.parsear().expect("parse");
        let mut almacen = VariableAlmacen::nuevo();
        let layout = extraer_layout(&programa.declaraciones, &mut almacen);

        let Layout::ThemeProvider { hijo, theme } = layout else {
            panic!("la raíz debe ser un ThemeProvider, llegó: {:?}", layout);
        };
        assert_eq!(theme, "fluent:light");

        // El tema envuelve un `desplazable(columna_con_gap(...))`.
        let Layout::Portal(columna) = *hijo else {
            panic!("el tema debe envolver un portal (desplazable)");
        };
        let Layout::Column { children, gap, .. } = *columna else {
            panic!("el desplazable debe envolver una columna");
        };
        assert_eq!(gap, 16.0);

        let grid = children
            .iter()
            .find(|c| matches!(c, Layout::Grid { .. }))
            .expect("falta el grid de tarjetas");
        match grid {
            Layout::Grid {
                children: celdas,
                columnas,
                gap,
            } => {
                assert_eq!(*columnas, 3);
                assert_eq!(*gap, 16.0);
                assert_eq!(celdas.len(), 6, "deben sobrevivir las 6 tarjetas");
                assert!(
                    celdas
                        .iter()
                        .all(|c| matches!(c, Layout::MaterialCard { .. })),
                    "cada celda del grid debe ser una tarjeta"
                );
            }
            _ => unreachable!(),
        }

        // La barra superior y el botón final también deben estar presentes.
        assert!(
            children
                .iter()
                .any(|c| matches!(c, Layout::TopAppBar { .. })),
            "falta la barra superior"
        );
        assert!(
            children
                .iter()
                .any(|c| matches!(c, Layout::MaterialButton { .. })),
            "falta el botón de guardar"
        );
    }
}

#[cfg(test)]
mod tests_tema_de_ventana {
    use super::*;

    fn base() -> MaterialTheme {
        MaterialTheme::light()
    }

    #[test]
    fn la_raiz_tema_fluent_manda_en_la_ventana() {
        let layout = Layout::ThemeProvider {
            hijo: Box::new(Layout::Spacer(0.0)),
            theme: "fluent".to_string(),
        };
        let tema = tema_de_raiz(&layout, &base());
        assert_eq!(tema.design, DesignStyle::Fluent);
        // El fondo de la ventana sale del esquema del diseño, no del Material You.
        assert_eq!(tema.scheme.background, RgbColor(243, 243, 243));
    }

    #[test]
    fn la_raiz_tema_liquid_glass_oscuro_manda_en_la_ventana() {
        let layout = Layout::ThemeProvider {
            hijo: Box::new(Layout::Spacer(0.0)),
            theme: "liquid_glass:dark".to_string(),
        };
        let tema = tema_de_raiz(&layout, &base());
        assert_eq!(tema.design, DesignStyle::LiquidGlass);
        assert!(tema.es_dark);
        assert_eq!(tema.scheme.background, RgbColor(10, 12, 16));
    }

    #[test]
    fn sin_wrapper_se_usa_el_tema_base() {
        let layout = Layout::Column {
            children: vec![],
            gap: 0.0,
            alignment: "start".to_string(),
        };
        let tema = tema_de_raiz(&layout, &base());
        assert_eq!(tema.design, DesignStyle::MaterialYou);
    }

    /// Las formas del diseño tienen que llegar a los widgets: 4px en Fluent,
    /// 24px en Liquid Glass, 20px en Material.
    #[test]
    fn las_formas_cambian_por_diseno() {
        let boton = |es_dark| {
            (
                MaterialTheme::fluent(es_dark)
                    .shapes
                    .for_family(FormaFamily::Button),
                MaterialTheme::liquid_glass(es_dark)
                    .shapes
                    .for_family(FormaFamily::Button),
                MaterialTheme::light()
                    .shapes
                    .for_family(FormaFamily::Button),
            )
        };
        for es_dark in [false, true] {
            let (fluent, glass, material) = boton(es_dark);
            assert_eq!(fluent, 4.0);
            assert_eq!(glass, 24.0);
            assert_eq!(material, 20.0);
        }
        // Fluent 2: ControlCornerRadius = 4 para contenedores (tarjetas/campos).
        assert_eq!(
            MaterialTheme::fluent(false)
                .shapes
                .for_family(FormaFamily::Container),
            4.0
        );
        assert_eq!(
            MaterialTheme::liquid_glass(false)
                .shapes
                .for_family(FormaFamily::Container),
            20.0
        );
    }
}

#[cfg(test)]
mod tests_modo_claro_oscuro {
    use super::*;

    /// `tema_fluent`/`tema_liquid_glass` son claros y sus variantes `_oscuro` son
    /// oscuras, sin importar el modo del sistema operativo.
    #[test]
    fn el_sufijo_manda_sobre_el_modo_del_sistema() {
        let claro = MaterialTheme::named("fluent:light", true);
        assert!(
            !claro.es_dark,
            "fluent:light debe ser claro aunque el SO sea oscuro"
        );
        assert_eq!(claro.design, DesignStyle::Fluent);

        let oscuro = MaterialTheme::named("fluent:dark", false);
        assert!(
            oscuro.es_dark,
            "fluent:dark debe ser oscuro aunque el SO sea claro"
        );

        let vidrio = MaterialTheme::named("liquid_glass:light", true);
        assert!(!vidrio.es_dark);
        assert_eq!(vidrio.scheme.background, RgbColor(242, 246, 252));
    }

    /// Los roles de color se resuelven contra el tema (no contra hex fijos).
    #[test]
    fn los_roles_de_color_salen_del_tema() {
        let fluent = MaterialTheme::fluent(false);
        let error = crate::theme_props::color_de_rol(&fluent, "error").expect("rol error");
        assert_eq!(error, RgbColor(196, 43, 28)); // #C42B1C, no el rojo de Material
        let primario = crate::theme_props::color_de_rol(&fluent, "primary").expect("rol primary");
        assert_eq!(primario, RgbColor(0, 95, 184)); // #005FB8
        assert!(crate::theme_props::color_de_rol(&fluent, "no_es_un_rol").is_none());
    }
}

#[cfg(test)]
mod tests_config_ventana {
    use super::*;
    use forja_syntax::{lexer::Lexer, parser::Parser};

    fn parsear(src: &str) -> Programa {
        let mut lexer = Lexer::nuevo(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = Parser::nuevo(tokens);
        parser.parsear().expect("parse")
    }

    #[test]
    fn ventana_completa_se_lee_del_ast() {
        let programa = parsear(
            r#"
importar gui
funcion main() {
    ventana("Mi app", 900, 640, "assets/icono.png", 700, 500, falso, falso)
    columna(etiqueta("hola"))
}
"#,
        );
        let cfg = crate::window_config::extraer_config_ventana(&programa.declaraciones);
        assert_eq!(cfg.titulo.as_deref(), Some("Mi app"));
        assert_eq!(cfg.ancho, 900.0);
        assert_eq!(cfg.alto, 640.0);
        assert_eq!(cfg.icono.as_deref(), Some("assets/icono.png"));
        assert_eq!(cfg.ancho_minimo, Some(700.0));
        assert_eq!(cfg.alto_minimo, Some(500.0));
        assert!(!cfg.redimensionable);
        assert!(!cfg.decoraciones);
    }

    #[test]
    fn ventana_minima_usa_los_defaults() {
        let programa = parsear(
            r#"
importar gui
funcion main() {
    ventana("Solo título", 800, 600)
    columna(etiqueta("x"))
}
"#,
        );
        let cfg = crate::window_config::extraer_config_ventana(&programa.declaraciones);
        assert_eq!(cfg.titulo.as_deref(), Some("Solo título"));
        assert_eq!((cfg.ancho, cfg.alto), (800.0, 600.0));
        assert_eq!(cfg.icono, None);
        assert!(
            cfg.redimensionable,
            "por defecto la ventana se puede redimensionar"
        );
        assert!(cfg.decoraciones, "por defecto usa el marco del sistema");
    }

    #[test]
    fn sin_ventana_quedan_los_defaults() {
        let programa = parsear("importar gui\nfuncion main() { columna(etiqueta(\"x\")) }");
        let cfg = crate::window_config::extraer_config_ventana(&programa.declaraciones);
        assert_eq!((cfg.ancho, cfg.alto), (1200.0, 800.0));
        assert_eq!(cfg.titulo, None);
    }

    /// `ventana(...)` no debe aparecer en el árbol de widgets.
    #[test]
    fn ventana_no_se_renderiza() {
        let programa = parsear(
            r#"
importar gui
funcion main() {
    ventana("App", 800, 600)
    titulo("Hola")
}
"#,
        );
        let mut almacen = VariableAlmacen::nuevo();
        match extraer_layout(&programa.declaraciones, &mut almacen) {
            Layout::Title(t) => assert_eq!(t, "Hola"),
            otro => panic!("se esperaba sólo el título, llegó {:?}", otro),
        }
    }
}
