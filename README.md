# forja-gui-rt

Runtime pre-compilado para aplicaciones GUI de **Forja (fa)**.

Basado en [`xilem`](https://github.com/linebender/xilem) v0.4, un framework UI reactivo con GPU (Vello).

## ¿Qué hace?

Pre-compila xilem y masonry para que las apps GUI de Forja se compilen mucho más rápido. En lugar de compilar xilem desde cero cada vez, se compila una vez y se reusa.

## Dependencias

- `xilem = "0.4"` — Framework UI reactivo
- `serde_json = "1"` — Para el sistema de variables reactivas

## Uso desde Forja

```fa
importar "gui"

funcion main() {
    columna(
        texto_grande("Mi App"),
        boton("Click", &saludar)
    )
}
```

Luego ejecutar con:

```bash
cargo run --features gui -- run app.fa
```

## Módulos re-exportados

El crate re-exporta los tipos principales de xilem:

- `WidgetView`, `AnyWidgetView` — Traits para widgets
- `Xilem`, `WindowOptions`, `EventLoop` — Inicialización de ventana
- `view::{flex, label, button, text_input, ...}` — Widgets
- `Length`, `Axis`, `FontWeight` — Layout y estilo
- `MaterialTheme`, `ColorScheme`, `RgbColor` — Tema Material You
- `icons` — Catálogo de iconos Material Design (vía emoji)
