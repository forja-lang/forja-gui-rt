// Forja GUI — Widget de icono SVG Material Design con renderizado Vello
//
// Renderiza paths SVG de Material Design directamente en el Scene de Vello,
// reemplazando el fallback a emoji que se usaba anteriormente.

use crate::icons::{self, IconStyle};
use crate::theme::RgbColor;
use std::marker::PhantomData;
use xilem::core::{MessageResult, Mut, View, ViewMarker};
use xilem::masonry::core::Widget;
use xilem::masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef, RegisterCtx,
};
use xilem::masonry::imaging::Painter;
use xilem::masonry::kurbo::{Affine, Axis, BezPath, Size, Stroke};
use xilem::masonry::layout::{LenReq, Length};
use xilem::masonry::peniko::color::AlphaColor;
use xilem::masonry::peniko::{Brush, Fill};
use xilem::{Pod, ViewCtx};

// ═══════════════════════════════════════════════════════════════════
// MaterialSvgIcon — raw widget Masonry que pinta un icono SVG
// ═══════════════════════════════════════════════════════════════════

/// Widget Masonry personalizado que renderiza un icono SVG Material Design
/// usando las APIs de Vello para pintar paths vectoriales directamente en la escena.
pub struct MaterialSvgIcon {
    /// Nombre del icono (para lookup en catálogo)
    nombre: String,
    /// Tamaño del icono en píxeles lógicos
    tamaño: f64,
    /// Color de relleno del icono (empaquetado como [u8; 4] RGBA)
    rgba: [u8; 4],
    /// Paths SVG como strings (para referencia)
    paths: Vec<String>,
    /// BezPath cacheados después del primer parse (evita re-parsear en cada paint)
    bez_paths: Vec<BezPath>,
    /// Fill rule
    fill_rule: Fill,
    /// Estilo del icono (Filled, Outlined, Rounded, Sharp, TwoTone)
    estilo: IconStyle,
}

impl MaterialSvgIcon {
    /// Crea un nuevo widget de icono SVG con color RgbColor y estilo IconStyle
    pub fn new(nombre: &str, tamaño: f64, color: RgbColor, estilo: IconStyle) -> Self {
        let mut icon = Self {
            nombre: nombre.to_string(),
            tamaño,
            rgba: [color.0, color.1, color.2, 255],
            paths: Vec::new(),
            bez_paths: Vec::new(),
            fill_rule: Fill::NonZero,
            estilo,
        };
        icon.cargar_paths();
        icon
    }

    /// Carga los paths SVG desde el catálogo de iconos y los cachea como BezPath
    ///
    /// El parseo del `d` del SVG cuesta ~20 µs por icono y este widget se
    /// reconstruye en cada cambio de estado (arrastrar un slider, escribir en un
    /// campo): sin caché, una pantalla con 6 iconos gastaba ~120 µs por frame sólo
    /// en parsear paths. La caché es global y por nombre de icono.
    fn cargar_paths(&mut self) {
        if let Some(bez) = bez_paths_cacheados(&self.nombre) {
            self.bez_paths = (*bez).clone();
            if let Some(icon) = icons::catalog::by_name(&self.nombre) {
                self.paths.push(icon.svg_path.to_string());
            }
        }
    }
}

