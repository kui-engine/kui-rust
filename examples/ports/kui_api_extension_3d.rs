// Copied by scripts/generate_go_rust_examples.py from examples/rust/ports/kui_api_extension_3d.rs; edit that file, not this copy.
//! API tour: dynamic extensions and a host-driven 3D scene (examples/api_tours/extension_3d).
//! The host builds a scene in <kui-3d-view> through Element::invoke_extension (meshes, lights,
//! materials, a glTF model, an orbit camera driven by mouse events), inspects the kui_ext_3d
//! extension (state, perf stats, watch), unloads and reloads it and reads every invoke result.
//! One #tour-log row per check; #tour-result ends as "PASS n/n". The panel buttons repeat the
//! calls by hand.

use crate::canonical_runner::Context;
use kui::{
    Document, ExtensionValue, InjectedMouse, Modifiers, MouseButton, MouseInjection, Platform,
    Point, TimerId,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::f64::consts::PI;
use std::path::PathBuf;
use std::rc::Rc;

const TAG_3D: &str = "kui-3d-view";

// ---- Self-check bookkeeping: count, log one <tr> per check, publish the verdict ----------
// check:     a documented behaviour; a failure makes the tour FAIL.
// known_bug: a documented behaviour the engine currently gets wrong (reported bug). A failure is
//            logged as "KNOWN BUG" and kept out of the verdict; once it passes it counts as ok.

struct Tour {
    doc: Document,
    total: Cell<u32>,
    failed: Cell<u32>,
    known: Cell<u32>,
    failed_names: RefCell<Vec<String>>,
}

impl Tour {
    fn new(doc: &Document) -> Self {
        Tour {
            doc: doc.clone(),
            total: Cell::new(0),
            failed: Cell::new(0),
            known: Cell::new(0),
            failed_names: RefCell::new(Vec::new()),
        }
    }

    fn row(&self, name: &str, result: &str, cell: &str) {
        let Ok(Some(log)) = self.doc.get_element_by_id("tour-log") else {
            return;
        };
        let Ok(row) = self.doc.create_element("tr") else {
            return;
        };
        let _ = row.set_attribute("data-check", name);
        let _ = row.set_attribute("data-result", result);
        if result != "ok" {
            let _ = row.class_list_add("bad");
        }
        for text in [name, cell] {
            if let Ok(td) = self.doc.create_element("td") {
                let _ = td.set_text_content(text);
                let _ = row.append_child(&td);
            }
        }
        let _ = log.append_child(&row);
    }

    fn check(&self, name: &str, ok: bool, detail: String) {
        self.total.set(self.total.get() + 1);
        if ok {
            self.row(name, "ok", "ok");
            return;
        }
        self.failed.set(self.failed.get() + 1);
        self.failed_names.borrow_mut().push(name.to_string());
        eprintln!("tour check failed: {name}: {detail}");
        self.row(name, "fail", &format!("FAIL: {detail}"));
    }

    fn known_bug(&self, name: &str, ok: bool, bug: &str, detail: String) {
        if ok {
            self.total.set(self.total.get() + 1);
            self.row(name, "ok", "ok");
            return;
        }
        self.known.set(self.known.get() + 1);
        eprintln!("tour known bug: {name}: {detail}");
        self.row(name, "known", &format!("KNOWN BUG: {bug} ({detail})"));
    }

    fn finish(&self) {
        let Ok(Some(result)) = self.doc.get_element_by_id("tour-result") else {
            return;
        };
        let (total, failed, known) = (self.total.get(), self.failed.get(), self.known.get());
        let mut text = if failed == 0 {
            format!("PASS {total}/{total}")
        } else {
            format!(
                "FAIL {failed}/{total}: {}",
                self.failed_names.borrow().join(", ")
            )
        };
        if known > 0 {
            text.push_str(&format!(" (known bugs: {known})"));
        }
        let _ = result.set_text_content(&text);
        let _ = result.set_attribute("data-total", &total.to_string());
        let _ = result.set_attribute("data-failed", &failed.to_string());
        let _ = result.set_attribute("data-known", &known.to_string());
        let _ = result.class_list_add(if failed == 0 { "pass" } else { "fail" });
    }
}

// ---- Host state: the document, paths next to the executable, camera and scene ------------

#[derive(Default, Clone, Copy)]
struct Scene {
    ground: i64,
    boxed: i64,
    sphere: i64,
    cylinder: i64,
    sun: i64,
    lamp: i64,
    tank: i64,
    transforms_ok: bool,
    materials_ok: bool,
    tank_transform_ok: bool,
}

struct Host {
    doc: Document,
    platform: Rc<Platform>,
    exe_dir: String,
    ext_path: String,
    tank_path: String,
    camera: Cell<(f64, f64, f64)>, // azimuth, elevation (degrees), distance
    scene: Cell<Scene>,
    color: Cell<usize>,
    spin: Cell<Option<TimerId>>,
    spin_angle: Cell<f64>,
    tank_on: Cell<bool>,
    drag: Cell<MouseButton>,
    last: Cell<(f32, f32)>,
}

const BOX_COLORS: [[f64; 3]; 4] = [
    [0.20, 0.55, 0.95],
    [0.95, 0.65, 0.15],
    [0.65, 0.30, 0.90],
    [0.95, 0.95, 0.95],
];

fn ext_file_name() -> &'static str {
    if cfg!(windows) {
        "kui_ext_3d.dll"
    } else if cfg!(target_os = "macos") {
        "libkui_ext_3d.dylib"
    } else {
        "libkui_ext_3d.so"
    }
}

