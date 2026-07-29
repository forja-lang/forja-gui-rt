# forja-gui-rt

Runtime pre-compilado para aplicaciones GUI del lenguaje **Forja (fa)**.
Basado en [`xilem`](https://github.com/linebender/xilem) v0.4 (Masonry + Vello), un framework UI reactivo con renderizado GPU.

---

## Índice

1. [Quick Start](#quick-start)
2. [Arquitectura](#arquitectura)
3. [Widgets de Layout](#widgets-de-layout)
4. [Widgets de Texto](#widgets-de-texto)
5. [Botones Material Design 3](#botones-material-design-3)
6. [Campos de Entrada](#campos-de-entrada)
7. [Selectores y Grupos](#selectores-y-grupos)
8. [Tarjetas, Listas y Tablas](#tarjetas-listas-y-tablas)
9. [Superposiciones y Feedback](#superposiciones-y-feedback)
10. [Navegación](#navegación)
11. [Indicadores, Avatares y Estados](#indicadores-avatares-y-estados)
12. [Gráficos y Charts](#gráficos-y-charts)
13. [Widgets Avanzados](#widgets-avanzados)
14. [Widgets Expressive](#widgets-expressive)
15. [Canvas de Dibujo](#canvas-de-dibujo)
16. [Iconos Material Design](#iconos-material-design)
17. [Sistema de Temas Material You](#sistema-de-temas-material-you)
18. [Estado Reactivo](#estado-reactivo)
19. [Callbacks y El Evaluador](#callbacks-y-el-evaluador)
20. [Animaciones y Motion](#animaciones-y-motion)
21. [Gestos y Touch](#gestos-y-touch)
22. [SQLite y Archivos](#sqlite-y-archivos)
23. [Accesibilidad](#accesibilidad)
24. [API Rust (bajo nivel)](#api-rust-bajo-nivel)
25. [Ejemplos Completos](#ejemplos-completos)

---

## Quick Start

Crea un archivo `app.fa`:

```fa
importar "gui"

funcion al_click() {
    escribir("¡Hiciste clic!")
}

funcion main() {
    columna(
        texto_grande("Mi App"),
        boton("Saludar", &al_click)
    )
}
```

Ejecutar con:

```bash
# Desde el CLI de Forja:
forja ejecutar --native app.fa

# O transpilar y compilar manualmente:
forja transpile app.fa
cd app
cargo run --release
```

---

## Arquitectura

```
Código Forja (.fa) ──→ AST (forja::ast::Programa)
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
      inicializar_estado  extraer_layout  build_and_run
       (evalúa vars y     (convierte AST    (crea ventana
        main() nativo)     → Layout enum)    Xilem + Vello)
                              │
                              ▼
                        layout_a_view
                     (Layout → AnyWidgetView)
```

**Flujo completo:**
1. El transpilador genera `static PROGRAMA: Programa` con el AST completo
2. `build_and_run()` inicia el runtime GUI
3. `inicializar_estado()` evalúa las variables del módulo y `main()` usando el evaluador tree-walking
4. `extraer_layout()` busca la función `main()` y convierte sus expresiones al enum `Layout`
5. `layout_a_view()` convierte cada `Layout` en un widget Xilem/Masonry
6. El event loop de Xilem maneja renders, interacciones y callbacks
7. Los callbacks ejecutan código Forja a través del evaluador, mutando el `VariableStore`
8. Xilem detecta los cambios via `memoize()` y re-renderiza los widgets afectados

---

## Widgets de Layout

### Columnas y Filas

```fa
columna(etiqueta("Uno"), etiqueta("Dos"))
fila(etiqueta("A"), etiqueta("B"))

// Con gap y alineación:
columna_con_gap(12, [etiqueta("Uno"), etiqueta("Dos")])
fila_con_gap(8, "center", [etiqueta("A"), etiqueta("B")])

// Centrado:
columna_centrada(etiqueta("Centrado"))
```

### Apilamiento (ZStack)

```fa
pila(
    etiqueta("Fondo"),
    boton("Frente", &fn)
)
```

### Scroll

```fa
desplazable(columna(etiqueta("Largo contenido...")))
```

### Contenedor con ancho máximo

```fa
contenedor(columna(...), 600)
```

### Espaciadores

```fa
espacio(24)         // Spacer fijo
expansor(etiqueta("Empuja al final"))
```

### Centrado

```fa
centrado(etiqueta("Centrado en eje transversal"))
```

### Relleno (Padding)

```fa
relleno(16, etiqueta("Con padding"))
```

### Relación de Aspecto

```fa
caja_relativa(16.0/9.0, video)
```

### Flex Layout

```fa
flex_layout("horizontal", 8, verdadero, [etiqueta("A"), etiqueta("B")])
flex_layout("vertical", 4, falso, [etiqueta("1"), etiqueta("2")])
```

### Flow Layout

```fa
flujo(4, [etiqueta("Tag1"), etiqueta("Tag2"), etiqueta("Tag3")])
```

### Layout Adaptable (Responsive)

Tres variantes según el ancho de ventana:
- Compact (< 600px)
- Medium (600–840px)
- Expanded (> 840px)

```fa
adaptable(
    columna(etiqueta("Compact")),     // compact
    fila(etiqueta("Medium")),         // medium
    fila(etiqueta("Expanded"))        // expanded
)
```

---

## Widgets de Texto

```fa
// Texto simple
etiqueta("Hola Mundo")

// Texto desde variable
etiqueta_dinamica("mi_variable")

// Título
titulo("Mi App")

// Texto coloreado
etiqueta_color("Rojo", "rojo")

// Tipografía Material 3 (15 estilos)
texto_grande("Display Large")       // 57sp
texto_mediano("Display Medium")     // 45sp
titular_grande("Headline Large")    // 32sp
titular_mediano("Headline Medium")  // 28sp
encabezado_grande("Title Large")    // 22sp
encabezado_mediano("Title Medium")  // 16sp, Medium weight
cuerpo_grande("Body Large")         // 16sp
cuerpo_mediano("Body Medium")       // 14sp
etiqueta_grande("Label Large")      // 14sp, Medium weight
etiqueta_pequeña("Label Small")     // 11sp, Medium weight

// Markdown básico
visor_markdown("# Título\n\nPárrafo con **negrita**")

// Texto enriquecido (HTML-like)
texto_enriquecido("<b>Negrita</b> y <i>itálica</i>")
```

---

## Botones Material Design 3

### Botones

```fa
// 5 variantes Material Design 3:
boton("Relleno", &fn)                     // Filled (default)
boton_relleno("Relleno", &fn)             // Filled
boton_tonal("Tonal", &fn)                // Tonal
boton_perfilado("Perfilado", &fn)         // Outlined
boton_texto("Texto", &fn)                // Text
boton_elevado("Elevado", &fn)            // Elevated

// Con icono:
boton("Con icono", &fn, "favorite")

// Botón deshabilitado:
boton("Inactivo", &fn, "home", falso, verdadero)
// argumentos: (texto, callback, icono, disabled)

// Con subrayado (boton básico):
boton_subrayado("Click", &fn)
```

### FAB (Floating Action Button)

```fa
fab("+", &agregar)                     // Medium (default)
fab_pequeño("+", &agregar)            // Small
fab_grande("+", &agregar)             // Large
fab_extendido("add", "Nuevo", &fn)    // Extended con texto
// argumentos fab_extendido: (icono, texto_extendido, callback)
```

### Icon Button

```fa
boton_icono("search", &buscar)              // Standard
boton_icono_relleno("search", &buscar)      // Filled
boton_icono_tonal("favorite", &like)        // Tonal
boton_icono_perfilado("close", &cerrar)     // Outlined

// Con selección (toggle):
boton_icono("favorite", &like, falso, seleccionado)
// argumentos: (icono, callback, [seleccionado])
```

### Botón Segmentado

```fa
segmentado(["Día", "Semana", "Mes"], &cambiar, seleccion)
segmentado_multiple(["A", "B", "C"], &cambiar, selecciones)
```

### Chip

```fa
subconjunto_asistente("Ayuda", &ayuda)
subconjunto_filtro("Filtrar", &filtrar, activo)
subconjunto_entrada("Tag", &click, &eliminar)   // con on_remove
subconjunto_sugerencia("Sugerencia", &sel)
```

---

## Campos de Entrada

### Texto

```fa
campo_texto("nombre", "Nombre", "Tu nombre")
campo_perfilado("email", "Email", "email@ejemplo.com")
campo_email("email")
campo_telefono("telefono")
campo_url("sitio")
campo_numero("edad", "Edad", 0, 150, 0)  // (variable, label, min, max, decimales)

// Campos especializados:
campo_contraseña("pass", "Contraseña")
campo_busqueda("query", "Buscar...")

// Con contador de caracteres y mensaje de error:
campo_texto("nombre", "Nombre", "Tu nombre", "filled", falso, "El nombre es obligatorio", verdadero)
// (variable, label, placeholder, variant, multiline, error, counter)

// Multilínea:
area_texto("descripcion", "Descripción")
```

### Dropdown y Select

```fa
// Dropdown cíclico (al hacer clic cambia al siguiente):
contraer_desplegable(["Op1", "Op2", "Op3"], seleccion, "Elige...")

// Menú de selección (con label):
menu_seleccion(["Op1", "Op2"], seleccionada, "Selecciona")

// Autocompletar:
autocompletar(["Manzana", "Banana", "Cereza"], "fruta")
```

### Switch y Checkbox

```fa
interruptor("Wifi", "wifi_activado")
casilla("Acepto términos", "terminos_aceptados")
```

### Sliders

```fa
// Continuo:
deslizante("volumen", 0, 100)

// Discreto:
deslizante_discreto("brillo", 0, 100, 10)   // steps=10

// Rango (dos valores):
deslizante_rango("temp_min", "temp_max", 0, 100)
```

---

## Selectores y Grupos

### Radio Button

```fa
grupo_radio("color", ["Rojo", "Verde", "Azul"], seleccion, &cambio_color, "horizontal")
// (nombre, opciones, selección, callback, dirección)
```

### Chip Group

```fa
grupo_subconjuntos(["Tag1", "Tag2", "Tag3"], selecciones, &cambio, verdadero)
// (chips, selecciones, callback, multiple)
```

### DatePicker

```fa
selector_fecha("fecha_nacimiento")
```

### TimePicker

```fa
selector_hora("hora_alarma")
```

### ColorPicker

```fa
selector_color("color_fondo")
```

---

## Tarjetas, Listas y Tablas

### Tarjetas (Cards)

```fa
tarjeta(columna(etiqueta("Título"), etiqueta("Subtítulo")))
tarjeta_elevada(columna(...))
tarjeta_perfilada(columna(...))
tarjeta_seleccionable(columna(...), &on_click, seleccionado)
```

### Listas

```fa
// Item simple:
elemento_lista("Título", "Subtítulo", icono_estrella(), &on_click)
elemento_lista_doble("Título", "Subtítulo línea 2", icono_check(), &on_click)

// Lista completa:
lista([item1, item2, item3])
lista_con_dividores([item1, item2, item3])

// Lista con controles (checkboxes/switches):
lista_control([item1, item2], "checkbox", ["var1", "var2"])

// Lista de selección:
lista_seleccion([item1, item2, item3], selecciones, &on_select, verdadero)
```

### Data Table

```fa
tabla_datos(
    ["Nombre", "Edad", "Ciudad"],
    [["Ana", "30", "Bs As"], ["Luis", "25", "Córdoba"]]
)
tabla_ordenable(...)     // con ordenamiento por columna
tabla_seleccion(...)     // con selección de filas
```

### Superficies

```fa
superficie(columna(...))
superficie_tonal(columna(...))
```

### Scaffold

```fa
andamio(
    barra_superior("App", [icono_buscar()]),  // top
    columna(...),                               // body
    barra_inferior([icono_casa()]),             // bottom (opcional)
    fab("+", &agregar)                          // fab (opcional)
)
```

---

## Superposiciones y Feedback

### Diálogos

```fa
// Diálogo de alerta simple:
dialogo_alerta("Error", "Ocurrió un error", "OK", "", &ok, &ignorar)

// Diálogo de confirmación:
dialogo_confirmacion("¿Salir?", "¿Estás seguro?", "Salir", "Cancelar", &salir, &cancelar)

// Diálogo personalizado con contenido:
dialogo_personalizado("Configurar", columna(...), &cerrar)

// Diálogo a pantalla completa:
dialogo_completo("Editor", columna(...), &cerrar)

// Diálogo superpuesto (toggle con variable):
dialogo_superpuesto(columna(...), "dialogo_visible")
```

### Bottom Sheet

```fa
hoja_inferior(columna(...), "sheet_visible")
hoja_inferior_modal(columna(...), "sheet_visible", &al_cerrar)
hoja_inferior_grande(columna(...), "sheet_visible")
```

### Snackbar

```fa
notificación("Mensaje guardado")
notificación_accion("¿Deshacer?", "Deshacer", &deshacer, 4000, "snack_visible")
// (mensaje, texto_accion, callback_accion, duración_ms, variable_visible)
```

### Tooltip

```fa
información(boton("Info", &fn), "Esto es un tooltip")
```

### Menú

```fa
menú_desplegable(["Op1", "Op2", "Op3"], &on_select, "menu_visible")
menú_contexto(["Copiar", "Pegar", "Eliminar"], &on_select, "ctx_visible")
```

---

## Navegación

### Navigator (gestor de pantallas)

El `navegador` es el sistema de navegación completo. Soporta bottom bar, tabs, rail y drawer.

```fa
variable pantalla_actual = "inicio"

funcion cambiar_pantalla(id) {
    pantalla_actual = id
}

funcion main() {
    navegador(navegador_pantallas(
        pantalla("Inicio", columna(etiqueta("Home"))),
        pantalla("Perfil", columna(etiqueta("Profile"))),
        pantalla("Ajustes", columna(etiqueta("Settings")))
    ), "pantalla_actual", "barra", &cambiar_pantalla)
}
```

**Argumentos:** `(pantallas, variable_indice, tipo, callback_on_change, [animación])`

| Tipo | Descripción |
|------|-------------|
| `"ninguno"` | Solo contenido, sin navegación |
| `"barra"` / `"bottom"` | Bottom navigation bar |
| `"riel"` / `"rail"` | Navigation rail (lateral) |
| `"pestañas"` / `"tabs"` | Tabs superiores |
| `"cajón"` / `"drawer"` | Drawer lateral |

**Animaciones (5to argumento opcional):**
- `"fade"` — Fundido entre pantallas
- `"slide"` — Deslizamiento horizontal
- `"ninguno"` — Sin animación (default)

### Barra Superior (TopAppBar)

```fa
barra_superior("Título", [icono_buscar(), icono_mas()])
barra_superior_media("Título", [icono_buscar()])
barra_superior_grande("Título", [icono_buscar()])
```

### Barra Inferior (BottomAppBar)

```fa
barra_inferior([icono_casa(), icono_buscar(), icono_perfil()])
```

### Tabs

```fa
pestañas(["Tab1", "Tab2", "Tab3"], seleccion, &on_change)
pestañas_desplazables(["A", "B", "C", "D", "E", "F"], sel, &on_change)
```

### Search Bar

```fa
barra_busqueda("Buscar...", &on_search, "query")
```

### Search View

```fa
vista_busqueda("query", [resultado1, resultado2], "resultados_visibles")
```

### NavigationRail / Drawer

```fa
// Barra lateral angosta:
riel_navegacion(
    [item_nav("Inicio", "home"), item_nav("Perfil", "person")],
    seleccion, &on_change
)

// Drawer expandido:
cajon_navegacion(items, seleccion, &on_change)
cajon_modal(items, seleccion, &on_change, "drawer_visible")
```

---

## Indicadores, Avatares y Estados

### Progress Indicators

```fa
// Barra de progreso:
barra_progreso("progreso_var")                 // 0.0–1.0
barra_progreso_indeterminada()                  // animación infinita

// Circular:
circulo_progreso("progreso_var", 48)            // tamaño 48px
circulo_progreso_indeterminado(48)
```

### Badge

```fa
distintivo(boton("Notif", &fn), "5")           // con número
distintivo_punto(boton("Notif", &fn))          // punto rojo
```

### Skeleton (placeholder de carga)

```fa
esqueleto(200, 20)                              // (ancho, alto)
esqueleto_tarjeta(300, 100)                     // tarjeta
esqueleto_linea(200, 16)                        // línea de texto
```

### EmptyState / ErrorState

```fa
estado_vacio("search", "Sin resultados", "Recargar", &recargar)
estado_error("Error de conexión", &reintentar)
```

### Avatares

```fa
avatar("JD")                                    // iniciales
avatar_icono("person")                          // icono
avatar_imagen("JD", "imagen")                   // texto + imagen

grupo_avatar(["A", "B", "C"], 3)               // max 3 visibles
```

---

## Gráficos y Charts

### Line Chart

```fa
gráfico_linea([10, 25, 15, 30, 20], "azul", ["Ene", "Feb", "Mar", "Abr", "May"])
```

### Bar Chart

```fa
gráfico_barras([30, 50, 20], ["#f59e0b", "#6366f1", "#10b981"], ["A", "B", "C"], falso)
gráfico_barras([...], [...], [...], verdadero)  // apilado
```

### Pie / Donut Chart

```fa
gráfico_pastel([30, 50, 20], ["A", "B", "C"])
gráfico_donut([30, 50, 20], ["A", "B", "C"])
```

### Gauge

```fa
gráfico_indicador(75, 0, 100, "#10b981")       // (valor, min, max, color)
```

### Sparkline

```fa
minigráfico([5, 10, 8, 15, 12, 20], "#6366f1")
```

---

## Widgets Avanzados

### Star Rating

```fa
calificación(3, 5, &on_rating)                  // (valor, max, callback)
```

### Stepper

```fa
asistente_pasos(["Paso 1", "Paso 2", "Paso 3"], paso_actual, &on_step)
```

### Breadcrumbs

```fa
migaja_de_pan(["Inicio", "Perfil", "Ajustes"], "→")
```

### Calendar

```fa
calendario(7, 2026, "fecha_sel", &on_date)     // (mes, año, variable, callback)
```

### QR Code

```fa
visor_qr("https://forja-lang.github.io", 200)
```

### File Picker

```fa
selector_archivo(["txt", "json"], falso, &on_file)
selector_archivo(["image"], verdadero, &on_files)  // multiple
```

### Markdown Viewer

```fa
visor_markdown("# Título\n\nPárrafo con **negrita**")
```

---

## Widgets Expressive

### Glass Card (Glassmorphism)

```fa
tarjeta_vidrio(columna(etiqueta("Contenido")), 10, 0.3)
// (child, blur, opacity)
```

### Gradient Box

```fa
gradiente_lineal(["#f59e0b", "#ef4444"], "vertical", columna(...))
gradiente_radial(["#6366f1", "#ec4899"], "center", columna(...))
```

### Morphing Button

```fa
boton_morphing("add", "Nuevo elemento", &on_click)
// (icono, texto_extendido, callback)
```

### Expressive Background

```fa
fondo_expresivo(["#1a1a2e", "#16213e"], verdadero)   // animado
```

### Glow Border

```fa
efecto_brillo(columna(...), "#f59e0b", 2.0)   // (child, color, ancho)
```

---

## Canvas de Dibujo

Canvas interactivo que renderiza comandos Vello desde una variable.

```fa
variable comandos = '[]'

funcion dibujar() {
    comandos = '[{"FillCircle": {"x": 100, "y": 100, "radius": 50, "color": "#f59e0b"}}]'
}

funcion main() {
    columna(
        lienzo("comandos", 400, 300),
        boton("Dibujar", &dibujar)
    )
}
```

**Comandos de canvas:**

| Comando | Parámetros |
|---------|------------|
| `FillCircle` | `{x, y, radius, color}` |
| `StrokeCircle` | `{x, y, radius, color, stroke_width}` |
| `FillRect` | `{x, y, width, height, color}` |
| `StrokeRect` | `{x, y, width, height, color, stroke_width}` |
| `FillLine` | `{x1, y1, x2, y2, color}` |
| `StrokeLine` | `{x1, y1, x2, y2, color, stroke_width}` |
| `FillPath` | `{path_data, color}` |
| `StrokePath` | `{path_data, color, stroke_width}` |
| `FillText` | `{x, y, text, size, color}` |
| `SetFillColor` | `{color}` |
| `SetStrokeColor` | `{color}` |
| `ClearCanvas` | `{}` |
| `SetStrokeWidth` | `{width}` |
| `FillPolygon` | `{points: [[x,y],...], color}` |
| `StrokePolygon` | `{points: [[x,y],...], color, stroke_width}` |
| `FillRoundedRect` | `{x, y, width, height, radius, color}` |
| `StrokeRoundedRect` | `{x, y, width, height, radius, color, stroke_width}` |
| `DrawEllipse` | `{x, y, rx, ry, color, filled}` |
| `DrawArc` | `{x, y, radius, start_angle, end_angle, color, stroke_width}` |
| `DrawBezier` | `{x1, y1, cx1, cy1, cx2, cy2, x2, y2, color, stroke_width}` |
| `DrawImage` | `{x, y, width, height, base64_data}` |
| `DrawLinearGradient` | `{x, y, width, height, colors: [...], angle}` |

---

## Iconos Material Design

```fa
// Icono simple (emoji fallback):
icono_material("home", 24, "#f59e0b")
icono_material("favorite", 32, "#ef4444")

// Con estilo:
icono_relleno("star", 24, "#f59e0b")
icono_perfilado("home", 24, "#6366f1")
icono_redondo("search", 24, "#10b981")
icono_agudo("settings", 24, "#ec4899")
icono_dos_tonos("email", 24, "#6366f1")
```

### Catálogo de iconos (70+)

**Navegación:** `home`, `search`, `settings`, `menu`, `arrow_back`, `arrow_forward`, `arrow_drop_down`, `arrow_up`, `arrow_down`, `chevron_left`, `chevron_right`, `more_vert`, `close`, `refresh`

**Acción:** `add`, `add_circle`, `delete`, `edit`, `save`, `copy`, `done`, `check`, `check_circle`, `cancel`, `print`, `share`, `open_in_new`, `lock`, `lock_open`, `visibility`, `visibility_off`

**Contenido:** `filter`, `sort`, `send`, `cloud`, `cloud_download`, `cloud_upload`, `link`, `flag`

**Comunicación:** `email`, `phone`, `chat`, `notifications`, `person`, `group`, `forum`

**Archivo:** `file`, `folder`, `folder_open`, `file_upload`, `download`, `upload`, `attach_file`, `image`, `description`, `picture_as_pdf`

**Dispositivo:** `wifi`, `bluetooth`, `battery_full`, `signal`, `location`

**Editor:** `code`, `format_bold`, `format_italic`, `format_underline`, `format_list`, `format_size`, `undo`, `redo`

**Imagen:** `photo`, `camera`, `brush`, `palette`

**Mapas:** `place`, `directions`, `map`, `local_shipping`, `restaurant`, `hotel`

**Notificación:** `info`, `warning`, `error`, `warning_amber`, `feedback`, `help`, `new_releases`

**Rating:** `favorite`, `favorite_outline`, `star`, `star_half`, `star_outline`, `thumb_up`, `thumb_down`

**Social:** `share_social`, `public`, `school`, `work`, `celebration`

**Toggle:** `check_box`, `check_box_outline`, `radio_button_checked`, `radio_button_unchecked`, `toggle_on`, `toggle_off`

**Fecha/Hora:** `date`, `calendar_today`, `time`, `alarm`

**Comercio:** `shopping_cart`, `payment`, `account_balance`, `store`, `trending_up`

---

## Sistema de Temas Material You

### Uso básico

El tema se detecta automáticamente del sistema operativo. Se puede personalizar con un color semilla:

```fa
// Usar tema con color semilla personalizado:
tema_material(columna(
    boton_relleno("Botón", &fn)
), "#f59e0b")

// Tema claro:
tema_material(columna(...), "#6366f1", falso)   // is_dark=false

// Tema oscuro:
tema_material(columna(...), "#10b981", verdadero)
```

### Colores del Scheme

| Función | Color Role |
|---------|-----------|
| `color_primario(child)` | `primary` |
| `color_sobre_primario(child)` | `on_primary` |
| `color_primario_contenedor(child)` | `primary_container` |
| `color_secundario(child)` | `secondary` |
| `color_terciario(child)` | `tertiary` |
| `color_error(child)` | `error` |
| `color_superficie(child)` | `surface` |
| `color_fondo(child)` | `background` |
| `color_perfil(child)` | `outline` |

**Roles disponibles:** `primary`, `on_primary`, `primary_container`, `on_primary_container`, `secondary`, `on_secondary`, `secondary_container`, `on_secondary_container`, `tertiary`, `on_tertiary`, `tertiary_container`, `on_tertiary_container`, `error`, `on_error`, `error_container`, `on_error_container`, `surface`, `on_surface`, `surface_variant`, `on_surface_variant`, `background`, `on_background`, `outline`, `outline_variant`, `inverse_surface`, `inverse_on_surface`, `inverse_primary`

### Shapes (Esquinas)

```fa
esquinas_pequeñas(child)    // 8px radius
esquinas_medianas(child)    // 12px
esquinas_grandes(child)     // 16px
esquinas_completas(child)   // 28px o 50%
```

### Sombras (Elevation)

```fa
sombra(child, 1)            // level 1 (1dp)
sombra(child, 3)            // level 3 (6dp)
// Niveles: 0 (0dp), 1 (1dp), 2 (3dp), 3 (6dp), 4 (8dp), 5 (12dp)
```

### Tipografía

Las funciones `texto_grande`, `titular_grande`, etc. (ver [Widgets de Texto](#widgets-de-texto)) usan estilos predefinidos de Material Design 3. Se pueden anidar dentro de `tema_material` para personalización.

---

## Estado Reactivo

### Variables

Las variables de Forja se sincronizan automáticamente con los widgets:

```fa
variable nombre = "Invitado"

funcion saludar() {
    escribir("Hola, " + nombre)
}

funcion main() {
    columna(
        campo_texto("nombre", "Nombre"),
        boton("Saludar", &saludar)
    )
}
```

Cuando `nombre` cambia (por el `campo_texto`), todos los widgets que la lean se re-renderizan automáticamente vía `memoize()`.

### VariableStore (API Rust)

```rust
// Las variables se almacenan como Signal<serde_json::Value> con generación atómica
store.get("nombre");             // → Option<serde_json::Value>
store.set("nombre", value);      // → u64 (nueva generación)
store.generation("nombre");      // → u64
store.global_generation();       // → generación global
store.contains("nombre");        // → bool
store.snapshot();                // → HashMap<String, Value> (para hot reload)
store.init_from(iterador);       // inicialización batch
```

### Hot Reload

El estado se preserva entre recargas gracias a `snapshot()` y `cargar_estado()`:

```bash
forja ejecutar --native app.fa     # primera ejecución
# Al modificar app.fa y re-ejecutar, el estado se restaura
```

---

## Callbacks y El Evaluador

Los callbacks se conectan con `&nombre_funcion`:

```fa
funcion al_click() {
    escribir("Click!")
    mi_variable = mi_variable + 1
}

funcion procesar(texto) {
    resultado = texto_en_mayusculas(texto)
}

funcion main() {
    columna(
        campo_texto("nombre"),
        boton("Procesar", &procesar),
        boton("Click", &al_click)
    )
}
```

### Funciones Built-in del Evaluador

| Función | Descripción |
|---------|-------------|
| `timestamp()` | Milisegundos desde Unix epoch |
| `fecha_desde_timestamp(ts)` | Formatea timestamp → `"YYYY\|MM\|DD\|HH\|mm\|ss\|n_dia\|n_mes"` |
| `dividir(texto, sep)` | Divide string → JSON array |
| `a_numero(valor)` | Convierte a entero |
| `longitud(valor)` | Longitud de array/text |
| `tipo(valor)` | Nombre del tipo: "entero", "decimal", "texto", etc. |
| `a_texto(valor)` | Convierte a string de display |
| `empujar(array_json, elemento)` | Agrega a array JSON |

### JSON

```fa
json_str = _json_stringificar(mi_variable)
```

### Archivos

```fa
contenido = _archivo_leer("ruta/archivo.txt")
bytes = _archivo_escribir("ruta/archivo.txt", contenido)
```

---

## SQLite

El runtime incluye soporte SQLite completo con bind parameters:

```fa
funcion conectar() {
    db = _sqlite_abrir("datos.db")
    _sqlite_ejecutar(db, "CREATE TABLE IF NOT EXISTS usuarios (id INTEGER PRIMARY KEY, nombre TEXT)")
}

funcion insertar(nombre) {
    db = 0  // o pasar por variable
    _sqlite_ejecutar_params(db, "INSERT INTO usuarios (nombre) VALUES (?)", [nombre])
    nuevo_id = _sqlite_ultimo_id(db)
}

funcion listar() {
    db = 0
    resultado_json = _sqlite_consultar(db, "SELECT * FROM usuarios")
    // resultado_json es un Texto con JSON: [{"id":1,"nombre":"Ana"},...]

    // Con parámetros:
    resultado_json = _sqlite_consultar_params(db, "SELECT * FROM usuarios WHERE id > ?", [min_id])
}
```

| Función | Descripción |
|---------|-------------|
| `_sqlite_abrir(ruta)` | Abre BD → índice (Entero) |
| `_sqlite_cerrar(indice)` | Cierra conexión |
| `_sqlite_ejecutar(idx, sql)` | Ejecuta SQL → filas afectadas |
| `_sqlite_consultar(idx, sql)` | Consulta → JSON array |
| `_sqlite_ejecutar_params(idx, sql, valores)` | Con bind parameters |
| `_sqlite_consultar_params(idx, sql, valores)` | Con bind parameters |
| `_sqlite_ultimo_id(idx)` | Último rowid insertado |
| `_sqlite_tablas(idx)` | Lista de tablas → JSON |
| `_sqlite_columnas(idx, tabla)` | Columnas de tabla → JSON |

---

## Animaciones y Motion

### FadeTransition

```fa
transición(columna(
    etiqueta("Aparece/desaparece")
), "visible", 300)
// (child, variable_bool, duración_ms)
```

### RippleEffect

```fa
efecto_onda(boton("Click", &fn))
```

### Sistema de Easing (API Rust)

```rust
use forja_gui_rt::theme::motion::*;

EASE_STANDARD     // (0.2, 0.0, 0.0, 1.0)
EASE_EMPHASIZED   // (0.3, 0.0, 0.0, 1.0)
EASE_DECELERATE   // (0.0, 0.0, 0.0, 1.0)
EASE_ACCELERATE   // (0.3, 0.0, 1.0, 1.0)
EASE_EXPRESSIVE   // (0.34, 1.56, 0.64, 1.0)

// Aplicar:
let result = EASE_EMPHASIZED.apply(0.5); // → 0.604...
```

### AnimationEngine (API Rust)

```rust
use forja_gui_rt::theme::animation::*;

let mut engine = AnimationEngine::new();
engine.begin_frame(16.0); // delta_ms

// AnimatedValue (from → to)
let mut anim = AnimatedValue::new(0.0, 1.0, 300.0, EASE_EMPHASIZED);
engine.add_animation(&mut anim);
anim.update(16.0); // → 0.08...
anim.value();      // → valor interpolado
anim.is_finished();

// SpringAnimation
let mut spring = SpringAnimation::new(1.0);
spring.with_rigidez(200.0).with_amortiguacion(15.0);
spring.update(16.0);
```

### AnimationPresets

```rust
AnimationPresets::button_ripple()        // 150ms, emphasized
AnimationPresets::button_hover()         // 100ms, standard
AnimationPresets::card_elevate()         // 200ms, standard
AnimationPresets::page_transition()      // 300ms, emphasized
AnimationPresets::spinner_loop()         // 800ms, standard (loop)
AnimationPresets::expressive_fade()      // 450ms, expressive
AnimationPresets::expressive_morph()     // 500ms, expressive
```

---

## Gestos y Touch

### Pull to Refresh

```fa
pull_to_refresh(columna(
    etiqueta("Tira para recargar")
), &recargar, "refrescando")
```

### Swipe to Dismiss

```fa
swipe_to_dismiss(tarjeta(columna(...)), &al_descartar, "Descartar", "descartado")
```

### Pinch to Zoom

```fa
zoom_pellizco(imagen, 0.5, 3.0)      // (child, min_scale, max_scale)
```

### Rotate

```fa
rotar(imagen)
```

---

## Accesibilidad

El runtime emite descripciones de accesibilidad para TalkBack/VoiceOver automáticamente. Soporta roles Material Design:

```
Button, Label, TextInput, Slider, Checkbox, Switch,
ProgressBar, Image, Navigation, Tab, Drawer, Dialog,
BottomSheet, Tooltip, Menu, Chip, Card, List, Avatar
```

Desde el código Rust se puede anunciar:

```rust
state.a11y_say("Bienvenido a la app");
state.a11y_focus("button", "Enviar", "", "Activo");
```

---

## API Rust (bajo nivel)

```rust
use forja_gui_rt::*;

// Punto de entrada
build_and_run(&programa, load_state, theme, auto_theme)?;

// Conversión de expresiones
let layout = extraer_layout(&declaraciones);
let layout = expr_a_layout(&expresion);

// Renderizado
let view = layout_a_view(&layout, &mut state, &programa, &theme);

// Estado
let val = state.leer("nombre");
state.escribir("nombre", ValorGUI::Texto("Juan"));

// Callbacks
ejecutar_callback_forja("fn_name", &state, &programa);
ejecutar_callback_y_actualizar("fn_name", &mut state, &programa);

// Ventana
current_window_width();  // obtener ancho actual
WindowSizeClass::from_width(600.0);  // Compact, Medium, Expanded
```

---

## Ejemplos Completos

### App de Contactos

```fa
importar "gui"

variable contactos = '[{"nombre":"Ana","tel":"123"},{"nombre":"Luis","tel":"456"}]'

funcion main() {
    columna(
        barra_superior("Contactos"),
        boton("Agregar", &agregar),
        lista_con_dividores([
            elemento_lista("Ana", "123", icono_material("person", 24, "#6366f1"), &ver_ana),
            elemento_lista("Luis", "456", icono_material("person", 24, "#10b981"), &ver_luis)
        ])
    )
}
```

### Dashboard con Charts

```fa
importar "gui"

funcion main() {
    columna(
        texto_grande("Dashboard"),
        fila(
            tarjeta(columna(
                etiqueta("Ventas"),
                gráfico_linea([30, 50, 20, 80, 45], "#6366f1", ["Ene", "Feb", "Mar", "Abr", "May"])
            )),
            tarjeta(columna(
                etiqueta("Distribución"),
                gráfico_donut([45, 30, 25], ["A", "B", "C"])
            ))
        ),
        tarjeta(columna(
            etiqueta("Rendimiento"),
            gráfico_indicador(75, 0, 100, "#10b981")
        ))
    )
}
```

### Formulario con Tema

```fa
importar "gui"

variable nombre = ""
variable email = ""
variable acepta = falso

funcion enviar() {
    si (nombre != "" && email != "" && acepta) {
        notificación("Formulario enviado")
    } sino {
        notificación("Completa todos los campos")
    }
}

funcion main() {
    tema_material(columna(
        titulo("Registro"),
        campo_texto("nombre", "Nombre", "Tu nombre"),
        campo_email("email"),
        casilla("Acepto términos", "acepta"),
        boton("Enviar", &enviar, "send")
    ), "#6366f1")
}
```

### Navegación por Tabs con SQLite

```fa
importar "gui"

variable tab_actual = 0

funcion cambiar_tab(tab) {
    tab_actual = tab
}

funcion main() {
    navegador(navegador_pantallas(
        pantalla("Inicio", columna(
            etiqueta_grande("Bienvenido"),
            boton("Abrir BD", &abrir_db)
        )),
        pantalla("Datos", columna(
            etiqueta_grande("Registros"),
            boton("Cargar", &cargar)
        )),
        pantalla("Gráficos", columna(
            gráfico_barras([10, 20, 15], [], ["Ene", "Feb", "Mar"])
        ))
    ), "tab_actual", "pestañas", &cambiar_tab, "fade")
}
```

---

## Compilación y Distribución

### Transpilar a Rust

```bash
forja transpile app.fa
cd app/
cargo build --release
./target/release/app
```

### Ejecutable independiente (AOT)

```bash
forja build app.fa
./app.exe
```

### Ejecución directa con hot reload

```bash
forja ejecutar --native app.fa
# Al modificar app.fa y re-ejecutar:
forja ejecutar --native app.fa --load-state=<snapshot>
# O simplemente:
forja ejecutar --native app.fa
# (el estado se restaura automáticamente)
```
