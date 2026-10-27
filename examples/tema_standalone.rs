// Ejemplo standalone: uso de `forja-theme` como crate independiente.
//
// Este ejemplo NO depende de Forja ni de xilem: demuestra que el sistema de
// tema Material You puede consumirse desde cualquier proyecto Rust.
//
// Ejecutar con:
//     cargo run --example tema_standalone -p forja-theme
//
// (El ejemplo vive en forja-gui-rt pero solo usa forja-theme, que es una
//  dependencia directa; así se valida el consumo real del subcrate.)

use forja_theme::theme::{MaterialTheme, RgbColor};

fn main() {
    // Tema claro a partir de un color semilla (hex).
    let claro = MaterialTheme::from_seed("#6750A4", false);
    println!("Tema claro (semilla #6750A4):");
    println!("  primary        = {}", claro.scheme.primary);
    println!("  surface        = {}", claro.scheme.surface);
    println!("  on_surface     = {}", claro.scheme.on_surface);
    println!("  ¿oscuro?       = {}", claro.is_dark);

    // Tema oscuro a partir de un nombre de color.
    let oscuro = MaterialTheme::from_seed("violeta", true);
    println!("\nTema oscuro (semilla \"violeta\"):");
    println!("  primary        = {}", oscuro.scheme.primary);
    println!("  ¿oscuro?       = {}", oscuro.is_dark);

    // Tipografía: consultar un estilo del TypeScale.
    let estilo = oscuro.typography.apply("headlineLarge");
    println!(
        "\nheadlineLarge: {}px, peso {:?}",
        estilo.font_size, estilo.weight
    );

    // Formas: radio de esquina por familia.
    let radio = oscuro
        .shapes
        .for_family(forja_theme::theme::ShapeFamily::Fab);
    println!("Radio de esquina (FAB): {}px", radio);

    // Conversión de color independiente de xilem.
    let c = RgbColor::from_hex("#FF5722").expect("hex válido");
    println!("\n#FF5722 -> {}", c);

    // Alternar claro/oscuro (devuelve un tema nuevo).
    let t = claro.toggle_dark_mode();
    println!("\nTras toggle_dark_mode(), ¿oscuro? = {}", t.is_dark);
}