fn nums(values: &[f64]) -> Vec<ExtensionValue> {
    values
        .iter()
        .map(|value| ExtensionValue::Number(*value))
        .collect()
}

fn with_entity(entity: i64, values: &[f64]) -> Vec<ExtensionValue> {
    let mut args = vec![ExtensionValue::Integer(entity)];
    args.extend(nums(values));
    args
}

fn describe(result: &Result<Vec<ExtensionValue>, kui::Error>) -> String {
    match result {
        Ok(values) => {
            let parts: Vec<String> = values
                .iter()
                .map(|value| match value {
                    ExtensionValue::Integer(number) => number.to_string(),
                    other => format!("{other:?}"),
                })
                .collect();
            format!("count={} values={}", values.len(), parts.join(","))
        }
        Err(error) => format!("error={error}"),
    }
}

impl Host {
    fn set_text(&self, id: &str, text: &str) {
        if let Ok(Some(element)) = self.doc.get_element_by_id(id) {
            let _ = element.set_text_content(text);
        }
    }

    // ---- Extension calls: invoke_extension returns the values, or an error carrying the
    // result's status and error text when the call itself failed. ---------------------------

    fn call(
        &self,
        method: &str,
        args: &[ExtensionValue],
        max: usize,
    ) -> Result<Vec<ExtensionValue>, kui::Error> {
        match self.doc.get_element_by_id("view")? {
            Some(view) => view.invoke_extension(method, args, max),
            None => Ok(Vec::new()),
        }
    }

    /// First result as an entity / asset handle; 0 when the call failed.
    fn call_int(&self, method: &str, args: &[ExtensionValue]) -> i64 {
        match self.call(method, args, 1).as_deref() {
            Ok([ExtensionValue::Integer(value), ..]) => *value,
            _ => 0,
        }
    }

    /// True when the call succeeded and its first result (if any) is true.
    fn call_ok(&self, method: &str, args: &[ExtensionValue]) -> bool {
        match self.call(method, args, 1).as_deref() {
            Ok([ExtensionValue::Boolean(value), ..]) => *value,
            Ok(_) => true,
            Err(_) => false,
        }
    }

    // ---- Orbit camera: azimuth / elevation / distance around the origin -> setCameraLookAt

    fn apply_camera(&self) -> bool {
        let (az, el, dist) = self.camera.get();
        let (az, el, dist) = (
            (az + 360.0) % 360.0,
            el.clamp(-10.0, 80.0),
            dist.clamp(3.0, 30.0),
        );
        self.camera.set((az, el, dist));
        let (a, e) = (az * PI / 180.0, el * PI / 180.0);
        let ok = self
            .call(
                "setCameraLookAt",
                &nums(&[
                    dist * e.cos() * a.sin(),
                    0.3 + dist * e.sin(),
                    dist * e.cos() * a.cos(),
                    0.0,
                    0.3,
                    0.0,
                ]),
                0,
            )
            .is_ok();
        self.set_text("cam-out", &format!("az={az:.0} el={el:.0} dist={dist:.1}"));
        ok
    }

