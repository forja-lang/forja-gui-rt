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
pub fn is_debug_enabled() -> bool {
    true // FORZADO PARA DEPURACIÓN INTEGRAL EN ANDROID
}

/// Imprime un mensaje formateado con prefijo si el modo debug está activo
pub fn debug_log(category: &str, message: &str) {
    if is_debug_enabled() {
        log::debug!("[Forja GUI::{}] {}", category, message);
        eprintln!("🔍 [Forja GUI::{}] {}", category, message);
    }
}

/// Registra el cambio de valor de una variable reactiva en el VariableStore
pub fn log_state_change(
    var_name: &str,
    old_val: Option<&serde_json::Value>,
    new_val: &serde_json::Value,
) {
    if is_debug_enabled() {
        match old_val {
            Some(old) => {
                log::info!(
                    "[GUI State] Variable '{}' cambió: {} ──→ {}",
                    var_name,
                    old,
                    new_val
                );
                eprintln!(
                    "⚡ [GUI State] Variable '{}' cambió: {} ──→ {}",
                    var_name, old, new_val
                );
            }
            None => {
                log::info!(
                    "[GUI State] Variable '{}' inicializada: {}",
                    var_name,
                    new_val
                );
                eprintln!(
                    "⚡ [GUI State] Variable '{}' inicializada: {}",
                    var_name, new_val
                );
            }
        }
    }
}

/// Registra el inicio de ejecución de un callback
pub fn log_callback_start(nombre_fn: &str) {
    if is_debug_enabled() {
        log::info!("[GUI Callback] Ejecutando callback '{}'...", nombre_fn);
        eprintln!("▶️ [GUI Callback] Ejecutando callback '{}'...", nombre_fn);
    }
}

/// Registra el éxito de un callback
pub fn log_callback_success(nombre_fn: &str, resultado: &str) {
    if is_debug_enabled() {
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
pub fn log_layout_warning(widget_name: &str, reason: &str) {
    log::warn!("[GUI Layout Warning] Widget '{}': {}", widget_name, reason);
    eprintln!(
        "⚠️ [GUI Layout Warning] Widget '{}': {}",
        widget_name, reason
    );
}

/// Convierte un Layout a una representación textual en árbol con sangría
pub fn format_layout_tree(layout: &crate::gui_nativa::Layout, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    use crate::gui_nativa::Layout::*;

    match layout {
        Column {
            children,
            gap,
            alignment,
        } => {
            let mut out = format!("{}Column (gap: {}, align: {})\n", indent, gap, alignment);
            for c in children {
                out.push_str(&format_layout_tree(c, depth + 1));
            }
            out
        }
        CenteredColumn(children) => {
            let mut out = format!("{}CenteredColumn\n", indent);
            for c in children {
                out.push_str(&format_layout_tree(c, depth + 1));
            }
            out
        }
        Row {
            children,
            gap,
            alignment,
        } => {
            let mut out = format!("{}Row (gap: {}, align: {})\n", indent, gap, alignment);
            for c in children {
                out.push_str(&format_layout_tree(c, depth + 1));
            }
            out
        }
        ZStack(children) => {
            let mut out = format!("{}ZStack\n", indent);
            for c in children {
                out.push_str(&format_layout_tree(c, depth + 1));
            }
            out
        }
        Container { child, max_width } => {
            format!(
                "{}Container (max_width: {})\n{}",
                indent,
                max_width,
                format_layout_tree(child, depth + 1)
            )
        }
        Portal(child) => {
            format!("{}Portal\n{}", indent, format_layout_tree(child, depth + 1))
        }
        Label { texto, es_variable } => {
            if *es_variable {
                format!("{}Label (var: '{}')\n", indent, texto)
            } else {
                format!("{}Label (\"{}\")\n", indent, texto)
            }
        }
        VariableLabel { variable } => {
            format!("{}VariableLabel ('{}')\n", indent, variable)
        }
        Title(texto) => {
            format!("{}Title (\"{}\")\n", indent, texto)
        }
        ColoredLabel { texto, color } => {
            format!("{}ColoredLabel (\"{}\", color: {})\n", indent, texto, color)
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
                indent, texto, callback, variant, disabled
            )
        }
        TextInput {
            variable,
            multiline,
            placeholder,
        } => {
            format!(
                "{}TextInput (var: '{}', multiline: {}, placeholder: \"{}\")\n",
                indent, variable, multiline, placeholder
            )
        }
        ProgressBar { variable } => {
            format!("{}ProgressBar (var: '{}')\n", indent, variable)
        }
        Slider { variable, min, max } => {
            format!(
                "{}Slider (var: '{}', min: {}, max: {})\n",
                indent, variable, min, max
            )
        }
        Checkbox { variable } => {
            format!("{}Checkbox (var: '{}')\n", indent, variable)
        }
        Prose(texto) => {
            format!("{}Prose (\"{}\")\n", indent, texto)
        }
        Spinner => format!("{}Spinner\n", indent),
        Separator => format!("{}Separator\n", indent),
        Spacer(size) => format!("{}Spacer ({})\n", indent, size),
        Expanded { child } => format!(
            "{}Expanded\n{}",
            indent,
            format_layout_tree(child, depth + 1)
        ),
        Centered { child } => format!(
            "{}Centered\n{}",
            indent,
            format_layout_tree(child, depth + 1)
        ),
        Padding { child, amount } => format!(
            "{}Padding ({})\n{}",
            indent,
            amount,
            format_layout_tree(child, depth + 1)
        ),
        ResponsiveLayout {
            compact,
            medium,
            expanded,
        } => {
            let mut out = format!("{}ResponsiveLayout\n", indent);
            out.push_str(&format!("{}  [Compact]\n", indent));
            out.push_str(&format_layout_tree(compact, depth + 2));
            out.push_str(&format!("{}  [Medium]\n", indent));
            out.push_str(&format_layout_tree(medium, depth + 2));
            out.push_str(&format!("{}  [Expanded]\n", indent));
            out.push_str(&format_layout_tree(expanded, depth + 2));
            out
        }
        _ => format!("{}Layout({:?})\n", indent, layout),
    }
}

/// Imprime el árbol de layouts en consola si el modo debug está activo
pub fn print_layout_tree(layout: &crate::gui_nativa::Layout) {
    if is_debug_enabled() {
        let tree = format_layout_tree(layout, 0);
        log::debug!("[Forja GUI] Árbol de Layouts Extraído:\n{}", tree);
        eprintln!("\n🌳 [Forja GUI] Árbol de Layouts Extraído:");
        eprintln!("{}", tree);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_nativa::Layout;

    #[test]
    fn test_format_layout_tree_label() {
        let l = Layout::Label {
            texto: "Hola".to_string(),
            es_variable: false,
        };
        let formatted = format_layout_tree(&l, 0);
        assert!(formatted.contains("Label (\"Hola\")"));
    }

    #[test]
    fn test_format_layout_tree_column() {
        let l = Layout::Column {
            children: vec![Layout::Title("Titulo".to_string())],
            gap: 10.0,
            alignment: "center".to_string(),
        };
        let formatted = format_layout_tree(&l, 0);
        assert!(formatted.contains("Column"));
        assert!(formatted.contains("Title (\"Titulo\")"));
    }
}
