// Medición del costo de armar un frame.
//
// `layout_a_view` reconstruye el árbol de widgets en cada cambio de estado (cada
// movimiento del slider, cada tecla, cada scroll). Si eso tarda más que el
// presupuesto del monitor (4,2 ms a 240 Hz, 16,7 ms a 60 Hz) la interacción se
// siente pegajosa.
//
// Se activa con `FORJA_GUI_PERF=1` y reporta cada `INTERVALO` reconstrucciones:
//
//     [perf] 120 frames: 2.4 ms/frame (extraer 0.9 + construir 1.5) | 148/s

use std::cell::RefCell;
use std::time::Duration;

/// Cada cuántas reconstrucciones se emite un reporte.
const INTERVALO: u32 = 30;

#[derive(Default)]
struct Stats {
    frames: u32,
    nodos: u32,
    extraer: Duration,
    construir: Duration,
    ultimo: Option<std::time::Instant>,
}

thread_local! {
    static STATS: RefCell<Stats> = RefCell::new(Stats::default());
}

/// `true` si se pidió medir (`FORJA_GUI_PERF=1`).
///
/// Se resuelve una sola vez: esta función se llama por widget en cada frame.
pub fn activo() -> bool {
    static ACTIVO: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ACTIVO.get_or_init(|| {
        std::env::var("FORJA_GUI_PERF")
            .map(|v| v != "0")
            .unwrap_or(false)
    })
}

/// Cuenta un widget construido en el frame actual (se llama desde `layout_a_view`).
pub fn contar_nodo() {
    // El `if` externo evita el `thread_local` cuando no se está midiendo.
    if activo() {
        STATS.with(|s| s.borrow_mut().nodos += 1);
    }
}

/// Acumula los tiempos de una reconstrucción y devuelve un reporte cada
/// `INTERVALO` frames (o `None` si todavía no toca).
pub fn registrar(extraer: Duration, construir: Duration) -> Option<String> {
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        s.frames += 1;
        s.extraer += extraer;
        s.construir += construir;

        if s.frames < INTERVALO {
            return None;
        }

        let n = s.frames as f64;
        let extraer_ms = s.extraer.as_secs_f64() * 1000.0 / n;
        let construir_ms = s.construir.as_secs_f64() * 1000.0 / n;
        let total_ms = extraer_ms + construir_ms;

        let por_segundo = match s.ultimo {
            Some(t) => {
                let secs = t.elapsed().as_secs_f64();
                if secs > 0.0 {
                    n / secs
                } else {
                    0.0
                }
            }
            None => 0.0,
        };

        let nodos = s.nodos as f64 / n;
        let reporte = format!(
            "[perf] {} frames: {:.2} ms/frame (extraer {:.2} + construir {:.2}) | {:.0}/s | {:.0} widgets por frame",
            s.frames, total_ms, extraer_ms, construir_ms, por_segundo, nodos
        );

        s.frames = 0;
        s.nodos = 0;
        s.extraer = Duration::ZERO;
        s.construir = Duration::ZERO;
        s.ultimo = Some(std::time::Instant::now());
        Some(reporte)
    })
}

// ─── Frames realmente pintados ───────────────────────────────────────
//
// El costo de armar el árbol no dice si la ventana va fluida: lo que importa es
// cada cuánto se pinta. El widget que envuelve la raíz (`WindowWidthProbe`) se
// pinta una vez por frame, así que sirve de contador.

thread_local! {
    static FRAMES: RefCell<(u32, Option<std::time::Instant>, f64)> =
        const { RefCell::new((0, None, 0.0)) };
}

/// Marca un frame pintado. Cada segundo devuelve el reporte:
/// `[perf] 143 fps (7.0 ms por frame)`.
pub fn marcar_frame() -> Option<String> {
    if !activo() {
        return None;
    }
    FRAMES.with(|f| {
        let mut f = f.borrow_mut();
        let ahora = std::time::Instant::now();
        let (ref mut frames, ref mut inicio, ref mut peor) = *f;
        *frames += 1;
        match *inicio {
            None => {
                *inicio = Some(ahora);
                None
            }
            Some(t0) => {
                let segs = ahora.duration_since(t0).as_secs_f64();
                if segs >= 1.0 {
                    let fps = *frames as f64 / segs;
                    let ms = segs * 1000.0 / *frames as f64;
                    let reporte = format!(
                        "[perf] {:.0} fps ({:.1} ms por frame, peor {:.1} ms)",
                        fps, ms, *peor
                    );
                    *frames = 0;
                    *inicio = Some(ahora);
                    *peor = 0.0;
                    Some(reporte)
                } else {
                    // guardar el intervalo más largo visto en este segundo
                    let intervalo = ahora.duration_since(t0).as_secs_f64() * 1000.0;
                    if intervalo > *peor {
                        *peor = intervalo;
                    }
                    None
                }
            }
        }
    })
}
