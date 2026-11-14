//! Fachada del intérprete compartido (`forja-interprete`).
//!
//! El evaluador tree-walking ya no vive en la GUI: es un crate propio que usan
//! la GUI nativa y la web. Acá queda sólo el puente con el almacén reactivo del
//! host (`VariableAlmacen` de `gui-core`).

pub use forja_interprete::*;

use forja_interprete::AlmacenValores;

/// Adapta el almacén reactivo de `gui-core` al trait del intérprete.
///
/// El trait vive en `forja-interprete` y el tipo en `gui-core`, así que el impl
/// no puede ser directo (regla de orphan): este newtype presta el almacén.
pub struct AlmacenGui<'a>(pub &'a crate::signals::VariableAlmacen);

impl AlmacenValores for AlmacenGui<'_> {
    fn obtener(&self, nombre: &str) -> Option<serde_json::Value> {
        crate::signals::leer_variable(self.0, nombre, None)
    }

    fn fijar(&self, nombre: &str, valor: serde_json::Value) {
        crate::signals::escribir_variable(self.0, nombre, valor);
    }
}
