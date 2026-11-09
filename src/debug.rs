// Forja GUI Runtime — Módulo de Depuración y Diagnóstico (forja-gui-rt)
//
// Proporciona trazabilidad completa del estado de la aplicación GUI:
//   - Inspección del árbol de Layouts
//   - Registro de mutaciones en VariableStore
//   - Trazado de llamadas y errores en Callbacks
//   - Errores sintácticos / de parámetros en widgets
//
// Se activa mediante la variable de entorno FORJA_GUI_DEBUG=1 o FORJA_DEBUG=1

/// Retorna true si el modo depuración de GUI está activo
pub fn es_depuracion_enabled() -> bool {
    true // FORZADO PARA DEPURACIÓN INTEGRAL EN ANDROID
}

/// Imprime un mensaje formateado con prefijo si el modo debug está activo
pub fn depuracion_registrar(category: &str, message: &str) {
    if es_depuracion_enabled() {
        log::debug!("[Forja GUI::{}] {}", category, message);
        eprintln!("🔍 [Forja GUI::{}] {}", category, message);
    }
}

/// Registra el cambio de valor de una variable reactiva en el VariableStore
pub fn log_state_change(
    var_name: &str,
    old_val: Option<&serde_json::Value>,
    nuevo_val: &serde_json::Value,
) {
    if es_depuracion_enabled() {
        match old_val {
            Some(old) => {
                log::info!(
                    "[GUI State] Variable '{}' cambió: {} ──→ {}",
                    var_name,
                    old,
                    nuevo_val
                );
                eprintln!(
                    "⚡ [GUI State] Variable '{}' cambió: {} ──→ {}",
                    var_name, old, nuevo_val
                );
            }
            None => {
                log::info!(
                    "[GUI State] Variable '{}' inicializada: {}",
                    var_name,
                    nuevo_val
                );
                eprintln!(
                    "⚡ [GUI State] Variable '{}' inicializada: {}",
                    var_name, nuevo_val
                );
            }
        }
    }
}

/// Registra el inicio de ejecución de un callback
pub fn registrar_callback_inicio(nombre_fn: &str) {
    if es_depuracion_enabled() {
        log::info!("[GUI Callback] Ejecutando callback '{}'...", nombre_fn);
        eprintln!("▶️ [GUI Callback] Ejecutando callback '{}'...", nombre_fn);
    }
}

/// Registra el éxito de un callback
pub fn registrar_callback_exito(nombre_fn: &str, resultado: &str) {
    if es_depuracion_enabled() {
        log::info!(
            "[GUI Callback] Callback '{}' completado con éxito: {}",
            nombre_fn,
            resultado
        );
        eprintln!(
            "✅ [GUI Callback] Callback '{}' completado con éxito: {}",
            nombre_fn, resultado
        );
    }
}

/// Registra un error durante la ejecución de un callback o evaluador
pub fn log_callback_error(nombre_fn: &str, error: &str) {
    // Los errores SIEMPRE se imprimen en log y stderr para no ocultar fallos
    log::error!("[GUI Error] Error en callback '{}': {}", nombre_fn, error);
    eprintln!(
        "❌ [GUI Error] Error en callback '{}': {}",
        nombre_fn, error
    );
}

/// Registra una advertencia de conversión de layout
pub fn registrar_layout_aviso(widget_name: &str, reason: &str) {
    log::warn!("[GUI Layout Warning] Widget '{}': {}", widget_name, reason);
    eprintln!(
        "⚠️ [GUI Layout Warning] Widget '{}': {}",
        widget_name, reason
    );
}

