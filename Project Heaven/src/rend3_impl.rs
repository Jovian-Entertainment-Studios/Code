use std::sync::Arc;

mod gltf_loading;
mod mesh_generator;
use gltf_loading::load_gltf;
use mesh_generator::create_mesh;

struct RenderingData {
    _object_handle: rend3::types::ObjectHandle,
    material_handle: rend3::types::MaterialHandle,
    _directional_handle: rend3::types::DirectionalLightHandle,

    egui_routine: rend3_egui::EguiRenderRoutine,
    platform: egui_winit_platform::Platform,
    start_time: instant::Instant,
    color: [f32; 4],
}

const SAMPLE_COUNT: rend3::types::SampleCount = rend3::types::SampleCount::One;

#[derive(Default)]
pub struct Rendering {
    data: Option<RenderingData>,
    menu_toggle: bool,
    gltf_cube_toggle: bool,
}
impl rend3_framework::App for Rendering {
    const HANDEDNESS: rend3::types::Handedness = rend3::types::Handedness::Left;

    fn sample_count(&self) -> rend3::types::SampleCount {
        SAMPLE_COUNT
    }

    fn setup(
        &mut self,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        _routines: &Arc<rend3_framework::DefaultRoutines>,
        surface_format: rend3::types::TextureFormat,
    ) {
        let window_size = window.inner_size();

        // Create the egui render routine
        let egui_routine = rend3_egui::EguiRenderRoutine::new(
            renderer,
            surface_format,
            rend3::types::SampleCount::One,
            window_size.width,
            window_size.height,
            window.scale_factor() as f32,
        );

        // Create mesh and calculate smooth normals based on vertices
        let mesh = create_mesh();

        // Add mesh to renderer's world.
        //
        // All handles are refcounted, so we only need to hang onto the handle until we make an object.
        let mesh_handle = renderer.add_mesh(mesh);

        // Add PBR material with all defaults except a single color.
        let material = rend3_routine::material::PbrMaterial {
            albedo: rend3_routine::material::AlbedoComponent::Value(glam::Vec4::new(
                0.0, 0.5, 0.5, 1.0,
            )),
            ..rend3_routine::material::PbrMaterial::default()
        };
        let material_handle = renderer.add_material(material);

        // Combine the mesh and the material with a location to give an object.
        let object = rend3::types::Object {
            mesh: mesh_handle,
            material: material_handle.clone(),
            transform: glam::Mat4::IDENTITY,
        };

        // Creating an object will hold onto both the mesh and the material
        // even if they are deleted.
        //
        // We need to keep the object handle alive.
        let _object_handle = renderer.add_object(object);

        let camera_pitch = std::f32::consts::FRAC_PI_4;
        let camera_yaw = -std::f32::consts::FRAC_PI_4;
        // These values may seem arbitrary, but they center the camera on the cube in the scene
        let camera_location = glam::Vec3A::new(5.0, 7.5, -5.0);
        let view = glam::Mat4::from_euler(glam::EulerRot::XYZ, -camera_pitch, -camera_yaw, 0.0);
        let view = view * glam::Mat4::from_translation((-camera_location).into());

        // Set camera location data
        renderer.set_camera_data(rend3::types::Camera {
            projection: rend3::types::CameraProjection::Perspective {
                vfov: 60.0,
                near: 0.1,
            },
            view,
        });

        // Create a single directional light
        //
        // We need to keep the directional light handle alive.
        let _directional_handle = renderer.add_directional_light(rend3::types::DirectionalLight {
            color: glam::Vec3::ONE,
            intensity: 10.0,
            // Direction will be normalized
            direction: glam::Vec3::new(-1.0, -4.0, 2.0),
            distance: 400.0,
        });

        // Create the winit/egui integration, which manages our egui context for us.
        let platform =
            egui_winit_platform::Platform::new(egui_winit_platform::PlatformDescriptor {
                physical_width: window_size.width as u32,
                physical_height: window_size.height as u32,
                scale_factor: window.scale_factor(),
                font_definitions: egui::FontDefinitions::default(),
                style: Default::default(),
            });

        let start_time = instant::Instant::now();
        let color: [f32; 4] = [0.0, 0.5, 0.5, 1.0];

        self.data = Some(RenderingData {
            _object_handle,
            material_handle,
            _directional_handle,

            egui_routine,
            platform,
            start_time,
            color,
        })
    }

