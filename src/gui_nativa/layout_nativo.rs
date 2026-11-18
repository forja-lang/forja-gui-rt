//! Layouts nativos: el enum [`Layout`] y cómo se extrae del AST.
//!
//! `expr_a_layout` traduce una expresión Forja (`columna(...)`, `boton(...)`)
//! a [`Layout`] y `extraer_layout` arma el árbol de `main()` con el estado
//! actual. El mapeo a widgets de xilem vive en `vista_nativa.rs`.

use super::*;
use crate::*;

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
            let condicion_verdadera = crate::evaluador::evaluar_condicion(
                condicion,
                &crate::evaluador::AlmacenGui(almacen),
                decls,
            )
            .unwrap_or(true);
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
                        .map(|a| {
                            crate::evaluador::evaluar_valor(
                                a,
                                &crate::evaluador::AlmacenGui(almacen),
                                decls,
                            )
                        })
                        .collect();
                    if let Ok(args) = args {
                        if let Ok(valor) = crate::evaluador::ejecutar_funcion(
                            nombre,
                            &args,
                            decls,
                            &crate::evaluador::AlmacenGui(almacen),
                        ) {
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
                            // `escribir(resultado)`: keyword del lenguaje, variable
                            // donde el runtime deja el valor del último callback.
                            Expresion::Resultado => Some(Layout::Label {
                                texto: "resultado".to_string(),
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
        // `resultado` es keyword del lenguaje (valor de retorno en
        // postcondiciones), pero es también donde el runtime guarda el valor que
        // devuelve un callback: `escribir(resultado)` tiene que leer la variable.
        Expresion::Resultado => almacen.obtener("resultado").map(|v| ValorGUI::from_serde(&v)),
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
            crate::evaluador::ejecutar_funcion(
                nombre,
                &args,
                declaraciones,
                &crate::evaluador::AlmacenGui(&almacen_clonar),
            )
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
pub(super) fn estimate_hijo_ancho(layout: &Layout) -> Option<f64> {
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