    fn camera_step(&self, d_az: f64, d_el: f64, zoom: f64) {
        let (az, el, dist) = self.camera.get();
        self.camera.set((az + d_az, el + d_el, dist * zoom));
        self.apply_camera();
    }

    fn camera_reset(&self) {
        self.camera.set((35.0, 22.0, 8.0));
        self.apply_camera();
    }

    // ---- Scene: ground, three meshes, two lights, PBR materials and the glTF tank ---------

    /// setEntityTransform(entity, position, rotation quaternion about Y, uniform scale) -> bool.
    fn place(&self, entity: i64, x: f64, y: f64, z: f64, angle: f64, scale: f64) -> bool {
        let (s, c) = ((angle / 2.0).sin(), (angle / 2.0).cos());
        self.call_ok(
            "setEntityTransform",
            &with_entity(entity, &[x, y, z, 0.0, s, 0.0, c, scale, scale, scale]),
        )
    }

    /// createBox / createSphere / ...(r, g, b) -> entity, then addToScene and place it.
    fn add_mesh(
        &self,
        scene: &mut Scene,
        method: &str,
        rgb: [f64; 3],
        pos: [f64; 3],
        scale: f64,
    ) -> i64 {
        let entity = self.call_int(method, &nums(&rgb));
        if entity == 0 || !self.call_ok("addToScene", &[ExtensionValue::Integer(entity)]) {
            return 0;
        }
        if !self.place(entity, pos[0], pos[1], pos[2], 0.0, scale) {
            scene.transforms_ok = false;
        }
        entity
    }

    /// setMaterialProperties(entity, r, g, b, metallic, roughness) -> bool.
    fn set_material(&self, entity: i64, values: [f64; 5]) -> bool {
        self.call_ok("setMaterialProperties", &with_entity(entity, &values))
    }

    /// loadGltf(path) -> asset handle (also adds it to the scene); setAssetTransform places it.
    fn load_tank(&self, scene: &mut Scene) {
        scene.tank = self.call_int(
            "loadGltf",
            &[ExtensionValue::String(self.tank_path.clone())],
        );
        scene.tank_transform_ok = scene.tank != 0
            && self.call_ok(
                "setAssetTransform",
                &with_entity(
                    scene.tank,
                    &[0.0, -0.75, -2.6, 0.0, 0.0, 0.0, 1.0, 0.9, 0.9, 0.9],
                ),
            );
    }

    fn recolor_box(&self) {
        let c = BOX_COLORS[self.color.get()];
        self.set_material(self.scene.get().boxed, [c[0], c[1], c[2], 0.1, 0.45]);
    }

    fn build_scene(&self) {
        let mut scene = Scene {
            transforms_ok: true,
            ..Scene::default()
        };
        self.spin_angle.set(0.0);
        self.call_ok("setCameraProjection", &nums(&[55.0, 0.1, 300.0]));
        self.call_ok("setSkyboxColor", &nums(&[0.04, 0.07, 0.14]));
        // createDirectionalLight(r, g, b, intensity, dirX, dirY, dirZ);
        // createPointLight(r, g, b, intensity, x, y, z, range)
        scene.sun = self.call_int(
            "createDirectionalLight",
            &nums(&[1.0, 0.95, 0.88, 110000.0, -0.8, -1.0, -0.5]),
        );
        scene.lamp = self.call_int(
            "createPointLight",
            &nums(&[0.35, 0.6, 1.0, 60000.0, -2.5, 3.5, 2.0, 12.0]),
        );
        scene.ground = self.add_mesh(
            &mut scene,
            "createPlane",
            [0.20, 0.24, 0.30],
            [0.0, -0.75, 0.0],
            9.0,
        );
        scene.boxed = self.add_mesh(
            &mut scene,
            "createBox",
            [0.2, 0.55, 0.95],
            [0.0, 0.0, 0.0],
            1.4,
        );
        scene.sphere = self.add_mesh(
            &mut scene,
            "createSphere",
            [0.9, 0.25, 0.2],
            [-2.4, 0.0, 0.0],
            1.3,
        );
        scene.cylinder = self.add_mesh(
            &mut scene,
            "createCylinder",
            [0.25, 0.85, 0.45],
            [2.4, 0.0, 0.0],
            1.1,
        );
        scene.materials_ok = self.set_material(scene.sphere, [0.9, 0.25, 0.2, 0.9, 0.2])
            && self.set_material(scene.cylinder, [0.25, 0.85, 0.45, 0.0, 0.8]);
        if self.tank_on.get() {
            self.load_tank(&mut scene);
        }
        self.scene.set(scene);
        self.recolor_box();
        self.apply_camera();
    }

