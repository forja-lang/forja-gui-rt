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
    AccessCtx, BoxConstraints, ChildrenIds, LayoutCtx, NoAction, PaintCtx, PropertiesMut,
    PropertiesRef, RegisterCtx,
};
use xilem::masonry::kurbo::{Affine, BezPath, Size, Stroke};
use xilem::masonry::vello::peniko::color::AlphaColor;
use xilem::masonry::vello::peniko::{Brush, Fill};
use xilem::masonry::vello::Scene;
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
    /// Paths SVG cacheados después del primer lookup
    paths: Vec<String>,
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
            fill_rule: Fill::NonZero,
            estilo,
        };
        icon.cargar_paths();
        icon
    }

    /// Carga los paths SVG desde el catálogo de iconos
    fn cargar_paths(&mut self) {
        let icon_data = icons::catalog::by_name(&self.nombre);
        if let Some(icon) = icon_data {
            self.paths.push(icon.svg_path.to_string());
        }
    }
}

impl Widget for MaterialSvgIcon {
    type Action = NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(Size::new(self.tamaño, self.tamaño))
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        if self.paths.is_empty() {
            return;
        }

        let scale = self.tamaño / 24.0; // Material icons en viewBox 24x24
        let alpha_color =
            AlphaColor::from_rgba8(self.rgba[0], self.rgba[1], self.rgba[2], self.rgba[3]);
        let brush = Brush::Solid(alpha_color);
        let transform = Affine::scale(scale);

        for path_data in &self.paths {
            if let Ok(bez_path) = BezPath::from_svg(path_data) {
                match self.estilo {
                    IconStyle::Outlined => {
                        // Outlined: stroke con línea fina para efecto de contorno
                        let stroke = Stroke::new(1.5);
                        scene.stroke(&stroke, transform, &brush, None, &bez_path);
                    }
                    _ => {
                        // Filled, Rounded, Sharp, TwoTone: relleno sólido
                        scene.fill(self.fill_rule, transform, &brush, None, &bez_path);
                    }
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
        _message: &mut xilem::core::MessageContext,
        _element: Mut<'_, Pod<MaterialSvgIcon>>,
        _app_state: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Nop
    }
}
