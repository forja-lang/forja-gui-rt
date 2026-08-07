// Material Design 3 — Inputs: TextField, PasswordField, NumberField,
// SearchField, Dropdown, Select, Autocomplete, RadioGroup, Switch,
// SliderDiscrete, SliderRange, ChipGroup
// Funciones de rendering extraídas de gui_nativa.rs

use crate::view;
use crate::helpers::*;
use crate::gui_nativa::{AppStateNativo, TextFieldVariant};
use crate::{AnyWidgetView, Background, FontWeight, MaterialTheme};
use crate::Length;
use forja::ast::Declaracion;
use xilem::style::Style;

// ─── MaterialTextField ───────────────────────────────────────────

pub(crate) fn render_text_field(
    variable: &str,
    label: &str,
    placeholder: &str,
    variant: &TextFieldVariant,
    error: &str,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = data.leer(variable).to_string();
    let label_text = label.to_string();
    let placeholder_text = placeholder.to_string();
    let err_text = error.to_string();

    // Label flotante
    let label_color: crate::Color = if !err_text.is_empty() {
        scheme.error.into()
    } else {
        scheme.on_surface_variant.into()
    };
    let label_widget = if label_text.is_empty() {
        None
    } else {
        Some(
            view::label(label_text.clone())
                .text_size(12.0)
                .color(label_color),
        )
    };

    // Campo de texto
    let mut ti =
        view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
            data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
        })
        .text_color(scheme.on_surface.into());
    if !placeholder_text.is_empty() {
        ti = ti.placeholder(placeholder_text.as_str());
    }

    // Aplicar colores según variante
    let input_widget = match variant {
        TextFieldVariant::Filled => {
            let bg: crate::Color = scheme.surface_variant.into();
            Box::new(
                view::sized_box(ti)
                    .background(Background::Color(bg))
                    .corner_radius(4.0),
            ) as Box<AnyWidgetView<AppStateNativo>>
        }
        TextFieldVariant::Outlined => {
            let border: crate::Color = if !err_text.is_empty() {
                scheme.error.into()
            } else {
                scheme.outline.into()
            };
            Box::new(
                view::sized_box(ti)
                    .border_color(border)
                    .border_width(1.0)
                    .corner_radius(4.0),
            ) as Box<AnyWidgetView<AppStateNativo>>
        }
    };

    // Error text
    let children: Vec<Box<AnyWidgetView<AppStateNativo>>> = if !err_text.is_empty() {
        let err_color: crate::Color = scheme.error.into();
        let err_label = view::label(err_text.clone())
            .text_size(11.0)
            .color(err_color);
        match label_widget {
            Some(lw) => vec![
                Box::new(lw) as Box<AnyWidgetView<AppStateNativo>>,
                input_widget,
                Box::new(err_label) as Box<AnyWidgetView<AppStateNativo>>,
            ],
            None => vec![
                input_widget,
                Box::new(err_label) as Box<AnyWidgetView<AppStateNativo>>,
            ],
        }
    } else {
        match label_widget {
            Some(lw) => vec![
                Box::new(lw) as Box<AnyWidgetView<AppStateNativo>>,
                input_widget,
            ],
            None => vec![input_widget],
        }
    };

    Box::new(view::flex(crate::view::Axis::Vertical, (children,)).gap(Length::px(4.0)))
}

// ─── MaterialPasswordField ───────────────────────────────────────

pub(crate) fn render_password_field(
    variable: &str,
    label: &str,
    _data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = _data.leer(variable).to_string();
    let label_text = label.to_string();

    let label_color: crate::Color = scheme.on_surface_variant.into();
    let bg: crate::Color = scheme.surface_variant.into();

    let ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
    })
    .placeholder("••••••••")
    .text_color(scheme.on_surface.into());

    let input_widget = Box::new(
        view::sized_box(ti)
            .background(Background::Color(bg))
            .corner_radius(4.0),
    ) as Box<AnyWidgetView<AppStateNativo>>;

    let children: Vec<Box<AnyWidgetView<AppStateNativo>>> = if label_text.is_empty() {
        vec![input_widget]
    } else {
        vec![
            Box::new(
                view::label(label_text.clone())
                    .text_size(12.0)
                    .color(label_color),
            ) as Box<AnyWidgetView<AppStateNativo>>,
            input_widget,
        ]
    };

    Box::new(view::flex(crate::view::Axis::Vertical, (children,)).gap(Length::px(4.0)))
}

// ─── MaterialNumberField ─────────────────────────────────────────