    /// A fresh <kui-3d-view id="view"> in #view-host.
    fn create_view(&self) {
        if let (Ok(Some(parent)), Ok(view)) = (
            self.doc.get_element_by_id("view-host"),
            self.doc.create_element(TAG_3D),
        ) {
            let _ = view.set_attribute("id", "view");
            let _ = parent.append_child(&view);
        }
    }

    /// Removes the 3D element before its extension goes away.
    fn remove_view(&self) {
        if let Ok(Some(view)) = self.doc.get_element_by_id("view") {
            let _ = view.remove();
        }
    }

    fn show_scene(&self) {
        let s = self.scene.get();
        let on_off = |on: bool| if on { "on" } else { "off" };
        self.set_text(
            "scene-out",
            &format!(
                "color={} spin={} tank={} entities={},{},{},{}",
                self.color.get(),
                on_off(self.spin.get().is_some()),
                on_off(s.tank != 0),
                s.ground,
                s.boxed,
                s.sphere,
                s.cylinder
            ),
        );
    }

    fn stop_spin(&self) {
        if let Some(id) = self.spin.take() {
            let _ = self.platform.clear_timer(id);
        }
    }

    fn ext_count(&self) -> usize {
        self.doc.extension_state().map_or(0, |(count, _)| count)
    }

    fn ext_loaded(&self) -> bool {
        self.doc.extension_is_loaded(TAG_3D).unwrap_or(false)
    }

    fn show_ext_state(&self, prefix: &str) {
        let (count, watching) = self.doc.extension_state().unwrap_or((0, false));
        self.set_text(
            "ext-out",
            &format!(
                "{prefix}loaded={} extensions={count} watching={}",
                self.ext_loaded() as i32,
                watching as i32
            ),
        );
    }

    fn show_perf(&self, prefix: &str) {
        let stats = self.doc.extension_perf_stats(TAG_3D).unwrap_or_default();
        self.set_text(
            "ext-out",
            &format!(
                "{prefix}ticks={} paints={} max_tick={:.2}ms max_paint={:.2}ms",
                stats.tick_count, stats.paint_count, stats.max_tick_ms, stats.max_paint_ms
            ),
        );
    }
}

// ---- Mouse orbit: listeners on #view-host (events bubble up from the 3D element) ----------
// Left drag orbits, right drag (vertical) zooms, every wheel event zooms in one step: the C ABI
// has no accessor for a DOM wheel event's delta, so the direction is not available.

fn listen_mouse(ctx: &Context<'_>, host: &Rc<Host>) -> Result<(), Box<dyn Error>> {
    let view_host = ctx
        .document
        .get_element_by_id("view-host")?
        .ok_or("page has no #view-host")?;
    let h = host.clone();
    view_host.add_event_listener("mousedown", move |event| {
        if let Ok(point) = event.mouse_client() {
            h.last.set((point.x, point.y));
            h.drag
                .set(event.mouse_button().unwrap_or(MouseButton::None));
        }
    })?;
    let h = host.clone();
    view_host.add_event_listener("mousemove", move |event| {
        let Ok(point) = event.mouse_client() else {
            return;
        };
        let (lx, ly) = h.last.get();
        match h.drag.get() {
            MouseButton::Left => h.camera_step(
                -((point.x - lx) as f64) * 0.5,
                ((point.y - ly) as f64) * 0.3,
                1.0,
            ),
            MouseButton::Right => h.camera_step(0.0, 0.0, 1.0 + ((point.y - ly) as f64) * 0.01),
            _ => return,
        }
        h.last.set((point.x, point.y));
    })?;
    for name in ["mouseup", "mouseleave"] {
        let h = host.clone();
        view_host.add_event_listener(name, move |_event| h.drag.set(MouseButton::None))?;
    }
    let h = host.clone();
    view_host.add_event_listener("wheel", move |_event| h.camera_step(0.0, 0.0, 0.9))?;
    Ok(())
}

