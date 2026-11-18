// Forja GUI Nativa — construye widgets xilem directamente desde el AST
// con soporte completo de tema Material You

use crate::helpers::*;
use crate::layout::AsUnit as _;
use crate::theme::animation::AnimationEngine;
use crate::theme::motion::{EASE_EMPHASIZED, EASE_STANDARD};
use crate::view;
use crate::Axis;
use crate::Length;
use crate::*;
use crate::{MessageResult};
use crate::{
    MaterialTheme,   // Tema Material You completo
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

// El layout del AST y su mapeo a widgets viven en archivos aparte:
// acá quedan el estado de la app, el arranque y los tests.
mod layout_nativo;
mod vista_nativa;

pub use layout_nativo::*;
pub use vista_nativa::*;

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

/// Pide a Windows que traiga la ventana propia al frente.
///
/// Se ejecuta en un hilo aparte, ~400 ms después: cuando vuelve
/// `construir_y_ejecutar` la ventana ya existe, y no se bloquea el event loop.
#[cfg(target_os = "windows")]
fn traer_al_frente_al_arrancar() {
    #[link(name = "user32")]
    extern "system" {
        fn EnumThreadWindows(
            hilo: u32,
            callback: extern "system" fn(isize, isize) -> i32,
            datos: isize,
        ) -> i32;
        fn SetForegroundWindow(ventana: isize) -> i32;
        fn ShowWindow(ventana: isize, comando: i32) -> i32;
        fn IsWindowVisible(ventana: isize) -> i32;
        fn GetClassNameW(ventana: isize, buffer: *mut u16, largo: i32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentThreadId() -> u32;
    }

    /// HWND encontrado (0 = ninguno).
    static ENCONTRADA: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

    extern "system" fn visitar(ventana: isize, _datos: isize) -> i32 {
        if unsafe { IsWindowVisible(ventana) } == 0 {
            return 1; // seguir
        }
        // Ignorar consolas: la app GUI tiene que ser la otra ventana del hilo.
        let mut clase = [0u16; 64];
        let largo = unsafe { GetClassNameW(ventana, clase.as_mut_ptr(), clase.len() as i32) };
        let clase = String::from_utf16_lossy(&clase[..largo.max(0) as usize]);
        if clase.contains("ConsoleWindow") {
            return 1;
        }
        ENCONTRADA.store(ventana, std::sync::atomic::Ordering::Release);
        0 // cortar
    }

    let hilo = unsafe { GetCurrentThreadId() };
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(400));
        unsafe {
            EnumThreadWindows(hilo, visitar, 0);
        }
        let ventana = ENCONTRADA.load(std::sync::atomic::Ordering::Acquire);
        if ventana != 0 {
            unsafe {
                ShowWindow(ventana, 9); // SW_RESTORE
                SetForegroundWindow(ventana);
            }
        }
    });
}

/// Fuera de Windows el gestor de ventanas ya enfoca la ventana nueva.
#[cfg(not(target_os = "windows"))]
fn traer_al_frente_al_arrancar() {}

/// Posición centrada en la pantalla primaria.
///
/// En Windows se consulta con `GetSystemMetrics` (no hay `Window` todavía: el
/// event loop se construye después, y `EventLoopBuilder` no expone monitores).
#[cfg(target_os = "windows")]
fn posicion_centrada(ancho: f64, alto: f64) -> Option<winit::dpi::Position> {
    #[link(name = "user32")]
    extern "system" {
        fn GetSystemMetrics(indice: i32) -> i32;
        fn GetDpiForSystem() -> u32;
    }
    const SM_CXSCREEN: i32 = 0;
    const SM_CYSCREEN: i32 = 1;
    let (ancho_pantalla, alto_pantalla, dpi) = unsafe {
        (
            f64::from(GetSystemMetrics(SM_CXSCREEN)),
            f64::from(GetSystemMetrics(SM_CYSCREEN)),
            f64::from(GetDpiForSystem()),
        )
    };
    if ancho_pantalla <= 0.0 || alto_pantalla <= 0.0 {
        return None;
    }
    let escala = if dpi > 0.0 { dpi / 96.0 } else { 1.0 };
    let x = ((ancho_pantalla - ancho * escala) / 2.0).max(0.0) as i32;
    let y = ((alto_pantalla - alto * escala) / 2.0).max(0.0) as i32;
    Some(winit::dpi::Position::Physical(
        winit::dpi::PhysicalPosition::new(x, y),
    ))
}

/// Fuera de Windows se deja la posición que elige el sistema.
#[cfg(not(target_os = "windows"))]
fn posicion_centrada(_ancho: f64, _alto: f64) -> Option<winit::dpi::Position> {
    None
}