pub(crate) fn render_number_field(
    variable: &str,
    label: &str,
    min: &f64,
    max: &f64,
    _data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = _data.leer(variable).to_string();
    let label_text = label.to_string();
    let mn = *min;
    let mx = *max;

    let label_color: crate::Color = scheme.on_surface_variant.into();
    let border: crate::Color = scheme.outline.into();

    let range_text = format!("{}-{}", mn, mx);
    let ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        if new_val.parse::<f64>().is_ok()
            || new_val.is_empty()
            || new_val == "-"
            || new_val == "."
        {
            data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
        }
    })
    .placeholder(range_text.as_str());

    let input_widget: Box<AnyWidgetView<AppStateNativo>> = if label_text.is_empty() {
        Box::new(
            view::sized_box(ti)
                .border_color(border)
                .border_width(1.0)
                .corner_radius(4.0),
        )
    } else {
        Box::new(
            view::flex(
                crate::view::Axis::Vertical,
                (
                    view::label(label_text.clone())
                        .text_size(12.0)
                        .color(label_color),
                    view::sized_box(ti)
                        .border_color(border)
                        .border_width(1.0)
                        .corner_radius(4.0),
                ),
            )
            .gap(Length::px(4.0)),
        )
    };

    input_widget
}

// ─── MaterialSearchField ─────────────────────────────────────────

pub(crate) fn render_search_field(
    variable: &str,
    placeholder: &str,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = data.leer(variable).to_string();
    let ph = placeholder.to_string();

    let bg: crate::Color = scheme.surface_variant.into();
    let ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
    })
    .placeholder(ph.as_str());

    Box::new(
        view::flex(
            crate::view::Axis::Horizontal,
            (
                view::label("🔍 ")
                    .text_size(16.0)
                    .color(scheme.on_surface_variant.into()),
                view::sized_box(ti)
                    .background(Background::Color(bg))
                    .corner_radius(20.0),
            ),
        )
        .gap(Length::px(4.0)),
    )
}

// ─── MaterialDropdown ────────────────────────────────────────────

pub(crate) fn render_dropdown(
    opciones: &[String],
    seleccionada: &usize,
    placeholder: &str,
    _data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let opts: Vec<String> = opciones.to_vec();
    let sel = *seleccionada;
    let ph = placeholder.to_string();

    let display_text = opts.get(sel).cloned().unwrap_or(ph);
    let fg: crate::Color = scheme.on_surface.into();
    let border: crate::Color = scheme.outline.into();
    let bg: crate::Color = scheme.surface_variant.into();

    let cb_btn = move |_data: &mut AppStateNativo| {
        // No hacemos nada en el placeholder de dropdown (ciclo)
    };

    Box::new(
        view::button(view::label(display_text).text_size(14.0).color(fg), cb_btn)
            .background(Background::Color(bg))
            .border_color(border)
            .border_width(1.0)
            .corner_radius(4.0),
    )
}

// ─── MaterialSelect ──────────────────────────────────────────────

pub(crate) fn render_select(
    opciones: &[String],
    seleccionada: &usize,
    label: &str,
    _data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let opts: Vec<String> = opciones.to_vec();
    let sel = *seleccionada;
    let label_text = label.to_string();

    let display_text = opts
        .get(sel)
        .cloned()
        .unwrap_or_else(|| "Seleccionar...".to_string());
    let fg: crate::Color = scheme.on_surface.into();
    let border: crate::Color = scheme.outline.into();
    let label_color: crate::Color = scheme.on_surface_variant.into();

    Box::new(
        view::flex(
            crate::view::Axis::Vertical,
            (
                view::label(label_text.clone())
                    .text_size(12.0)
                    .color(label_color),
                view::button(
                    view::label(display_text).text_size(14.0).color(fg),
                    move |data: &mut AppStateNativo| {
                        let _ = data;
                    },
                )
                .border_color(border)
                .border_width(1.0)
                .corner_radius(4.0),
            ),
        )
        .gap(Length::px(4.0)),
    )
}

// ─── MaterialAutocomplete ────────────────────────────────────────

pub(crate) fn render_autocomplete(
    variable: &str,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = data.leer(variable).to_string();
    let border: crate::Color = scheme.outline.into();

    let ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
    })
    .placeholder("Escribir...");

    Box::new(
        view::sized_box(ti)
            .border_color(border)
            .border_width(1.0)
            .corner_radius(4.0),
    )
}

// ─── MaterialRadioGroup ──────────────────────────────────────────

pub(crate) fn render_radio_group(
    opciones: &[String],
    seleccion: &usize,
    callback: &str,
    direction: &str,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let cb = callback.to_string();
    let prog = prog.to_vec();
    let opts: Vec<String> = opciones.to_vec();
    let sel = *seleccion;

    let mut radios: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, opcion) in opts.iter().enumerate() {
        let cb_inner = cb.clone();
        let t = opcion.clone();
        let prog_inner = prog.clone();
        let is_selected = i == sel;

        let fg: crate::Color = if is_selected {
            scheme.primary.into()
        } else {
            scheme.on_surface_variant.into()
        };
        let radio_color: crate::Color = if is_selected {
            scheme.primary.into()
        } else {
            scheme.outline.into()
        };

        let radio_widget = view::flex(
            crate::view::Axis::Horizontal,
            (
                view::sized_box(
                    view::label(if is_selected { "◉" } else { "○" }.to_string())
                        .text_size(20.0)
                        .color(radio_color),
                )
                .width(Length::px(24.0))
                .height(Length::px(24.0)),
                view::label(t.clone()).text_size(14.0).color(fg),
            ),
        )
        .gap(Length::px(8.0));

        let btn = view::button(radio_widget, move |data: &mut AppStateNativo| {
            ejecutar_callback_y_actualizar(&cb_inner, data, &prog_inner);
        });

        radios.push(Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>);
    }

    let ax = if direction == "horizontal" {
        crate::view::Axis::Horizontal
    } else {
        crate::view::Axis::Vertical
    };
    Box::new(view::flex(ax, (radios,)).gap(Length::px(8.0)))
}