// ---- Panel buttons: camera, scene, extension manager, invoke results ------------------------

fn on<F>(
    ctx: &Context<'_>,
    host: &Rc<Host>,
    element_id: &str,
    mut handler: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&Rc<Host>) + 'static,
{
    let h = host.clone();
    let element = ctx
        .document
        .get_element_by_id(element_id)?
        .ok_or(format!("page has no #{element_id}"))?;
    element.add_event_listener("click", move |_event| handler(&h))?;
    Ok(())
}

fn bind_buttons(ctx: &Context<'_>, host: &Rc<Host>) -> Result<(), Box<dyn Error>> {
    on(ctx, host, "btn-cam-left", |h| {
        h.camera_step(-15.0, 0.0, 1.0)
    })?;
    on(ctx, host, "btn-cam-right", |h| {
        h.camera_step(15.0, 0.0, 1.0)
    })?;
    on(ctx, host, "btn-cam-up", |h| h.camera_step(0.0, 10.0, 1.0))?;
    on(ctx, host, "btn-cam-down", |h| {
        h.camera_step(0.0, -10.0, 1.0)
    })?;
    on(ctx, host, "btn-cam-in", |h| h.camera_step(0.0, 0.0, 0.8))?;
    on(ctx, host, "btn-cam-out", |h| h.camera_step(0.0, 0.0, 1.25))?;
    on(ctx, host, "btn-cam-reset", |h| h.camera_reset())?;
    on(ctx, host, "btn-scene-color", |h| {
        h.color.set((h.color.get() + 1) % 4);
        h.recolor_box();
        h.show_scene();
    })?;
    on(ctx, host, "btn-scene-spin", |h| {
        if h.spin.get().is_some() {
            h.stop_spin();
        } else {
            let tick = h.clone();
            let id = h.platform.set_interval(33, move || {
                let angle = tick.spin_angle.get() + 0.06;
                tick.spin_angle.set(angle);
                let scene = tick.scene.get();
                tick.place(scene.boxed, 0.0, 0.0, 0.0, angle, 1.4);
                tick.place(scene.cylinder, 2.4, 0.0, 0.0, -angle, 1.1);
            });
            h.spin.set(id.ok());
        }
        h.show_scene();
    })?;
    on(ctx, host, "btn-scene-tank", |h| {
        let mut scene = h.scene.get();
        if scene.tank != 0 {
            h.call_ok("destroyAsset", &[ExtensionValue::Integer(scene.tank)]);
            scene.tank = 0;
            h.tank_on.set(false);
        } else {
            h.tank_on.set(true);
            h.load_tank(&mut scene);
        }
        h.scene.set(scene);
        h.show_scene();
    })?;
    on(ctx, host, "btn-ext-state", |h| h.show_ext_state(""))?;
    on(ctx, host, "btn-ext-perf", |h| h.show_perf(""))?;
    on(ctx, host, "btn-ext-reset", |h| {
        let _ = h.doc.reset_extension_perf_stats();
        h.show_perf("reset: ");
    })?;
    // Unload -> reload from the DLL path -> fresh element -> rebuilt scene.
    on(ctx, host, "btn-ext-reload", |h| {
        h.stop_spin();
        h.remove_view();
        let _ = h.doc.unload_extension(TAG_3D);
        let reloaded = h.doc.reload_extension(&h.ext_path).unwrap_or(false);
        h.create_view();
        h.build_scene();
        let state = if h.scene.get().boxed != 0 {
            "rebuilt"
        } else {
            "missing"
        };
        h.show_ext_state(&format!("reloaded={} box={state} | ", reloaded as i32));
    })?;
    on(ctx, host, "btn-ext-watch", |h| {
        let watching = h
            .doc
            .extension_state()
            .map_or(false, |(_, watching)| watching);
        let _ = h.doc.watch_extensions(if watching {
            None
        } else {
            Some(h.exe_dir.as_str())
        });
        h.show_ext_state("");
    })?;
    on(ctx, host, "btn-res-ok", |h| {
        h.set_text(
            "res-out",
            &format!("status=0 {}", describe(&h.call("getRenderStats", &[], 4))),
        )
    })?;
    on(ctx, host, "btn-res-unknown", |h| {
        h.set_text(
            "res-out",
            &format!(
                "status=3 count=0 {}",
                describe(&h.call("noSuchMethod", &[], 1))
            ),
        )
    })?;
    on(ctx, host, "btn-res-args", |h| {
        h.set_text(
            "res-out",
            &format!(
                "status=4 count=0 {}",
                describe(&h.call("addToScene", &[], 1))
            ),
        )
    })?;
    Ok(())
}