/// Convierte un Layout a una representación textual en árbol con sangría
pub fn formato_layout_arbol(layout: &crate::gui_nativa::Layout, depth: usize) -> String {
    let sangria = "  ".repeat(depth);
    use crate::gui_nativa::Layout::*;

    match layout {
        Column {
            children,
            gap,
            alignment,
        } => {
            let mut out = format!("{}Column (gap: {}, align: {})\n", sangria, gap, alignment);
            for c in children {
                out.push_str(&formato_layout_arbol(c, depth + 1));
            }
            out
        }
        CenteredColumn(children) => {
            let mut out = format!("{}CenteredColumn\n", sangria);
            for c in children {
                out.push_str(&formato_layout_arbol(c, depth + 1));
            }
            out
        }
        Row {
            children,
            gap,
            alignment,
        } => {
            let mut out = format!("{}Row (gap: {}, align: {})\n", sangria, gap, alignment);
            for c in children {
                out.push_str(&formato_layout_arbol(c, depth + 1));
            }
            out
        }
        ZStack(children) => {
            let mut out = format!("{}ZStack\n", sangria);
            for c in children {
                out.push_str(&formato_layout_arbol(c, depth + 1));
            }
            out
        }
        Container { hijo, max_ancho } => {
            format!(
                "{}Container (max_width: {})\n{}",
                sangria,
                max_ancho,
                formato_layout_arbol(hijo, depth + 1)
            )
        }
        Portal(hijo) => {
            format!(
                "{}Portal\n{}",
                sangria,
                formato_layout_arbol(hijo, depth + 1)
            )
        }
        Label { texto, es_variable } => {
            if *es_variable {
                format!("{}Label (var: '{}')\n", sangria, texto)
            } else {
                format!("{}Label (\"{}\")\n", sangria, texto)
            }
        }
        VariableLabel { variable } => {
            format!("{}VariableLabel ('{}')\n", sangria, variable)
        }
        Title(texto) => {
            format!("{}Title (\"{}\")\n", sangria, texto)
        }
        ColoredLabel { texto, color } => {
            format!(
                "{}ColoredLabel (\"{}\", color: {})\n",
                sangria, texto, color
            )
        }
        MaterialButton {
            texto,
            callback,
            variant,
            disabled,
            ..
        } => {
            format!(
                "{}MaterialButton (\"{}\", cb: '{}', variant: {:?}, disabled: {})\n",
                sangria, texto, callback, variant, disabled
            )
        }
        TextInput {
            variable,
            multiline,
            placeholder,
        } => {
            format!(
                "{}TextInput (var: '{}', multiline: {}, placeholder: \"{}\")\n",
                sangria, variable, multiline, placeholder
            )
        }
        ProgressBar { variable } => {
            format!("{}ProgressBar (var: '{}')\n", sangria, variable)
        }
        Slider { variable, min, max } => {
            format!(
                "{}Slider (var: '{}', min: {}, max: {})\n",
                sangria, variable, min, max
            )
        }
        Checkbox { variable } => {
            format!("{}Checkbox (var: '{}')\n", sangria, variable)
        }
        Prose(texto) => {
            format!("{}Prose (\"{}\")\n", sangria, texto)
        }
        Spinner => format!("{}Spinner\n", sangria),
        Separator => format!("{}Separator\n", sangria),
        Spacer(tamano) => format!("{}Spacer ({})\n", sangria, tamano),
        Expanded { hijo } => format!(
            "{}Expanded\n{}",
            sangria,
            formato_layout_arbol(hijo, depth + 1)
        ),
        Centered { hijo } => format!(
            "{}Centered\n{}",
            sangria,
            formato_layout_arbol(hijo, depth + 1)
        ),
        Padding { hijo, amount } => format!(
            "{}Padding ({})\n{}",
            sangria,
            amount,
            formato_layout_arbol(hijo, depth + 1)
        ),
        ResponsiveLayout {
            compact,
            medium,
            expanded,
        } => {
            let mut out = format!("{}ResponsiveLayout\n", sangria);
            out.push_str(&format!("{}  [Compact]\n", sangria));
            out.push_str(&formato_layout_arbol(compact, depth + 2));
            out.push_str(&format!("{}  [Medium]\n", sangria));
            out.push_str(&formato_layout_arbol(medium, depth + 2));
            out.push_str(&format!("{}  [Expanded]\n", sangria));
            out.push_str(&formato_layout_arbol(expanded, depth + 2));
            out
        }
        _ => format!("{}Layout({:?})\n", sangria, layout),
    }
}

/// Imprime el árbol de layouts en consola si el modo debug está activo
pub fn imprimir_layout_arbol(layout: &crate::gui_nativa::Layout) {
    if es_depuracion_enabled() {
        let arbol = formato_layout_arbol(layout, 0);
        log::debug!("[Forja GUI] Árbol de Layouts Extraído:\n{}", arbol);
        eprintln!("\n🌳 [Forja GUI] Árbol de Layouts Extraído:");
        eprintln!("{}", arbol);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_nativa::Layout;

    #[test]
    fn test_formato_layout_arbol_etiqueta() {
        let l = Layout::Label {
            texto: "Hola".to_string(),
            es_variable: false,
        };
        let formatted = formato_layout_arbol(&l, 0);
        assert!(formatted.contains("Label (\"Hola\")"));
    }

    #[test]
    fn test_formato_layout_arbol_column() {
        let l = Layout::Column {
            children: vec![Layout::Title("Titulo".to_string())],
            gap: 10.0,
            alignment: "center".to_string(),
        };
        let formatted = formato_layout_arbol(&l, 0);
        assert!(formatted.contains("Column"));
        assert!(formatted.contains("Title (\"Titulo\")"));
    }
}