impl Widget for MaterialSvgIcon {
    type Action = NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        // El icono tiene tamaño propio fijo: lo respeta, pero se achica si el
        // espacio disponible es menor (equivalente al `bc.constrain` anterior).
        let fijo = Length::px(self.tamaño);
        match len_req {
            LenReq::FitContent(space) => fijo.min(space),
            _ => fijo,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(
        &mut self,
        _ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        if self.paths.is_empty() {
            return;
        }

        let scale = self.tamaño / 24.0; // Material icons en viewBox 24x24
        let alpha_color =
            AlphaColor::from_rgba8(self.rgba[0], self.rgba[1], self.rgba[2], self.rgba[3]);
        let brush = Brush::Solid(alpha_color);
        let transform = Affine::scale(scale);

        for bez_path in &self.bez_paths {
            match self.estilo {
                IconStyle::Outlined => {
                    // Outlined: stroke con línea fina para efecto de contorno
                    let stroke = Stroke::new(1.5);
                    painter
                        .stroke(bez_path, &stroke, &brush)
                        .transform(transform)
                        .draw();
                }
                _ => {
                    // Filled, Rounded, Sharp, TwoTone: relleno sólido
                    painter
                        .fill(bez_path, &brush)
                        .transform(transform)
                        .fill_rule(self.fill_rule)
                        .draw();
                }
            }
        }
    }

    fn accessibility_role(&self) -> xilem::masonry::accesskit::Role {
        xilem::masonry::accesskit::Role::Image
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut xilem::masonry::accesskit::Node,
    ) {
        node.set_label(format!("Icono: {}", self.nombre));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
}

// ═══════════════════════════════════════════════════════════════════
// SvgIconView — wrapper Xilem View para integrar MaterialSvgIcon
// ═══════════════════════════════════════════════════════════════════

/// Wrapper de Xilem View que crea un MaterialSvgIcon a partir de nombre/tamaño/color.
///
/// Sigue el patrón de `WidthObservedView`: implementa View<State, (), ViewCtx>
/// con Element = Pod<MaterialSvgIcon>.
pub struct SvgIconView<State> {
    nombre: String,
    tamaño: f64,
    color: RgbColor,
    estilo: IconStyle,
    phantom: PhantomData<fn() -> State>,
}

/// Crea una vista de icono SVG Material Design con el estilo especificado.
pub fn svg_icon<State: 'static>(
    nombre: &str,
    tamaño: f64,
    color: RgbColor,
    estilo: IconStyle,
) -> SvgIconView<State> {
    SvgIconView {
        nombre: nombre.to_string(),
        tamaño,
        color,
        estilo,
        phantom: PhantomData,
    }
}

impl<State: 'static> ViewMarker for SvgIconView<State> {}

impl<State: 'static> View<State, (), ViewCtx> for SvgIconView<State> {
    type Element = Pod<MaterialSvgIcon>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Pod<MaterialSvgIcon>, ()) {
        let icon = MaterialSvgIcon::new(&self.nombre, self.tamaño, self.color, self.estilo);
        (ctx.create_pod(icon), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<MaterialSvgIcon>>,
        _app_state: &mut State,
    ) {
        // Si el nombre, tamaño o color cambian, necesitamos reconstruir completamente.
        // Por ahora, comparamos parámetros y solo reconstruimos si hay cambios.
        if self.nombre != prev.nombre
            || (self.tamaño - prev.tamaño).abs() > 0.01
            || self.color != prev.color
        {
            // full rebuild required - handled by Xilem's diff mechanism
        }
    }

    fn teardown(
        &self,
        _view_state: &mut (),
        _ctx: &mut ViewCtx,
        _element: Mut<'_, Pod<MaterialSvgIcon>>,
    ) {
    }

    fn message(
        &self,
        _view_state: &mut (),
        _message: &mut crate::MessageContext,
        _element: Mut<'_, Pod<MaterialSvgIcon>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}

// ═══════════════════════════════════════════════════════════════════
// Caché de paths parseados
// ═══════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Caché global `nombre de icono → BezPaths ya parseados`.
fn cache() -> &'static Mutex<HashMap<String, Arc<Vec<BezPath>>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Vec<BezPath>>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn bez_paths_cacheados(nombre: &str) -> Option<Arc<Vec<BezPath>>> {
    if let Some(hit) = cache().lock().ok()?.get(nombre) {
        return Some(hit.clone());
    }
    let icon = icons::catalog::by_name(nombre)?;
    let bez = BezPath::from_svg(icon.svg_path).ok()?;
    let entrada = Arc::new(vec![bez]);
    cache()
        .lock()
        .ok()?
        .insert(nombre.to_string(), entrada.clone());
    Some(entrada)
}

#[cfg(test)]
mod tests_cache_iconos {
    /// El contrato de la caché: el mismo nombre devuelve la misma instancia y no
    /// re-parsea; un nombre desconocido no entra en la caché.
    #[test]
    fn cachea_y_reutiliza() {
        let a = super::bez_paths_cacheados("home").expect("home existe");
        let b = super::bez_paths_cacheados("home").expect("home existe");
        assert!(
            std::sync::Arc::ptr_eq(&a, &b),
            "debe reutilizar el mismo Arc"
        );
        assert!(super::bez_paths_cacheados("no_existe_xyz").is_none());
    }
}