/// Arma las `WindowOptions` con lo declarado en `ventana(...)`.
fn opciones_ventana(
    titulo: String,
    config: &crate::window_config::VentanaConfig,
    ancho: f64,
    alto: f64,
    posicion: Option<winit::dpi::Position>,
) -> WindowOptions<AppStateNativo> {
    let mut opciones = WindowOptions::new(titulo)
        .with_initial_inner_size(LogicalSize::new(ancho, alto))
        .with_resizable(config.redimensionable)
        .with_decorations(config.decoraciones);
    if let Some(posicion) = posicion {
        opciones = opciones.with_initial_position(posicion);
    }
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
    // Una app lanzada desde otra app (p. ej. `forja ejecutar` o una terminal)
    // nace DETRÁS de la ventana que la lanzó. Se trae al frente un instante
    // después de crear la ventana.
    traer_al_frente_al_arrancar();

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

    // Centrar la ventana en la pantalla. Sin esto Windows la ubica donde estaba
    // la última ventana (por ejemplo, debajo de la consola desde la que se lanzó).
    let posicion = posicion_centrada(initial_ancho, initial_alto);

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
            opciones_ventana(titulo, &config, initial_ancho, initial_alto, posicion),
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
    match crate::evaluador::ejecutar_funcion(
        nombre_fn,
        &[],
        programa,
        &crate::evaluador::AlmacenGui(&almacen),
    ) {
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

/// Click real sin ventana: recorre el mismo camino que dispara el botón.
///
/// `view::text_button` llama a `ejecutar_callback_y_actualizar(&callback, ...)`,
/// así que acá se busca el callback en el `Layout` y se invoca esa misma función.
/// No se usa `masonry_testing`: en esta revisión de xilem el `TestHarness` monta
/// widgets de Masonry, no vistas de Xilem, y el árbol de la app es una vista.
#[cfg(test)]
mod tests_click {
    use super::*;
    use forja_syntax::{lexer::Lexer, parser::Parser};

    const APP: &str = r#"
importar gui
funcion main() {
    variable cuenta = 0
    variable resultado = "0"
    columna(escribir(resultado), boton("+1", &incrementar))
}
funcion incrementar() -> Texto {
    cuenta = cuenta + 1
    retornar a_texto(cuenta)
}
"#;

    fn declaraciones(src: &str) -> Vec<Declaracion> {
        let mut lexer = Lexer::nuevo(src);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = Parser::nuevo(tokens);
        parser.parsear().expect("parse").declaraciones
    }

    fn callback_del_primer_boton(layout: &Layout) -> Option<String> {
        match layout {
            Layout::Button { callback, .. } | Layout::MaterialButton { callback, .. } => {
                Some(callback.clone())
            }
            Layout::Column { children, .. } | Layout::Row { children, .. } => {
                children.iter().find_map(callback_del_primer_boton)
            }
            _ => None,
        }
    }

    #[test]
    fn el_click_del_boton_actualiza_el_store_y_el_label() {
        let decls = declaraciones(APP);
        let mut state = AppStateNativo::nuevo();
        inicializar_estado(&decls, &mut state);

        let layout = extraer_layout(&decls, &mut state.almacen);
        let callback =
            callback_del_primer_boton(&layout)
                .unwrap_or_else(|| panic!("`boton(...)` tiene que traer su callback: {layout:?}"));
        assert_eq!(callback, "incrementar");

        // Esto es lo que ejecuta el botón al recibir el click.
        crate::helpers::ejecutar_callback_y_actualizar(&callback, &mut state, &decls);

        assert_eq!(state.almacen.obtener("cuenta"), Some(serde_json::json!(1)));
        assert_eq!(
            state.almacen.obtener("resultado"),
            Some(serde_json::json!("1")),
            "el valor del callback tiene que quedar en `resultado` (lo que pinta el label)"
        );

        // Re-render: el layout se vuelve a extraer con el estado nuevo y sigue
        // apuntando a la variable (el valor lo resuelve `expr_a_valor` al pintar).
        let layout2 = extraer_layout(&decls, &mut state.almacen);
        assert!(
            matches!(&layout2, Layout::Column { children, .. }
                if children.iter().any(|h| matches!(h, Layout::Label { es_variable: true, .. }))),
            "el label de `resultado` tiene que seguir siendo de variable: {layout2:?}"
        );
    }

    #[test]
    fn dos_clicks_seguidos_acumulan() {
        let decls = declaraciones(APP);
        let mut state = AppStateNativo::nuevo();
        inicializar_estado(&decls, &mut state);
        for _ in 0..2 {
            crate::helpers::ejecutar_callback_y_actualizar("incrementar", &mut state, &decls);
        }
        assert_eq!(state.almacen.obtener("cuenta"), Some(serde_json::json!(2)));
        assert_eq!(state.almacen.obtener("resultado"), Some(serde_json::json!("2")));
    }
}
