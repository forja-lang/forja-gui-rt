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
    MaterialTheme, // Tema Material You completo
    RgbColor,      // Color RGB (convierte a xilem::Color vía From)
    VariableStore, // Store reactivo de variables
};
use chrono::Datelike;
use forja_syntax::ast::*;
use std::collections::HashMap;
use std::time::Instant;
// icons module used implicitly through svg_icon() call

#[derive(Debug, Clone)]
pub enum ValorGUI {
    Texto(String),
    Entero(i64),
    Decimal(f64),
    Booleano(bool),
    Nulo,
    /// Mapa con claves String para representar valores estructurados (Ok, Error, Algo, etc.)
    Mapa(HashMap<String, ValorGUI>),
}

impl ValorGUI {
    pub(crate) fn to_string(&self) -> String {
        match self {
            ValorGUI::Texto(s) => s.clone(),
            ValorGUI::Entero(n) => n.to_string(),
            ValorGUI::Decimal(f) => f.to_string(),
            ValorGUI::Booleano(b) => {
                if *b {
                    "verdadero".to_string()
                } else {
                    "falso".to_string()
                }
            }
            ValorGUI::Nulo => "nulo".to_string(),
            ValorGUI::Mapa(m) => format!("Mapa({:?})", m),
        }
    }

    pub(crate) fn to_f64(&self) -> f64 {
        match self {
            ValorGUI::Entero(n) => *n as f64,
            ValorGUI::Decimal(f) => *f,
            ValorGUI::Texto(s) => s.parse().unwrap_or(0.0),
            _ => 0.0,
        }
    }

    pub(crate) fn to_bool(&self) -> bool {
        match self {
            ValorGUI::Booleano(b) => *b,
            ValorGUI::Texto(s) => s == "verdadero" || s == "true",
            ValorGUI::Entero(n) => *n != 0,
            _ => false,
        }
    }
}

impl From<&str> for ValorGUI {
    fn from(s: &str) -> Self {
        ValorGUI::Texto(s.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct AppStateNativo {
    pub store: VariableStore,
    pub window_size: WindowSizeClass,
    pub window_width: f64,
    pub window_height: f64,
    pub animation_engine: AnimationEngine,
    /// Previous visibility values keyed by variable name, for detecting transitions
    anim_prev_visibility: HashMap<String, bool>,
    /// Animation start times for transitions keyed by variable name
    anim_start_time: HashMap<String, Instant>,
    /// Previous screen ID for navigator transitions
    pub(crate) nav_prev_screen: String,
    /// Animation start time for navigator screen transitions
    pub(crate) nav_anim_start: Option<Instant>,
    pub orientation: Orientation,
    /// 5.7: Contexto reactivo para tracking de dependencias de widgets
    pub reactive_ctx: ReactiveCtx,
}

// ─── Conversión ValorGUI ↔ serde_json::Value ─────────────────────

impl From<ValorGUI> for serde_json::Value {
    fn from(val: ValorGUI) -> Self {
        match val {
            ValorGUI::Texto(s) => serde_json::Value::String(s),
            ValorGUI::Entero(n) => serde_json::Value::Number(n.into()),
            ValorGUI::Decimal(f) => serde_json::Value::String(f.to_string()),
            ValorGUI::Booleano(b) => serde_json::Value::Bool(b),
            ValorGUI::Nulo => serde_json::Value::Null,
            ValorGUI::Mapa(m) => {
                let mut map = serde_json::Map::new();
                for (k, v) in m {
                    map.insert(k, serde_json::Value::from(v));
                }
                serde_json::Value::Object(map)
            }
        }
    }
}

impl From<serde_json::Value> for ValorGUI {
    fn from(val: serde_json::Value) -> Self {
        match val {
            serde_json::Value::String(s) => ValorGUI::Texto(s),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    ValorGUI::Entero(i)
                } else if let Some(f) = n.as_f64() {
                    ValorGUI::Decimal(f)
                } else {
                    ValorGUI::Nulo
                }
            }
            serde_json::Value::Bool(b) => ValorGUI::Booleano(b),
            serde_json::Value::Object(obj) => {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    map.insert(k, ValorGUI::from(v));
                }
                ValorGUI::Mapa(map)
            }
            serde_json::Value::Array(arr) => {
                // Convertir array a JSON string para mantener compatibilidad
                if let Some(first) = arr.first() {
                    ValorGUI::from(first.clone())
                } else {
                    ValorGUI::Nulo
                }
            }
            serde_json::Value::Null => ValorGUI::Nulo,
        }
    }
}

impl AppStateNativo {
    pub fn new() -> Self {
        AppStateNativo {
            store: VariableStore::new(),
            window_size: WindowSizeClass::Compact,
            window_width: 800.0,
            window_height: 600.0,
            animation_engine: AnimationEngine::new(),
            anim_prev_visibility: HashMap::new(),
            anim_start_time: HashMap::new(),
            nav_prev_screen: String::new(),
            nav_anim_start: None,
            orientation: Orientation::Portrait,
            reactive_ctx: ReactiveCtx::new(),
        }
    }

    /// Detecta si una variable de visibilidad cambió de estado
    pub fn anim_visibility_changed(&mut self, var_name: &str, current_visible: bool) -> bool {
        let prev = self
            .anim_prev_visibility
            .get(var_name)
            .copied()
            .unwrap_or(false);
        let changed = prev != current_visible;
        self.anim_prev_visibility
            .insert(var_name.to_string(), current_visible);
        if changed {
            self.anim_start_time
                .insert(var_name.to_string(), Instant::now());
        }
        changed
    }

    /// Obtiene el progreso de animación (0.0 a 1.0) para una transición
    pub fn anim_progress(&self, var_name: &str, duracion_ms: f64) -> f64 {
        if let Some(start) = self.anim_start_time.get(var_name) {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            let progress = (elapsed / duracion_ms).clamp(0.0, 1.0);
            progress
        } else {
            0.0
        }
    }

    /// Obtiene el progreso de animación suavizado con easing emphasized
    pub fn anim_progress_eased(&self, var_name: &str, duracion_ms: f64) -> f64 {
        let raw = self.anim_progress(var_name, duracion_ms);
        EASE_EMPHASIZED.apply(raw)
    }

    pub fn nav_transition_start(&mut self, screen_id: &str) {
        if self.nav_prev_screen != screen_id {
            self.nav_prev_screen = screen_id.to_string();
            self.nav_anim_start = Some(Instant::now());
        }
    }

    /// Anuncia un mensaje de accesibilidad (TalkBack)
    pub fn a11y_say(&self, mensaje: &str) {
        println!("  ♿ [TalkBack] {}", mensaje);
    }

    /// Anuncia foco en un widget
    pub fn a11y_focus(&self, widget_type: &str, label: &str, value: &str, state: &str) {
        let desc = descripcion_accesible(widget_type, label, value, state);
        self.a11y_say(&desc);
    }

    /// Actualiza el tamaño de ventana y la clase de tamaño
    pub fn update_window_size(&mut self, width: f64, height: f64) {
        self.window_width = width;
        self.window_height = height;
        self.window_size = WindowSizeClass::from_width(width);
        self.orientation = Orientation::from_dimensions(width, height);
    }

    pub fn leer(&self, nombre: &str) -> ValorGUI {
        // 5.7: Registrar lectura para reactive tracking
        self.reactive_ctx.track_dependency(nombre);
        self.store
            .get(nombre)
            .map(ValorGUI::from)
            .unwrap_or(ValorGUI::Nulo)
    }

    pub fn escribir(&mut self, nombre: &str, valor: ValorGUI) {
        let json_val: serde_json::Value = valor.into();
        self.store.set(nombre, json_val);
    }

    /// Serializa todas las variables de estado a JSON.
    /// Usado por hot reload para preservar estado entre reinicios.
    pub fn guardar_estado(&self) -> String {
        let snapshot = self.store.snapshot();
        serde_json::to_string(&snapshot).unwrap_or_default()
    }

    /// Carga estado desde un JSON, restaurando las variables guardadas.
    /// Usado por hot reload para restaurar estado tras recompilar.
    pub fn cargar_estado(&mut self, json: &str) {
        if let Ok(map) =
            serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(json)
        {
            for (key, value) in map {
                self.store.set(&key, value);
            }
        }
    }
}