// ---- Self-check: the scene built through extension calls -----------------------------------

fn check_scene(t: &Tour, h: &Host, prefix: &str) {
    let s = h.scene.get();
    t.check(
        &format!("{prefix}.lights"),
        s.sun != 0 && s.lamp != 0,
        format!("sun {}, point light {}", s.sun, s.lamp),
    );
    t.check(
        &format!("{prefix}.meshes"),
        s.ground != 0
            && s.boxed != 0
            && s.sphere != 0
            && s.cylinder != 0
            && s.boxed != s.sphere
            && s.sphere != s.cylinder
            && s.ground != s.boxed,
        format!(
            "entities {} {} {} {}",
            s.ground, s.boxed, s.sphere, s.cylinder
        ),
    );
    t.check(
        &format!("{prefix}.transforms"),
        s.transforms_ok,
        "setEntityTransform returned false".into(),
    );
    t.check(
        &format!("{prefix}.materials"),
        s.materials_ok,
        "setMaterialProperties returned false".into(),
    );
    t.check(
        &format!("{prefix}.gltf"),
        s.tank != 0 && s.tank_transform_ok,
        format!(
            "loadGltf({}) -> {}, setAssetTransform {}",
            h.tank_path, s.tank, s.tank_transform_ok
        ),
    );
}

// ---- Self-check: invoke results (values / count, and the status + error of a failed call) --

fn check_results(t: &Tour, h: &Host) {
    let stats = h.call("getRenderStats", &[], 4);
    let all_int = stats.as_ref().map_or(false, |v| {
        v.iter().all(|x| matches!(x, ExtensionValue::Integer(_)))
    });
    t.check(
        "res.get_count",
        matches!(&stats, Ok(v) if v.len() == 4) && all_int,
        format!("getRenderStats: {}", describe(&stats)),
    );
    t.check(
        "res.get_error-empty",
        stats.is_ok(),
        format!("error on success: {}", describe(&stats)),
    );

    let unknown = h.call("noSuchMethod", &[], 1);
    t.check(
        "res.status-unknown-method",
        matches!(&unknown, Err(e) if e.status() == kui::sys::KUI_ERR_NOT_FOUND
            && e.to_string().contains("status 3") && e.to_string().contains("noSuchMethod")),
        describe(&unknown),
    );
    let bad_args = h.call("addToScene", &[], 1);
    t.check(
        "res.status-arg-count",
        matches!(&bad_args, Err(e) if e.to_string().contains("status 4") && e.to_string().contains("argument count")),
        describe(&bad_args),
    );
}

// ---- Self-check: the orbit camera, through the buttons' function and real mouse events -----

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

