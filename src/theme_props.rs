// Propiedades por defecto de Masonry derivadas del tema de Forja.
//
// Los widgets nativos de Xilem (Button, Checkbox, Switch, TextInput, Slider,
// ProgressBar, Label...) se pintan con el `DefaultProperties` global, que trae
// una paleta oscura fija. Sin esto, en modo claro los campos y switches salían
// negros aunque el resto de la app fuera clara.
//
// Se parte del set de Masonry y se sobreescriben sólo los colores y formas, así
// los widgets que no tocamos siguen funcionando igual.

use crate::layout::AsUnit as _;
use crate::theme::{MaterialTheme, RgbColor, ShapeFamily};
use xilem::masonry::core::{DefaultProperties, PropertyStack, Selector};
use xilem::masonry::properties::{
    Background, BorderColor, BorderWidth, CaretColor, CheckmarkColor, ContentColor, CornerRadius,
    PlaceholderColor, SelectionColor, ThumbColor, TrackColor,
};
use xilem::masonry::theme::default_property_set;
use xilem::masonry::widgets::{
    Button, Checkbox, Label, ProgressBar, RadioButton, Slider, Switch, TextArea, TextInput,
};
use xilem::Color;

/// Color del texto principal, con el borde de campo y el acento del tema.
fn paleta(theme: &MaterialTheme) -> (Color, Color, Color, Color, Color) {
    let s = &theme.scheme;
    (
        s.on_surface.into(),         // texto principal
        s.on_surface_variant.into(), // texto secundario
        s.outline.into(),            // bordes
        s.primary.into(),            // acento
        s.surface_variant.into(),    // rellenos suaves / pistas
    )
}

/// `DefaultProperties` de Masonry con los colores y formas del tema.
pub fn default_properties(theme: &MaterialTheme) -> DefaultProperties {
    let mut props = default_property_set();
    let (texto, texto_suave, borde, acento, relleno) = paleta(theme);
    let s = &theme.scheme;
    let superficie: Color = s.surface.into();
    let radio_control = theme.shapes.for_family(ShapeFamily::TextField);

    // Label y texto en general
    props.insert::<Label, _>(ContentColor::new(texto));
    props.insert::<TextArea<false>, _>(ContentColor::new(texto));
    props.insert::<TextArea<true>, _>(ContentColor::new(texto));

    // Campos de texto
    props.insert::<TextInput, _>(Background::Color(superficie));
    props.insert::<TextInput, _>(BorderColor { color: borde });
    props.insert::<TextInput, _>(BorderWidth { width: 1.0.px() });
    props.insert::<TextInput, _>(CornerRadius {
        radius: radio_control.px(),
    });
    props.insert::<TextInput, _>(PlaceholderColor::new(texto_suave));
    props.insert::<TextInput, _>(CaretColor { color: acento });
    props.insert::<TextInput, _>(SelectionColor {
        color: s.primary_container.into(),
    });

    // Botones: superficie suave + hairline, como Fluent/Liquid Glass esperan.
    props.insert::<Button, _>(Background::Color(relleno));
    props.insert::<Button, _>(BorderColor { color: borde });
    props.insert::<Button, _>(BorderWidth { width: 1.0.px() });
    props.insert::<Button, _>(CornerRadius {
        radius: theme.shapes.for_family(ShapeFamily::Button).px(),
    });

    // Estados de interacción: Fluent aclara el fondo al pasar el mouse y lo
    // oscurece al presionar (antes los controles no daban ninguna señal).
    let claro = RgbColor(255, 255, 255);
    let mut stack_boton = PropertyStack::new();
    stack_boton.push_layer(
        Selector::new().with_hovered(true),
        Background::Color(claro.with_alpha(if theme.is_dark { 0.09 } else { 0.6 })),
    );
    stack_boton.push_layer(
        Selector::new().with_active(true),
        Background::Color(claro.with_alpha(if theme.is_dark { 0.04 } else { 0.4 })),
    );
    stack_boton.push_layer(
        Selector::new().with_focused(true),
        BorderColor { color: acento },
    );
    props.insert_stack::<Button>(stack_boton);

    // Checkbox
    props.insert::<Checkbox, _>(Background::Color(superficie));
    props.insert::<Checkbox, _>(BorderColor { color: borde });
    props.insert::<Checkbox, _>(CheckmarkColor { color: acento });
    props.insert::<Checkbox, _>(CornerRadius {
        radius: radio_control.px(),
    });

    // Foco en campos de texto: subrayado de acento (Fluent pinta 2px abajo).
    let mut stack_campo = PropertyStack::new();
    stack_campo.push_layer(
        Selector::new().with_focused(true),
        BorderColor { color: acento },
    );
    stack_campo.push_layer(
        Selector::new().with_focused(true),
        BorderWidth { width: 2.0.px() },
    );
    props.insert_stack::<TextInput>(stack_campo);

    // RadioButton
    props.insert::<RadioButton, _>(Background::Color(superficie));
    props.insert::<RadioButton, _>(BorderColor { color: borde });
    props.insert::<RadioButton, _>(CheckmarkColor { color: acento });

    // Switch: riel apagado = relleno suave; encendido = color de acento.
    // (`#toggled` es la clase que pone Masonry cuando está activo.)
    props.insert::<Switch, _>(Background::Color(relleno));
    props.insert::<Switch, _>(BorderColor { color: borde });
    props.insert::<Switch, _>(ThumbColor(Color::WHITE));
    let mut stack_switch = PropertyStack::new();
    stack_switch.push_layer(
        Selector::classes(&["#toggled"]),
        Background::Color(acento),
    );
    stack_switch.push_layer(
        Selector::classes(&["#toggled"]),
        BorderColor { color: acento },
    );
    props.insert_stack::<Switch>(stack_switch);

    // Slider
    props.insert::<Slider, _>(TrackColor {
        active: acento,
        inactive: relleno,
    });
    props.insert::<Slider, _>(ThumbColor(acento));
    // El slider nativo dibuja un marco de 1px alrededor de toda su caja; con el
    // riel ya visible no hace falta.
    props.insert::<Slider, _>(BorderWidth { width: 0.0.px() });
    props.insert::<Checkbox, _>(BorderWidth { width: 1.0.px() });

    // ProgressBar (la barra llena usa el acento del tema)
    props.insert::<ProgressBar, _>(Background::Color(relleno));
    props.insert::<ProgressBar, _>(BorderColor {
        color: Color::TRANSPARENT,
    });
    props.insert::<ProgressBar, _>(xilem::masonry::properties::BarColor(acento));

    props
}

/// Color de un rol del tema para usar en widgets de Forja (`texto_color`,
/// `ColoredLabel`). Devuelve `None` si el nombre no es un rol del esquema.
pub(crate) fn color_de_rol(theme: &MaterialTheme, nombre: &str) -> Option<RgbColor> {
    crate::helpers::color_de_rol(&theme.scheme, nombre)
}