impl Default for AppStateNativo {
    fn default() -> Self {
        Self::new()
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
pub enum WindowSizeClass {
    Compact,  // < 600dp
    Medium,   // 600-840dp
    Expanded, // > 840dp
}

impl WindowSizeClass {
    pub fn from_width(width: f64) -> Self {
        if width < 600.0 {
            WindowSizeClass::Compact
        } else if width < 840.0 {
            WindowSizeClass::Medium
        } else {
            WindowSizeClass::Expanded
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
pub enum FabSize {
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
    Input,
    Suggestion,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextFieldVariant {
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
pub struct NavItem {
    pub icono: String,
    pub label: String,
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
pub enum NavigatorType {
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
    ZStack(Vec<Layout>),
    Portal(Box<Layout>),
    Container {
        child: Box<Layout>,
        max_width: f64,
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
        child: Box<Layout>,
        theme: String,
    },
    /// Aplica un color del esquema al hijo (primary, secondary, etc.)
    ColoredBox {
        child: Box<Layout>,
        color_role: String,
    },
    /// Etiqueta con estilo tipográfico predefinido (display_large, body_medium, etc.)
    StyledLabel {
        texto: String,
        style: String,
    },
    /// Aplica una familia de forma (border-radius) al hijo
    ShapedBox {
        child: Box<Layout>,
        shape_family: String,
    },
    /// Aplica elevación (sombra) al hijo, con familia de forma opcional
    ElevatedBox {
        child: Box<Layout>,
        level: u8,
        shape_family: String,
    },
    /// Aplica un estado visual al hijo (hover, pressed, etc.)
    StateLayerBox {
        child: Box<Layout>,
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
        child: Box<Layout>,
        amount: f64,
    },
    /// Hace que el hijo se expanda para llenar el espacio disponible
    Expanded {
        child: Box<Layout>,
    },
    /// Centra al hijo en el eje transversal
    Centered {
        child: Box<Layout>,
    },
    Button {
        texto: String,
        callback: String,
    },
    TextInput {
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
        wrap: bool,
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
        child: Box<Layout>,
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
        size: FabSize,
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
        on_remove: Option<String>,
    },

    // ─── Inputs Material Design 3 ────────────────────────────────
    MaterialTextField {
        variable: String,
        label: String,
        placeholder: String,
        variant: TextFieldVariant,
        multiline: bool,
        error: String,
        counter: bool,
    },
    MaterialPasswordField {
        variable: String,
        label: String,
        visible: bool,
    },
    MaterialNumberField {
        variable: String,
        label: String,
        min: f64,
        max: f64,
        decimales: i32,
    },
    MaterialSearchField {
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
        label: String,
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
        label: String,
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
        child: Box<Layout>,
        variant: CardVariant,
        on_click: Option<String>,
        seleccionado: bool,
    },
    MaterialListItem {
        leading: Option<Box<Layout>>,
        titulo: String,
        subtitulo: Option<String>,
        trailing: Option<Box<Layout>>,
        on_click: Option<String>,
    },
    MaterialList {
        items: Vec<Layout>,
        dividers: bool,
    },
    MaterialListControl {
        items: Vec<Layout>,
        control_type: String,
        variables: Vec<String>,
    },
    MaterialListSelection {
        items: Vec<Layout>,
        seleccion: Vec<bool>,
        callback: String,
        multiple: bool,
    },
    /// Lista dinámica: renderiza los items de una variable (JSON array)
    /// usando una función Forja como callback de render (lista_dinamica).
    DynamicList {
        variable: String,
        item_fn: String,
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
    MaterialDataTable {
        columnas: Vec<String>,
        filas: Vec<Vec<String>>,
        ordenable: bool,
        seleccionable: bool,
        col_orden: usize,
        orden_asc: bool,
    },
    MaterialSurface {
        child: Box<Layout>,
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
    LinearProgressValue(f64),

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
        child: Box<Layout>,
        on_close: String,
    },

    // Bottom Sheets
    BottomSheet {
        child: Box<Layout>,
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
        child: Box<Layout>,
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
        current_var: String,
        /// Variable para pila de navegación (historial)
        history_var: String,
        /// Tipo de navegación visual
        nav_type: NavigatorType,
        /// Animación de transición
        anim: NavigatorAnim,
        /// Callback opcional para cuando cambia la pantalla
        on_change: Option<String>,
    },
    NavigationBar {
        items: Vec<NavItem>,
        seleccion: usize,
        /// Variable del `.fa` que guarda el índice seleccionado (2º argumento);
        /// sin esto la barra escribía en una variable con el nombre del callback
        /// y la página nunca cambiaba.
        variable_seleccion: Option<String>,
        on_change: String,
    },
    NavigationRail {
        items: Vec<NavItem>,
        seleccion: usize,
        variable_seleccion: Option<String>,
        on_change: String,
        extended: bool, // muestra labels
    },
    NavigationDrawer {
        items: Vec<NavItem>,
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
    SearchBar {
        placeholder: String,
        on_search: String,
        variable: String,
    },
    SearchView {
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
        size: f64,
        indeterminado: bool,
    },
    /// Badge (distintivo) sobre un hijo
    Badge {
        child: Box<Layout>,
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
        child: Box<Layout>,
        visible: String,
        duracion: f64,
    },
    /// Efecto ripple (onda al hacer clic)
    RippleEffect {
        child: Box<Layout>,
        color: String,
    },

    // ═══════════════════════════════════════════════════════════════════
    // INTERACCIONES (Pull-to-refresh, Swipe-to-dismiss)
    // ═══════════════════════════════════════════════════════════════════
    /// Pull-to-refresh: envuelve un hijo con capacidad de recargar al tirar hacia abajo
    PullToRefresh {
        child: Box<Layout>,
        callback: String,
        refreshing: String,
    },
    /// Swipe-to-dismiss: deslizar para descartar con opción de deshacer
    SwipeToDismiss {
        child: Box<Layout>,
        on_dismiss: String,
        label: String,
        dismissed: String,
    },

    /// Pinch-to-zoom: zoom por pellizco multi-touch
    PinchZoom {
        child: Box<Layout>,
        min_scale: f64,
        max_scale: f64,
    },
    /// Rotate: rotación por gesto multi-touch
    Rotate {
        child: Box<Layout>,
    },

    // ═══════════════════════════════════════════════════════════════════
    // GRÁFICOS (Fase 9)
    // ═══════════════════════════════════════════════════════════════════
    /// Gráfico de líneas
    LineChart {
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
    FilePicker {
        tipos: Vec<String>,
        multiple: bool,
        callback: String,
    },

    // ═══════════════════════════════════════════════════════════════════
    // EXPRESSIVE (Fase 10)
    // ═══════════════════════════════════════════════════════════════════
    /// Tarjeta con efecto vidrio (Glassmorphism)
    GlassCard {
        child: Box<Layout>,
        blur: f64,
        opacity: f64,
    },
    /// Caja con gradiente
    GradientBox {
        child: Box<Layout>,
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
        child: Box<Layout>,
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
pub fn extraer_layout(decls: &[Declaracion], store: &mut VariableStore) -> Layout {
    for decl in decls {
        if let Declaracion::Funcion { nombre, cuerpo, .. } = decl {
            if nombre == "main" {
                let children = layouts_de_bloque(cuerpo, store, decls);
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
    store: &mut VariableStore,
    decls: &[Declaracion],
) -> Vec<Layout> {
    cuerpo
        .iter()
        .flat_map(|d| expandir_declaracion_layout(d, store, decls, 0))
        .filter_map(|expr| expr_a_layout(&expr))
        .collect()
}

/// Cuántas llamadas a funciones de usuario se inlinean como máximo.
const MAX_INLINE_LAYOUT: u8 = 8;

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
    store: &mut VariableStore,
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
            store,
            decls,
            depth,
        ),
        Declaracion::Expresion(expr) => expandir_expresion_layout(expr, store, decls, depth),
        // Las funciones que arman layouts suelen devolverlo con `retornar`:
        // el valor es la expresión de layout (y en una función de valor, el
        // texto literal que después lee `extraer_texto`).
        Declaracion::Retornar { valor: Some(expr) } => {
            expandir_expresion_layout(expr, store, decls, depth)
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
            store,
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
///    cuerpo, con tope [`MAX_INLINE_LAYOUT`] para no colgarse con recursión.
/// 3. Cualquier otra expresión se devuelve tal cual, expandiendo sus argumentos.
fn expandir_expresion_layout(
    expr: &Expresion,
    store: &mut VariableStore,
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
                crate::evaluador::evaluar_condicion(condicion, store, decls).unwrap_or(true);
            let rama = if condicion_verdadera {
                Some(bloque_verdadero)
            } else {
                bloque_falso.as_ref()
            };
            match rama {
                Some(bloque) => bloque
                    .iter()
                    .flat_map(|d| expandir_declaracion_layout(d, store, decls, depth))
                    .collect(),
                None => Vec::new(),
            }
        }
        Expresion::LlamadaFuncion { nombre, argumentos } => {
            let argumentos: Vec<Expresion> = argumentos
                .iter()
                .flat_map(|a| expandir_expresion_layout(a, store, decls, depth))
                .collect();
            // Función de usuario: puede ser de **valor** (`titulo_categoria(pagina)`)
            // o armar un **layout** (`seccion_botones(...)`). Se intenta primero
            // evaluarla: si devuelve un valor, se reemplaza la llamada por el
            // literal; si no (las llamadas a widgets no son evaluables), se inlinea
            // el cuerpo, que es lo que espera `expr_a_layout`.
            if depth < MAX_INLINE_LAYOUT {
                if let Some(cuerpo) = cuerpo_de_funcion(nombre, decls) {
                    let args: Result<Vec<ValorGUI>, String> = argumentos
                        .iter()
                        .map(|a| crate::evaluador::evaluar_valor(a, store, decls))
                        .collect();
                    if let Ok(args) = args {
                        if let Ok(valor) =
                            crate::evaluador::ejecutar_funcion(nombre, &args, decls, store)
                        {
                            return vec![valor_a_expresion(&valor)];
                        }
                    }
                    return cuerpo
                        .iter()
                        .flat_map(|d| expandir_declaracion_layout(d, store, decls, depth + 1))
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
                .flat_map(|a| expandir_expresion_layout(a, store, decls, depth))
                .collect(),
        )],
        Expresion::Grupo(inner) => {
            let mut expandido = expandir_expresion_layout(inner, store, decls, depth);
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
    fn(&[Expresion], &HashMap<String, ValorGUI>, &VariableStore, &[Declaracion]) -> Vec<Layout>;

/// 5.8: Wrapper que adapta `procesar_args` al signature de `ChildProcessor`.
/// Disponible para futura migración de `expr_a_layout` a usar `parse_layout_call`.
#[allow(dead_code)]
fn procesar_args_wrapper(
    args: &[Expresion],
    _params: &HashMap<String, ValorGUI>,
    _store: &VariableStore,
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
    store: &VariableStore,
    declaraciones: &[Declaracion],
    process_children: ChildProcessor,
) -> Vec<Layout> {
    match arg {
        Expresion::Arreglo(exprs) => process_children(exprs, params, store, declaraciones),
        _ => Vec::new(),
    }
}

/// 5.8: Función compartida para parsear llamadas de función → Layout.
/// Maneja los casos comunes entre `expr_a_layout` y `expr_a_layout_item`.
/// Retorna Some(Layout) si es un caso compartido, None si el caller debe manejarlo.
fn parse_layout_call(
    nombre: &str,
    argumentos: &[Expresion],
    params: &HashMap<String, ValorGUI>,
    store: &VariableStore,
    declaraciones: &[Declaracion],
    process_children: ChildProcessor,
) -> Option<Layout> {
    match nombre {
        "columna" | "gui_columna" => Some(Layout::Column {
            children: process_children(argumentos, params, store, declaraciones),
            gap: 0.0,
            alignment: "start".to_string(),
        }),
        "fila" | "gui_fila" => Some(Layout::Row {
            children: process_children(argumentos, params, store, declaraciones),
            gap: 0.0,
            alignment: "start".to_string(),
        }),
        "columna_con_gap" | "column_with_gap" => {
            let children = argumentos
                .first()
                .map(|a| {
                    extract_children_from_array(a, params, store, declaraciones, process_children)
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
                    extract_children_from_array(a, params, store, declaraciones, process_children)
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
                        Some(process_children(exprs, params, store, declaraciones))
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
                        Some(process_children(exprs, params, store, declaraciones))
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
                    Some(Layout::TextInput {
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
                    Some(Layout::TextInput {
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
                "pila" | "gui_pila" | "zstack" => Some(Layout::ZStack(procesar_args(argumentos))),
                "desplazable" | "gui_desplazable" | "scroll" => argumentos
                    .first()
                    .and_then(|a| expr_a_layout(a))
                    .map(|child| Layout::Portal(Box::new(child))),
                "contenedor" | "container" | "caja" => {
                    let child = argumentos.first().and_then(|a| expr_a_layout(a));
                    let max_width = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            _ => None,
                        })
                        .unwrap_or(400.0);
                    child.map(|c| Layout::Container {
                        child: Box::new(c),
                        max_width,
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
                    let child = argumentos.get(1).and_then(expr_a_layout);
                    child.map(|c| Layout::ThemeProvider {
                        child: Box::new(c),
                        theme: seed,
                    })
                }
                // ─── Diseños predeterminados (claro/oscuro) ──────────
                // tema_fluent(hijo) / tema_fluent_oscuro(hijo)
                "tema_fluent" | "fluent" | "tema_windows11" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::ThemeProvider {
                        child: Box::new(c),
                        theme: "fluent:light".to_string(),
                    })
                }
                "tema_fluent_oscuro" | "fluent_oscuro" | "tema_windows11_oscuro" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::ThemeProvider {
                        child: Box::new(c),
                        theme: "fluent:dark".to_string(),
                    })
                }
                // tema_liquid_glass(hijo) / tema_liquid_glass_oscuro(hijo)
                "tema_liquid_glass" | "tema_vidrio" | "liquid_glass" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::ThemeProvider {
                        child: Box::new(c),
                        theme: "liquid_glass:light".to_string(),
                    })
                }
                "tema_liquid_glass_oscuro" | "tema_vidrio_oscuro" | "liquid_glass_oscuro" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::ThemeProvider {
                        child: Box::new(c),
                        theme: "liquid_glass:dark".to_string(),
                    })
                }
                // Colores del tema (envuelven un hijo con un color role)
                "color_primario" | "color_primary" => wrap_with_color_role(argumentos, "primary"),
                "color_sobre_primario" | "color_on_primary" => {
                    wrap_with_color_role(argumentos, "on_primary")
                }
                "color_primario_contenedor" | "color_primary_container" => {
                    wrap_with_color_role(argumentos, "primary_container")
                }
                "color_sobre_primario_contenedor" | "color_on_primary_container" => {
                    wrap_with_color_role(argumentos, "on_primary_container")
                }
                "color_secundario" | "color_secondary" => {
                    wrap_with_color_role(argumentos, "secondary")
                }
                "color_sobre_secundario" | "color_on_secondary" => {
                    wrap_with_color_role(argumentos, "on_secondary")
                }
                "color_secundario_contenedor" | "color_secondary_container" => {
                    wrap_with_color_role(argumentos, "secondary_container")
                }
                "color_sobre_secundario_contenedor" | "color_on_secondary_container" => {
                    wrap_with_color_role(argumentos, "on_secondary_container")
                }
                "color_terciario" | "color_tertiary" => {
                    wrap_with_color_role(argumentos, "tertiary")
                }
                "color_sobre_terciario" | "color_on_tertiary" => {
                    wrap_with_color_role(argumentos, "on_tertiary")
                }
                "color_error" | "color_error_role" => wrap_with_color_role(argumentos, "error"),
                "color_sobre_error" | "color_on_error" => {
                    wrap_with_color_role(argumentos, "on_error")
                }
                "color_superficie" | "color_surface" => wrap_with_color_role(argumentos, "surface"),
                "color_sobre_superficie" | "color_on_surface" => {
                    wrap_with_color_role(argumentos, "on_surface")
                }
                "color_fondo" | "color_background" => {
                    wrap_with_color_role(argumentos, "background")
                }
                "color_sobre_fondo" | "color_on_background" => {
                    wrap_with_color_role(argumentos, "on_background")
                }
                "color_perfil" | "color_outline" => wrap_with_color_role(argumentos, "outline"),

                // Estilos tipográficos (15 estilos Material You)
                "texto_grande" | "display_large" => styled_label(argumentos, "display_large"),
                "texto_mediano" | "display_medium" => styled_label(argumentos, "display_medium"),
                "texto_pequeño" | "texto_pequeno" | "display_small" => {
                    styled_label(argumentos, "display_small")
                }
                "titular_grande" | "headline_large" => styled_label(argumentos, "headline_large"),
                "titular_mediano" | "headline_medium" => {
                    styled_label(argumentos, "headline_medium")
                }
                "titular_pequeño" | "headline_small" => styled_label(argumentos, "headline_small"),
                "encabezado_grande" | "title_large" => styled_label(argumentos, "title_large"),
                "encabezado_mediano" | "title_medium" => styled_label(argumentos, "title_medium"),
                "encabezado_pequeño" | "title_small" => styled_label(argumentos, "title_small"),
                "cuerpo_grande" | "body_large" => styled_label(argumentos, "body_large"),
                "cuerpo_mediano" | "body_medium" => styled_label(argumentos, "body_medium"),
                "cuerpo_pequeño" | "body_small" => styled_label(argumentos, "body_small"),
                "etiqueta_grande" | "label_large" => styled_label(argumentos, "label_large"),
                "etiqueta_mediana" | "label_medium" => styled_label(argumentos, "label_medium"),
                "etiqueta_pequeña" | "label_small" => styled_label(argumentos, "label_small"),

                // Formas (border-radius)
                "esquinas_pequeñas" | "shape_small" => wrap_with_shape(argumentos, "small"),
                "esquinas_medianas" | "shape_medium" => wrap_with_shape(argumentos, "medium"),
                "esquinas_grandes" | "shape_large" => wrap_with_shape(argumentos, "large"),
                "esquinas_completas" | "shape_full" => wrap_with_shape(argumentos, "full"),

                // Layout avanzado
                "relleno" | "padding" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    let amount = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            _ => None,
                        })
                        .unwrap_or(8.0);
                    child.map(|c| Layout::Padding {
                        child: Box::new(c),
                        amount,
                    })
                }
                "expansor" | "expanded" => argumentos
                    .first()
                    .and_then(expr_a_layout)
                    .map(|c| Layout::Expanded { child: Box::new(c) }),
                "centrado" | "centered" | "center" => argumentos
                    .first()
                    .and_then(expr_a_layout)
                    .map(|c| Layout::Centered { child: Box::new(c) }),
                "sombra" | "shadow" | "elevated" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    let level = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as u8),
                            _ => None,
                        })
                        .unwrap_or(1);
                    let shape_family = argumentos
                        .get(2)
                        .and_then(|a| match a {
                            Expresion::LiteralTexto(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "surface".to_string());
                    child.map(|c| Layout::ElevatedBox {
                        child: Box::new(c),
                        level,
                        shape_family,
                    })
                }
                "adaptable" | "responsive" => {
                    let compact = argumentos.get(0).and_then(expr_a_layout);
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
                    let child = argumentos.first().and_then(expr_a_layout);
                    let ratio = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(1.0);
                    child.map(|c| Layout::AspectRatioBox {
                        child: Box::new(c),
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
                    let wrap = argumentos
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
                        wrap,
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
                        size: FabSize::Medium,
                        texto_extendido: None,
                    })
                }
                "fab_pequeño" | "fab_pequeno" | "fab_small" => {
                    let icono = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::FAB {
                        icono,
                        callback,
                        size: FabSize::Small,
                        texto_extendido: None,
                    })
                }
                "fab_grande" | "fab_large" => {
                    let icono = extraer_texto(argumentos, 0);
                    let callback = extraer_callback(argumentos, 1);
                    Some(Layout::FAB {
                        icono,
                        callback,
                        size: FabSize::Large,
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
                        size: FabSize::Medium,
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
                        on_remove: None,
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
                        on_remove: None,
                    })
                }
                "subconjunto_entrada" | "chip_input" => {
                    let texto = extraer_texto(argumentos, 0);
                    let cb_fn = extraer_callback(argumentos, 1);
                    let cb_clone = cb_fn.clone();
                    Some(Layout::Chip {
                        texto,
                        callback: cb_fn,
                        variant: ChipVariant::Input,
                        activo: true,
                        on_remove: Some(cb_clone),
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
                        on_remove: None,
                    })
                }

                // ─── Inputs Material Design 3 ──────────────────────────

                // Text Fields
                "campo_texto" | "campo_texto_error" => {
                    let variable = extraer_texto(argumentos, 0);
                    let label = extraer_texto(argumentos, 1);
                    let error = if nombre == "campo_texto_error" {
                        extraer_texto(argumentos, 2)
                    } else {
                        String::new()
                    };
                    Some(Layout::MaterialTextField {
                        variable,
                        label,
                        placeholder: String::new(),
                        variant: TextFieldVariant::Filled,
                        multiline: false,
                        error,
                        counter: false,
                    })
                }
                "campo_perfilado" => {
                    let variable = extraer_texto(argumentos, 0);
                    let label = extraer_texto(argumentos, 1);
                    Some(Layout::MaterialTextField {
                        variable,
                        label,
                        placeholder: String::new(),
                        variant: TextFieldVariant::Outlined,
                        multiline: false,
                        error: String::new(),
                        counter: false,
                    })
                }
                "campo_email" => {
                    let variable = extraer_texto(argumentos, 0);
                    let label = extraer_texto(argumentos, 1);
                    Some(Layout::MaterialTextField {
                        variable,
                        label,
                        placeholder: "email@ejemplo.com".to_string(),
                        variant: TextFieldVariant::Outlined,
                        multiline: false,
                        error: String::new(),
                        counter: false,
                    })
                }
                "campo_telefono" => {
                    let variable = extraer_texto(argumentos, 0);
                    let label = extraer_texto(argumentos, 1);
                    Some(Layout::MaterialTextField {
                        variable,
                        label,
                        placeholder: "+54 11 1234-5678".to_string(),
                        variant: TextFieldVariant::Outlined,
                        multiline: false,
                        error: String::new(),
                        counter: false,
                    })
                }
                "campo_url" => {
                    let variable = extraer_texto(argumentos, 0);
                    let label = extraer_texto(argumentos, 1);
                    Some(Layout::MaterialTextField {
                        variable,
                        label,
                        placeholder: "https://".to_string(),
                        variant: TextFieldVariant::Outlined,
                        multiline: false,
                        error: String::new(),
                        counter: false,
                    })
                }
                "campo_contraseña" | "campo_contrasena" => {
                    let variable = extraer_texto(argumentos, 0);
                    let label = extraer_texto(argumentos, 1);
                    Some(Layout::MaterialPasswordField {
                        variable,
                        label,
                        visible: false,
                    })
                }
                "campo_numero" => {
                    let variable = extraer_texto(argumentos, 0);
                    let label = extraer_texto(argumentos, 1);
                    let min = extraer_f64(argumentos, 2);
                    let max = extraer_f64(argumentos, 3);
                    Some(Layout::MaterialNumberField {
                        variable,
                        label,
                        min,
                        max,
                        decimales: 0,
                    })
                }
                "campo_busqueda" | "campo_search" => {
                    let variable = extraer_texto(argumentos, 0);
                    Some(Layout::MaterialSearchField {
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
                    let label = extraer_texto(argumentos, 2);
                    Some(Layout::MaterialSelect {
                        opciones,
                        seleccionada,
                        label,
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
                    let label = extraer_texto(argumentos, 0);
                    let variable = extraer_texto(argumentos, 1);
                    Some(Layout::MaterialSwitch { label, variable })
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
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::MaterialCard {
                        child: Box::new(c),
                        variant: CardVariant::Filled,
                        on_click: None,
                        seleccionado: false,
                    })
                }
                "tarjeta_elevada" | "elevated_card" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::MaterialCard {
                        child: Box::new(c),
                        variant: CardVariant::Elevated,
                        on_click: None,
                        seleccionado: false,
                    })
                }
                "tarjeta_perfilada" | "outlined_card" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::MaterialCard {
                        child: Box::new(c),
                        variant: CardVariant::Outlined,
                        on_click: None,
                        seleccionado: false,
                    })
                }
                "tarjeta_seleccionable" | "selectable_card" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    let _variable = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    child.map(|c| Layout::MaterialCard {
                        child: Box::new(c),
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
                    Some(Layout::MaterialListItem {
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
                    Some(Layout::MaterialListItem {
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
                    Some(Layout::MaterialList {
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
                    Some(Layout::MaterialList {
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
                    let control_type = extraer_texto(argumentos, 1);
                    let variables = extraer_array_strings(argumentos, 2);
                    Some(Layout::MaterialListControl {
                        items,
                        control_type,
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
                    Some(Layout::MaterialListSelection {
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
                    let item_fn = argumentos
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
                    Some(Layout::DynamicList { variable, item_fn })
                }

                // ─── Data Tables ─────────────────────────────────
                "tabla_datos" | "data_table" => {
                    let columnas = extraer_array_strings(argumentos, 0);
                    let filas = extraer_array_arrays_strings(argumentos, 1);
                    Some(Layout::MaterialDataTable {
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
                    Some(Layout::MaterialDataTable {
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
                    Some(Layout::MaterialDataTable {
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
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::MaterialSurface {
                        child: Box::new(c),
                        color_role: "surface".to_string(),
                    })
                }
                "superficie_tonal" | "tonal_surface" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::MaterialSurface {
                        child: Box::new(c),
                        color_role: "tonal".to_string(),
                    })
                }
                "andamio" | "scaffold" => {
                    let top = argumentos.get(0).and_then(expr_a_layout).map(Box::new);
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
                    let child = argumentos.get(1).and_then(expr_a_layout);
                    child.map(|c| Layout::DialogCustom {
                        titulo,
                        child: Box::new(c),
                        on_close: String::new(),
                    })
                }
                "dialogo_completo" | "dialog_full" => {
                    let titulo = extraer_texto(argumentos, 0);
                    let child = argumentos.get(1).and_then(expr_a_layout);
                    child.map(|c| Layout::DialogCustom {
                        titulo,
                        child: Box::new(c),
                        on_close: String::new(),
                    })
                }

                // ─── FEEDBACK: Bottom Sheets ─────────────────────────
                "hoja_inferior" | "bottom_sheet" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    let visible = extraer_texto(argumentos, 1);
                    child.map(|c| Layout::BottomSheet {
                        child: Box::new(c),
                        variant: SheetVariant::Standard,
                        visible,
                        on_dismiss: None,
                    })
                }
                "hoja_inferior_modal" | "modal_sheet" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    let visible = extraer_texto(argumentos, 1);
                    child.map(|c| Layout::BottomSheet {
                        child: Box::new(c),
                        variant: SheetVariant::Modal,
                        visible,
                        on_dismiss: None,
                    })
                }
                "hoja_inferior_grande" | "expanded_sheet" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    let visible = extraer_texto(argumentos, 1);
                    child.map(|c| Layout::BottomSheet {
                        child: Box::new(c),
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
                    let child = argumentos.first().and_then(expr_a_layout);
                    let texto = extraer_texto(argumentos, 1);
                    child.map(|c| Layout::Tooltip {
                        child: Box::new(c),
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
                    let current_var = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => "pantalla_actual".to_string(),
                        })
                        .unwrap_or_else(|| "pantalla_actual".to_string());
                    let current_var_clone = current_var.clone();
                    let nav_type_str = argumentos
                        .get(2)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => "ninguno".to_string(),
                        })
                        .unwrap_or_else(|| "ninguno".to_string());
                    let nav_type = match nav_type_str.to_lowercase().as_str() {
                        "barra" | "bottom" | "inferior" => NavigatorType::BottomBar,
                        "riel" | "rail" | "lateral" => NavigatorType::Rail,
                        "pestañas" | "tabs" | "tab" => NavigatorType::Tabs,
                        "cajon" | "drawer" => NavigatorType::Drawer,
                        _ => NavigatorType::None,
                    };
                    // Arg 3 opcional: callback on_change (ej: &cambiar_pantalla)
                    let on_change = argumentos
                        .get(3)
                        .map(|_| extraer_callback(argumentos, 3))
                        .filter(|s| !s.is_empty());
                    // Arg 4 opcional: tipo de animación ("fade", "slide", "ninguno")
                    let anim_type = argumentos
                        .get(4)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => s.to_lowercase(),
                            _ => "ninguno".to_string(),
                        })
                        .unwrap_or_else(|| "ninguno".to_string());
                    let anim = match anim_type.as_str() {
                        "fade" => NavigatorAnim::Fade,
                        "slide" => NavigatorAnim::Slide,
                        _ => NavigatorAnim::None,
                    };
                    if screens.is_empty() {
                        Some(Layout::Spacer(0.0))
                    } else {
                        Some(Layout::Navigator {
                            screens,
                            current_var: current_var_clone,
                            history_var: format!("{}_historial", current_var),
                            nav_type,
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
                    Some(Layout::SearchBar {
                        placeholder,
                        on_search: String::new(),
                        variable,
                    })
                }

                // ─── item_navegacion (helper, se procesa en extraer_nav_items) ──
                "item_navegacion" | "nav_item" => {
                    // item_navegacion se procesa dentro de extraer_nav_items,
                    // pero como llamada directa devolvemos un placeholder
                    let icono = extraer_texto(argumentos, 0);
                    let label = extraer_texto(argumentos, 1);
                    Some(Layout::StyledLabel {
                        texto: format!("{} {}", icono, label),
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
                    let size = argumentos
                        .get(1)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(40.0);
                    Some(Layout::CircularProgress {
                        variable,
                        size,
                        indeterminado: false,
                    })
                }
                "circulo_progreso_indeterminado" | "circular_progress_indeterminate" => {
                    let size = argumentos
                        .first()
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(40.0);
                    Some(Layout::CircularProgress {
                        variable: String::new(),
                        size,
                        indeterminado: true,
                    })
                }

                // ─── Badge ───────────────────────────────────────────
                "distintivo" | "badge" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    let valor = argumentos
                        .get(1)
                        .map(|a| match a {
                            Expresion::LiteralTexto(s) => Some(s.clone()),
                            Expresion::Identificador { nombre: s, .. } => Some(s.clone()),
                            Expresion::LiteralNumero(n) => Some(n.to_string()),
                            _ => None,
                        })
                        .unwrap_or(None);
                    child.map(|c| Layout::Badge {
                        child: Box::new(c),
                        valor,
                        dot: false,
                    })
                }
                "distintivo_punto" | "badge_dot" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::Badge {
                        child: Box::new(c),
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
                    let child = argumentos.first().and_then(expr_a_layout);
                    let visible = extraer_texto(argumentos, 1);
                    let duracion = argumentos
                        .get(2)
                        .and_then(|a| match a {
                            Expresion::LiteralNumero(n) => Some(*n as f64),
                            Expresion::LiteralDecimal(f) => Some(*f),
                            _ => None,
                        })
                        .unwrap_or(0.3);
                    child.map(|c| Layout::FadeTransition {
                        child: Box::new(c),
                        visible,
                        duracion,
                    })
                }

                // ─── RippleEffect ────────────────────────────────────
                "efecto_onda" | "ripple_effect" | "ripple" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    let color = extraer_texto(argumentos, 1);
                    child.map(|c| Layout::RippleEffect {
                        child: Box::new(c),
                        color: if color.is_empty() {
                            "primary".to_string()
                        } else {
                            color
                        },
                    })
                }

                // ─── PullToRefresh ──────────────────────────────────
                "pull_to_refresh" | "pulltorefresh" | "tirar_recargar" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    let callback = extraer_callback(argumentos, 1);
                    let refreshing = argumentos
                        .get(2)
                        .map(|a| match a {
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    child.map(|c| Layout::PullToRefresh {
                        child: Box::new(c),
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
                    let child = argumentos.first().and_then(expr_a_layout);
                    let on_dismiss = extraer_callback(argumentos, 1);
                    let label = extraer_texto(argumentos, 2);
                    let dismissed = argumentos
                        .get(3)
                        .map(|a| match a {
                            Expresion::Identificador { nombre: s, .. } => s.clone(),
                            Expresion::LiteralTexto(s) => s.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();
                    child.map(|c| Layout::SwipeToDismiss {
                        child: Box::new(c),
                        on_dismiss: if on_dismiss.is_empty() {
                            String::new()
                        } else {
                            on_dismiss
                        },
                        label: if label.is_empty() {
                            "Descartar".to_string()
                        } else {
                            label
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
                    let child = argumentos.first().and_then(expr_a_layout);
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
                    child.map(|c| Layout::PinchZoom {
                        child: Box::new(c),
                        min_scale,
                        max_scale,
                    })
                }

                // ─── Rotate ───────────────────────────────────────
                "rotar" | "rotate" | "girar" => {
                    let child = argumentos.first().and_then(expr_a_layout);
                    child.map(|c| Layout::Rotate { child: Box::new(c) })
                }

                // ═══════════════════════════════════════════════════════
                // GRÁFICOS (Fase 9)
                // ═══════════════════════════════════════════════════════

                // ─── LineChart ────────────────────────────────────────
                "gráfico_linea" | "grafico_linea" | "line_chart" | "chart_line" => {
                    let datos = extraer_array_f64(argumentos, 0);
                    let etiquetas = extraer_array_strings(argumentos, 1);
                    let color = extraer_texto(argumentos, 2);
                    Some(Layout::LineChart {
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
                    Some(Layout::FilePicker {
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
                    let child = argumentos.first().and_then(expr_a_layout);
                    let blur = extraer_f64(argumentos, 1);
                    let opacity = extraer_f64(argumentos, 2);
                    child.map(|c| Layout::GlassCard {
                        child: Box::new(c),
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
                    let child = argumentos.first().and_then(expr_a_layout);
                    let colores = extraer_array_strings(argumentos, 1);
                    let direccion = extraer_texto(argumentos, 2);
                    child.map(|c| Layout::GradientBox {
                        child: Box::new(c),
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
                    let child = argumentos.first().and_then(expr_a_layout);
                    let color = extraer_texto(argumentos, 1);
                    let ancho = extraer_f64(argumentos, 2);
                    child.map(|c| Layout::GlowBorder {
                        child: Box::new(c),
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
fn wrap_with_color_role(args: &[Expresion], role: &str) -> Option<Layout> {
    args.first()
        .and_then(expr_a_layout)
        .map(|c| Layout::ColoredBox {
            child: Box::new(c),
            color_role: role.to_string(),
        })
}

/// Crea una etiqueta con estilo tipográfico predefinido
fn styled_label(args: &[Expresion], style: &str) -> Option<Layout> {
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
    store: &VariableStore,
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
            store.get(nombre).map(|v| ValorGUI::from_serde(&v))
        }
        Expresion::Index { objeto, indice } => {
            let obj = expr_a_valor(objeto, params, store, declaraciones)?;
            let idx = expr_a_valor(indice, params, store, declaraciones)?;
            match obj {
                ValorGUI::Mapa(m) => {
                    let key = match idx {
                        ValorGUI::Texto(s) => s,
                        ValorGUI::Entero(i) => i.to_string(),
                        other => other.to_string(),
                    };
                    m.get(&key).cloned()
                }
                ValorGUI::Texto(s) => {
                    // Objeto o array serializado (JSON)
                    let key = idx.to_string();
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                        match v {
                            serde_json::Value::Object(obj) => {
                                if let Some(item) = obj.get(&key) {
                                    return Some(ValorGUI::from_serde(item));
                                }
                            }
                            serde_json::Value::Array(arr) => {
                                if let Ok(pos) = key.parse::<usize>() {
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
                let a = expr_a_texto(izquierda, params, store, declaraciones);
                let b = expr_a_texto(derecha, params, store, declaraciones);
                Some(ValorGUI::Texto(a + &b))
            } else {
                let a = expr_a_valor(izquierda, params, store, declaraciones)?;
                let b = expr_a_valor(derecha, params, store, declaraciones)?;
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
                    let var_name = expr_a_texto(arg, params, store, declaraciones);
                    if !var_name.is_empty() {
                        if let Some(v) = store.get(&var_name) {
                            return Some(ValorGUI::from_serde(&v));
                        }
                    }
                }
                return None;
            }
            // Convertir argumentos y ejecutar función (helper o del AST)
            let mut args = Vec::new();
            for a in argumentos {
                match expr_a_valor(a, params, store, declaraciones) {
                    Some(v) => args.push(v),
                    None => return None,
                }
            }
            let mut store_clone = store.clone();
            crate::evaluador::ejecutar_funcion(nombre, &args, declaraciones, &mut store_clone).ok()
        }
        _ => None,
    }
}

/// Evalúa una expresión a texto (para etiquetas dinámicas).
pub(crate) fn expr_a_texto(
    expr: &Expresion,
    params: &HashMap<String, ValorGUI>,
    store: &VariableStore,
    declaraciones: &[Declaracion],
) -> String {
    expr_a_valor(expr, params, store, declaraciones)
        .map(|v| v.to_display())
        .unwrap_or_default()
}

/// Convierte una expresión de texto (o layout) a un string simple para
/// titulo/subtitulo de listas: extrae el primer texto de una columna.
fn texto_item_columna(
    expr: &Expresion,
    params: &HashMap<String, ValorGUI>,
    store: &VariableStore,
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
                                    textos.push(expr_a_texto(t, params, store, declaraciones));
                                }
                            }
                        }
                    }
                    return textos.join(" | ");
                }
            }
            // etiqueta(x), texto_pequeno(x) directo
            if let Some(t) = argumentos.first() {
                return expr_a_texto(t, params, store, declaraciones);
            }
            String::new()
        }
        _ => expr_a_texto(expr, params, store, declaraciones),
    }
}

/// Evalúa el layout de un item de lista dinámica, resolviendo accesos a mapa
/// (item["clave"]), funciones helper (formatear_fecha, formatear_monto) y
/// etiquetas dinámicas usando los parámetros del item.
pub(crate) fn expr_a_layout_item(
    expr: &Expresion,
    params: &HashMap<String, ValorGUI>,
    store: &VariableStore,
    declaraciones: &[Declaracion],
) -> Option<Layout> {
    match expr {
        Expresion::LlamadaFuncion { nombre, argumentos } => {
            // 5.8: Intentar parse_layout_call para casos compartidos primero
            if let Some(layout) = parse_layout_call(
                nombre,
                argumentos,
                params,
                store,
                declaraciones,
                procesar_args_item,
            ) {
                return Some(layout);
            }
            // Casos específicos de expr_a_layout_item
            match nombre.as_str() {
                "elemento_lista_doble" | "two_line_list_item" => {
                    let leading = argumentos
                        .first()
                        .and_then(|a| expr_a_layout_item(a, params, store, declaraciones))
                        .map(Box::new);
                    let titulo = texto_item_columna(
                        argumentos.get(1).unwrap_or(argumentos.first()?),
                        params,
                        store,
                        declaraciones,
                    );
                    let subtitulo = argumentos
                        .get(2)
                        .map(|a| texto_item_columna(a, params, store, declaraciones));
                    let trailing = argumentos
                        .get(3)
                        .and_then(|a| expr_a_layout_item(a, params, store, declaraciones))
                        .map(Box::new);
                    Some(Layout::MaterialListItem {
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
                        .map(|a| expr_a_texto(a, params, store, declaraciones))
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
                        .map(|a| expr_a_texto(a, params, store, declaraciones))
                        .unwrap_or_default();
                    Some(Layout::ColoredLabel {
                        texto: String::new(),
                        color,
                    })
                }
                "etiqueta" | "label" | "text" | "escribir" => {
                    if let Some(arg) = argumentos.first() {
                        let txt = expr_a_texto(arg, params, store, declaraciones);
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
                        .map(|a| expr_a_texto(a, params, store, declaraciones))
                        .unwrap_or_default();
                    Some(Layout::StyledLabel {
                        texto: txt,
                        style: style.to_string(),
                    })
                }
                "tarjeta" | "card" => argumentos
                    .first()
                    .and_then(|a| expr_a_layout_item(a, params, store, declaraciones))
                    .map(|c| Layout::MaterialCard {
                        child: Box::new(c),
                        variant: CardVariant::Elevated,
                        on_click: None,
                        seleccionado: false,
                    }),
                "tarjeta_seleccionable" | "selectable_card" => argumentos
                    .first()
                    .and_then(|a| expr_a_layout_item(a, params, store, declaraciones))
                    .map(|c| Layout::MaterialCard {
                        child: Box::new(c),
                        variant: CardVariant::Outlined,
                        on_click: extraer_callback(argumentos, 1).into(),
                        seleccionado: false,
                    }),
                "barra_progreso" | "progress" | "progress_bar" => {
                    let val = argumentos
                        .first()
                        .and_then(|a| expr_a_valor(a, params, store, declaraciones))
                        .map(|v| v.to_f64())
                        .unwrap_or(0.0);
                    Some(Layout::LinearProgressValue(val))
                }
                _ => expr_a_layout(expr),
            }
        }
        _ => expr_a_layout(expr),
    }
}

/// procesar_args con contexto de item (para listas dinámicas).
fn procesar_args_item(
    args: &[Expresion],
    params: &HashMap<String, ValorGUI>,
    store: &VariableStore,
    declaraciones: &[Declaracion],
) -> Vec<Layout> {
    args.iter()
        .filter_map(|a| expr_a_layout_item(a, params, store, declaraciones))
        .collect()
}

/// Envuelve un child con una familia de forma
fn wrap_with_shape(args: &[Expresion], family: &str) -> Option<Layout> {
    args.first()
        .and_then(expr_a_layout)
        .map(|c| Layout::ShapedBox {
            child: Box::new(c),
            shape_family: family.to_string(),
        })
}

// ─── Funciones de tema/color → movidas a crate::helpers ──────────────

// ─── Layout → xilem widgets (con tema Material You) ──────────────

/// 5.10: Estima el ancho de un Layout basado en su tipo (para wrap de FlowLayout).
/// Retorna None si no se puede estimar (fallback: 100px).
fn estimate_child_width(layout: &Layout) -> Option<f64> {
    match layout {
        Layout::MaterialButton { texto, .. } => Some((texto.len() as f64) * 8.0 + 32.0),
        Layout::Chip { texto, .. } => Some((texto.len() as f64) * 7.0 + 24.0),
        Layout::Label { texto, .. } => Some((texto.len() as f64) * 8.0),
        Layout::MaterialCard { .. } => Some(300.0),
        Layout::Container { max_width, .. } => Some(*max_width),
        Layout::MaterialListItem { .. } => Some(280.0),
        Layout::Row { gap, .. } => Some(200.0 + gap),
        Layout::Column { gap, .. } => Some(200.0 + gap),
        _ => None,
    }
}

/// Convierte Layout a xilem usando AnyWidgetView para type erasure.
/// Recibe el tema actual como referencia para aplicar colores, tipografía y formas.
pub fn layout_a_view<'a>(
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
                widgets.push(layout_a_view(h, data, _prog, theme));
            }
            let ma = parse_alignment(alignment);
            // Ancho dinámico: en portrait usa el ancho completo, en landscape limita a 720dp máximo
            let ancho = match data.orientation {
                Orientation::Portrait => (data.window_width - 16.0).max(200.0),
                Orientation::Landscape => (data.window_width - 16.0).min(720.0).max(200.0),
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
                widgets.push(layout_a_view(h, data, _prog, theme));
            }
            // Ancho dinámico: en portrait usa el ancho completo, en landscape limita a 720dp máximo
            let ancho = match data.orientation {
                Orientation::Portrait => (data.window_width - 16.0).max(200.0),
                Orientation::Landscape => (data.window_width - 16.0).min(720.0).max(200.0),
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
            let ma = parse_alignment(alignment);
            let flex_eligible = children.iter().all(|h| {
                matches!(
                    h,
                    Layout::MaterialCard { .. } | Layout::MaterialListItem { .. }
                )
            });
            if flex_eligible && !children.is_empty() && children.len() <= 3 {
                // Casos con 1-3 hijos flex: usar tuplas con FlexExt
                match children.len() {
                    1 => {
                        let a = layout_a_view(&children[0], data, _prog, theme);
                        let row =
                            view::flex(Axis::Horizontal, (xilem::view::FlexExt::flex(a, 1.0),))
                                .main_axis_alignment(ma)
                                .gap(Length::px(*gap));
                        return Box::new(row) as Box<AnyWidgetView<AppStateNativo>>;
                    }
                    2 => {
                        let a = layout_a_view(&children[0], data, _prog, theme);
                        let b = layout_a_view(&children[1], data, _prog, theme);
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
                        let a = layout_a_view(&children[0], data, _prog, theme);
                        let b = layout_a_view(&children[1], data, _prog, theme);
                        let c = layout_a_view(&children[2], data, _prog, theme);
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
                widgets.push(layout_a_view(h, data, _prog, theme));
            }
            Box::new(
                view::flex(Axis::Horizontal, (widgets,))
                    .main_axis_alignment(ma)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(Length::px(*gap)),
            )
        }
        Layout::ZStack(hijos) => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in hijos {
                widgets.push(layout_a_view(h, data, _prog, theme));
            }
            Box::new(view::zstack((widgets,)))
        }
        Layout::Portal(child) => {
            let inner = layout_a_view(child, data, _prog, theme);
            // El área desplazable ocupa el ancho de la ventana (menos un margen
            // para el scrollbar). El contenido interno se estira con cross Fill.
            let ancho = (data.window_width - 18.0).max(200.0);
            Box::new(view::portal(
                view::sized_box(inner)
                    .width(Length::px(ancho))
                    .padding(16.0.px()),
            ))
        }
        Layout::Container { child, max_width } => {
            let inner = layout_a_view(child, data, _prog, theme);
            Box::new(view::sized_box(inner).width(Length::px(*max_width)))
        }
        Layout::Label { texto, es_variable } => {
            if *es_variable {
                let var_name = texto.clone();
                let txt = data.leer(&var_name).to_string();
                let gen = data.store.generation(&var_name);
                let color: forja_gui_rt::Color = get_color_role(&theme.scheme, "on_surface").into();
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
            let style = get_text_style(&theme.typography, "headline_medium");
            let color: forja_gui_rt::Color = get_color_role(&theme.scheme, "on_surface").into();
            Box::new(
                view::label(texto.clone())
                    .text_size(style.font_size as f32)
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
            let gen = data.store.generation(&var_name);
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
        Layout::TextInput {
            variable,
            multiline: _,
            placeholder,
        } => {
            let var_name = variable.clone();
            let val = data.leer(&var_name).to_string();
            let gen = data.store.generation(&var_name);
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
                        move |data: &mut AppStateNativo, new_val: String| {
                            data.escribir(&vn, ValorGUI::Texto(new_val));
                        },
                    )
                    .text_color(*tc);
                    if !pl.is_empty() {
                        ti = ti.placeholder(pl.as_str());
                    }
                    map_message(
                        ti,
                        |_data: &mut AppStateNativo, result: MessageResult<()>| match result {
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
            let gen = data.store.generation(&var_name);
            Box::new(memoize((gen, num), move |(_, n): &(u64, f64)| {
                view::progress_bar(Some(*n))
            }))
        }
        Layout::Slider { variable, min, max } => {
            let var_name = variable.clone();
            let val = data.leer(&var_name).to_f64();
            let gen = data.store.generation(&var_name);
            let mn = *min;
            let mx = *max;
            if debug_activo() {
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
                        move |data: &mut AppStateNativo, new_val: f64| {
                            data.escribir(&vn, ValorGUI::Decimal(new_val));
                        },
                    )
                },
            ))
        }
        Layout::Checkbox { variable } => {
            let var_name = variable.clone();
            let txt = variable.clone();
            let checked = data.leer(&var_name).to_bool();
            let gen = data.store.generation(&var_name);
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
            let color: forja_gui_rt::Color = get_color_role(&theme.scheme, "outline").into();
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
        Layout::ThemeProvider { child, theme: seed } => {
            let new_theme = MaterialTheme::named(seed, theme.is_dark);
            let inner = layout_a_view(child, data, _prog, &new_theme);
            // Cada diseño tiene su propio fondo (Mica en Fluent, vidrio en Liquid
            // Glass): lo pintamos acá para que se vea también si el wrapper no es
            // el nodo raíz.
            let bg: Color = new_theme.scheme.background.into();
            Box::new(view::sized_box(inner).background(Background::Color(bg)))
        }

        // ColoredBox: aplica un color del esquema como fondo
        Layout::ColoredBox { child, color_role } => {
            let inner = layout_a_view(child, data, _prog, theme);
            let bg: Color = get_color_role(&theme.scheme, color_role).into();
            Box::new(view::sized_box(inner).background(Background::Color(bg)))
        }

        // StyledLabel: etiqueta con estilo tipográfico predefinido
        Layout::StyledLabel { texto, style } => {
            let text_style = get_text_style(&theme.typography, style);
            let color = color_segun_monto(&theme.scheme, texto);
            Box::new(
                view::label(texto.clone())
                    .text_size(text_style.font_size as f32)
                    .weight(text_style.weight.to_xilem_weight())
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
            let text_style = get_text_style(&theme.typography, style);
            let txt = expr_a_texto(expr, params, &data.store, _prog);
            let color = color_segun_monto(&theme.scheme, &txt);
            let gen = data.store.global_generation();
            let fs = text_style.font_size as f32;
            let wt = text_style.weight.to_xilem_weight();
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
            let txt = expr_a_texto(expr, params, &data.store, _prog);
            let color = color_segun_monto(&theme.scheme, &txt);
            let gen = data.store.global_generation();
            Box::new(memoize(
                (gen, txt, color),
                move |(_, t, clr): &(u64, String, forja_gui_rt::Color)| {
                    view::label(t.clone()).color(*clr)
                },
            ))
        }

        // ShapedBox: aplica border-radius al hijo
        Layout::ShapedBox {
            child,
            shape_family,
        } => {
            let radius = get_shape_radius(&theme.shapes, shape_family);
            let inner = layout_a_view(child, data, _prog, theme);
            Box::new(view::sized_box(inner).corner_radius(radius.px()))
        }

        // 5.5: ElevatedBox — aplica corner_radius como indicador visual de elevación.
        // xilem 0.4: BoxShadow no está disponible en sized_box; se documenta la restricción.
        Layout::ElevatedBox {
            child,
            level: _level,
            shape_family,
        } => {
            let inner = layout_a_view(child, data, _prog, theme);
            let shape_radius = get_shape_radius(&theme.shapes, shape_family);
            // NOTA: shadow_to_box_shadow(&shadow_cfg) calcula la sombra correctamente
            // pero xilem 0.4 no expone box-shadow en WidgetPod. Aplicar solo corner_radius.
            Box::new(view::sized_box(inner).corner_radius(shape_radius.px()))
        }

        // 5.6: StateLayerBox — overlay semi-transparente según el estado visual
        Layout::StateLayerBox { child, state } => {
            let inner = layout_a_view(child, data, _prog, theme);
            let overlay_alpha: f64 = match state.as_str() {
                "hover" => 0.08,
                "pressed" => 0.12,
                "focused" => 0.12,
                "disabled" => 0.38,
                _ => 0.0,
            };
            if overlay_alpha > 0.0 {
                let overlay_color = RgbColor(0, 0, 0).with_alpha(overlay_alpha);
                Box::new(view::zstack((Box::new(
                    view::sized_box(inner).background(Background::Color(overlay_color.into())),
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
        } => match data.window_size {
            WindowSizeClass::Expanded => layout_a_view(expanded, data, _prog, theme),
            WindowSizeClass::Medium => layout_a_view(medium, data, _prog, theme),
            WindowSizeClass::Compact => layout_a_view(compact, data, _prog, theme),
        },

        // Padding: añade espacio alrededor del hijo
        Layout::Padding { child, amount } => {
            let inner = layout_a_view(child, data, _prog, theme);
            Box::new(view::sized_box(inner).padding((*amount).px()))
        }

        // Expanded: llena el espacio disponible
        Layout::Expanded { child } => {
            let inner = layout_a_view(child, data, _prog, theme);
            // Envolver en flex vertical con must_fill para expandir el espacio disponible
            Box::new(view::flex(Axis::Vertical, (inner,)).dims(
                xilem::masonry::properties::Dimensions::height(
                    xilem::masonry::layout::Dim::Stretch,
                ),
            ))
        }

        // Centered: centra el hijo en el eje transversal
        Layout::Centered { child } => {
            let inner = layout_a_view(child, data, _prog, theme);
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
            wrap: _,
        } => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in children {
                widgets.push(layout_a_view(h, data, _prog, theme));
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
                    if data.window_width < 400.0 {
                        1
                    } else if data.window_width < 600.0 {
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
                let widget = layout_a_view(item, data, _prog, theme);
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
                    let w = layout_a_view(h, data, _prog, theme);
                    let child_width = estimate_child_width(h).unwrap_or(100.0);
                    (w, child_width)
                })
                .collect();

            let available_width = (data.window_width - 48.0).max(200.0);
            let mut rows: Vec<Vec<Box<AnyWidgetView<AppStateNativo>>>> = Vec::new();
            let mut current_row: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            let mut current_width = 0.0;

            for (widget, width) in rendered {
                if current_row.is_empty() || current_width + width + *gap <= available_width {
                    current_row.push(widget);
                    current_width += width + *gap;
                } else {
                    rows.push(current_row);
                    current_row = vec![widget];
                    current_width = width;
                }
            }
            if !current_row.is_empty() {
                rows.push(current_row);
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
            let label_style = get_text_style(&theme.typography, "label_large");
            match variant {
                ButtonVariant::Filled => make_material_button(
                    texto,
                    scheme.on_primary.into(),
                    Some(scheme.primary.into()),
                    None,
                    label_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Tonal => make_material_button(
                    texto,
                    scheme.on_secondary_container.into(),
                    Some(scheme.secondary_container.into()),
                    None,
                    label_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Outlined => make_material_button(
                    texto,
                    scheme.primary.into(),
                    None,
                    Some(scheme.outline.into()),
                    label_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Text => make_material_button(
                    texto,
                    scheme.primary.into(),
                    None,
                    None,
                    label_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Elevated => make_material_button(
                    texto,
                    scheme.primary.into(),
                    Some(scheme.surface.into()),
                    None,
                    label_style,
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
            size,
            texto_extendido,
        } => crate::widgets::material_buttons::render_fab(
            icono,
            callback,
            size,
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
            on_remove: _,
        } => {
            let scheme = &theme.scheme;
            let label_style = get_text_style(&theme.typography, "label_small");
            match variant {
                ChipVariant::Assist | ChipVariant::Suggestion => make_chip_button(
                    texto,
                    scheme.on_surface.into(),
                    None,
                    Some(scheme.outline.into()),
                    label_style,
                    callback,
                    _prog,
                    8.0,
                ),
                ChipVariant::Filter => {
                    if *activo {
                        make_chip_button(
                            texto,
                            scheme.on_secondary_container.into(),
                            Some(scheme.secondary_container.into()),
                            None,
                            label_style,
                            callback,
                            _prog,
                            8.0,
                        )
                    } else {
                        make_chip_button(
                            texto,
                            scheme.on_surface.into(),
                            None,
                            Some(scheme.outline.into()),
                            label_style,
                            callback,
                            _prog,
                            8.0,
                        )
                    }
                }
                ChipVariant::Input => make_chip_button(
                    texto,
                    scheme.on_secondary_container.into(),
                    Some(scheme.secondary_container.into()),
                    None,
                    label_style,
                    callback,
                    _prog,
                    8.0,
                ),
            }
        }

        // ─── Inputs Material Design 3 ──────────────────────────────────

        // ─── MaterialTextField ─────────────────────────────────────────
        Layout::MaterialTextField {
            variable,
            label,
            placeholder,
            variant,
            multiline: _,
            error,
            counter: _,
        } => crate::widgets::material_inputs::render_text_field(
            variable,
            label,
            placeholder,
            variant,
            error,
            data,
            _prog,
            theme,
        ),

        // ─── MaterialPasswordField ─────────────────────────────────────
        Layout::MaterialPasswordField {
            variable,
            label,
            visible: _,
        } => crate::widgets::material_inputs::render_password_field(
            variable, label, data, _prog, theme,
        ),

        // ─── MaterialNumberField ───────────────────────────────────────
        Layout::MaterialNumberField {
            variable,
            label,
            min,
            max,
            decimales: _,
        } => crate::widgets::material_inputs::render_number_field(
            variable, label, min, max, data, _prog, theme,
        ),

        // ─── MaterialSearchField ───────────────────────────────────────
        Layout::MaterialSearchField {
            variable,
            placeholder,
        } => crate::widgets::material_inputs::render_search_field(
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
            label,
        } => crate::widgets::material_inputs::render_select(
            opciones,
            seleccionada,
            label,
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
        Layout::MaterialSwitch { label, variable } => {
            crate::widgets::material_inputs::render_switch(label, variable, data, _prog, theme)
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
            crate::pickers::date_picker_view(variable, data, &theme.scheme, theme)
        }

        // ─── MaterialTimePicker ────────────────────────────────────────
        Layout::MaterialTimePicker { variable } => {
            crate::pickers::time_picker_view(variable, data, &theme.scheme, theme)
        }

        // ─── ColorPicker ───────────────────────────────────────────────
        Layout::ColorPicker { variable } => {
            crate::pickers::color_picker_view(variable, data, &theme.scheme, theme)
        }

        // AspectRatioBox: caja con relación de aspecto fija
        Layout::AspectRatioBox { child, ratio } => {
            let inner = layout_a_view(child, data, _prog, theme);
            // Calcular alto basado en el ancho disponible y el ratio
            let available_width = (data.window_width - 48.0).max(200.0);
            let height = available_width / ratio;
            Box::new(view::sized_box(inner).dims((Length::px(available_width), Length::px(height))))
        }

        // ─── Tarjetas, Listas y Tablas ──────────────────────────────
        Layout::MaterialCard {
            child,
            variant,
            on_click: _,
            seleccionado,
        } => crate::widgets::material_cards::render_card(
            child,
            variant,
            seleccionado,
            data,
            _prog,
            theme,
        ),

        Layout::MaterialListItem {
            leading,
            titulo,
            subtitulo,
            trailing,
            on_click: _,
        } => crate::widgets::material_cards::render_list_item(
            leading, titulo, subtitulo, trailing, data, _prog, theme,
        ),

        Layout::MaterialList { items, dividers } => {
            crate::widgets::material_cards::render_list(items, dividers, data, _prog, theme)
        }

        Layout::MaterialListControl {
            items,
            control_type: _,
            variables: _,
        } => crate::widgets::material_cards::render_list_control(items, data, _prog, theme),

        Layout::MaterialListSelection {
            items,
            seleccion: _,
            callback: _,
            multiple: _,
        } => crate::widgets::material_cards::render_list_selection(items, data, _prog, theme),

        // ─── DynamicList: renderiza items desde una variable ────────
        // 5.4: NOTA sobre virtualización: view::virtual_scroll() de xilem 0.4 requiere
        // un closure que retorne un WidgetView concreto (no type-erased). Dado que
        // render_dynamic_list usa expr_a_layout_item → Box<AnyWidgetView>, la virtualización
        // directa no es compatible sin un wrapper intermedio. Se documenta para futura
        // migración cuando xilem soporte virtual_scroll con type erasure.
        Layout::DynamicList { variable, item_fn } => {
            crate::widgets::material_cards::render_dynamic_list(
                variable, item_fn, data, _prog, theme,
            )
        }

        Layout::MaterialDataTable {
            columnas,
            filas,
            ordenable,
            seleccionable: _,
            col_orden,
            orden_asc,
        } => crate::widgets::material_cards::render_data_table(
            columnas, filas, ordenable, col_orden, orden_asc, data, _prog, theme,
        ),

        Layout::MaterialSurface { child, color_role } => {
            crate::widgets::material_cards::render_surface(child, color_role, data, _prog, theme)
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
            let eased = EASE_EMPHASIZED.apply(progress);

            if show || progress > 0.0 {
                let inner = layout_a_view(dialog, data, _prog, theme);
                // Overlay con opacidad animada (0 → 0.32)
                let overlay_alpha = eased * 0.32;
                let overlay_color: Color = RgbColor(0, 0, 0).with_alpha(overlay_alpha);
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
            let overlay_color: Color = RgbColor(0, 0, 0).with_alpha(0.32);
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
            let has_cancel = !cancelar_texto.is_empty();
            let cancel_btn = if has_cancel {
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
            let button_row: Box<AnyWidgetView<AppStateNativo>> = if has_cancel {
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
            let title_label = {
                let title_style = get_text_style(&theme.typography, "title_large");
                view::label(titulo.clone())
                    .text_size(title_style.font_size as f32)
                    .weight(FontWeight::BOLD)
                    .color(fg)
            };

            // Mensaje
            let msg_label = {
                let body_style = get_text_style(&theme.typography, "body_medium");
                view::label(mensaje.clone())
                    .text_size(body_style.font_size as f32)
                    .color(fg_var)
            };

            // Card del diálogo
            let dialog_card = view::flex(
                Axis::Vertical,
                (
                    title_label,
                    view::sized_box(view::label(String::new())).height(Length::px(8.0)),
                    msg_label,
                    view::sized_box(view::label(String::new())).height(Length::px(16.0)),
                    button_row,
                ),
            );

            // Card del diálogo con padding interior
            let dialog_inner = view::sized_box(dialog_card)
                .background(Background::Color(surface_bg))
                .corner_radius(get_shape_radius(&theme.shapes, "extra_large").px())
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
            child,
            on_close: _,
        } => {
            let scheme = &theme.scheme;
            let overlay_color: Color = RgbColor(0, 0, 0).with_alpha(0.32);
            let surface_bg: Color = scheme.surface.into();
            let fg: Color = scheme.on_surface.into();

            let title_style = get_text_style(&theme.typography, "title_large");
            let title_label = view::label(titulo.clone())
                .text_size(title_style.font_size as f32)
                .weight(FontWeight::BOLD)
                .color(fg);

            let inner = layout_a_view(child, data, _prog, theme);

            let dialog_card = view::flex(
                Axis::Vertical,
                (
                    title_label,
                    view::sized_box(view::label(String::new())).height(Length::px(8.0)),
                    inner,
                ),
            );

            let dialog_container = view::sized_box(dialog_card)
                .background(Background::Color(surface_bg))
                .corner_radius(get_shape_radius(&theme.shapes, "extra_large").px())
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
            child,
            variant,
            visible,
            on_dismiss: _,
        } => {
            let show = data.leer(visible).to_string() == "true";
            // Detectar cambio y animar
            data.anim_visibility_changed(visible, show);
            let progress = data.anim_progress(visible, 250.0);
            let eased = EASE_EMPHASIZED.apply(progress);

            if show || progress > 0.0 {
                let scheme = &theme.scheme;
                let inner = layout_a_view(child, data, _prog, theme);
                let sheet_bg: Color = scheme.surface.into();
                // Slide-up: offset_y desde 300px hasta 0
                let sheet_offset_y = 300.0 * (1.0 - eased);

                match variant {
                    SheetVariant::Standard => {
                        // Sin overlay, solo panel con slide-up
                        let sheet = view::transformed(
                            view::sized_box(inner)
                                .background(Background::Color(sheet_bg))
                                .corner_radius(get_shape_radius(&theme.shapes, "extra_large").px()),
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
                        let overlay_color: Color = RgbColor(0, 0, 0).with_alpha(overlay_alpha);
                        let sheet = view::transformed(
                            view::sized_box(inner)
                                .background(Background::Color(sheet_bg))
                                .corner_radius(get_shape_radius(&theme.shapes, "extra_large").px()),
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
                        let overlay_color: Color = RgbColor(0, 0, 0).with_alpha(overlay_alpha);
                        let sheet = view::transformed(
                            view::sized_box(inner)
                                .background(Background::Color(sheet_bg))
                                .corner_radius(get_shape_radius(&theme.shapes, "extra_large").px()),
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
                let body_style = get_text_style(&theme.typography, "body_medium");
                let msg_label = view::label(mensaje.clone())
                    .text_size(body_style.font_size as f32)
                    .color(fg);

                // Botón de acción (opcional)
                let has_action = accion_texto.is_some() && accion_callback.is_some();
                let action_btn: Option<Box<AnyWidgetView<AppStateNativo>>> = if has_action {
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
                row_children.push(Box::new(msg_label) as Box<AnyWidgetView<AppStateNativo>>);
                if let Some(btn) = action_btn {
                    row_children.push(Box::new(
                        view::sized_box(view::label(String::new())).width(Length::px(16.0)),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                    row_children.push(btn);
                }

                let row = view::flex(Axis::Horizontal, (row_children,)).gap(Length::px(8.0));

                let snackbar = view::sized_box(row)
                    .background(Background::Color(bg))
                    .corner_radius(get_shape_radius(&theme.shapes, "small").px())
                    .padding(16.0.px());

                Box::new(snackbar)
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── Tooltip ─────────────────────────────────────────────────
        // Pequeña etiqueta de ayuda contextual
        Layout::Tooltip { child, texto } => {
            let scheme = &theme.scheme;
            let tooltip_bg: Color = scheme.on_surface.into();
            let tooltip_fg: Color = scheme.surface.into();

            let inner = layout_a_view(child, data, _prog, theme);
            let label_style = get_text_style(&theme.typography, "label_small");
            let tooltip_label = view::label(texto.clone())
                .text_size(label_style.font_size as f32)
                .color(tooltip_fg);

            let tooltip_box = view::sized_box(tooltip_label)
                .background(Background::Color(tooltip_bg))
                .corner_radius(get_shape_radius(&theme.shapes, "small").px())
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
                    let item_label = view::label(t.clone()).text_size(14.0).color(fg);
                    let item_btn = view::button(item_label, move |data: &mut AppStateNativo| {
                        if !cb_inner.is_empty() {
                            ejecutar_callback_y_actualizar(&cb_inner, data, &p);
                        }
                    });
                    menu_items.push(Box::new(item_btn) as Box<AnyWidgetView<AppStateNativo>>);
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
                        .corner_radius(get_shape_radius(&theme.shapes, "small").px());

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
                let _prog = _prog;
                let prog = _prog.to_vec();
                let cb = on_select.clone();

                let mut menu_items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let cb_inner = cb.clone();
                    let t = item.clone();
                    let p = prog.clone();
                    let item_label = view::label(t.clone()).text_size(14.0).color(fg);
                    let item_btn = view::button(item_label, move |data: &mut AppStateNativo| {
                        if !cb_inner.is_empty() {
                            ejecutar_callback_y_actualizar(&cb_inner, data, &p);
                        }
                    });
                    menu_items.push(Box::new(item_btn) as Box<AnyWidgetView<AppStateNativo>>);
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
                        .corner_radius(get_shape_radius(&theme.shapes, "small").px());

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
            current_var,
            history_var: _,
            nav_type,
            anim,
            on_change,
        } => crate::widgets::navigation::render_navigator(
            screens,
            current_var,
            nav_type,
            anim,
            on_change,
            data,
            _prog,
            theme,
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
            data,
            _prog,
            theme,
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
            data,
            _prog,
            theme,
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
        Layout::SearchBar {
            placeholder,
            on_search: _,
            variable,
        } => {
            crate::widgets::navigation::render_search_bar(placeholder, variable, data, _prog, theme)
        }

        // ─── SearchView ──────────────────────────────────────────────
        Layout::SearchView {
            query: _,
            resultados,
            visible,
        } => {
            crate::widgets::navigation::render_search_view(resultados, visible, data, _prog, theme)
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
        Layout::LinearProgressValue(val) => {
            crate::widgets::indicators::render_linear_progress_value(val, theme)
        }

        // ─── CircularProgress ────────────────────────────────────────
        Layout::CircularProgress {
            variable,
            size,
            indeterminado,
        } => crate::widgets::indicators::render_circular_progress(
            variable,
            size,
            indeterminado,
            data,
            _prog,
            theme,
        ),

        // ─── Badge ──────────────────────────────────────────────────
        Layout::Badge { child, valor, dot } => {
            crate::widgets::indicators::render_badge(child, valor, dot, data, _prog, theme)
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
            child,
            visible,
            duracion,
        } => {
            let show = data.leer(visible).to_string() == "true";
            data.anim_visibility_changed(visible, show);
            let d = if *duracion > 0.0 { *duracion } else { 200.0 };
            let progress = data.anim_progress(visible, d);
            let eased = EASE_STANDARD.apply(progress);

            if show || progress > 0.0 {
                let inner = layout_a_view(child, data, _prog, theme);
                // Fade mediante opacidad del overlay
                let overlay_alpha = 1.0 - eased;
                if overlay_alpha < 0.01 {
                    inner
                } else {
                    Box::new(view::sized_box(inner).background(Background::Color(
                        RgbColor(0, 0, 0).with_alpha(overlay_alpha),
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
        Layout::RippleEffect { child, color: _ } => {
            let inner = layout_a_view(child, data, _prog, theme);
            Box::new(view::sized_box(inner).corner_radius(get_shape_radius(&theme.shapes, "extra_small").px()))
        }

        // ─── PullToRefresh ────────────────────────────────────────────
        Layout::PullToRefresh {
            child,
            callback,
            refreshing,
        } => {
            let is_refreshing = data.leer(refreshing).to_bool();
            let prog = _prog.to_vec();
            let cb = callback.clone();
            let ref_var = refreshing.clone();

            let inner = layout_a_view(child, data, _prog, theme);

            if is_refreshing {
                // Si está refrescando, mostrar spinner real + texto
                let fg: Color = theme.scheme.on_surface.into();
                let refresh_row = view::flex(
                    Axis::Horizontal,
                    (vec![
                        Box::new(view::sized_box(view::spinner()).dims((
                            Length::px(18.0),
                            Length::px(18.0),
                        ))) as Box<AnyWidgetView<AppStateNativo>>,
                        Box::new(view::label("Actualizando...").text_size(14.0).color(fg)),
                    ],),
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(Length::px(8.0));
                let header = Box::new(view::sized_box(refresh_row).padding(8.0.px()))
                    as Box<AnyWidgetView<AppStateNativo>>;
                let mut column_children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                column_children.push(header);
                column_children.push(inner);
                Box::new(view::portal(
                    view::flex(Axis::Vertical, (column_children,)).gap(Length::px(4.0)),
                ))
            } else {
                // Envolver en PullToRefreshView con detección de gesto real
                let view = PullToRefreshView::new(inner, cb, ref_var, prog);
                Box::new(view)
            }
        }

        // ─── SwipeToDismiss ───────────────────────────────────────────
        Layout::SwipeToDismiss {
            child,
            on_dismiss,
            label,
            dismissed,
        } => {
            let is_dismissed = data.leer(dismissed).to_bool();
            let prog = _prog.to_vec();
            let cb = on_dismiss.clone();
            let dismiss_var = dismissed.clone();
            let action_text = label.clone();

            if is_dismissed {
                // Ya descartado: mostrar opción de deshacer
                let scheme = &theme.scheme;
                let fg: Color = scheme.on_surface.into();
                let btn_fg: Color = scheme.primary.into();
                let undo_label = view::label("Elemento descartado")
                    .text_size(14.0)
                    .color(fg);
                let undo_btn = view::button(
                    view::label(action_text.clone())
                        .text_size(14.0)
                        .weight(FontWeight::MEDIUM)
                        .color(btn_fg),
                    move |data: &mut AppStateNativo| {
                        data.escribir(&dismiss_var, ValorGUI::Booleano(false));
                    },
                );
                let mut row_children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                row_children.push(Box::new(undo_label) as Box<AnyWidgetView<AppStateNativo>>);
                row_children.push(Box::new(
                    view::sized_box(view::label(String::new())).width(Length::px(8.0)),
                ) as Box<AnyWidgetView<AppStateNativo>>);
                row_children.push(Box::new(undo_btn) as Box<AnyWidgetView<AppStateNativo>>);
                Box::new(
                    view::sized_box(
                        view::flex(Axis::Horizontal, (row_children,)).gap(Length::px(4.0)),
                    )
                    .padding(12.0.px())
                    .corner_radius(get_shape_radius(&theme.shapes, "small").px()),
                )
            } else {
                // Mostrar contenido envuelto en SwipeToDismissView con gesto real
                let inner = layout_a_view(child, data, _prog, theme);
                let view = SwipeToDismissView::new(inner, cb, dismiss_var, action_text, prog);
                Box::new(view)
            }
        }

        // ─── PinchZoom ──────────────────────────────────────────────
        Layout::PinchZoom {
            child,
            min_scale,
            max_scale,
        } => {
            let inner = layout_a_view(child, data, _prog, theme);
            let view = PinchZoomView::new(inner, *min_scale, *max_scale);
            Box::new(view)
        }

        // ─── Rotate ─────────────────────────────────────────────────
        Layout::Rotate { child } => {
            let inner = layout_a_view(child, data, _prog, theme);
            let view = RotateView::new(inner);
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
        Layout::LineChart {
            datos,
            color,
            etiquetas,
        } => {
            let c = color_role_to_rgba(&theme.scheme, color);
            let puntos: Vec<ChartDataPoint> = datos
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let label = etiquetas.get(i).cloned().unwrap_or_default();
                    ChartDataPoint::new(&label, v, c)
                })
                .collect();
            Box::new(LineChartView::<AppStateNativo>::new(puntos))
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
                        Rgba::new(rgb.0, rgb.1, rgb.2, 255)
                    } else {
                        color_role_to_rgba(&theme.scheme, &c_str)
                    };
                    let label = etiquetas.get(i).cloned().unwrap_or_default();
                    ChartDataPoint::new(&label, v, c)
                })
                .collect();
            Box::new(BarChartView::<AppStateNativo>::new(puntos, *apilado))
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
                    let c = Rgba::new(rgb.0, rgb.1, rgb.2, 255);
                    let label = etiquetas.get(i).cloned().unwrap_or_default();
                    ChartDataPoint::new(&label, v, c)
                })
                .collect();
            Box::new(PieChartView::<AppStateNativo>::new(puntos, *donut, 0.5))
        }

        // ─── GaugeChart ──────────────────────────────────────────────
        Layout::GaugeChart {
            valor,
            min: _,
            max,
            color: _,
        } => {
            let threshold = *max * 0.75;
            Box::new(GaugeChartView::<AppStateNativo>::new(
                *valor, *max, threshold,
            ))
        }

        // ─── Sparkline ───────────────────────────────────────────────
        Layout::Sparkline { datos, color } => {
            let c = color_role_to_rgba(&theme.scheme, color);
            Box::new(SparklineView::<AppStateNativo>::new(datos.clone(), c))
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
                    let num_text = if completed {
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
                            view::label(num_text)
                                .text_size(14.0)
                                .weight(FontWeight::BOLD)
                                .color(num_color),
                        )
                    };
                    let circle = view::sized_box(num)
                        .dims((Length::px(32.0), Length::px(32.0)))
                        .background(Background::Color(bg_color))
                        .corner_radius(get_shape_radius(&theme.shapes, "small").px());
                    let label = view::label(paso.clone())
                        .text_size(12.0)
                        .color(scheme.on_surface.into());
                    let cb_name = cb.clone();
                    let p = prog.clone();
                    let btn = view::button(
                        view::flex(
                            Axis::Vertical,
                            (
                                Box::new(circle) as Box<AnyWidgetView<AppStateNativo>>,
                                Box::new(label) as Box<AnyWidgetView<AppStateNativo>>,
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
                    let is_last = i == items.len() - 1;
                    let lbl = view::label(item.clone())
                        .text_size(14.0)
                        .weight(if is_last {
                            FontWeight::BOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .color(if is_last {
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
            let first_date = chrono::NaiveDate::from_ymd_opt(año_val, mes_val as u32, 1)
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
            let first_weekday = first_date.weekday().num_days_from_monday() as usize; // 0=Lun, 6=Dom
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
            let _prev_cb = cb_name.clone();
            let _prog_prev = prog.clone();
            let prev_btn = view::button(
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
                    Box::new(prev_btn) as Box<AnyWidgetView<AppStateNativo>>,
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
            let total_cells = first_weekday + num_dias;
            let num_rows = (total_cells + 6) / 7;
            let mut all_rows: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            all_rows.push(Box::new(header_row) as Box<AnyWidgetView<AppStateNativo>>);
            all_rows.push(Box::new(day_header) as Box<AnyWidgetView<AppStateNativo>>);

            for row in 0..num_rows {
                let mut day_cells: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                for col in 0..7 {
                    let cell_idx = row * 7 + col;
                    if cell_idx < first_weekday || cell_idx >= first_weekday + num_dias {
                        day_cells.push(Box::new(
                            view::sized_box(view::label(String::new()))
                                .dims((Length::px(36.0), Length::px(36.0))),
                        )
                            as Box<AnyWidgetView<AppStateNativo>>);
                    } else {
                        let dia = (cell_idx - first_weekday + 1) as u32;
                        let cb = cb_name.clone();
                        let p = prog.clone();
                        let fecha_str = format!("{:04}-{:02}-{:02}", año_val, mes_val, dia);
                        let is_selected = fecha_sel == fecha_str;

                        if is_selected {
                            let lbl = view::label(dia.to_string())
                                .text_size(12.0)
                                .weight(FontWeight::BOLD)
                                .color(scheme.on_surface.into());
                            day_cells.push(Box::new(
                                view::sized_box(lbl)
                                    .dims((Length::px(36.0), Length::px(36.0)))
                                    .background(Background::Color(scheme.primary_container.into()))
                                    .corner_radius(get_shape_radius(&theme.shapes, "full").px()),
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
            let lines: Vec<&str> = texto.split('\n').collect();
            let mut children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for line in lines {
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
            let qr = crate::qr_widget::qr_view::<AppStateNativo>(texto, t);
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
        Layout::FilePicker {
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
                    .corner_radius(get_shape_radius(&theme.shapes, "small").px())
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
            child,
            blur: _,
            opacity,
        } => {
            let _scheme = &theme.scheme;
            let inner = layout_a_view(child, data, _prog, theme);
            // Glassmorphism simulado con alpha blending
            // with_alpha() ya retorna xilem::Color (alias Color)
            let bg_color = RgbColor(255, 255, 255).with_alpha(*opacity);
            let border_color = RgbColor(255, 255, 255).with_alpha(0.3);
            Box::new(
                view::sized_box(inner)
                    .background(Background::Color(bg_color))
                    .corner_radius(get_shape_radius(&theme.shapes, "extra_large").px())
                    .border_color(border_color)
                    .border_width(1.0.px()),
            )
        }

        // ─── GradientBox ─────────────────────────────────────────────
        Layout::GradientBox {
            child,
            colores,
            direccion,
        } => {
            let scheme = &theme.scheme;
            let inner = layout_a_view(child, data, _prog, theme);
            // Gradiente real con Background::Gradient de Masonry
            let gradient = if colores.len() >= 2 {
                // Convertir colores a xilem::Color (AlphaColor<Srgb>)
                let color_vec: Vec<Color> = colores
                    .iter()
                    .map(|c| {
                        let rgb = parse_color(c).unwrap_or(scheme.primary);
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
                let c = parse_color(&colores[0]).unwrap_or(scheme.primary);
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
                    .corner_radius(get_shape_radius(&theme.shapes, "container").px()),
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
                    .corner_radius(get_shape_radius(&theme.shapes, "button").px())
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
                let c = parse_color(&colores[0]).unwrap_or(scheme.primary);
                c.into()
            } else {
                scheme.primary_container.into()
            };
            // Layout de fondo expansivo con gradiente simulado (múltiples capas)
            let base = view::sized_box(view::label(String::new()))
                .dims((Length::px(300.0), Length::px(120.0)))
                .background(Background::Color(bg_color))
                .corner_radius(get_shape_radius(&theme.shapes, "extra_large").px());
            if colores.len() > 1 {
                let c2 = parse_color(&colores[1]).unwrap_or(scheme.secondary);
                let overlay = view::sized_box(view::label(String::new()))
                    .dims((Length::px(300.0), Length::px(60.0)))
                    .background(Background::Color(c2.into()))
                    .corner_radius(get_shape_radius(&theme.shapes, "extra_large").px());
                Box::new(view::zstack((
                    Box::new(base) as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(overlay) as Box<AnyWidgetView<AppStateNativo>>,
                )))
            } else {
                Box::new(base)
            }
        }

        // ─── GlowBorder ──────────────────────────────────────────────
        Layout::GlowBorder {
            child,
            color,
            ancho,
        } => {
            let scheme = &theme.scheme;
            let inner = layout_a_view(child, data, _prog, theme);
            let glow_color: Color = get_color_role(scheme, color).into();
            Box::new(
                view::sized_box(inner)
                    .border_color(glow_color)
                    .border_width((*ancho).px())
                    .corner_radius(get_shape_radius(&theme.shapes, "container").px()),
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
                crate::canvas_widget::CanvasView::<AppStateNativo>::new(
                    commands_var.clone(),
                    commands_json,
                    *width,
                    *height,
                )
            } else {
                crate::canvas_widget::CanvasView::<AppStateNativo>::empty(
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
                .or_else(|| parse_color(color))
                .unwrap_or(scheme.primary);
            let icon_size = *tamaño;
            let icon_estilo = IconStyle::from_str(estilo);

            // Si el nombre no está en el catálogo, mostrar el nombre en texto:
            // un icono inexistente tiene que verse, no desaparecer.
            if crate::icons::catalog::by_name(nombre).is_none() {
                return Box::new(
                    view::label(format!("[{}]", nombre))
                        .text_size((icon_size * 0.5) as f32)
                        .color(icon_color.into()),
                );
            }

            // Crear el view wrapper del icono SVG (renderiza paths reales de Vello)
            Box::new(svg_icon::<AppStateNativo>(
                nombre,
                icon_size,
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
    static WINDOW_WIDTH_TL: Cell<f64> = const { Cell::new(800.0) };
    static WINDOW_HEIGHT_TL: Cell<f64> = const { Cell::new(600.0) };
}

/// Obtiene el ancho de ventana almacenado en el thread-local.
pub fn current_window_width() -> f64 {
    WINDOW_WIDTH_TL.with(|w| w.get())
}

/// Obtiene el alto de ventana almacenado en el thread-local.
pub fn current_window_height() -> f64 {
    WINDOW_HEIGHT_TL.with(|h| h.get())
}

/// Widget Masonry que observa el ancho disponible durante el layout
/// y lo almacena en el thread-local para que la vista Xilem lo lea.
pub struct WindowWidthProbe {
    child: Option<WidgetPod<dyn Widget>>,
}

impl WindowWidthProbe {
    /// Crea un nuevo probe que envuelve al widget hijo.
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            child: Some(child.erased().to_pod()),
        }
    }

    /// Obtiene una referencia mutable al widget hijo.
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        let child = this
            .widget
            .child
            .as_mut()
            .expect("WidthProbe always has child");
        this.ctx.get_mut(child)
    }
}

impl Widget for WindowWidthProbe {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        if let Some(ref mut child) = self.child {
            ctx.register_child(child);
        }
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        // Adopta el tamaño del hijo (igual que el `layout` viejo); sin hijo usa
        // el tamaño de ventana por defecto (800x600) que antes imponía `bc.constrain`.
        match self.child.as_mut() {
            Some(child) => {
                let context_size = crate::layout::LayoutSize::maybe(axis.cross(), cross_length);
                ctx.compute_length(child, len_req.into(), context_size, axis, cross_length)
            }
            None => {
                let fijo = match axis {
                    Axis::Horizontal => 800.0.px(),
                    Axis::Vertical => 600.0.px(),
                };
                match len_req {
                    LenReq::FitContent(space) => fijo.min(space),
                    _ => fijo,
                }
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        // Guardar el ancho y alto resueltos en el thread-local
        WINDOW_WIDTH_TL.with(|w| w.set(size.width));
        WINDOW_HEIGHT_TL.with(|h| h.set(size.height));

        // Layout del hijo ocupando todo el espacio disponible
        if let Some(child) = self.child.as_mut() {
            ctx.run_layout(child, size);
            ctx.place_child(child, Point::ORIGIN);
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
        _node: &mut Node,
    ) {
    }

    fn children_ids(&self) -> ChildrenIds {
        if let Some(child) = &self.child {
            ChildrenIds::from_slice(&[child.id()])
        } else {
            ChildrenIds::from_slice(&[])
        }
    }
}

// ─── View wrapper que inserta el WindowWidthProbe en el árbol ─────

/// Wrapper de Xilem View que envuelve la vista raíz con `WindowWidthProbe`
/// para detectar resize automáticamente.
pub struct WidthObservedView<V, State> {
    inner: V,
    phantom: PhantomData<fn() -> State>,
}

/// Envuelve cualquier `WidgetView` con detección de resize.
pub fn observe_width<State: 'static, V: WidgetView<State>>(
    inner: V,
) -> WidthObservedView<V, State> {
    WidthObservedView {
        inner,
        phantom: PhantomData,
    }
}

impl<V: WidgetView<State>, State: 'static> ViewMarker for WidthObservedView<V, State> {}

impl<V: WidgetView<State>, State: 'static> View<State, (), ViewCtx>
    for WidthObservedView<V, State>
{
    type Element = Pod<WindowWidthProbe>;
    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: &mut State,
    ) -> (Pod<WindowWidthProbe>, V::ViewState) {
        let (child, child_state) = self.inner.build(ctx, app_state);
        let widget = WindowWidthProbe::new(child.new_widget);
        (ctx.create_pod(widget), child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut V::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Pod<WindowWidthProbe>>,
        app_state: &mut State,
    ) {
        let mut child = WindowWidthProbe::child_mut(&mut element);
        self.inner
            .rebuild(&prev.inner, view_state, ctx, child.downcast(), app_state);
    }

    fn teardown(
        &self,
        view_state: &mut V::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Pod<WindowWidthProbe>>,
    ) {
        let mut child = WindowWidthProbe::child_mut(&mut element);
        self.inner.teardown(view_state, ctx, child.downcast());
    }

    fn message(
        &self,
        view_state: &mut V::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Pod<WindowWidthProbe>>,
        app_state: &mut State,
    ) -> MessageResult<()> {
        let mut child = WindowWidthProbe::child_mut(&mut element);
        self.inner
            .message(view_state, message, child.downcast(), app_state)
    }
}

// WidthObservedView implementa WidgetView automáticamente vía el blanket impl de xilem
// (requiere View<..., Element=Pod<W>> + Send + Sync, que WidthObservedView cumple)

// ─── Punto de entrada ─────────────────────────────────────────────

/// Tema con el que se pinta la ventana: si la raíz del layout es un `tema_*`,
/// ese diseño manda también en el fondo de la ventana (no sólo en los hijos).
fn tema_de_raiz(layout: &Layout, base: &MaterialTheme) -> MaterialTheme {
    match layout {
        Layout::ThemeProvider { theme, .. } => MaterialTheme::named(theme, base.is_dark),
        _ => base.clone(),
    }
}

/// `true` si se pidió salida de depuración (`FORJA_GUI_DEBUG=1`).
///
/// Por defecto el runtime no imprime nada: en una app compilada esos logs sólo
/// ensucian la consola y retrasan el arranque.
fn debug_activo() -> bool {
    std::env::var("FORJA_GUI_DEBUG").map(|v| v != "0").unwrap_or(false)
}

/// Arma las `WindowOptions` con lo declarado en `ventana(...)`.
fn opciones_ventana(
    titulo: String,
    config: &crate::window_config::WindowConfig,
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
pub fn build_and_run(
    programa: &Programa,
    load_state: Option<&str>,
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

    let mut state = AppStateNativo::new();
    inicializar_estado(&programa.declaraciones, &mut state);
    // Restaurar estado guardado (hot reload) si se proporcionó
    if let Some(json) = load_state {
        state.cargar_estado(json);
    }
    // Árbol de arranque: sólo para el log (el layout se re-extrae en cada render,
    // así `si/sino` ve el estado actual y la navegación cambia de página).
    let layout_inicial = extraer_layout(&programa.declaraciones, &mut state.store);
    if debug_activo() {
        crate::debug::print_layout_tree(&layout_inicial);
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
            if tema_ventana.is_dark { "oscuro" } else { "claro" }
        )
    });
    if debug_activo() {
        println!(
            "  🪟 Ventana «{}» {}x{} (icono: {:?})",
            titulo, config.ancho, config.alto, config.icono
        );
        println!(
            "  {} Diseño {:?} — modo {} (seed: {})",
            if tema_ventana.is_dark { "🌙" } else { "☀️" },
            tema_ventana.design,
            if tema_ventana.is_dark { "oscuro" } else { "claro" },
            tema_ventana.seed_color
        );
    }

    // Inicializar window_width con un valor por defecto (se actualizará en cada render)
    let initial_width: f64 = config.ancho;
    let initial_height: f64 = config.alto;
    state.update_window_size(initial_width, initial_height);
    WINDOW_WIDTH_TL.with(|w| w.set(initial_width));
    WINDOW_HEIGHT_TL.with(|h| h.set(initial_height));
    if debug_activo() {
        println!(
            "  📐 Clase de tamaño inicial: {:?} ({:.0}px)",
            state.window_size, initial_width
        );
    }

    // Crear la app con WindowOptions que fija el tamaño inicial de la ventana
    let app = Xilem::new_simple(
        state,
        move |data: &mut AppStateNativo| -> Box<AnyWidgetView<AppStateNativo>> {
            // Leer el ancho actual desde el thread-local (actualizado por WindowWidthProbe)
            let current_width = current_window_width();
            let current_height = current_window_height();
            data.update_window_size(current_width, current_height);
            let medir = crate::perf::activo();
            let t0 = std::time::Instant::now();
            let layout = extraer_layout(&prog, &mut data.store);
            let t1 = std::time::Instant::now();
            // El diseño de la raíz se re-evalúa en cada render (por si un `si`
            // cambia de tema según el estado).
            let tema = tema_de_raiz(&layout, &theme);
            let root_view = layout_a_view(&layout, data, &prog, &tema);
            if medir {
                if let Some(reporte) =
                    crate::perf::registrar(t1 - t0, std::time::Instant::now() - t1)
                {
                    eprintln!("{}", reporte);
                }
            }
            let root_bg = crate::chrome::fondo_pagina(&tema);
            let root_with_bg = view::sized_box(root_view)
                .dims((
                    Length::px(current_width.max(200.0)),
                    Length::px(current_height.max(200.0)),
                ))
                .background(Background::Color(root_bg));
            Box::new(observe_width(root_with_bg))
        },
        crate::chrome::aplicar_chrome(
            opciones_ventana(titulo, &config, initial_width, initial_height),
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
pub fn build_and_run_android(
    programa: &Programa,
    load_state: Option<&str>,
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

    let mut state = AppStateNativo::new();
    inicializar_estado(&programa.declaraciones, &mut state);
    // Restaurar estado guardado (hot reload) si se proporcionó
    if let Some(json) = load_state {
        state.cargar_estado(json);
    }
    // Árbol de arranque: sólo para el log (el layout se re-extrae en cada render,
    // así `si/sino` ve el estado actual y la navegación cambia de página).
    crate::debug::print_layout_tree(&extraer_layout(&programa.declaraciones, &mut state.store));
    let prog = programa.declaraciones.clone();

    let initial_width: f64 = 1200.0;
    let initial_height: f64 = 800.0;
    state.update_window_size(initial_width, initial_height);
    WINDOW_WIDTH_TL.with(|w| w.set(initial_width));
    WINDOW_HEIGHT_TL.with(|h| h.set(initial_height));

    let app = Xilem::new_simple(
        state,
        move |data: &mut AppStateNativo| -> Box<AnyWidgetView<AppStateNativo>> {
            let current_width = current_window_width();
            let current_height = current_window_height();
            data.update_window_size(current_width, current_height);
            let layout = extraer_layout(&prog, &mut data.store);
            let tema = tema_de_raiz(&layout, &theme);
            let root_view = layout_a_view(&layout, data, &prog, &tema);
            let root_bg = crate::chrome::fondo_pagina(&tema);
            let root_with_bg = view::sized_box(root_view)
                .dims((
                    Length::px(current_width.max(200.0)),
                    Length::px(current_height.max(200.0)),
                ))
                .background(Background::Color(root_bg));
            Box::new(observe_width(root_with_bg))
        },
        WindowOptions::new("Forja GUI — diseño de la app")
            .with_initial_inner_size(LogicalSize::new(initial_width, 800.0)),
    );

    let app = app.with_default_properties(crate::theme_props::default_properties(&theme));
    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(android_app);

    app.run_in(event_loop)
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
    let mut store = state.store.clone();
    match crate::evaluador::ejecutar_funcion(nombre_fn, &[], programa, &mut store) {
        Ok(val) => val,
        Err(e) => ValorGUI::Texto(format!("Error: {}", e)),
    }
}

// ejecutar_callback_y_actualizar → movido a crate::helpers

/// Inicializa el estado de la GUI evaluando las variables de la función `main`.
pub fn inicializar_estado(decls: &[Declaracion], state: &mut AppStateNativo) {
    crate::evaluador::inicializar_estado(decls, &mut state.store);
}

// ═══════════════════════════════════════════════════════════════════
// View wrappers para gestos (PullToRefreshView, SwipeToDismissView)
// ═══════════════════════════════════════════════════════════════════
//
// Estos wrappers permiten usar los widgets Masonry de gestos (de
// gesture_widgets.rs) como vistas Xilem, siguiendo el mismo patrón
// que el Button view de xilem.

use crate::GestureResult as GestureAction;
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
pub struct PullToRefreshView<V> {
    child: V,
    callback: String,
    refreshing: String,
    program: Vec<Declaracion>,
}

impl<V> PullToRefreshView<V> {
    pub fn new(child: V, callback: String, refreshing: String, program: Vec<Declaracion>) -> Self {
        Self {
            child,
            callback,
            refreshing,
            program,
        }
    }
}

impl<V> ViewMarker for PullToRefreshView<V> {}

impl<V> View<AppStateNativo, (), ViewCtx> for PullToRefreshView<V>
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
        let (child, child_state) = ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::build(&self.child, ctx, app_state)
        });
        let widget = PullRefreshWidget::new(child.new_widget);
        (
            ctx.with_action_widget(|ctx| ctx.create_pod(widget)),
            child_state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::rebuild(
                &self.child,
                &prev.child,
                view_state,
                ctx,
                PullRefreshWidget::child_mut(&mut element).downcast(),
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
                &self.child,
                view_state,
                ctx,
                PullRefreshWidget::child_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) -> crate::MessageResult<()> {
        match message.take_first() {
            Some(GESTURE_CHILD_VIEW_ID) => View::<AppStateNativo, (), _>::message(
                &self.child,
                _view_state,
                message,
                PullRefreshWidget::child_mut(&mut element).downcast(),
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
pub struct SwipeToDismissView<V> {
    child: V,
    on_dismiss: String,
    dismissed: String,
    label: String,
    program: Vec<Declaracion>,
}

impl<V> SwipeToDismissView<V> {
    pub fn new(
        child: V,
        on_dismiss: String,
        dismissed: String,
        label: String,
        program: Vec<Declaracion>,
    ) -> Self {
        Self {
            child,
            on_dismiss,
            dismissed,
            label,
            program,
        }
    }
}

impl<V> ViewMarker for SwipeToDismissView<V> {}

impl<V> View<AppStateNativo, (), ViewCtx> for SwipeToDismissView<V>
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
        let (child, child_state) = ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::build(&self.child, ctx, app_state)
        });
        let widget = SwipeGestureWidget::new(child.new_widget).with_threshold(100.0);
        (
            ctx.with_action_widget(|ctx| ctx.create_pod(widget)),
            child_state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::rebuild(
                &self.child,
                &prev.child,
                view_state,
                ctx,
                SwipeGestureWidget::child_mut(&mut element).downcast(),
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
                &self.child,
                view_state,
                ctx,
                SwipeGestureWidget::child_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) -> crate::MessageResult<()> {
        match message.take_first() {
            Some(GESTURE_CHILD_VIEW_ID) => View::<AppStateNativo, (), _>::message(
                &self.child,
                _view_state,
                message,
                SwipeGestureWidget::child_mut(&mut element).downcast(),
                app_state,
            ),
            None => {
                match message.take_message::<GestureAction>() {
                    Some(boxed) => match *boxed {
                        GestureAction::Swipe { .. } => {
                            // 5.1: Usar campo label para debugging durante swipe
                            if !self.label.is_empty() {
                                eprintln!(
                                    "[SwipeToDismiss] '{}' — deslizando para descartar",
                                    &self.label
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
pub struct PinchZoomView<V> {
    child: V,
    min_scale: f64,
    max_scale: f64,
}

impl<V> PinchZoomView<V> {
    pub fn new(child: V, min_scale: f64, max_scale: f64) -> Self {
        Self {
            child,
            min_scale,
            max_scale,
        }
    }
}

impl<V> ViewMarker for PinchZoomView<V> {}

impl<V> View<AppStateNativo, (), ViewCtx> for PinchZoomView<V>
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
        let (child, child_state) = ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::build(&self.child, ctx, app_state)
        });
        let widget = PinchZoomWidget::new(child.new_widget)
            .with_min_scale(self.min_scale)
            .with_max_scale(self.max_scale);
        (
            ctx.with_action_widget(|ctx| ctx.create_pod(widget)),
            child_state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::rebuild(
                &self.child,
                &prev.child,
                view_state,
                ctx,
                PinchZoomWidget::child_mut(&mut element).downcast(),
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
                &self.child,
                view_state,
                ctx,
                PinchZoomWidget::child_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) -> crate::MessageResult<()> {
        match message.take_first() {
            Some(GESTURE_CHILD_VIEW_ID) => View::<AppStateNativo, (), _>::message(
                &self.child,
                _view_state,
                message,
                PinchZoomWidget::child_mut(&mut element).downcast(),
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
pub struct RotateView<V> {
    child: V,
}

impl<V> RotateView<V> {
    pub fn new(child: V) -> Self {
        Self { child }
    }
}

impl<V> ViewMarker for RotateView<V> {}

impl<V> View<AppStateNativo, (), ViewCtx> for RotateView<V>
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
        let (child, child_state) = ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::build(&self.child, ctx, app_state)
        });
        let widget = RotateWidget::new(child.new_widget);
        (
            ctx.with_action_widget(|ctx| ctx.create_pod(widget)),
            child_state,
        )
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) {
        ctx.with_id(GESTURE_CHILD_VIEW_ID, |ctx| {
            View::<AppStateNativo, (), _>::rebuild(
                &self.child,
                &prev.child,
                view_state,
                ctx,
                RotateWidget::child_mut(&mut element).downcast(),
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
                &self.child,
                view_state,
                ctx,
                RotateWidget::child_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_action_source(element);
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut AppStateNativo,
    ) -> crate::MessageResult<()> {
        match message.take_first() {
            Some(GESTURE_CHILD_VIEW_ID) => View::<AppStateNativo, (), _>::message(
                &self.child,
                _view_state,
                message,
                RotateWidget::child_mut(&mut element).downcast(),
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
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = Parser::new(tokens);
        let programa = parser.parse().expect("parse");
        let mut store = VariableStore::new();
        extraer_layout(&programa.declaraciones, &mut store)
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
        let src = include_str!("../../../ejemplos/208_gui_galeria.fa");
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = Parser::new(tokens);
        let programa = parser.parse().expect("parse");
        let mut store = VariableStore::new();
        let layout = extraer_layout(&programa.declaraciones, &mut store);

        let Layout::ThemeProvider { child, theme } = layout else {
            panic!("la raíz debe ser un ThemeProvider, llegó: {:?}", layout);
        };
        assert_eq!(theme, "fluent:light");

        // El tema envuelve un `desplazable(columna_con_gap(...))`.
        let Layout::Portal(columna) = *child else {
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
                    celdas.iter().all(|c| matches!(c, Layout::MaterialCard { .. })),
                    "cada celda del grid debe ser una tarjeta"
                );
            }
            _ => unreachable!(),
        }

        // La barra superior y el botón final también deben estar presentes.
        assert!(
            children.iter().any(|c| matches!(c, Layout::TopAppBar { .. })),
            "falta la barra superior"
        );
        assert!(
            children.iter().any(|c| matches!(c, Layout::MaterialButton { .. })),
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
            child: Box::new(Layout::Spacer(0.0)),
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
            child: Box::new(Layout::Spacer(0.0)),
            theme: "liquid_glass:dark".to_string(),
        };
        let tema = tema_de_raiz(&layout, &base());
        assert_eq!(tema.design, DesignStyle::LiquidGlass);
        assert!(tema.is_dark);
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
        let boton = |is_dark| {
            (
                MaterialTheme::fluent(is_dark).shapes.for_family(ShapeFamily::Button),
                MaterialTheme::liquid_glass(is_dark)
                    .shapes
                    .for_family(ShapeFamily::Button),
                MaterialTheme::light().shapes.for_family(ShapeFamily::Button),
            )
        };
        for is_dark in [false, true] {
            let (fluent, glass, material) = boton(is_dark);
            assert_eq!(fluent, 4.0);
            assert_eq!(glass, 24.0);
            assert_eq!(material, 20.0);
        }
        // Fluent 2: ControlCornerRadius = 4 para contenedores (tarjetas/campos).
        assert_eq!(
            MaterialTheme::fluent(false)
                .shapes
                .for_family(ShapeFamily::Container),
            4.0
        );
        assert_eq!(
            MaterialTheme::liquid_glass(false)
                .shapes
                .for_family(ShapeFamily::Container),
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
        assert!(!claro.is_dark, "fluent:light debe ser claro aunque el SO sea oscuro");
        assert_eq!(claro.design, DesignStyle::Fluent);

        let oscuro = MaterialTheme::named("fluent:dark", false);
        assert!(oscuro.is_dark, "fluent:dark debe ser oscuro aunque el SO sea claro");

        let vidrio = MaterialTheme::named("liquid_glass:light", true);
        assert!(!vidrio.is_dark);
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
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = Parser::new(tokens);
        parser.parse().expect("parse")
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
        assert!(cfg.redimensionable, "por defecto la ventana se puede redimensionar");
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
        let mut store = VariableStore::new();
        match extraer_layout(&programa.declaraciones, &mut store) {
            Layout::Title(t) => assert_eq!(t, "Hola"),
            otro => panic!("se esperaba sólo el título, llegó {:?}", otro),
        }
    }
}