fn check_camera(t: &Tour, h: &Host) {
    t.check(
        "cam.set_camera_look_at",
        h.apply_camera(),
        "setCameraLookAt failed".into(),
    );
    h.camera_step(15.0, 0.0, 1.0);
    t.check(
        "cam.step",
        approx(h.camera.get().0, 50.0),
        format!("azimuth {:.2} after +15", h.camera.get().0),
    );

    let rect = h
        .doc
        .get_element_by_id("view-host")
        .ok()
        .flatten()
        .and_then(|e| e.bounding_rect().ok())
        .unwrap_or_default();
    let (cx, cy) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let inject = |kind, x: f32, y: f32, button, click_count, dy: f32| {
        let _ = h.doc.dispatch_mouse_event(&MouseInjection {
            kind,
            position: Point { x, y },
            button,
            modifiers: Modifiers::default(),
            click_count,
            delta: Point { x: 0.0, y: dy },
        });
    };

    // Left drag 40 px to the right: the listeners turn it into -20 degrees of azimuth.
    let before = h.camera.get().0;
    inject(InjectedMouse::Pressed, cx, cy, MouseButton::Left, 1, 0.0);
    inject(
        InjectedMouse::Moved,
        cx + 40.0,
        cy,
        MouseButton::Left,
        0,
        0.0,
    );
    inject(
        InjectedMouse::Released,
        cx + 40.0,
        cy,
        MouseButton::Left,
        1,
        0.0,
    );
    t.check(
        "cam.mouse-drag-orbit",
        approx(h.camera.get().0, before - 20.0) && h.drag.get() == MouseButton::None,
        format!(
            "azimuth {before:.2} -> {:.2}, drag {:?}",
            h.camera.get().0,
            h.drag.get()
        ),
    );

    // Right drag 20 px down zooms out by 20 %.
    let before = h.camera.get().2;
    inject(InjectedMouse::Pressed, cx, cy, MouseButton::Right, 1, 0.0);
    inject(
        InjectedMouse::Moved,
        cx,
        cy + 20.0,
        MouseButton::Right,
        0,
        0.0,
    );
    inject(
        InjectedMouse::Released,
        cx,
        cy + 20.0,
        MouseButton::Right,
        1,
        0.0,
    );
    t.check(
        "cam.mouse-drag-zoom",
        approx(h.camera.get().2, before * 1.2),
        format!("distance {before:.2} -> {:.2}", h.camera.get().2),
    );

    let before = h.camera.get().2;
    inject(InjectedMouse::Wheel, cx, cy, MouseButton::None, 0, -120.0);
    t.check(
        "cam.mouse-wheel",
        approx(h.camera.get().2, before * 0.9),
        format!("distance {before:.2} -> {:.2}", h.camera.get().2),
    );
    h.camera_reset();
}

// ---- Self-check: extension manager state, directory watching, unload / reload / load ------

/// Returns the initial number of loaded extensions.
fn check_manager(t: &Tour, h: &Host) -> usize {
    t.check(
        "ext.is_loaded",
        h.doc.extension_is_loaded(TAG_3D).unwrap_or(false),
        "kui-3d-view not loaded".into(),
    );
    let state = h.doc.extension_state();
    let initial = state.as_ref().map_or(0, |s| s.0);
    t.check(
        "ext.get_extension_state",
        matches!(state, Ok((count, false)) if count >= 1),
        format!("{state:?}"),
    );
    let on = h.doc.watch_extensions(Some(&h.exe_dir));
    let watching = h.doc.extension_state().map_or(false, |s| s.1);
    t.check(
        "ext.watch_extensions-on",
        on.is_ok() && watching,
        format!("{on:?} watching {watching}"),
    );
    let off = h.doc.watch_extensions(None);
    let watching = h.doc.extension_state().map_or(true, |s| s.1);
    t.check(
        "ext.watch_extensions-off",
        off.is_ok() && !watching,
        format!("{off:?} watching {watching}"),
    );
    let stats = h.doc.extension_perf_stats(TAG_3D);
    t.check(
        "perf.get_perf_stats",
        stats.is_ok(),
        format!("{:?}", stats.err()),
    );
    initial
}