// ─── MaterialSwitch ──────────────────────────────────────────────

pub(crate) fn render_switch(
    label: &str,
    variable: &str,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let lbl = label.to_string();
    let checked = data.leer(variable).to_bool();

    let track_color: crate::Color = if checked {
        scheme.primary.into()
    } else {
        scheme.surface_variant.into()
    };

    let checkbox = view::checkbox(
        lbl.clone(),
        checked,
        move |data: &mut AppStateNativo, new_checked: bool| {
            data.escribir(&var_name, crate::ValorGUI::Booleano(new_checked));
        },
    );

    Box::new(
        view::sized_box(checkbox)
            .background(Background::Color(track_color))
            .corner_radius(12.0),
    )
}

// ─── MaterialSliderDiscrete ──────────────────────────────────────

pub(crate) fn render_slider_discrete(
    variable: &str,
    min: &f64,
    max: &f64,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = data.leer(variable).to_f64();
    let mn = *min;
    let mx = *max;

    let slider = view::slider(
        mn,
        mx,
        val,
        move |data: &mut AppStateNativo, new_val: f64| {
            data.escribir(&var_name, crate::ValorGUI::Decimal(new_val));
        },
    );

    let display_val = format!("{:.1}", val);
    let fg: crate::Color = scheme.on_surface.into();

    Box::new(
        view::flex(
            crate::view::Axis::Vertical,
            (view::label(display_val).text_size(12.0).color(fg), slider),
        )
        .gap(Length::px(4.0)),
    )
}

// ─── MaterialSliderRange ─────────────────────────────────────────

pub(crate) fn render_slider_range(
    variable_inicio: &str,
    variable_fin: &str,
    min: &f64,
    max: &f64,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var1 = variable_inicio.to_string();
    let var2 = variable_fin.to_string();
    let val1 = data.leer(variable_inicio).to_f64();
    let val2 = data.leer(variable_fin).to_f64();
    let mn = *min;
    let mx = *max;

    let slider1 = view::slider(
        mn, mx, val1,
        move |data: &mut AppStateNativo, new_val: f64| {
            data.escribir(&var1, crate::ValorGUI::Decimal(new_val));
        },
    );
    let slider2 = view::slider(
        mn, mx, val2,
        move |data: &mut AppStateNativo, new_val: f64| {
            data.escribir(&var2, crate::ValorGUI::Decimal(new_val));
        },
    );

    let fg: crate::Color = scheme.on_surface.into();

    Box::new(
        view::flex(
            crate::view::Axis::Vertical,
            (
                view::label(format!("Inicio: {:.1}", val1))
                    .text_size(11.0)
                    .color(fg),
                slider1,
                view::label(format!("Fin: {:.1}", val2))
                    .text_size(11.0)
                    .color(fg),
                slider2,
            ),
        )
        .gap(Length::px(4.0)),
    )
}

// ─── MaterialChipGroup ───────────────────────────────────────────

pub(crate) fn render_chip_group(
    chips: &[String],
    seleccion: &[bool],
    callback: &str,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let cb = callback.to_string();
    let prog = prog.to_vec();
    let chip_texts: Vec<String> = chips.to_vec();
    let sels: Vec<bool> = seleccion.to_vec();

    let mut chip_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, chip_text) in chip_texts.iter().enumerate() {
        let cb_inner = cb.clone();
        let t = chip_text.clone();
        let prog_inner = prog.clone();
        let is_selected = sels.get(i).copied().unwrap_or(false);

        if is_selected {
            let fg: crate::Color = scheme.on_secondary_container.into();
            let bg: crate::Color = scheme.secondary_container.into();
            let label = view::label(t.clone())
                .text_size(12.0)
                .weight(FontWeight::MEDIUM)
                .color(fg);
            let btn = view::button(label, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb_inner, data, &prog_inner);
            });
            chip_widgets.push(Box::new(
                btn.background(Background::Color(bg)).corner_radius(8.0),
            ) as Box<AnyWidgetView<AppStateNativo>>);
        } else {
            let fg: crate::Color = scheme.on_surface.into();
            let border: crate::Color = scheme.outline.into();
            let label = view::label(t.clone())
                .text_size(12.0)
                .weight(FontWeight::MEDIUM)
                .color(fg);
            let btn = view::button(label, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb_inner, data, &prog_inner);
            });
            chip_widgets.push(Box::new(
                btn.border_color(border)
                    .border_width(1.0)
                    .corner_radius(8.0),
            ) as Box<AnyWidgetView<AppStateNativo>>);
        }
    }

    Box::new(view::flex(crate::view::Axis::Horizontal, (chip_widgets,)).gap(Length::px(8.0)))
}