    fn handle_event(
        &mut self,
        window: &winit::window::Window,
        renderer: &Arc<rend3::Renderer>,
        routines: &Arc<rend3_framework::DefaultRoutines>,
        surface: Option<&Arc<rend3::types::Surface>>,
        event: rend3_framework::Event<'_, ()>,
        control_flow: impl FnOnce(winit::event_loop::ControlFlow),
    ) {
        let data = self.data.as_mut().unwrap();

        // Pass the winit events to the platform integration.
        data.platform.handle_event(&event);

        match event {
            rend3_framework::Event::RedrawRequested(..) => {
                data.platform
                    .update_time(data.start_time.elapsed().as_secs_f64());
                data.platform.begin_frame();

                // Insert egui commands here
                let ctx = data.platform.context();

                egui::TopBottomPanel::top("Taskbar").show(&ctx, |ui| {
                    if ui.add(egui::Button::new("Menu")).clicked() {
                        self.menu_toggle = !self.menu_toggle;
                    }
                    if self.menu_toggle == true {
                        egui::Window::new("Change color")
                            .resizable(false)
                            .anchor(egui::Align2::LEFT_TOP, [3.0, 30.0])
                            .show(&ctx, |ui| {
                                if ui.add(egui::Button::new("GLTF/Cube")).clicked() {
                                    self.gltf_cube_toggle = !self.gltf_cube_toggle;
                                }
                                ui.label("Change the color of the cube");
                                if ui
                                    .color_edit_button_rgba_unmultiplied(&mut data.color)
                                    .changed()
                                {
                                    renderer.update_material(
                                        &data.material_handle.clone(),
                                        rend3_routine::material::PbrMaterial {
                                            albedo: rend3_routine::material::AlbedoComponent::Value(
                                                glam::Vec4::from(data.color),
                                            ),
                                            ..rend3_routine::material::PbrMaterial::default()
                                        },
                                    );
                                }
                            });
                    }
                });

                // End the UI frame. Now let's draw the UI with our Backend, we could also handle the output here
                let (_output, paint_commands) = data.platform.end_frame(Some(window));
                let paint_jobs = data.platform.context().tessellate(paint_commands);

                let input = rend3_egui::Input {
                    clipped_meshes: &paint_jobs,
                    context: data.platform.context(),
                };

                // Get a frame
                let frame = rend3::util::output::OutputFrame::Surface {
                    surface: Arc::clone(surface.unwrap()),
                };

                // Ready up the renderer
                let (cmd_bufs, ready) = renderer.ready();

                // Lock the routines
                let pbr_routine = rend3_framework::lock(&routines.pbr);
                let tonemapping_routine = rend3_framework::lock(&routines.tonemapping);

                // Build a rendergraph
                let mut graph = rend3::RenderGraph::new();

                // Add the default rendergraph without a skybox
                rend3_routine::add_default_rendergraph(
                    &mut graph,
                    &ready,
                    &pbr_routine,
                    None,
                    &tonemapping_routine,
                    SAMPLE_COUNT,
                );

                // Add egui on top of all the other passes
                let surface = graph.add_surface_texture();
                data.egui_routine.add_to_graph(&mut graph, input, surface);

                // Dispatch a render using the built up rendergraph!
                graph.execute(renderer, frame, cmd_bufs, &ready);

                control_flow(winit::event_loop::ControlFlow::Poll);
            }
            rend3_framework::Event::MainEventsCleared => {
                window.request_redraw();
            }
            rend3_framework::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::Resized(size) => {
                    data.egui_routine
                        .resize(size.width, size.height, window.scale_factor() as f32);
                }
                winit::event::WindowEvent::CloseRequested => {
                    control_flow(winit::event_loop::ControlFlow::Exit);
                }
                _ => {}
            },
            _ => {}
        }
    }
}