/// Unload -> confirm -> reload -> load, before the page has any 3D element: the element is
/// created afterwards, on the extension that is loaded at the end. (The "Unload + reload 3D"
/// button does the same with a live view; the example's pixel check covers that second view.)
fn check_lifecycle(t: &Tour, h: &Host, initial: usize) {
    let unloaded = h.doc.unload_extension(TAG_3D);
    t.check(
        "ext.unload_extension",
        unloaded.is_ok() && !h.ext_loaded() && h.ext_count() + 1 == initial,
        format!(
            "{unloaded:?} loaded {} count {}",
            h.ext_loaded(),
            h.ext_count()
        ),
    );
    let again = h.doc.unload_extension(TAG_3D);
    t.check(
        "ext.unload_extension-again",
        matches!(&again, Err(e) if e.status() == kui::sys::KUI_ERR_NOT_FOUND),
        format!("{again:?}"),
    );
    let reloaded = h.doc.reload_extension(&h.ext_path);
    t.check(
        "ext.reload_extension",
        matches!(reloaded, Ok(true)) && h.ext_loaded() && h.ext_count() == initial,
        format!("{reloaded:?} count {}", h.ext_count()),
    );
    let missing = PathBuf::from(&h.exe_dir).join(format!("no_such_extension_{}", ext_file_name()));
    let loaded = h.doc.load_extension(&missing.to_string_lossy());
    t.check(
        "ext.load_extension-missing",
        matches!(loaded, Ok(false)) && h.ext_count() == initial,
        format!("{loaded:?} count {}", h.ext_count()),
    );

    // unload_all removes the custom element definitions of every extension it unloads (like
    // unload_extension does), so the DLLs can be loaded again below.
    let all = h.doc.unload_all_extensions();
    t.check(
        "ext.unload_all_extensions",
        all.is_ok() && h.ext_count() == 0 && !h.ext_loaded(),
        format!("{all:?} count {}", h.ext_count()),
    );
    let defined = h
        .doc
        .custom_element_is_defined("kui-editor")
        .unwrap_or(false);
    t.check(
        "ext.unload_all-undefines-elements",
        !defined,
        "<kui-editor> still defined".into(),
    );
    let loaded = h.doc.load_extension(&h.ext_path);
    t.check(
        "ext.load_extension",
        matches!(loaded, Ok(true)) && h.ext_count() == 1 && h.ext_loaded(),
        format!("{loaded:?} count {}", h.ext_count()),
    );
}

/// After a few frames: what the perf counters saw, then reset them; publish the verdict.
fn check_perf_and_finish(t: &Tour, h: &Host) {
    let stats = h.doc.extension_perf_stats(TAG_3D).unwrap_or_default();
    t.known_bug(
        "perf.paints-recorded",
        stats.paint_count > 0 || stats.tick_count > 0,
        "ExtensionManager never records tick / paint timings (recordTickPerf / recordPaintPerf have no caller)",
        format!("ticks {} paints {} after rendering", stats.tick_count, stats.paint_count),
    );
    let reset = h.doc.reset_extension_perf_stats();
    let after = h.doc.extension_perf_stats(TAG_3D);
    t.check(
        "perf.reset_perf_stats",
        reset.is_ok()
            && matches!(&after, Ok(s) if s.tick_count == 0 && s.paint_count == 0 && s.total_paint_ms == 0.0),
        format!("{reset:?} {after:?}"),
    );
    h.show_ext_state("self-check done | ");
    t.finish();
}

pub fn ready(ctx: &Context<'_>) -> Result<(), Box<dyn Error>> {
    let exe = std::env::current_exe()?;
    let exe_dir = exe
        .parent()
        .ok_or("executable has no directory")?
        .to_path_buf();
    let host = Rc::new(Host {
        doc: ctx.document.clone(),
        platform: ctx.platform.clone(),
        exe_dir: exe_dir.to_string_lossy().into_owned(),
        ext_path: exe_dir.join(ext_file_name()).to_string_lossy().into_owned(),
        tank_path: exe_dir
            .join("assets")
            .join("native_3d")
            .join("stylized_tank.glb")
            .to_string_lossy()
            .into_owned(),
        camera: Cell::new((35.0, 22.0, 8.0)),
        scene: Cell::new(Scene::default()),
        color: Cell::new(0),
        spin: Cell::new(None),
        spin_angle: Cell::new(0.0),
        tank_on: Cell::new(true),
        drag: Cell::new(MouseButton::None),
        last: Cell::new((0.0, 0.0)),
    });
    let tour = Rc::new(Tour::new(ctx.document));

    listen_mouse(ctx, &host)?;
    let initial = check_manager(&tour, &host);
    check_lifecycle(&tour, &host, initial);
    host.create_view();
    host.build_scene();
    check_scene(&tour, &host, "scene");
    check_results(&tour, &host);
    check_camera(&tour, &host);
    host.show_scene();
    {
        let (t, h) = (tour.clone(), host.clone());
        if let Err(error) = ctx
            .platform
            .set_timeout(400, move || check_perf_and_finish(&t, &h))
        {
            tour.check("perf.timer", false, error.to_string());
            tour.finish();
        }
    }
    bind_buttons(ctx, &host)
}
