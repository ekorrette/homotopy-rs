use homotopy_gl::{
    draw,
    frame::{DepthTest, Frame},
    GlCtx, Result,
};
use ultraviolet::{Mat4, Vec4};

use self::{
    axes::Axes,
    gbuffer::GBuffer,
    quad::Quad,
    scene::{Component, Scene},
    shaders::Shaders,
};
use super::{buffers, orbit_camera::OrbitCamera, DiagramGlProps};
use crate::{app::AppSettings, model::proof::Signature};

mod axes;
mod gbuffer;
mod quad;
mod scene;
mod shaders;

pub struct Renderer {
    // outside world
    ctx: GlCtx,
    signature: Signature,
    // state
    animated_3d: bool,
    cubical_subdivision: bool,
    smooth_time: bool,
    subdivision_depth: u8,
    geometry_samples: u8,
    // resources
    shaders: Shaders,
    scene: Scene,
    axes: Axes,
    quad: Quad,
    // pipeline state
    gbuffer: GBuffer,
    cylinder_buffer: GBuffer,
}

impl Renderer {
    pub fn new(ctx: GlCtx, props: &DiagramGlProps) -> Result<Self> {
        let animated_3d = AppSettings::get_animated_3d();
        let cubical_subdivision = AppSettings::get_cubical_subdivision();
        let smooth_time = AppSettings::get_smooth_time();
        let subdivision_depth = AppSettings::get_subdivision_depth() as u8;
        let samples = AppSettings::get_geometry_samples() as u8;
        let signature = props.signature.clone();

        Ok(Self {
            scene: Scene::new(
                &ctx,
                &props.diagram,
                props.view,
                animated_3d,
                cubical_subdivision,
                smooth_time,
                subdivision_depth,
                samples,
                &signature,
            )?,
            shaders: Shaders::new(&ctx)?,
            axes: Axes::new(&ctx)?,
            quad: Quad::new(&ctx)?,
            gbuffer: GBuffer::new(&ctx)?,
            cylinder_buffer: GBuffer::new(&ctx)?,
            ctx,
            signature,
            animated_3d,
            cubical_subdivision,
            smooth_time,
            subdivision_depth,
            geometry_samples: samples,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        let animated_3d = AppSettings::get_animated_3d();
        let cubical_subdivision = AppSettings::get_cubical_subdivision();
        let smooth_time = AppSettings::get_smooth_time();
        let subdivision_depth = AppSettings::get_subdivision_depth() as u8;
        let samples = AppSettings::get_geometry_samples() as u8;
        let pixel_ratio = if AppSettings::get_dpr_scale() {
            web_sys::window().unwrap().device_pixel_ratio()
        } else {
            1.
        };

        self.ctx.set_pixel_ratio(pixel_ratio)?;

        if self.animated_3d != animated_3d
            || self.cubical_subdivision != cubical_subdivision
            || self.smooth_time != smooth_time
            || self.subdivision_depth != subdivision_depth
            || self.geometry_samples != samples
        {
            self.animated_3d = animated_3d;
            self.cubical_subdivision = cubical_subdivision;
            self.smooth_time = smooth_time;
            self.subdivision_depth = subdivision_depth;
            self.geometry_samples = samples;
            self.scene.reload_meshes(
                &self.ctx,
                animated_3d,
                cubical_subdivision,
                smooth_time,
                subdivision_depth,
                samples,
                &self.signature,
            )?;
        }

        Ok(())
    }

    pub fn render(&mut self, camera: &OrbitCamera, t: f32) {
        let n = self.scene.view.dimension();
        let animated = n == 4 || n == 3 && self.animated_3d;

        let geometry_scale = AppSettings::get_geometry_scale() as f32 / 10.;

        let v = camera.view_transform(&self.ctx);
        let p = camera.perspective_transform(&self.ctx);

        let program = if animated {
            &self.shaders.geometry_4d
        } else {
            &self.shaders.geometry_3d
        };

        // Render animated wireframes to cylinder buffer
        if animated {
            let mut frame = Frame::new(&mut self.ctx)
                .with_frame_buffer(&self.cylinder_buffer.framebuffer)
                .with_clear_color(Vec4::new(0., 0., 0., 0.));

            if !AppSettings::get_mesh_hidden() {
                for Component {
                    vertices, albedo, ..
                } in &self.scene.cylinder_components
                {
                    frame.draw(draw!(program, vertices, &[], {
                        mv: v,
                        p: p,
                        albedo: *albedo,
                        t: t,
                    }));
                }
            }
        }

        // Render surfaces to GBuffer and cylindrify anything in the cylinder buffer
        {
            let mut frame = Frame::new(&mut self.ctx)
                .with_frame_buffer(&self.gbuffer.framebuffer)
                .with_clear_color(Vec4::new(0., 0., 0., 0.));

            if !AppSettings::get_mesh_hidden() {
                for Component {
                    vertices, albedo, ..
                } in &self.scene.components
                {
                    // Set color lightening amount based on how generator is viewed.
                    frame.draw(draw!(program, vertices, &[], {
                        mv: v,
                        p: p,
                        albedo: *albedo,
                        t: t,
                    }));
                }

                if animated {
                    let duration = self.scene.diagram.size().unwrap() as f32;

                    for animation_curve in &self.scene.animation_curves {
                        if let (Some(position), Some(vertex_shape)) =
                            (animation_curve.at(t), animation_curve.vertex_shape.as_ref())
                        {
                            frame.draw(draw!(&self.shaders.geometry_3d, vertex_shape, &[], {
                                mv: v * Mat4::from_translation(position.xyz()) * Mat4::from_scale(geometry_scale),
                                p: p,
                                albedo: animation_curve.albedo,
                                t: t,
                            }));
                        }
                    }

                    if AppSettings::get_animate_singularities() {
                        let radius = AppSettings::get_singularity_duration() as f32 / 10.;

                        for singularity in &self.scene.animation_singularities {
                            if let Some(vertex_shape) = &singularity.vertex_shape {
                                let point = singularity.vertices;
                                let dt = duration * (point.w - t).abs();
                                if dt > radius {
                                    continue;
                                }

                                let scale = geometry_scale * 1.4 * f32::sqrt(1. - dt / radius);
                                frame.draw(draw!(&self.shaders.geometry_3d, vertex_shape, &[], {
                                    mv: v * Mat4::from_translation(point.xyz()) * Mat4::from_scale(scale),
                                    p: p,
                                    albedo: singularity.albedo,
                                    t: t,
                                }));
                            }
                        }
                    }

                    frame.draw(draw! {
                        &self.shaders.cylinder_pass,
                        &self.quad.array,
                        &[
                            &self.cylinder_buffer.positions,
                            &self.cylinder_buffer.albedo,
                        ],
                        {
                            in_position: 0,
                            in_albedo: 1,
                            p: p,
                        }
                    });
                }
            }
        }

        // Final pass
        {
            // Apply lighting to scene
            let mut frame = Frame::new(&mut self.ctx).with_clear_color(Vec4::broadcast(1.));
            frame.draw(draw! {
                &self.shaders.lighting_pass,
                &self.quad.array,
                &[
                    &self.gbuffer.positions,
                    &self.gbuffer.normals,
                    &self.gbuffer.albedo,
                ],
                {
                    g_position: 0,
                    g_normal: 1,
                    g_albedo: 2,
                    disable_lighting: AppSettings::get_disable_lighting(),
                    debug_normals: AppSettings::get_debug_normals(),
                    spec: 1e-2 * AppSettings::get_specularity() as f32,
                    alpha: AppSettings::get_shininess() as f32,
                    gamma: 0.1 * AppSettings::get_gamma() as f32,
                    camera_pos: camera.position(),
                }
            });

            // Add in relevant wireframes
            if AppSettings::get_wireframe_3d() {
                for array in &self.scene.wireframe_components {
                    frame.draw(draw! {
                        &self.shaders.wireframe,
                        array,
                        &[],
                        DepthTest::Disable,
                        {
                            mv: v,
                            p: p,
                        }
                    });
                }
            }

            // Render axes
            if AppSettings::get_debug_axes() {
                frame.draw(draw! {
                    &self.shaders.wireframe,
                    &self.axes.array,
                    &[],
                    DepthTest::Disable,
                    {
                        mv: v,
                        p: p,
                    }
                });
            }
        }
    }
}
