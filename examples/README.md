# Ejemplos standalone de `forja-gui-rt`

Esta carpeta contiene ejemplos que demuestran el uso de los crates de GUI de
Forja **como librerías Rust independientes**, sin necesidad de instalar el
compilador `forja`.

## Ejemplos

| Archivo | Descripción | Cómo ejecutar |
|---------|-------------|---------------|
| [`tema_standalone.rs`](tema_standalone.rs) | Consume `forja-theme` (Material You) desde Rust puro, sin xilem ni Forja. | `cargo run --example tema_standalone -p forja-theme` |
| [`app_minima.fa`](app_minima.fa) | App GUI mínima escrita en Forja, transpilada contra `forja-gui-rt`. | `forja run app_minima.fa` |

## Notas

- `tema_standalone.rs` valida que el subcrate [`forja-theme`](../../forja-theme)
  es autocontenido: no arrastra `xilem` ni `forja`.
- `app_minima.fa` requiere el compilador `forja` (o el runtime pre-compilado
  `forja-gui-rt`) para ejecutarse; se incluye como referencia de la API de alto
  nivel.
